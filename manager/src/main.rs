use common::PackRat;
use log::{error, info, warn};
use tarpc::server::Channel;
use tarpc::{server::BaseChannel, transport};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

use futures_util::{SinkExt, StreamExt, TryStreamExt};

#[tokio::main]
async fn main() {
    server_loop("0.0.0.0:9090".to_string()).await
}

async fn server_loop(addr: String) {
    let try_socket = TcpListener::bind(&addr).await;

    let listener = try_socket.expect("Failed to bind");

    while let Ok((stream, _addr)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }
}

async fn accept_connection(stream: TcpStream) {
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(stream) => stream,
        Err(e) => {
            warn!("Error during the websocket handshake occurred; {e}");
            return;
        }
    };

    type IdkHowElseToFixThis = tokio_tungstenite::tungstenite::Error;

    let transport = ws_stream
        .filter_map(|req| async {
            match req {
                Ok(Message::Binary(b)) => Some(b),
                _ => None,
            }
        })
        .map(|binary| common::encode(&binary).map_err(|_| IdkHowElseToFixThis::AttackAttempt))
        .with(|resp| async move {
            common::decode(resp)
                .map(Message::Binary)
                .map_err(|_| IdkHowElseToFixThis::AttackAttempt)
        });

    let server = BaseChannel::with_defaults(transport);

    tokio::spawn(
        server
            .execute(PackRatServer.serve())
            .for_each(|response| async move {
                tokio::spawn(response);
            }),
    );
}

#[derive(Clone)]
struct PackRatServer;

impl PackRat for PackRatServer {
    async fn hello(self, _context: tarpc::context::Context, name: String) -> String {
        name + "Said hi!"
    }
}
