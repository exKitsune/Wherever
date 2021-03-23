use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::convert::TryInto;
use std::rc::Rc;

use futures::stream::StreamExt;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::{Document, Window};

use websocket_wasm::WebSocket;

use wherever::{
    initiate_discovery, relay_reciever, respond_discovery, Tofu, TofuEntry, TofuStorage,
    UntrustedEntry,
};
use wherever_crypto::{Key, Pubkey};

use wherever_crypto::noise_protocol::{U8Array, DH};
use wherever_crypto::noise_rust_crypto::X25519;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(main())
}

async fn main() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let storage = window.local_storage().unwrap().unwrap();
    let key = load_key(&storage);
    setup_sender(
        window.clone(),
        document.clone(),
        storage.clone(),
        key.clone(),
    )
    .await;
    setup_settings(window.clone(), document.clone(), storage.clone()).await;

    let mut tofu = Tofu::load(LocalStorage(storage, "tofu".into()));
    let tofu_list = Rc::new(TofuList::new(&document, "tofu_list", "tofu_list_template"));
    let accepted = Rc::new(RefCell::new(Vec::<UntrustedEntry>::new()));
    let discovered = Rc::new(RefCell::new(Vec::<Pubkey>::new()));

    let qr = qr_code(&key, window.location());
    let qr_element = document.get_element_by_id("qrcode").unwrap();
    let mut qr_html = qr_element.inner_html();
    qr_html.push_str(&qr);
    qr_element.set_inner_html(&qr_html);

    for delay in (0..20).chain(std::iter::repeat(20)).map(|i| 1 << i) {
        // This might leak memory from repeatedly registering event handlers each loop
        if let None = receiver(
            window.clone(),
            document.clone(),
            key.clone(),
            &mut tofu,
            tofu_list.clone(),
            accepted.clone(),
            discovered.clone(),
        )
        .await
        {
            console_log!("ERROR ENCOUNTERED");
            break;
            // TODO: display error message, possibly reload page
        }

        console_log!("AAAAAA WE FINISHED: {}", delay);
    }
}

async fn receiver(
    window: Window,
    document: Document,
    key: Key,
    mut tofu: &mut Tofu<LocalStorage>,
    tofu_list: Rc<TofuList>,
    accepted: Rc<RefCell<Vec<UntrustedEntry>>>,
    discovered: Rc<RefCell<Vec<Pubkey>>>,
) -> Option<()> {
    let location = window.location();
    let protocol = match &*location.protocol().unwrap() {
        "http:" => "ws://",
        _ => "wss://",
    };
    let host = location.host().unwrap();
    let discover_url = format!("{}{}/discover", protocol, host);
    add_onclick(&document, "discover", {
        let document = document.clone();
        let key = key.clone();
        let discovered = discovered.clone();
        move |_e| {
            let key = key.clone();
            let url = discover_url.clone();
            let document = document.clone();
            let discovered = discovered.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some((phrase, f)) = initiate_discovery::<WebSocket, _>(key, url).await {
                    let discover_text: web_sys::HtmlElement = document
                        .get_element_by_id("discover_phrase")
                        .unwrap()
                        .dyn_into()
                        .unwrap();
                    discover_text.set_inner_html(&phrase);
                    if let Some(key) = f.await {
                        discover_text.set_inner_html(&"");
                        discovered.borrow_mut().push(key);
                    }
                }
            });
        }
    });

    let socket = WebSocket::new(&format!("{}{}/stream", protocol, host))
        .await
        .ok()?;
    let mut stream = relay_reciever(socket, key.clone()).await?;

    while let Some(msg) = stream.next().await {
        for e in accepted.borrow_mut().drain(..) {
            e.trust(&mut tofu);
        }
        for key in discovered.borrow_mut().drain(..) {
            tofu.trust(key);
        }
        tofu.save().unwrap();
        if let Some(msg) = match tofu.decrypt_message(&key, &msg) {
            Ok(TofuEntry::Untrusted(entry)) => {
                let entry = Rc::new(std::cell::RefCell::new(Some(entry)));
                tofu_list.add(
                    entry.borrow().as_ref().unwrap(),
                    {
                        let entry = entry.clone();
                        let tofu_list = tofu_list.clone();
                        let accepted = accepted.clone();
                        let window = window.clone();
                        move || {
                            if let Some(entry) = entry.take() {
                                tofu_list.remove(&entry);
                                window
                                    .open_with_url_and_target_and_features(
                                        entry.peek(),
                                        "_blank",
                                        "noreferrer,noopener",
                                    )
                                    .unwrap();
                                accepted.borrow_mut().push(entry);
                            }
                            // handle success
                        }
                    },
                    {
                        let entry = entry.clone();
                        let tofu_list = tofu_list.clone();
                        move || {
                            if let Some(entry) = entry.take() {
                                tofu_list.remove(&entry);
                            }
                            // handle failure
                        }
                    },
                );
                None
                //prompt_async(channel.clone(), entry).await.unwrap_or(None)
            }
            Ok(TofuEntry::Trusted(msg)) => Some(msg),
            Err(()) => None,
        } {
            window
                .open_with_url_and_target_and_features(&msg, "_blank", "noreferrer,noopener")
                .unwrap();
        }
    }
    Some(())
}

