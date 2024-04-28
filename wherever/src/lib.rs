use std::collections::{hash_map::Entry, HashMap};
use std::convert::TryInto;
use std::fs::File;
use std::future::Future;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};

use futures::stream::{Stream, StreamExt};

use rand::seq::SliceRandom;

use wherever_crypto::{relay_client_handshake, DiscoveryProtocol, Key, Pubkey};

use wherever_crypto::noise_rust_crypto::X25519;

use wordlist::EFF_SHORT_2;

pub trait WebSocket: Stream<Item = Self::StreamItem> + Sized + Unpin {
    type StreamItem;
    type Message: WebSocketMessage;
    type Error: std::fmt::Debug;
    //type SendFuture: Future<Output = Result<(), Self::Error>> + 'a;
    //fn send(&'a mut self, msg: Self::Message) -> Self::SendFuture;
    fn send<'b>(
        &'b mut self,
        msg: Self::Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + 'b + Send>>;
    fn item_to_message(i: Self::StreamItem) -> Option<Self::Message>;
}

pub trait WebSocketMessage {
    fn binary(data: Vec<u8>) -> Self;
    fn into_data(self) -> Vec<u8>;
    fn to_text<'a>(&'a self) -> Option<&'a str>;
}

pub trait ConnectWebSocket<'b, Url: 'b>: WebSocket {
    type ConnectFuture: Future<Output = Result<Self, Self::Error>>;
    fn connect(url: Url) -> Self::ConnectFuture;
}

#[cfg(feature = "wasm")]
impl WebSocket for websocket_wasm::WebSocket {
    type Message = websocket_wasm::Message;
    type StreamItem = Self::Message;
    type Error = ();
    //type SendFuture = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Unpin + 'a>>;
    fn send<'b>(
        &'b mut self,
        msg: Self::Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + 'b + Send>> {
        //fn send(&'a mut self, msg: Self::Message) -> Self::SendFuture {
        Box::pin(std::future::ready(websocket_wasm::WebSocket::send(
            self, msg,
        )))
    }
    fn item_to_message(i: Self::StreamItem) -> Option<Self::Message> {
        Some(i)
    }
}

#[cfg(feature = "wasm")]
impl WebSocketMessage for websocket_wasm::Message {
    fn binary(data: Vec<u8>) -> Self {
        Self::binary(data)
    }
    fn into_data(self) -> Vec<u8> {
        self.into_data()
    }
    fn to_text<'a>(&'a self) -> Option<&'a str> {
        match self {
            websocket_wasm::Message::Text(s) => Some(&s),
            _ => None,
        }
    }
}

#[cfg(feature = "wasm")]
impl<'b> ConnectWebSocket<'b, String> for websocket_wasm::WebSocket {
    type ConnectFuture = Pin<Box<dyn Future<Output = Result<Self, Self::Error>> + 'b>>;
    fn connect(url: String) -> Pin<Box<dyn Future<Output = Result<Self, Self::Error>> + 'b>> {
        Box::pin(async move { websocket_wasm::WebSocket::new(&url).await })
    }
}

#[cfg(feature = "tungstenite")]
impl WebSocket for tokio_tungstenite::WebSocketStream<tokio::net::TcpStream> {
    type StreamItem = Result<tokio_tungstenite::tungstenite::Message, Self::Error>;
    type Message = tokio_tungstenite::tungstenite::Message;
    type Error = tokio_tungstenite::tungstenite::Error;
    //type SendFuture =
    //    Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Unpin + 'a + Send + Sync>>;
    //fn send(&'a mut self, msg: Self::Message) -> Self::SendFuture {
    fn send<'b>(
        &'b mut self,
        msg: Self::Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + 'b + Send>> {
        Box::pin(futures::SinkExt::send(self, msg))
    }
    fn item_to_message(i: Self::StreamItem) -> Option<Self::Message> {
        i.ok()
    }
}

#[cfg(feature = "tungstenite")]
impl WebSocketMessage for tokio_tungstenite::tungstenite::Message {
    fn binary(data: Vec<u8>) -> Self {
        Self::binary(data)
    }
    fn into_data(self) -> Vec<u8> {
        self.into_data()
    }
    fn to_text(&self) -> Option<&str> {
        self.to_text().ok()
    }
}
#[cfg(feature = "tungstenite")]
impl<'b, Url: 'b + Send + Sync> ConnectWebSocket<'b, Url>
    for tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>
where
    Url: tokio_tungstenite::tungstenite::client::IntoClientRequest + Unpin,
{
    type ConnectFuture =
        Pin<Box<dyn Future<Output = Result<Self, Self::Error>> + Send + Sync + 'b>>;
    fn connect(url: Url) -> Self::ConnectFuture {
        use futures::FutureExt;
        Box::pin(tokio_tungstenite::connect_async(url).map(|x| x.map(|x| x.0)))
    }
}

