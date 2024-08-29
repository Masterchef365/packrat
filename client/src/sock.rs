use std::{
    convert::Infallible,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use ewebsock::{WsReceiver, WsSender};
use futures_util::{Sink, Stream};

pub struct Sock {
    buf: SharedBuffers,
}

pub struct Remote {
    buf: SharedBuffers,
    ws_tx: WsSender,
    ws_rx: WsReceiver,
    can_send: bool,
}

pub type SharedBuffers = Arc<Mutex<Buffers>>;

#[derive(Default)]
pub struct Buffers {
    tx: Vec<Vec<u8>>,
    rx: Vec<Vec<u8>>,
    wakers: Vec<Waker>,
}

pub fn connect((ws_tx, ws_rx): (WsSender, WsReceiver)) -> (Sock, Remote) {
    let buf = SharedBuffers::default();

    let sock = Sock { buf: buf.clone() };

    let remote = Remote {
        buf,
        ws_tx,
        ws_rx,
        can_send: false,
    };

    (sock, remote)
}

impl Remote {
    pub fn receive(&mut self) {
        while let Some(msg) = self.ws_rx.try_recv() {
            match msg {
                ewebsock::WsEvent::Message(ewebsock::WsMessage::Binary(binary)) => {
                    let mut buf = self.buf.lock().unwrap();
                    buf.rx.push(binary);
                    for waker in buf.wakers.drain(..) {
                        waker.wake();
                    }
                }
                ewebsock::WsEvent::Opened => self.can_send = true,
                ewebsock::WsEvent::Error(e) => panic!("{:#}", e),
                other => log::warn!("Other WS type: {:?}", other),
            }
        }
    }

    pub fn send(&mut self) {
        if self.can_send {
            // Flush RPC changes to the server
            let mut buf = self.buf.lock().unwrap();
            for msg in buf.tx.drain(..) {
                self.ws_tx.send(ewebsock::WsMessage::Binary(msg));
            }
        }
    }
}

impl Stream for Sock {
    type Item = Vec<u8>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = self.buf.lock().unwrap();
        if buf.rx.is_empty() {
            buf.wakers.push(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(Some(buf.rx.remove(0)))
        }
    }
}

impl Sink<Vec<u8>> for Sock {
    type Error = Infallible;
    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        let mut buf = self.buf.lock().unwrap();
        buf.tx.push(item);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
