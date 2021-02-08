use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::thread;

use warp::ws::{self, WebSocket};
use warp::Filter;

use futures::{select, FutureExt, SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};

use wherever_crypto::{Blake2b, ChaCha20Poly1305, HandshakeState, U8Array, DH, X25519};
use wherever_crypto::{Key, Pubkey};

#[tokio::main]
async fn main() {
    let addr: SocketAddr = ([0, 0, 0, 0], 8998).into();
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("standalone") => {
            let addr = args
                .next()
                .map(|s| s.parse())
                .unwrap_or(Ok(addr))
                .expect("Invalid host");
            standalone(addr).await;
        }
        Some("relay_server") => {
            let addr = args
                .next()
                .map(|s| s.parse())
                .unwrap_or(Ok(addr))
                .expect("Invalid host");
            println!("Starting relay server");
            relay(addr).await;
        }
        Some("relay_client") => {
            println!("Starting relay client");
            if let Some(addr) = args.next().and_then(|s| s.parse().ok()) {
                relay_client(addr);
            } else {
                eprintln!("Relay server address required");
            }
        }
        Some(arg) => {
            eprintln!("Unrecognized argument: \"{}\"", arg);
        }
    }
}

async fn standalone(addr: SocketAddr) {
    let key = KeyWrapper(load_server_key("server_key").unwrap());
    let pubkey = base64::encode(X25519::pubkey(&key.0));
    let tofu = Arc::new(RwLock::new(Tofu::load("allowed_devices.txt").unwrap()));
    qr2term::print_qr(format!("where://{}/#{}", addr, pubkey)).unwrap();
    println!("where://{}/#{}", addr, pubkey);
    warp::serve(
        warp::path("open")
            .and(warp::post())
            .and(warp::body::content_length_limit(4096))
            .and(warp::body::bytes())
            .and_then(move |body: warp::hyper::body::Bytes| {
                let key = key.0.clone();
                let tofu = tofu.clone();
                async move {
                    if let Ok((client_key, msg)) =
                        wherever_crypto::decrypt_client_message(&*body, key)
                    {
                        if prompt_async(&tofu, &client_key).await.unwrap_or(false) {
                            let url =
                                std::str::from_utf8(&msg).map_err(|_| warp::reject::reject())?;
                            launch(url.to_owned());
                        }
                    }
                    Ok::<_, warp::Rejection>("Good")
                }
            }),
    )
    .run(addr)
    .await;
}

fn launch(url: String) {
    println!("Opening {:?}", url);
    thread::spawn(|| open::that_in_background(url));
}

struct Message(Vec<u8>);

type Registry = HashMap<Pubkey, mpsc::Sender<Message>>;

async fn relay(addr: SocketAddr) {
    let registry = Arc::new(RwLock::new(Registry::new()));
    let routes = warp::path("stream")
        .and(warp::ws())
        .and(warp::filters::addr::remote())
        .map({
            let registry = registry.clone();
            move |ws: warp::ws::Ws, addr| {
                let registry = registry.clone();
                ws.on_upgrade(move |websocket| handle_client(websocket, registry, addr))
            }
        })
        .or(warp::path("open")
            .and(warp::post())
            .and(warp::body::content_length_limit(4096))
            .and(warp::body::bytes())
            .and_then(move |body: warp::hyper::body::Bytes| {
                let registry = registry.clone();
                async move {
                    if let Some(key) = wherever_crypto::get_destination(&body) {
                        if let Some(channel) = registry.read().await.get(&key) {
                            channel.send(Message(body.to_vec())).await.ok().unwrap();
                            return Ok::<_, warp::Rejection>("Good");
                        }
                    }
                    Err(warp::reject::not_found())
                }
            }));
    warp::serve(routes).run(addr).await;
}

async fn handle_client_handshake(
    ws: &mut WebSocket,
) -> Option<HandshakeState<X25519, ChaCha20Poly1305, Blake2b>> {
    let mut handshake = wherever_crypto::relay_server_handshake();
    let msg = ws.next().await?.ok()?;
    handshake.read_message_vec(msg.as_bytes()).ok()?;
    let send_msg = handshake.write_message_vec(&[]).ok()?;
    ws.send(ws::Message::binary(send_msg)).await.ok()?;
    let msg = ws.next().await?.ok()?;
    handshake.read_message_vec(msg.as_bytes()).ok()?;
    Some(handshake)
}

async fn handle_client(
    mut ws: WebSocket,
    state: Arc<RwLock<Registry>>,
    remote: Option<SocketAddr>,
) {
    println!("Client connected {:?}", remote);
    // get public key of client
    if let Some((key, mut cipher)) = handle_client_handshake(&mut ws)
        .await
        .and_then(|hs| hs.get_rs().map(|rs| (rs, hs.get_ciphers().1)))
    {
        println!("Client key: {:?}", base64::encode(&key));
        // register key + channel in shared state
        let (sender, mut receiver) = mpsc::channel(1);
        state.write().await.insert(Clone::clone(&key), sender);
        {
            let (mut outgoing, mut incoming) = (&mut ws).split();
            let out_ref = &mut outgoing;
            // wait for messages
            while let Ok(()) = select! {
                res = receiver.recv().then(|msg| async {
                    let cipher = &mut cipher;
                    // process message, send to
                    if let Some(msg) = msg {
                        let msg = cipher.encrypt_vec(&msg.0);
                        out_ref.send(ws::Message::binary(msg)).await.map_err(|_|())
                    } else {
                        Err(())
                    }
                }) => res,
                res = incoming.next().map(|msg| match msg {
                    Some(Ok(msg)) if msg.is_close() => Err(()),
                    Some(Ok(_)) => Ok(()),
                    Some(Err(_)) => Err(()),
                    None => Err(()),
                }) => res
            } {}
        }
        // on close unregister key
        state.write().await.remove(&key);
    }
    println!("Client disconnected {:?}", remote);
    let _ = ws.close().await;
}