struct TofuList {
    list_div: web_sys::HtmlElement,
    template: web_sys::HtmlElement,
}

impl TofuList {
    fn new(document: &web_sys::Document, list_div_id: &str, template_id: &str) -> Self {
        let list_div: web_sys::HtmlElement = document
            .get_element_by_id(list_div_id)
            .unwrap()
            .dyn_into()
            .unwrap();
        let template: web_sys::HtmlElement = document
            .get_element_by_id(template_id)
            .unwrap()
            .dyn_into()
            .unwrap();
        let template = list_div
            .remove_child(&template)
            .unwrap()
            .dyn_into()
            .unwrap();
        Self { list_div, template }
    }
    fn add(
        &self,
        entry: &UntrustedEntry,
        yes: impl FnOnce() + 'static,
        no: impl FnOnce() + 'static,
    ) {
        let template: web_sys::HtmlElement = self
            .template
            .clone_node_with_deep(true)
            .unwrap()
            .dyn_into()
            .unwrap();
        let children = template.child_nodes();
        let key = base64::encode(entry.key);
        let mut yes = Some(yes);
        let mut no = Some(no);
        for elem in (0..children.length())
            .into_iter()
            .flat_map(|i| children.get(i)?.dyn_into::<web_sys::HtmlElement>().ok())
        {
            match elem.id().as_str() {
                "yes" => {
                    yes.take()
                        .map(|yes| elem.set_onclick(Some(&Closure::once_into_js(yes).into())));
                }
                "no" => {
                    no.take()
                        .map(|no| elem.set_onclick(Some(&Closure::once_into_js(no).into())));
                }
                "key" => {
                    elem.set_id(&key);
                    elem.set_inner_html(&key);
                }
                _ => {}
            }
            // todo
        }
        template.set_id(&base64::encode(entry.key));
        self.list_div.append_child(&template).unwrap();
    }
    fn remove(&self, entry: &UntrustedEntry) {
        let children = self.list_div.child_nodes();
        let key = base64::encode(entry.key);
        let elems: Vec<_> = (0..children.length())
            .into_iter()
            .flat_map(|i| children.get(i)?.dyn_into::<web_sys::HtmlElement>().ok())
            .filter(|elem| elem.id().as_str() == &key)
            .collect();
        for elem in elems {
            self.list_div.remove_child(&elem).unwrap();
        }
    }
}

struct LocalStorage(web_sys::Storage, String);
use std::io;
use std::sync::atomic::AtomicU64;

impl TofuStorage for LocalStorage {
    fn save<I: Iterator<Item = String>>(&mut self, allowed: &mut I) -> io::Result<()> {
        let out: String = allowed.collect();
        self.0
            .set_item(&self.1, &out)
            .map_err(|_| io::ErrorKind::UnexpectedEof.into())
    }
    fn load<F: Fn(&str) -> Option<(Pubkey, AtomicU64)>>(
        &mut self,
        f: F,
    ) -> io::Result<HashMap<Pubkey, AtomicU64>> {
        Ok(self
            .0
            .get_item(&self.1)
            .ok()
            .flatten()
            .ok_or(io::ErrorKind::UnexpectedEof)?
            .lines()
            .flat_map(|l| f(l))
            .collect())
    }
}

fn load_key(storage: &web_sys::Storage) -> Key {
    if let Some(key) = storage
        .get_item("key")
        .ok()
        .flatten()
        .and_then(|key| base64::decode(key).ok())
        .map(|k| Key::from_slice(&*k))
    {
        key
    } else {
        console_log!("MAKING NEW KEY");
        let key = X25519::genkey();
        storage.set_item("key", &base64::encode(&*key)).unwrap();
        key
    }
}

fn clear_key(storage: &web_sys::Storage) {
    storage.remove_item("key").unwrap();
}
fn clear_tofu(storage: &web_sys::Storage) {
    storage.remove_item("tofu").unwrap();
}

pub fn qr_code(key: &Key, location: web_sys::Location) -> String {
    use qrcode::{render::svg, QrCode};
    let pubkey = X25519::pubkey(key);
    let protocol = match &*location.protocol().unwrap() {
        "http:" => "where",
        _ => "wheres",
    };
    let host = location.host().unwrap();
    let code = QrCode::new(format!(
        "{}://{}/#{}",
        protocol,
        host,
        base64::encode(pubkey)
    ))
    .unwrap();
    let string = code
        .render()
        .min_dimensions(300, 300)
        .dark_color(svg::Color("#000000FF"))
        .light_color(svg::Color("#FFFFFF00"))
        .build();
    string
}

