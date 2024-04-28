use std::collections::{hash_map::Entry, HashMap};
use std::convert::TryInto;

use wasm_bindgen::prelude::*;

use wherever_crypto::{relay_client_handshake, Key, Pubkey};

use wherever_crypto::noise_protocol::{CipherState, HandshakeState, U8Array, DH};
use wherever_crypto::noise_rust_crypto::{Blake2b, ChaCha20Poly1305, X25519};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub struct JSState {
    key: Key,
    handshake: HandshakeState<X25519, ChaCha20Poly1305, Blake2b>,
    tofu: Tofu,
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
        let key = X25519::genkey();
        storage.set_item("key", &base64::encode(&*key));
        key
    }
}

fn load_tofu(storage: &web_sys::Storage) -> Tofu {
    storage
        .get_item("tofu")
        .unwrap()
        .map(|s| Tofu::read(&s))
        .unwrap_or(Tofu {
            allowed: HashMap::new(),
        })
}

#[wasm_bindgen]
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

#[wasm_bindgen]
pub fn new_state() -> JSState {
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    let key = load_key(&storage);
    let tofu = load_tofu(&storage);
    let handshake = relay_client_handshake(key.clone());
    JSState {
        key,
        handshake,
        tofu,
    }
}

#[wasm_bindgen]
impl JSState {
    pub fn first_message(&mut self) -> Vec<u8> {
        self.handshake.write_message_vec(&[]).unwrap()
    }
    pub fn response(&mut self, msg: &[u8]) {
        self.handshake.read_message_vec(msg).unwrap();
    }
    pub fn second_message(&mut self) -> Vec<u8> {
        self.handshake.write_message_vec(&[]).unwrap()
    }
    pub fn into_cipher(self) -> JSCipher {
        JSCipher {
            key: self.key,
            relay_cipher: self.handshake.get_ciphers().1,
            tofu: self.tofu,
        }
    }
}

#[wasm_bindgen]
pub struct JSCipher {
    key: Key,
    relay_cipher: CipherState<ChaCha20Poly1305>,
    tofu: Tofu,
}

struct Tofu {
    allowed: HashMap<Pubkey, u64>,
}

impl Tofu {
    fn read(buf: &str) -> Self {
        Self {
            allowed: buf
                .lines()
                .flat_map(|line| {
                    let mut split = line.split(",");
                    let rest = split.next()?;
                    let seq: &str = split.next()?;
                    Some((
                        (&base64::decode(rest).ok()?[..]).try_into().ok()?,
                        seq.parse().ok()?,
                    ))
                })
                .collect(),
        }
    }
    fn write(&self) -> String {
        let mut out = String::new();
        for (key, seq) in self.allowed.iter().map(|(k, seq)| (base64::encode(k), seq)) {
            let s = format!("{},{}\n", key, seq);
            out.push_str(&s);
        }
        out
    }
    fn save(&mut self) {
        let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
        storage.set_item("tofu", &self.write());
    }
    fn validate_seq_not_tofu(&mut self, key: &Pubkey, seq: u64) -> bool {
        match self.allowed.entry(*key) {
            Entry::Occupied(mut o) => {
                let last_seq = o.get_mut();
                if *last_seq < seq {
                    *last_seq = seq;
                    true
                } else {
                    false
                }
            }
            Entry::Vacant(v) => {
                v.insert(seq);
                true
            }
        }
    }
}

#[wasm_bindgen]
impl JSCipher {
    pub fn decrypt_message(&mut self, msg: &[u8]) -> String {
        let relay_dec = self.relay_cipher.decrypt_vec(msg);
        let relay_dec = relay_dec.unwrap();
        if let Ok((client_key, seq, dec_msg)) =
            wherever_crypto::decrypt_client_message(&relay_dec, self.key.clone())
        {
            if self.tofu.validate_seq_not_tofu(&client_key, seq) {
                self.tofu.save();
                let s = String::from_utf8(dec_msg);
                s.unwrap()
            } else {
                panic!("REEEE")
            }
        } else {
            panic!()
        }
    }
}
