use std::{
    fmt::{Debug, Display},
    future::Future,
    hash::Hash,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use common::{MyOtherServiceClient, MyServiceClient};
use egui::{DragValue, Ui};
use framework::{tarpc::client::RpcError, ClientFramework};
use poll_promise::Promise;
use egui_shortcuts::SimpleSpawner;

#[derive(Clone)]
struct Connection {
    frame: ClientFramework,
    client: MyServiceClient,
}

pub struct PackRatApp {
    sess: Promise<Result<Connection>>,
    a: u32,
    b: u32,
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
            let newclient = MyServiceClient::new(Default::default(), channel);
            tokio::spawn(newclient.dispatch);
            let client = newclient.client;

            egui_ctx.request_repaint();

            Ok(Connection { frame, client })
        });

        Self {
            sess,
            a: 420,
            b: 69,
        }
    }
}

impl eframe::App for PackRatApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            connection_status(ui, &self.sess);

            if let Some(Ok(sess)) = self.sess.ready_mut() {
                // Adding
                ui.add(DragValue::new(&mut self.a).prefix("a: "));
                ui.add(DragValue::new(&mut self.b).prefix("b: "));

                let spawner = SimpleSpawner::new(ui.next_auto_id());

                if ui.button("Add").clicked() {
                    let ctx = framework::tarpc::context::current();
                    let client_clone = sess.client.clone();
                    let a = self.a;
                    let b = self.b;

                    spawner.spawn(ui, async move { client_clone.add(ctx, a, b).await });
                }

                spawner.show(ui, |ui, result| {
                    match result {
                        Ok(val) => ui.label(format!("Add result: {val}")),
                        Err(e) => ui.label(format!("Error: {e:?}")),
                    };
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
