use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};

use common::{PackRat, PackRatRequest, PackRatResponse};
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

    let listener = try_socket.expect("Failed to bind");

    while let Ok((stream, addr)) = listener.accept().await {
        dbg!(addr);
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
        .map(|binary| {
            common::decode::<tarpc::ClientMessage<PackRatRequest>>(&binary)
                .map_err(|_| IdkHowElseToFixThis::AttackAttempt)
        })
        .with(|resp| async move {
            common::encode::<tarpc::Response<PackRatResponse>>(&resp)
                .map(Message::Binary)
                .map_err(|_| IdkHowElseToFixThis::AttackAttempt)
        });

    let server = BaseChannel::with_defaults(transport);

    tokio::spawn(
        server
            .execute(PackRatServer::default().serve())
            .for_each(|response| async move {
                tokio::spawn(response);
            }),
    );
}

#[derive(Clone, Default)]
struct PackRatServer {
    number: Arc<Mutex<u32>>,
}

impl PackRat for PackRatServer {
    async fn hello(self, _context: tarpc::context::Context, name: String) -> String {
        let mut number = self.number.lock().unwrap();
        *number += 1;
        format!("Name: {name} Number: {number}")
    }
}
