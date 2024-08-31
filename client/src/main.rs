use ewebsock_async_tarpc_utils::{bincode_stream, RpcError};
use futures_util::{SinkExt, TryStreamExt};

use common::PackRatClient;

use ewebsock_async_simple::{ewebsock, Remote};
use poll_promise::Promise;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct App {
    rx_text: Option<Promise<String>>,
    client: PackRatClient,
    data: AppData,
    remote: Remote,
    dispatch_promise: Promise<()>,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct AppData {
    data: u32,
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
        let websock =
            ewebsock::connect_with_wakeup(addr, Default::default(), move || ctx.request_repaint())
                .unwrap();

        let (sock, remote) = ewebsock_async_simple::connect(websock);

        let sock = sock.map_err(RpcError::from).sink_map_err(RpcError::from);

        let sock = bincode_stream(sock);

        let client = PackRatClient::new(Default::default(), sock);

        let dispatch_promise = Promise::spawn_local(async {
            let _ = client.dispatch.await;
        });

        Self {
            remote,
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
        self.remote.receive().unwrap();

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
                    log::info!("Saying hello");
                    let ret = match client
                        .hello(tarpc::context::current(), "Client eastwood".to_string())
                        .await
                    {
                        Ok(text) => text,
                        Err(e) => format!("{:#}", e),
                    };
                    log::info!("Done saying hello");
                    ret
                }));
            }
        });

        self.remote.send();
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
