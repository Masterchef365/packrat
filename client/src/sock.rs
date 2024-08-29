use std::{
    convert::Infallible, pin::Pin, task::{Context, Poll, Waker}
};

use ewebsock::{WsReceiver, WsSender};
use futures_util::{Sink, Stream};

pub struct Sock {
    ws_tx: WsSender,
    tx_buf: Vec<Vec<u8>>,

    ws_rx: WsReceiver,
    rx_buf: Vec<Vec<u8>>,

    wake: Option<Waker>,
    can_send: bool,
}

impl Sock {
    pub fn new((ws_tx, ws_rx): (WsSender, WsReceiver)) -> Self {
        Self {
            ws_tx,
            ws_rx,
            tx_buf: vec![],
            rx_buf: vec![],
            can_send: false,
            wake: None,
        }
    }

    pub fn receive(&mut self) {
        while let Some(msg) = self.ws_rx.try_recv() {
            match msg {
                ewebsock::WsEvent::Message(ewebsock::WsMessage::Binary(binary)) => {
                    self.rx_buf.push(binary);
                    if let Some(waker) = &self.wake {
                        waker.wake_by_ref()
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
            for msg in self.tx_buf.drain(..) {
                self.ws_tx
                    .send(ewebsock::WsMessage::Binary(msg));
            }
        }
    }
}

impl Stream for Sock {
    type Item = Vec<u8>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.wake = Some(cx.waker().clone());
        if self.rx_buf.is_empty() {
            Poll::Pending
        } else {
            Poll::Ready(Some(self.rx_buf.remove(0)))
        }
    }
}

impl Sink<Vec<u8>> for Sock {
    type Error = Infallible;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        self.tx_buf.push(item);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
