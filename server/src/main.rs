use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, BufRead, Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::thread;

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use tokio_tungstenite::tungstenite::http::Uri;
use warp::ws::{self, WebSocket};
use warp::Filter;

use futures::{select, FutureExt, SinkExt, StreamExt};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot, RwLock};

use wherever_crypto::noise_protocol::{HandshakeState, U8Array, DH};
use wherever_crypto::noise_rust_crypto::{Blake2b, ChaCha20Poly1305, X25519};
use wherever_crypto::{Key, Pubkey};

use wherever::{Tofu, TofuEntry, TofuStorage, UntrustedEntry};

fn main() {
    let mut opts = getopts::Options::new();
    opts.optflag(
        "s",
        "standalone",
        "Run in standalone server mode.
        (may be combined with -c)",
    );
    opts.optopt(
        "c",
        "relay_client",
        "Run in relay client mode.
        (may be combined with -s)",
        "relay_uri",
    );
    opts.optflag("r", "relay", "Run in relay server mode");
    opts.optopt(
        "a",
        "address",
        "Address to listen on for -s or -r
        Defaults to 0.0.0.0:8998",
        "listen_address",
    );
    opts.optopt(
        "k",
        "server_key",
        "File the server's private key should be stored in.
        Defaults to \"server_key\"",
        "key_file",
    );
    opts.optopt(
        "l",
        "allowed_list",
        "File the list of allowed clients should be stored in.
        Defaults to \"allowed.txt\"",
        "list_file",
    );
    opts.optflag("h", "help", "Show this help message");
    if let Ok(matches) = opts.parse(std::env::args_os()) {
        if !matches.opt_present("h") {
            return start(matches);
        }
    }

    let program_name = std::env::args().next().unwrap_or("".to_owned());
    println!("{}", opts.short_usage(&program_name));

    println!(
        "{}",
        opts.usage("\nDefaults to standalone mode if no mode args are passed.")
    );
}

fn start(matches: getopts::Matches) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let bind_addr = matches
        .opt_get_default("a", SocketAddr::from(([0, 0, 0, 0], 8998)))
        .unwrap();

    if matches.opt_present("r") {
        rt.block_on(relay(bind_addr));
    } else {
        let (send, recv) = mpsc::channel(10);
        let standalone_explicit = matches.opt_present("s");

        let keyfile = matches.opt_str("k").unwrap_or("server_key".to_owned());
        let key =
            load_server_key(keyfile).expect("Invalid server key, check if keyfile is correct");
        let pubkey = base64::encode(X25519::pubkey(&key));
        let tofu_file = matches
            .opt_str("allowed_list")
            .unwrap_or("allowed.txt".to_owned());
        let tofu = Arc::new(RwLock::new(Tofu::load(tofu_file)));

        let relay_server_uri: Option<Uri> = matches.opt_get("c").unwrap();
        let launch_standalone = relay_server_uri.is_none() || standalone_explicit;

        if let Some(relay_server_uri) = relay_server_uri {
            println!("Launching relay client");
            let connect_url = format!(
                "where://{}/#{}",
                relay_server_uri
                    .authority()
                    .expect("Missing authority (host) in relay server uri"),
                pubkey
            );
            qr2term::print_qr(&connect_url).unwrap();
            println!("Connect over relay via: {}", connect_url);
            rt.spawn(relay_client(
                key.clone(),
                relay_server_uri,
                tofu.clone(),
                send.clone(),
            ));
        }
        if launch_standalone {
            println!("Launching standalone server");
            if bind_addr.ip() != IpAddr::from([0, 0, 0, 0]) {
                let connect_url = format!("where://{}/#{}", bind_addr, pubkey);
                qr2term::print_qr(&connect_url).unwrap();
                println!("Connect directly via: {}", connect_url);
            }
            rt.spawn(standalone(
                key.clone(),
                bind_addr,
                tofu.clone(),
                send.clone(),
            ));
        }
        prompt_user(rt, &*tofu, recv).unwrap()
    }
}

