use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

use warp::ws::{self, WebSocket};
use warp::Filter;

use futures::{select, FutureExt, SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};

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
            qr2term::print_qr(format!("where://{}", addr)).unwrap();
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
    warp::serve(
        warp::path("open")
            .and(warp::post())
            .and(warp::body::content_length_limit(4096))
            .and(warp::body::bytes())
            .and_then(|body: warp::hyper::body::Bytes| async move {
                let url = std::str::from_utf8(&body).map_err(|_| warp::reject::reject())?;
                launch(url.to_owned());
                Ok::<_, warp::Rejection>("Good")
            }),
    )
    .run(addr)
    .await;
}

fn launch(url: String) {
    println!("Opening {:?}", url);
    thread::spawn(|| open::that_in_background(url));
}

#[derive(Eq, PartialEq, Hash, Clone)]
struct Key(u64); // TODO:

struct Message(String);

type Registry = HashMap<Key, mpsc::Sender<Message>>;

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
                    let body = std::str::from_utf8(&body).map_err(|_| warp::reject::reject())?;
                    println!("AAAA {}", body);
                    for (_key, channel) in registry.read().await.iter() {
                        channel.send(Message(body.to_owned())).await.ok().unwrap();
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
    let key = Key(1234); // TODO:
                         // register key + channel in shared state
    let (sender, mut receiver) = mpsc::channel(1);
    state.write().await.insert(key.clone(), sender);
    {
        let (mut outgoing, mut incoming) = (&mut ws).split();
        let out_ref = &mut outgoing;
        // wait for messages
        while let Ok(()) = select! {
            res = receiver.recv().then(|msg| async {
                // process message, send to
                if let Some(msg) = msg {
                    out_ref.send(ws::Message::text(msg.0)).await.map_err(|_|())
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
    println!("Client disconnected {:?}", remote);
    let _ = ws.close().await;
}

fn relay_client(addr: SocketAddr) {
    let (mut socket, _resp) =
        tungstenite::client::connect(format!("ws://{}/stream", addr)).unwrap();
    while let Ok(msg) = socket.read_message() {
        if let Ok(url) = msg.into_text() {
            launch(url)
        }
    }
}
