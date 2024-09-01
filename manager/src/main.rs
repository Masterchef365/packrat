use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{PackRat, PackRatRequest, PackRatResponse};
use ewebsock_async_tarpc_utils::{bincode_stream, RpcError};
use log::{error, info, warn};
use tarpc::server::Channel;
use tarpc::ClientMessage;
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

    let pr = PackRatServer::default();

    let listener = try_socket.expect("Failed to bind");

    while let Ok((stream, addr)) = listener.accept().await {
        dbg!(addr);
        tokio::spawn(accept_connection(stream, pr.clone()));
    }
}

async fn accept_connection(stream: TcpStream, pr: PackRatServer) {
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
                Ok(Message::Binary(b)) => Some(Ok::<_, RpcError>(b)),
                _ => None,
            }
        })
        .with(|resp| async move { Ok(Message::Binary(resp)) });

    let transport = bincode_stream(transport);

    let server = BaseChannel::with_defaults(transport);

    tokio::spawn(server.execute(pr.serve()).for_each(|response| async move {
        tokio::spawn(response);
    }));
}

#[derive(Clone, Default)]
struct PackRatServer {
    number: Arc<Mutex<u32>>,
}

impl PackRat for PackRatServer {
    async fn hello(self, _context: tarpc::context::Context, name: String) -> String {
        {
            let mut number = self.number.lock().unwrap();
            *number += 1;
        }

        tokio::time::sleep(Duration::from_secs(3)).await;

        let number = self.number.lock().unwrap();
        format!("Name: {name} Number: {number}")
    }
}
