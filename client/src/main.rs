#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::task::Poll;

use common::{PackRatClient, PackRatRequest, PackRatResponse};
use egui::Ui;
use ewebsock_tarpc::ewebsock;
use ewebsock_tarpc::{
    ewebsock::{WsReceiver, WsSender},
    WebSocketPoller,
};
use futures::sink::SinkExt;
use futures::task::noop_waker_ref;
use futures::{StreamExt, TryStreamExt};
use poll_promise::Promise;
use tarpc::Request;
use tarpc::{client::NewClient, transport::channel::UnboundedChannel, ClientMessage, Response};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct App {
    rx_text: Option<Promise<String>>,
    client: PackRatClient,
    data: AppData,
    server_transport: UnboundedChannel<ClientMessage<PackRatRequest>, Response<PackRatResponse>>,
    ws_tx: WsSender,
    ws_rx: WsReceiver,
    can_send: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct AppData {
    data: u32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        let data: AppData = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        let (client_transport, server_transport) = tarpc::transport::channel::unbounded();
        let client = PackRatClient::new(Default::default(), client_transport);

        let addr = "ws://127.0.0.1:9090";

        let ctx = cc.egui_ctx.clone();
        let (ws_tx, ws_rx) =
            ewebsock::connect_with_wakeup(addr, Default::default(), move || ctx.request_repaint())
                .unwrap();

        Self {
            can_send: false,
            ws_tx,
            ws_rx,
            server_transport,
            client: client.client,
            rx_text: None,
            data,
        }
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.data);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Fetch frames received from the server, and send use them for RPC
        while let Some(msg) = self.ws_rx.try_recv() {
            match msg {
                ewebsock::WsEvent::Message(ewebsock::WsMessage::Binary(binary)) => {
                    let decoded = common::decode(&binary).unwrap();
                    self.server_transport.start_send_unpin(decoded).unwrap();
                }
                ewebsock::WsEvent::Opened => dbg!(self.can_send = true),
                ewebsock::WsEvent::Error(e) => panic!("{:#}", e),
                _ => todo!(),
            }
        }

        poll_promise::tick();

        // Do gui stuff
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(promise) = &mut self.rx_text {
                if let Some(value) = promise.ready() {
                    ui.label(value);
                } else {
                    ui.label("Waiting for a response ...");
                }
            } else {
                if ui.button("Do the thing").clicked() {
                    let client = self.client.clone();
                    self.rx_text = Some(Promise::spawn_local(async move {
                        client
                            .hello(tarpc::context::current(), "Hello from client".to_string())
                            .await
                            .unwrap()
                    }));
                }
            }
        });

        poll_promise::tick();

        if self.can_send {
            // Flush RPC changes to the server
            let waker = noop_waker_ref();
            let mut cx = std::task::Context::from_waker(&waker);
            while let Poll::Ready(Some(Ok(value))) = self.server_transport.poll_next_unpin(&mut cx) {
                self.ws_tx
                    .send(ewebsock::WsMessage::Binary(common::encode(&value).unwrap()));
                }
        }

        poll_promise::tick();
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let start_result = eframe::WebRunner::new()
            .start(
                "the_canvas_id",
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await;
    });
}
