use std::{
    fmt::{Debug, Display},
    future::Future,
    hash::Hash,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use common::PackRatFrontendClient;
use egui::{DragValue, Grid, Ui};
use egui_shortcuts::SimpleSpawner;
use framework::{tarpc::client::RpcError, ClientFramework};
use poll_promise::Promise;

#[derive(Clone)]
struct Connection {
    frame: ClientFramework,
    client: PackRatFrontendClient,
}

pub struct PackRatApp {
    sess: Promise<Result<Connection>>,
}

impl PackRatApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let egui_ctx = cc.egui_ctx.clone();

        let sess = Promise::spawn_async(async move {
            // Get framework and channel
            let url = url::Url::parse("https://127.0.0.1:9090/")?;
            let sess = quic_session::client_session(&url, common::CERTIFICATE.to_vec()).await?;
            let (frame, channel) = ClientFramework::new(sess).await?;

            // Get root client
            let newclient = PackRatFrontendClient::new(Default::default(), channel);
            tokio::spawn(newclient.dispatch);
            let client = newclient.client;

            egui_ctx.request_repaint();

            Ok(Connection { frame, client })
        });

        Self {
            sess,
        }
    }
}

impl eframe::App for PackRatApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            connection_status(ui, &self.sess);

            if let Some(Ok(sess)) = self.sess.ready_mut() {
                let spawner = SimpleSpawner::new(ui.next_auto_id());

                if ui.button("Get workers").clicked() {
                    let ctx = framework::tarpc::context::current();
                    let client_clone = sess.client.clone();

                    spawner.spawn(ui, async move { client_clone.get_workers(ctx).await });
                }

                spawner.show(ui, |ui, result| {
                    show_result(
                        ui,
                        result.as_ref().map_err(|e| format!("{:#?}", e)),
                        |ui, workers| {
                            Grid::new(ui.next_auto_id()).show(ui, |ui| {
                                for (worker_name, summary) in workers {
                                    ui.label(worker_name);
                                    ui.label(format!("{:?}", summary));
                                    ui.end_row();
                                }
                            });
                        },
                    );
                });
            }
        });
    }
}

fn connection_status<T: Send, E: Debug + Send>(ui: &mut Ui, prom: &Promise<Result<T, E>>) {
    match prom.ready() {
        None => ui.label("Connecting"),
        Some(Ok(_)) => ui.label("Connection open"),
        Some(Err(e)) => ui.label(format!("Error: {e:?}")),
    };
}

fn show_result<F: FnOnce(&mut Ui, T), T, E: Display>(ui: &mut Ui, result: Result<T, E>, f: F) {
    match result {
        Ok(val) => f(ui, val),
        Err(e) => {
            ui.label(format!("Error: {e:}"));
        }
    }
}
