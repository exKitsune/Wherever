use std::cell::{Cell, RefCell};
use std::collections::{hash_map::Entry, HashMap};
use std::convert::TryInto;
use std::future::Future;
use std::rc::Rc;

use futures::lock::Mutex;
use futures::pin_mut;
use futures::stream::{Stream, StreamExt};
use futures::FutureExt;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::{Document, Storage, Window};

use websocket_wasm::{Message, WebSocket};

use wherever::{
    initiate_discovery, relay_reciever, respond_discovery, Tofu, TofuEntry, TofuStorage,
    UntrustedEntry,
};
use wherever_crypto::{relay_client_handshake, DiscoveryProtocol, Key, Pubkey};

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
    receiver(window, document, storage, key.clone()).await
}
async fn receiver(window: Window, document: Document, storage: web_sys::Storage, key: Key) {
    let mut tofu = Tofu::load(LocalStorage(storage, "tofu".into()));
    let tofu_list = Rc::new(TofuList::new(&document, "tofu_list", "tofu_list_template"));
    let accepted = Rc::new(RefCell::new(Vec::<UntrustedEntry>::new()));
    let discovered = Rc::new(RefCell::new(Vec::<Pubkey>::new()));

    let qr = qr_code();
    let qr_element = document.get_element_by_id("qrcode").unwrap();
    let mut qr_html = qr_element.outer_html();
    qr_html.push_str(&qr);
    qr_element.set_outer_html(&qr_html);

    let location = window.location();
    let protocol = match &*location.protocol().unwrap() {
        "http:" => "ws://",
        _ => "wss://",
    };
    let host = location.host().unwrap();
    let discover_url = format!("{}{}/discover", protocol, host);
    let discover_button: web_sys::HtmlElement = document
        .get_element_by_id("discover")
        .unwrap()
        .dyn_into()
        .unwrap();
    {
        let discover_url = discover_url.clone();
        let document = document.clone();
        let discovered = discovered.clone();
        let k2 = key.clone();
        let a = Box::new(move |_e: JsValue| {
            let k3 = k2.clone();
            let url = discover_url.clone();
            let document = document.clone();
            let discovered = discovered.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some((phrase, f)) = initiate_discovery::<WebSocket, _>(k3, url).await {
                    let discover_text: web_sys::HtmlElement = document
                        .get_element_by_id("discover_phrase")
                        .unwrap()
                        .dyn_into()
                        .unwrap();
                    discover_text.set_inner_html(&phrase);
                    let key = f.await.unwrap();
                    discover_text.set_inner_html(&"");
                    discovered.borrow_mut().push(key);
                }
            });
        }) as Box<dyn FnMut(_)>;
        discover_button.set_onclick(Some(&Closure::wrap(a).into_js_value().into()));
    }

    let socket = WebSocket::new(&format!("{}{}/stream", protocol, host))
        .await
        .unwrap();
    let mut stream = relay_reciever(socket, key.clone()).await.unwrap();

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
        self.list_div.append_child(&template);
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
            self.list_div.remove_child(&elem);
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
        .unwrap()
        .and_then(|key| base64::decode(key).ok())
        .map(|k| Key::from_slice(&*k))
    {
        key
    } else {
        console_log!("MAKING NEW KEY");
        let key = X25519::genkey();
        storage.set_item("key", &base64::encode(&*key));
        key
    }
}

pub fn qr_code() -> String {
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    let key = load_key(&storage);
    let pubkey = X25519::pubkey(&key);
    use qrcode::{render::svg, QrCode};
    let code = QrCode::new(format!("where://10.9.0.3:3543/#{}", base64::encode(pubkey))).unwrap();
    let string = code
        .render()
        .min_dimensions(300, 300)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#FFFFFF"))
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

    {
        let discover_button: web_sys::HtmlElement = document
            .get_element_by_id("discover_respond")
            .unwrap()
            .dyn_into()
            .unwrap();
        let document = document.clone();
        let storage = storage.clone();
        let k2 = key.clone();
        let send_target = send_target.clone();
        let target_input = target_input.clone();
        let a = Box::new(move |_e: JsValue| {
            let send_target = send_target.clone();
            let k3 = k2.clone();
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
                        k3,
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
        }) as Box<dyn FnMut(_)>;
        discover_button.set_onclick(Some(&Closure::wrap(a).into_js_value().into()));
    }
    {
        let update_target: web_sys::HtmlElement = document
            .get_element_by_id("send_update")
            .unwrap()
            .dyn_into()
            .unwrap();
        let target_input = target_input.clone();
        let storage = storage.clone();
        let send_target = send_target.clone();
        let a = Box::new(move |_e: JsValue| {
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
        }) as Box<dyn FnMut(_)>;
        update_target.set_onclick(Some(&Closure::wrap(a).into_js_value().into()));
    }
    {
        let send_button: web_sys::HtmlElement = document
            .get_element_by_id("send")
            .unwrap()
            .dyn_into()
            .unwrap();
        let document = document.clone();
        let k2 = key.clone();
        let a = Box::new(move |_e: JsValue| {
            let send_target = send_target.clone();
            let k3 = k2.clone();
            let document = document.clone();
            let window = window.clone();
            let link: web_sys::HtmlInputElement = document
                .get_element_by_id("send_input")
                .unwrap()
                .dyn_into()
                .unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(server_key) = send_target.get() {
                    send_link(&window, &link.value(), &k3, &server_key).await;
                }
            });
        }) as Box<dyn FnMut(_)>;
        send_button.set_onclick(Some(&Closure::wrap(a).into_js_value().into()));
    }
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

async fn send_link(window: &web_sys::Window, link: &str, client_key: &Key, server_key: &Pubkey) {
    let storage = window.local_storage().unwrap().unwrap();
    let seq = storage
        .get_item("seq")
        .ok()
        .flatten()
        .and_then(|x| x.parse().ok())
        .unwrap_or(0);
    let body =
        wherever_crypto::encrypt_client_message(link, client_key.clone(), server_key.clone(), seq)
            .unwrap();
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
    .unwrap();
}