pub async fn relay_reciever<WS: WebSocket>(
    mut socket: WS,
    key: Key,
) -> Option<
    futures::stream::FilterMap<
        futures::stream::FilterMap<
            WS,
            std::future::Ready<Option<WS::Message>>,
            impl FnMut(WS::StreamItem) -> std::future::Ready<Option<WS::Message>>,
        >,
        std::future::Ready<Option<Vec<u8>>>,
        impl FnMut(WS::Message) -> std::future::Ready<Option<Vec<u8>>>,
    >,
> {
    let mut handshake = relay_client_handshake(key.clone());

    let mut relay_cipher = {
        let msg = handshake.write_message_vec(&[]).ok()?;
        socket.send(WS::Message::binary(msg)).await.ok()?;

        let msg = WS::item_to_message(socket.next().await?)?;
        handshake.read_message_vec(&msg.into_data()).ok()?;

        let msg = handshake.write_message_vec(&[]).ok()?;
        socket.send(WS::Message::binary(msg)).await.ok()?;

        handshake.get_ciphers().1
    };
    Some(
        socket
            .filter_map(move |x| std::future::ready(WS::item_to_message(x)))
            .filter_map(move |msg| {
                std::future::ready(relay_cipher.decrypt_vec(&msg.into_data()).ok())
            }),
    )
}

pub async fn handle_client_handshake<'a, WS>(
    ws: &'a mut WS,
) -> Option<
    wherever_crypto::noise_protocol::HandshakeState<
        X25519,
        wherever_crypto::noise_rust_crypto::ChaCha20Poly1305,
        wherever_crypto::noise_rust_crypto::Blake2b,
    >,
>
where
    WS: WebSocket,
{
    let mut handshake = wherever_crypto::relay_server_handshake();
    let msg = WS::item_to_message(ws.next().await?)?;
    handshake.read_message_vec(&msg.into_data()).ok()?;
    let send_msg = handshake.write_message_vec(&[]).ok()?;
    ws.send(WS::Message::binary(send_msg)).await.ok()?;
    let msg = WS::item_to_message(ws.next().await?)?;
    handshake.read_message_vec(&msg.into_data()).ok()?;
    Some(handshake)
}

pub async fn initiate_discovery<'b, WS, Url: 'b>(
    key: Key,
    url: Url,
) -> Option<(String, impl Future<Output = Option<Pubkey>>)>
where
    WS: ConnectWebSocket<'b, Url>,
{
    let mut rng = rand::thread_rng();
    let (w1, w2, w3) = (
        EFF_SHORT_2.choose(&mut rng)?,
        EFF_SHORT_2.choose(&mut rng)?,
        EFF_SHORT_2.choose(&mut rng)?,
    );
    let words = format!("{}-{}-{}", w1, w2, w3);
    let phrased = format_phrase(&words);
    // Generate 3 words
    let (disc, msg) = DiscoveryProtocol::initiator(key, &phrased);
    // Connect to server, get idx
    let mut ws = WS::connect(url).await.ok()?;
    ws.send(WS::Message::binary(msg)).await.ok()?;
    let idx = WS::item_to_message(ws.next().await?)?;
    let idx = idx.to_text()?;
    Some((format!("{}-{}", idx, words), async move {
        // Display idx + words
        // Do handshake
        let msg = WS::item_to_message(ws.next().await?)?;
        let (remote_key, response) = disc.read_message(&msg.into_data())?;
        ws.send(WS::Message::binary(response)).await.ok()?;
        Some(remote_key)
    }))
}

/// Url must include channel im E
pub async fn respond_discovery<'b, WS, Url: 'b>(key: Key, url: Url, words: String) -> Option<Pubkey>
where
    WS: ConnectWebSocket<'b, Url>,
{
    let mut ws = WS::connect(url).await.ok()?;
    let msg = WS::item_to_message(ws.next().await?)?;
    let (disc, msg) = DiscoveryProtocol::responder(key, &words, &msg.into_data())?;
    ws.send(WS::Message::binary(msg)).await.ok()?;
    let msg = WS::item_to_message(ws.next().await?)?;
    let remote_key = disc.read_message(&msg.into_data())?;

    Some(remote_key)
}

pub fn parse_phrase(phrase: &str) -> Option<(&str, String)> {
    let split_idx = phrase.find("-")?;
    let channel = &phrase[..split_idx];
    let phrase = format_phrase(phrase.get((split_idx + 1)..)?);
    Some((channel, phrase))
}

pub fn format_phrase(phrase: &str) -> String {
    let mut phrase = phrase.to_ascii_lowercase();
    phrase.retain(|c| match c {
        ' ' | '-' | '_' => false,
        _ => true,
    });
    phrase
}

pub struct Tofu<S> {
    storage: S,
    allowed: HashMap<Pubkey, AtomicU64>,
}

