#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::convert::Infallible;
use std::task::Poll;

use bincode::Error as BincodeError;
use common::{decode, encode, PackRatClient, PackRatRequest, PackRatResponse};
use egui::Ui;
use ewebsock::{WsMessage, WsReceiver, WsSender};
use futures_util::sink::SinkExt;
use futures_util::task::noop_waker_ref;
use futures_util::{StreamExt, TryStreamExt};

use poll_promise::Promise;
use sock::Sock;
use tarpc::Request;
use tarpc::{client::NewClient, transport::channel::UnboundedChannel, ClientMessage, Response};

mod sock;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct App {
    rx_text: Option<Promise<String>>,
    client: PackRatClient,
    data: AppData,
    dispatch_promise: Promise<()>,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct AppData {
    data: u32,
}

#[derive(thiserror::Error, Debug)]
pub enum RpcError {
    //#[error("Networking")]
    //WebSocket(#[from] WebSocketError),
    #[error("Serialization")]
    Bincode(#[from] BincodeError),
    #[error("no")]
    WebSocket(#[from] Infallible),
}
/*

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        let data: AppData = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        let ws = WebSocket::open("ws://127.0.0.1:9090").unwrap();
        *
 * */

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        let data: AppData = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        //tokio::task::spawn();

        let addr = "ws://127.0.0.1:9090";

        let ctx = cc.egui_ctx.clone();
        let sock =
            ewebsock::connect_with_wakeup(addr, Default::default(), move || ctx.request_repaint())
                .unwrap();

        let sock = Sock::new(sock)
            //.map_err(|e| RpcError::from(e))
            //.sink_map_err(|e| RpcError::from(e))
            .with(|client_msg| async move { encode(&client_msg).map_err(|e| RpcError::from(e)) })
            .map(|byt| decode(&byt).map_err(|e| RpcError::from(e)));

        let client = PackRatClient::new(Default::default(), sock);

        let dispatch_promise = Promise::spawn_local(async {
            let _ = client.dispatch.await;
        });

        Self {
            dispatch_promise,
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

        // Do gui stuff
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(promise) = &mut self.rx_text {
                if let Some(value) = promise.ready() {
                    ui.label(value);
                } else {
                    ui.label("Waiting for a response ...");
                }
            }

            if ui.button("Do the thing").clicked() {
                let client = self.client.clone();
                log::info!("Click");

                self.rx_text = Some(Promise::spawn_local(async move {
                    println!("Saying hello");
                    match client
                        .hello(tarpc::context::current(), "Hello from client".to_string())
                        .await
                    {
                        Ok(text) => text,
                        Err(e) => format!("{:#}", e),
                    }
                }));
            }
        });
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

    let _ = Promise::spawn_local(async {
        let start_result = eframe::WebRunner::new()
            .start(
                "the_canvas_id",
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await;
    });
}