fn relay_client(addr: SocketAddr) {
    let key = KeyWrapper(load_server_key("server_key").unwrap());
    let mut tofu = Tofu::load("allowed_devices.txt").unwrap();
    let pubkey = X25519::pubkey(&key.0);
    let pubkey_string = base64::encode(&pubkey);
    qr2term::print_qr(format!("where://{}/#{}", addr, pubkey_string)).unwrap();
    println!("where://{}/#{}", addr, pubkey_string);
    let (mut socket, _resp) =
        tungstenite::client::connect(format!("ws://{}/stream", addr)).unwrap();
    let mut handshake = wherever_crypto::relay_client_handshake(key.clone().0);
    let msg = handshake.write_message_vec(&[]).unwrap();
    socket
        .write_message(tungstenite::Message::Binary(msg))
        .unwrap();
    let response = socket.read_message().unwrap().into_data();
    handshake.read_message_vec(&response).unwrap();
    let msg = handshake.write_message_vec(&[]).unwrap();
    socket
        .write_message(tungstenite::Message::Binary(msg))
        .unwrap();
    let mut relay_cipher = handshake.get_ciphers().1;
    while let Ok(msg) = socket.read_message() {
        if let Ok(msg) = relay_cipher.decrypt_vec(&msg.into_data()) {
            if let Ok((client_key, msg)) =
                wherever_crypto::decrypt_client_message(&msg, key.clone().0)
            {
                if prompt(&mut tofu, &client_key).unwrap_or(false) {
                    if let Some(url) = std::str::from_utf8(&msg).ok() {
                        launch(url.to_owned());
                    }
                }
            }
        } else {
            println!("Invalid message from relay");
        }
    }
}

// Key doesn't implement Clone so we do this to make our closures Clone
struct KeyWrapper(Key);

impl Clone for KeyWrapper {
    fn clone(&self) -> Self {
        Self(U8Array::clone(&self.0))
    }
}

fn load_server_key<P: AsRef<Path>>(path: P) -> io::Result<Key> {
    if let Ok(mut file) = File::open(path.as_ref()) {
        let mut key = Key::new();
        file.read_exact(&mut *key)?;
        Ok(key)
    } else {
        let mut file = File::create(path.as_ref())?;
        let key = X25519::genkey();
        file.write_all(&*key)?;
        Ok(key)
    }
}

async fn prompt_async(tofu: &RwLock<Tofu>, key: &Pubkey) -> io::Result<bool> {
    if tofu.read().await.allowed.contains(key) {
        Ok(true)
    } else {
        let stdin_a = io::stdin();
        let accepted = {
            let mut stdin = stdin_a.lock();
            let mut line = String::new();
            loop {
                println!(
                    "Incoming link from {}, accept? (Y/N): ",
                    base64::encode(key)
                );
                stdin.read_line(&mut line)?;
                match line.chars().next() {
                    Some('Y') => {
                        break true;
                    }
                    Some('N') => {
                        break false;
                    }
                    _ => {
                        println!("Please answer \"Y\" or \"N\"");
                    }
                }
            }
        };
        if accepted {
            let mut tofu = tofu.write().await;
            tofu.allowed.insert(Clone::clone(key));
            tofu.save("allowed_devices.txt")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn prompt(tofu: &mut Tofu, key: &Pubkey) -> io::Result<bool> {
    if tofu.allowed.contains(key) {
        Ok(true)
    } else {
        let stdin_a = io::stdin();
        let mut stdin = stdin_a.lock();
        let mut line = String::new();
        let accepted = loop {
            println!(
                "Incoming link from {}, accept? (Y/N): ",
                base64::encode(key)
            );
            stdin.read_line(&mut line)?;
            match line.chars().next() {
                Some('Y') => {
                    break true;
                }
                Some('N') => {
                    break false;
                }
                _ => {
                    println!("Please answer \"Y\" or \"N\"");
                }
            }
        };
        if accepted {
            tofu.allowed.insert(Clone::clone(key));
            tofu.save("allowed_devices.txt")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

struct Tofu {
    allowed: HashSet<Pubkey>,
}

impl Tofu {
    fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        if let Ok(file) = File::open(path.as_ref()) {
            Ok(Self {
                allowed: BufReader::new(file)
                    .lines()
                    .flat_map(|l| l)
                    .flat_map(|line| (&base64::decode(line).ok()?[..]).try_into().ok())
                    .collect(),
            })
        } else {
            let _file = File::create(path.as_ref())?;
            Ok(Self {
                allowed: HashSet::new(),
            })
        }
    }
    fn save<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path.as_ref())?;
        for key in self.allowed.iter().map(|k| base64::encode(k)) {
            file.write_all(key.as_bytes())?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }
}
