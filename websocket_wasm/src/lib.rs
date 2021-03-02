use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::stream::Stream;
use futures_sink::Sink;
use futures_util::stream::unfold;
use futures_util::FutureExt;

use js_sys::Promise;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[derive(Debug)]
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
}

impl Message {
    pub fn text<S: Into<String>>(s: S) -> Self {
        Message::Text(s.into())
    }
    pub fn binary<S: Into<Vec<u8>>>(s: S) -> Self {
        Message::Binary(s.into())
    }
    pub fn is_text(&self) -> bool {
        match self {
            Message::Text(_) => true,
            Message::Binary(_) => false,
        }
    }
    pub fn is_binary(&self) -> bool {
        match self {
            Message::Text(_) => false,
            Message::Binary(_) => true,
        }
    }
    pub fn into_data(self) -> Vec<u8> {
        match self {
            Message::Text(s) => s.into_bytes(),
            Message::Binary(b) => b,
        }
    }
}

impl From<Vec<u8>> for Message {
    fn from(v: Vec<u8>) -> Self {
        Self::Binary(v)
    }
}

impl From<String> for Message {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

pub struct WebSocket {
    ws: web_sys::WebSocket, //todo
    stream: Pin<Box<dyn Stream<Item = Message>>>,
    close: Option<JsFuture>,
}

impl WebSocket {
    pub async fn new(url: &str) -> Result<Self, ()> {
        let inner = web_sys::WebSocket::new(url).map_err(|_| ())?;
        inner.set_binary_type(web_sys::BinaryType::Arraybuffer);
        let _ready = JsFuture::from(Promise::new(&mut |resolve, reject| {
            inner.set_onopen(Some(&resolve));
            inner.set_onerror(Some(&reject));
        }))
        .await
        .unwrap();
        let stream = Box::pin(unfold(inner.clone(), |ws| async {
            let msg = JsFuture::from(Promise::new(&mut |resolve, reject| {
                ws.set_onmessage(Some(&resolve));
                ws.set_onerror(Some(&reject));
                ws.set_onclose(Some(&reject));
            }))
            .await
            .ok()?;
            let e = msg.dyn_into::<web_sys::MessageEvent>().ok()?;
            if let Ok(a) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let msg = Message::Binary(js_sys::Uint8Array::new(&a).to_vec());
                Some((msg, ws))
            } else if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                let msg = Message::Text(text.into());
                Some((msg, ws))
            } else {
                None
            }
        }));
        Ok(Self {
            ws: inner,
            stream,
            close: None,
        })
    }
    pub fn send(&self, item: Message) -> Result<(), ()> {
        match item {
            Message::Text(s) => self.ws.send_with_str(&s).map_err(|_| ()),
            Message::Binary(v) => self.ws.send_with_u8_array(&v).map_err(|_| ()),
        }
        // According to MDN the errors here are only for if the socket is not OPEN, or if
        // the string is not valid UTF-8. We know the string is UTF-8 thanks to String's
        // guarantees, so the only error should be from if the connection was closed.
    }
    pub fn close(&mut self) {
        let tmp_ws = self.ws.clone();
        self.close.get_or_insert_with(|| {
            JsFuture::from(Promise::new(&mut |resolve, _reject| {
                tmp_ws.set_onclose(Some(&resolve))
            }))
        });
    }
}

impl Stream for WebSocket {
    type Item = Message;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx)
    }
}

// TODO: is there really any point in implementing this? poll_ready is really the only
// useful part, if we were to hook it up to onopen here instead of in the constructor
impl Sink<Message> for WebSocket {
    type Error = ();
    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.ws.ready_state() {
            web_sys::WebSocket::CONNECTING => {
                unreachable!() // web socket should already be connected in constructor
            }
            web_sys::WebSocket::OPEN => Poll::Ready(Ok(())),
            web_sys::WebSocket::CLOSED => Poll::Ready(Err(())),
            web_sys::WebSocket::CLOSING => Poll::Ready(Err(())),
            _ => todo!(),
        }
    }
    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        self.send(item)
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
        // We have no API for getting notified on when the message is flushed
        // We can query WebSocket.bufferedAmount but we have no way of knowing
        // when it hits zero (AFAIK)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let tmp_ws = self.ws.clone();
        self.close
            .get_or_insert_with(|| {
                JsFuture::from(Promise::new(&mut |resolve, _reject| {
                    tmp_ws.set_onclose(Some(&resolve))
                }))
            })
            .poll_unpin(cx)
            .map(|r| r.map(|_| ()).map_err(|_| ()))
    }
}