pub trait TofuStorage {
    fn save<I: Iterator<Item = String>>(&mut self, allowed: &mut I) -> io::Result<()>;
    fn load<F: Fn(&str) -> Option<(Pubkey, AtomicU64)>>(
        &mut self,
        f: F,
    ) -> io::Result<HashMap<Pubkey, AtomicU64>>;
}

impl<S: TofuStorage> Tofu<S> {
    pub fn save(&mut self) -> io::Result<()> {
        self.storage.save(
            &mut self.allowed.iter().map(|(key, seq)| {
                format!("{},{}\n", base64::encode(key), seq.load(Ordering::Relaxed))
            }),
        )
    }
    pub fn load(mut storage: S) -> Self {
        let allowed = storage
            .load(|line| {
                let mut split = line.split(",");
                let rest = split.next()?;
                let seq: &str = split.next()?;
                Some((
                    (&base64::decode(rest).ok()?[..]).try_into().ok()?,
                    AtomicU64::new(seq.parse().ok()?),
                ))
            })
            .unwrap_or_default();
        Self { storage, allowed }
    }
}
impl<P> TofuStorage for P
where
    P: AsRef<Path>,
{
    fn save<I: Iterator<Item = String>>(&mut self, allowed: &mut I) -> io::Result<()> {
        let mut file = File::create(self.as_ref())?;
        for line in allowed {
            file.write_all(line.as_bytes())?;
        }
        file.flush()?;
        Ok(())
    }
    fn load<F: Fn(&str) -> Option<(Pubkey, AtomicU64)>>(
        &mut self,
        f: F,
    ) -> io::Result<HashMap<Pubkey, AtomicU64>> {
        if let Ok(file) = File::open(self.as_ref()) {
            Ok(BufReader::new(file)
                .lines()
                .flat_map(|l| l)
                .flat_map(|x| f(&x))
                .collect())
        } else {
            let _file = File::create(self.as_ref())?;
            Ok(HashMap::new())
        }
    }
}

pub struct UntrustedEntry {
    message: String,
    pub key: Pubkey,
    seq: u64,
}
impl UntrustedEntry {
    pub fn trust<S>(self, tofu: &mut Tofu<S>) -> Option<String> {
        tofu.insert(self.key, self.seq).then(|| self.message)
    }
    pub fn check<S>(self, tofu: &mut Tofu<S>) -> Result<Option<String>, Self> {
        if let Some(valid) = tofu.check(&self.key, self.seq) {
            Ok(valid.then(|| {
                tofu.insert(self.key, self.seq);
                self.message
            }))
        } else {
            Err(self)
        }
    }
    pub fn peek(&self) -> &str {
        &self.message
    }
}

pub enum TofuEntry {
    Trusted(String),
    Untrusted(UntrustedEntry),
}

impl<S> Tofu<S> {
    /// Check if pubkey is valid, if so check seq number valid, if so set last seq number
    /// Some(true) - key trusted, seq valid
    /// Some(false) - key trusted, seq invalid
    /// None - unknown key
    fn check(&self, key: &Pubkey, seq: u64) -> Option<bool> {
        if let Some(last_seq) = self.allowed.get(key) {
            if let Ok(_) = last_seq.fetch_update(Ordering::AcqRel, Ordering::Acquire, |last_seq| {
                if last_seq < seq {
                    Some(seq)
                } else {
                    None
                }
            }) {
                Some(true)
            } else {
                Some(false)
            }
        } else {
            None
        }
    }
    fn insert(&mut self, key: Pubkey, seq: u64) -> bool {
        match self.allowed.entry(key) {
            Entry::Occupied(mut o) => {
                let last_seq = o.get_mut().get_mut();
                if *last_seq < seq {
                    println!("Seq good {} < {}", last_seq, seq);
                    *last_seq = seq;
                    true
                } else {
                    println!("Seq BAD  {} > {}", last_seq, seq);
                    false
                }
            }
            Entry::Vacant(v) => {
                v.insert(seq.into());
                true
            }
        }
    }

    /// Err(()) on decrypt failure / bad seq number
    /// Ok(Some(String)) on Success
    /// Ok(Some(String)) on unknown key - should prompt
    pub fn decrypt_message(&self, key: &Key, msg: &[u8]) -> Result<TofuEntry, ()> {
        if let Ok((client_key, seq, dec_msg)) =
            wherever_crypto::decrypt_client_message(&msg, key.clone())
        {
            match self.check(&client_key, seq) {
                Some(true) => String::from_utf8(dec_msg)
                    .map_err(|_| ())
                    .map(TofuEntry::Trusted),
                Some(false) => Err(()),
                None => Ok(TofuEntry::Untrusted(UntrustedEntry {
                    message: String::from_utf8(dec_msg).map_err(|_| ())?,
                    key: client_key,
                    seq,
                })),
            }
        } else {
            Err(())
        }
    }
    /// Mark a key as trusted
    pub fn trust(&mut self, key: Pubkey) {
        self.insert(key, 0);
    }
}
