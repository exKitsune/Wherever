use std::convert::TryInto;
use std::io;

use noise_protocol::DH;
use noise_rust_crypto::X25519;
use server::{encrypt_client_message, Pubkey};

fn main() -> io::Result<()> {
    let server_key = base64::decode(std::env::args().skip(1).next().unwrap()).unwrap();
    let server_key = server_key[..32].try_into().unwrap();
    let client_static_key = X25519::genkey();
    println!("{:?}", X25519::pubkey(&client_static_key));
    let client = reqwest::blocking::Client::new();
    let message = encrypt_client_message(
        "https://www.youtube.com/watch?v=mZ0sJQC8qkE",
        client_static_key,
        server_key,
    )
    .unwrap();
    let addr = std::env::args()
        .skip(2)
        .next()
        .unwrap_or("127.0.0.1:8998".into());
    let resp = client
        .post(
            format!("http://{}/open", addr)
                .parse::<reqwest::Url>()
                .unwrap(),
        )
        .body(message)
        .send()
        .unwrap();
    Ok(())
}