async fn setup_sender(window: Window, document: Document, storage: web_sys::Storage, key: Key) {
    let location = window.location();
    let host = location.host().unwrap();
    let ws_protocol = match &*location.protocol().unwrap() {
        "http:" => "ws://",
        _ => "wss://",
    };
    let discover_url = format!("{}{}/discover", ws_protocol, host);

    let target_input: web_sys::HtmlInputElement = document
        .get_element_by_id("send_target")
        .unwrap()
        .dyn_into()
        .unwrap();

    let target = storage
        .get_item("target")
        .ok()
        .flatten()
        .and_then(|x| base64::decode(x).ok())
        .and_then(|x| x.try_into().ok());
    if let Some(key) = target.as_ref() {
        target_input.set_value(&base64::encode(key));
    }
    let send_target = Rc::new(Cell::new(target));

    add_onclick(&document, "discover_respond", {
        let document = document.clone();
        let send_target = send_target.clone();
        let key = key.clone();
        let target_input = target_input.clone();
        let storage = storage.clone();
        move |_e| {
            let send_target = send_target.clone();
            let key = key.clone();
            let target_input = target_input.clone();
            let url = discover_url.clone();
            let document = document.clone();
            let storage = storage.clone();
            let phrase: web_sys::HtmlInputElement = document
                .get_element_by_id("discover_input")
                .unwrap()
                .dyn_into()
                .unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                let phrase = phrase.value();
                console_log!("phrase {}", &phrase);
                if let Some((channel, phrase)) = wherever::parse_phrase(&phrase) {
                    console_log!("phraseeee {}", &phrase);
                    if let Some(key) = respond_discovery::<WebSocket, _>(
                        key,
                        format!("{}/{}", url, channel),
                        phrase,
                    )
                    .await
                    {
                        target_input.set_value(&base64::encode(key));
                        set_target(&storage, &send_target, key);
                    }
                }
            });
        }
    });
    add_onclick(&document, "send_update", {
        let send_target = send_target.clone();
        move |_e| {
            let send_target = send_target.clone();
            let target_input = target_input.clone();
            let storage = storage.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(key) = base64::decode(target_input.value())
                    .ok()
                    .and_then(|x| x.try_into().ok())
                {
                    set_target(&storage, &send_target, key);
                }
            });
        }
    });
    add_onclick(&document, "send", {
        let document = document.clone();
        move |_e| {
            let send_target = send_target.clone();
            let window = window.clone();
            let key = key.clone();
            let link: web_sys::HtmlInputElement = document
                .get_element_by_id("send_input")
                .unwrap()
                .dyn_into()
                .unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(server_key) = send_target.get() {
                    send_link(&window, &link.value(), &key, &server_key).await;
                }
            });
        }
    });
}

async fn setup_settings(window: Window, document: Document, storage: web_sys::Storage) {
    let location = window.location();
    add_onclick(&document, "reset_key", {
        let storage = storage.clone();
        let location = location.clone();
        move |_e| {
            let storage = storage.clone();
            clear_key(&storage);
            location.reload().unwrap();
        }
    });
    add_onclick(&document, "reset_tofu", {
        move |_e| {
            let storage = storage.clone();
            clear_tofu(&storage);
            location.reload().unwrap();
        }
    });
}

fn add_onclick<F: FnMut(JsValue) + 'static>(document: &web_sys::Document, element_id: &str, f: F) {
    let element: web_sys::HtmlElement = document
        .get_element_by_id(element_id)
        .unwrap()
        .dyn_into()
        .unwrap();

    let a = Box::new(f) as Box<dyn FnMut(_)>;
    element.set_onclick(Some(&Closure::wrap(a).into_js_value().into()));
}

fn set_target(
    storage: &web_sys::Storage,
    send_target: &Rc<Cell<Option<Pubkey>>>,
    new_target: Pubkey,
) {
    storage
        .set_item("target", &base64::encode(new_target))
        .unwrap();
    send_target.set(Some(new_target));
}

async fn send_link(
    window: &web_sys::Window,
    link: &str,
    client_key: &Key,
    server_key: &Pubkey,
) -> Option<()> {
    let storage = window.local_storage().unwrap().unwrap();
    let seq = storage
        .get_item("seq")
        .ok()
        .flatten()
        .and_then(|x| x.parse().ok())
        .unwrap_or(1);
    let body =
        wherever_crypto::encrypt_client_message(link, client_key.clone(), server_key.clone(), seq)
            .ok()?;
    storage.set_item("seq", &format!("{}", seq + 1)).unwrap();
    wasm_bindgen_futures::JsFuture::from(
        window.fetch_with_str_and_init(
            "open",
            web_sys::RequestInit::new()
                .body(Some(&*js_sys::Uint8Array::from(&*body)))
                .method("POST"),
        ),
    )
    .await
    .ok()
    .map(|_| ())
}
