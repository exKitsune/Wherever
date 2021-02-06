use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::thread;

use warp::ws::{self, WebSocket};
use warp::Filter;

use futures::{select, FutureExt, SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};

use noise_protocol::{U8Array, DH};
use noise_rust_crypto::X25519;

use server::{Key, Pubkey};

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
            qr2term::print_qr(format!("where://{}", addr)).unwrap();
            relay(addr).await;
        }
        Some("relay_client") => {
            println!("Starting relay client");
            qr2term::print_qr(format!("where://{}", addr)).unwrap();
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
    qr2term::print_qr(format!("where://{}/#{}", addr, pubkey)).unwrap();
    println!("where://{}/#{}", addr, pubkey);
    warp::serve(
        warp::path("open")
            .and(warp::post())
            .and(warp::body::content_length_limit(4096))
            .and(warp::body::bytes())
            .and_then(move |body: warp::hyper::body::Bytes| {
                let key = key.0.clone();
                async move {
                    if let Ok(msg) = server::decrypt_client_message(&*body, key) {
                        let url = std::str::from_utf8(&msg).map_err(|_| warp::reject::reject())?;
                        launch(url.to_owned());
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
                    if let Some(key) = server::get_destination(&body) {
                        if let Some(channel) = registry.read().await.get(&key) {
                            channel.send(Message(body.to_vec())).await.ok().unwrap();
                        }
                    }
                    Ok::<_, warp::Rejection>("Good")
                }
            }));
    warp::serve(routes).run(addr).await;
}

async fn handle_client(
    mut ws: WebSocket,
    state: Arc<RwLock<Registry>>,
    remote: Option<SocketAddr>,
) {
    println!("Client connected {:?}", remote);
    // get public key of client
    if let Some(key) = ws.next().await.map(|x| x.ok()).flatten().and_then(|msg| {
        if msg.as_bytes().len() == 32 {
            Some(Pubkey::from_slice(msg.as_bytes()))
        } else {
            None
        }
    }) {
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
                    // process message, send to
                    if let Some(msg) = msg {
                        out_ref.send(ws::Message::binary(msg.0)).await.map_err(|_|())
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
    let pubkey = X25519::pubkey(&key.0);
    let pubkey_string = base64::encode(&pubkey);
    qr2term::print_qr(format!("where://{}/#{}", addr, pubkey_string)).unwrap();
    println!("where://{}/#{}", addr, pubkey_string);
    let (mut socket, _resp) =
        tungstenite::client::connect(format!("ws://{}/stream", addr)).unwrap();
    socket
        .write_message(tungstenite::Message::Binary((&pubkey).to_vec()))
        .unwrap();
    while let Ok(msg) = socket.read_message() {
        if let Ok(msg) = server::decrypt_client_message(&msg.into_data(), key.clone().0) {
            if let Some(url) = std::str::from_utf8(&msg).ok() {
                launch(url.to_owned());
            }
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