async fn standalone<S: Send + Sync + 'static>(
    key: Key,
    addr: SocketAddr,
    tofu: Arc<RwLock<Tofu<S>>>,
    channel: mpsc::Sender<(oneshot::Sender<Option<String>>, UntrustedEntry)>,
) {
    warp::serve(
        warp::path("open")
            .and(warp::post())
            .and(warp::body::content_length_limit(4096))
            .and(warp::body::bytes())
            .and_then(move |body: warp::hyper::body::Bytes| {
                let key = key.clone();
                let tofu = tofu.clone();
                let channel = channel.clone();
                async move {
                    if let Some(url) = match tofu.read().await.decrypt_message(&key, &body) {
                        Ok(TofuEntry::Untrusted(entry)) => {
                            prompt_async(channel, entry).await.unwrap_or(None)
                        }
                        Ok(TofuEntry::Trusted(msg)) => Some(msg),
                        Err(()) => None,
                    } {
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
    let discover = Arc::new(RwLock::new(DiscoveryTable::new(100)));
    let routes = warp::path("stream")
        .and(warp::ws())
        .map({
            let registry = registry.clone();
            move |ws: warp::ws::Ws| {
                let registry = registry.clone();
                ws.on_upgrade(move |websocket| handle_client(websocket, registry))
            }
        })
        .or(warp::path("discover")
            .and(warp::ws())
            .and(warp::path::tail())
            .map({
                let discover = discover.clone();
                move |ws: warp::ws::Ws, tail| {
                    let discover = discover.clone();
                    ws.on_upgrade(move |websocket| async {
                        discover_client(websocket, tail, discover).await;
                    })
                }
            }))
        .or(warp::path("open")
            .and(warp::post())
            .and(warp::body::content_length_limit(4096))
            .and(warp::body::bytes())
            .and_then(move |body: warp::hyper::body::Bytes| {
                let registry = registry.clone();
                async move {
                    if let Some(key) = wherever_crypto::get_destination(&body) {
                        if let Some(channel) = registry.read().await.get(&key) {
                            if let Ok(()) = channel.send(Message(body.to_vec())).await {
                                return Ok::<_, warp::Rejection>("Good");
                            }
                        }
                    }
                    Err(warp::reject::not_found())
                }
            }))
        .or(warp::path::full().and_then(serve_web_client));
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

async fn handle_client(mut ws: WebSocket, state: Arc<RwLock<Registry>>) {
    println!("Client connected");
    // get public key of client
    if let Some((key, mut cipher)) = handle_client_handshake(&mut ws)
        .await
        .and_then(|hs| hs.get_rs().map(|rs| (rs, hs.get_ciphers().1)))
    {
        println!("Client key: {:?}", base64::encode(&key));
        // register key + channel in shared state
        let (sender, mut receiver) = mpsc::channel(1);
        state.write().await.insert(key.clone(), sender);
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
    println!("Client disconnected");
    let _ = ws.close().await;
}

#[derive(Eq, PartialEq, Hash, Clone)]
struct TableIdx(u64);

type DiscoveryChannel = (
    Vec<u8>,
    oneshot::Sender<(Vec<u8>, oneshot::Sender<Vec<u8>>)>,
);

struct DiscoveryTable {
    map: HashMap<TableIdx, DiscoveryChannel>,
    slots: VecDeque<TableIdx>,
    rng: SmallRng,
}

impl DiscoveryTable {
    fn new(capacity: u64) -> Self {
        let mut slots: VecDeque<_> = (0..capacity).map(TableIdx).collect();
        let mut rng = SmallRng::from_entropy();
        // Shuffling it makes it more interesting
        slots.make_contiguous().shuffle(&mut rng);
        Self {
            map: HashMap::new(),
            slots,
            rng,
        }
    }
    fn reserve(&mut self, ch: DiscoveryChannel) -> Option<TableIdx> {
        if let Some(idx) = self.slots.pop_back() {
            self.map.insert(idx.clone(), ch);
            Some(idx)
        } else {
            None
        }
    }
    fn lookup(&mut self, idx: TableIdx) -> Option<DiscoveryChannel> {
        if let Some(a) = self.map.remove(&idx) {
            // incredible
            if self.rng.gen() {
                self.slots.push_back(idx);
            } else {
                self.slots.push_front(idx);
            }
            Some(a)
        } else {
            None
        }
    }
}

async fn discover_client(
    mut ws: WebSocket,
    tail: warp::path::Tail,
    state: Arc<RwLock<DiscoveryTable>>,
) -> Option<()> {
    match tail.as_str() {
        "" => {
            // Handle initiator
            let (sender, receiver) = oneshot::channel();
            let msg = ws.next().await?.ok()?.into_bytes(); // -> e
            let idx = state.write().await.reserve((msg, sender))?;
            ws.send(warp::ws::Message::text(format!("{}", idx.0))) // (<- idx)
                .await
                .ok()?;
            let (reply, sender) = receiver.await.ok()?;
            ws.send(warp::ws::Message::binary(reply)).await.ok()?; // <- e, ee, s, es
            let msg = ws.next().await?.ok()?.into_bytes(); // -> s, se
            sender.send(msg).ok()?;
            Some(())
        }
        tail => {
            // Handle responder
            let idx = TableIdx(tail.parse().ok()?);
            let (msg, sender) = state.write().await.lookup(idx)?;
            ws.send(warp::ws::Message::binary(msg)).await.ok()?; // -> e
            let (sender2, receiver2) = oneshot::channel();
            let reply = ws.next().await?.ok()?.into_bytes(); // <- e, ee, s, es
            sender.send((reply, sender2)).ok()?;
            let msg = receiver2.await.ok()?;
            ws.send(warp::ws::Message::binary(msg)).await.ok()?; // -> s, se
            Some(())
        }
    }
}

async fn serve_web_client(path: warp::path::FullPath) -> Result<impl warp::Reply, warp::Rejection> {
    match path.as_str() {
        "/" | "/index.html" => Ok(warp::reply::with_header(
            wherever_web_compiled::HTML,
            "Content-Type",
            "text/html; charset=UTF-8",
        )),
        "/wherever_web.js" => Ok(warp::reply::with_header(
            wherever_web_compiled::JS,
            "Content-Type",
            "text/javascript; charset=UTF-8",
        )),
        "/wherever_web_bg.wasm" => Ok(warp::reply::with_header(
            wherever_web_compiled::WASM,
            "Content-Type",
            "application/wasm",
        )),
        _ => Err(warp::reject()),
    }
}

async fn relay_client<S: Send + Sync>(
    key: Key,
    addr: Uri,
    tofu: Arc<RwLock<Tofu<S>>>,
    channel: mpsc::Sender<(oneshot::Sender<Option<String>>, UntrustedEntry)>,
) -> Option<()> {
    let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{}/stream", addr))
        .await
        .ok()?;
    let mut stream = wherever::relay_reciever(socket, key.clone()).await?;
    while let Some(msg) = stream.next().await {
        if let Some(msg) = match tofu.read().await.decrypt_message(&key, &msg) {
            Ok(TofuEntry::Untrusted(entry)) => {
                prompt_async(channel.clone(), entry).await.unwrap_or(None)
            }
            Ok(TofuEntry::Trusted(msg)) => Some(msg),
            Err(()) => None,
        } {
            launch(msg.to_owned());
        }
    }
    Some(())
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

async fn prompt_async(
    channel: mpsc::Sender<(oneshot::Sender<Option<String>>, UntrustedEntry)>,
    entry: UntrustedEntry,
) -> Result<Option<String>, ()> {
    let (send, recv) = oneshot::channel();
    channel.try_send((send, entry)).map_err(|_| ())?;
    Ok(recv.await.map_err(|_| ())?)
}

fn prompt_user<S: Send + Sync + TofuStorage>(
    rt: Runtime,
    tofu: &RwLock<Tofu<S>>,
    mut channel: mpsc::Receiver<(oneshot::Sender<Option<String>>, UntrustedEntry)>,
) -> io::Result<()> {
    let stdin_a = io::stdin();
    let mut stdin = stdin_a.lock();
    let mut line = String::new();
    while let Some((reply, entry)) = channel.blocking_recv() {
        if let Some(entry) = {
            match entry.check(&mut rt.block_on(tofu.write())) {
                Ok(res) => {
                    reply.send(res).map_err(|_| io::ErrorKind::BrokenPipe)?;
                    continue;
                }
                Err(entry) => loop {
                    println!(
                        "Incoming link from {}, accept? (Y/N): ",
                        base64::encode(entry.key)
                    );
                    stdin.read_line(&mut line)?;
                    match line.chars().next() {
                        Some('Y') => {
                            break Some(entry);
                        }
                        Some('N') => {
                            break None;
                        }
                        _ => {
                            println!("Please answer \"Y\" or \"N\"");
                        }
                    }
                },
            }
        } {
            let mut tofu = rt.block_on(tofu.write());
            if let Some(message) = entry.trust(&mut tofu) {
                tofu.save()?;
                reply
                    .send(Some(message))
                    .map_err(|_| io::ErrorKind::BrokenPipe)?;
            } else {
                reply.send(None).map_err(|_| io::ErrorKind::BrokenPipe)?;
            }
        } else {
            reply.send(None).map_err(|_| io::ErrorKind::BrokenPipe)?;
        }
    }
    Ok(())
}
