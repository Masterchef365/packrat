use std::{
    fmt::{Debug, Display},
    future::Future,
};

use anyhow::Result;
use common::{MyOtherServiceClient, MyServiceClient};
use egui::{DragValue, Ui};
use framework::{tarpc::client::RpcError, ClientFramework};
use poll_promise::Promise;

#[derive(Clone)]
struct Connection {
    frame: ClientFramework,
    client: MyServiceClient,
}

pub struct TemplateApp {
    sess: Promise<Result<Connection>>,
    other_client: Option<Promise<Result<MyOtherServiceClient>>>,
    a: u32,
    b: u32,
    add_result: Option<Promise<Result<u32, RpcError>>>,
    subtract_result: Option<Promise<Result<u32, RpcError>>>,
}

impl TemplateApp {
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
            add_result: None,
            subtract_result: None,
            other_client: None,
        }
    }
}

fn connection_status<T: Send, E: Debug + Send>(ui: &mut Ui, prom: &Promise<Result<T, E>>) {
    match prom.ready() {
        None => ui.label("Connecting"),
        Some(Ok(_)) => ui.label("Connection open"),
        Some(Err(e)) => ui.label(format!("Error: {e:?}")),
    };
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            connection_status(ui, &self.sess);

            if let Some(Ok(sess)) = self.sess.ready_mut() {
                // Adding
                ui.add(DragValue::new(&mut self.a).prefix("a: "));
                ui.add(DragValue::new(&mut self.b).prefix("b: "));

                if ui.button("Add").clicked() {
                    let ctx = framework::tarpc::context::current();
                    let client_clone = sess.client.clone();
                    let a = self.a;
                    let b = self.b;

                    self.add_result = Some(Promise::spawn_async(async move {
                        client_clone.add(ctx, a, b).await
                    }));
                }

                if let Some(result) = self.add_result.as_ref().and_then(|res| res.ready()) {
                    match result {
                        Ok(val) => ui.label(format!("Add Result: {val}")),
                        Err(e) => ui.label(format!("Error: {e:?}")),
                    };
                }

                ui.strong("Subtraction");

                if let Some(prom) = self.other_client.as_mut() {
                    connection_status(ui, prom);

                    if let Some(Ok(other_client)) = prom.ready_mut() {
                        // Subtracting
                        if ui.button("Subtract").clicked() {
                            let ctx = framework::tarpc::context::current();
                            let client_clone = other_client.clone();
                            let a = self.a;
                            let b = self.b;

                            self.subtract_result = Some(Promise::spawn_async(async move {
                                client_clone.subtract(ctx, a, b).await
                            }));
                        }

                        if let Some(result) =
                            self.subtract_result.as_ref().and_then(|res| res.ready())
                        {
                            match result {
                                Ok(val) => ui.label(format!("Subtract result: {val}")),
                                Err(e) => ui.label(format!("Error: {e:?}")),
                            };
                        }
                    }
                } else {
                    if ui.button("Connect to subtractor").clicked() {
                        let sess = sess.clone();
                        self.other_client = Some(Promise::spawn_async(async move {
                            // Call a method on that client, yielding another service!
                            let ctx = framework::tarpc::context::current();
                            let subservice = sess.client.get_sub(ctx).await?;
                            let other_channel = sess.frame.connect_subservice(subservice).await?;
                            let newclient =
                                MyOtherServiceClient::new(Default::default(), other_channel);
                            tokio::task::spawn(newclient.dispatch);
                            Ok(newclient.client)
                        }));
                    }
                }
            }
        });
    }
}
