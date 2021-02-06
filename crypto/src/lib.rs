use std::convert::TryInto;

use noise_protocol::patterns::noise_x;
use noise_protocol::{HandshakeState, HandshakeStateBuilder};
pub use noise_protocol::{U8Array, DH};
pub use noise_rust_crypto::X25519;
use noise_rust_crypto::{Blake2b, ChaCha20Poly1305};

pub type Key = <X25519 as DH>::Key;
pub type Pubkey = <X25519 as DH>::Pubkey;

pub fn encrypt_client_message(
    msg: &str,
    client_key: Key,
    server_key: Pubkey,
) -> Result<Vec<u8>, noise_protocol::Error> {
    //let mut handshake = make_handshake(true, server_key);
    let mut handshake = HandshakeStateBuilder::new();
    handshake
        .set_pattern(noise_x())
        .set_is_initiator(true)
        .set_prologue(&server_key)
        .set_s(client_key)
        .set_rs(server_key);
    let mut handshake: HandshakeState<X25519, ChaCha20Poly1305, Blake2b> =
        handshake.build_handshake_state();
    let mut msg_out = vec![];
    msg_out.extend(&server_key);
    let mut msg_crypt = handshake.write_message_vec(msg.as_bytes())?;
    msg_out.append(&mut msg_crypt);
    Ok(msg_out)
}

pub fn decrypt_client_message(
    msg: &[u8],
    server_key: Key,
) -> Result<Vec<u8>, noise_protocol::ErrorKind> {
    if 32 > msg.len() {
        return Err(noise_protocol::ErrorKind::TooShort);
    }
    let (_prologue, msg) = msg.split_at(32);

    let pubkey = X25519::pubkey(&server_key);
    let mut handshake = HandshakeStateBuilder::new();
    handshake
        .set_pattern(noise_x())
        .set_is_initiator(false)
        .set_prologue(&pubkey)
        .set_s(server_key);
    let mut handshake: HandshakeState<X25519, ChaCha20Poly1305, Blake2b> =
        handshake.build_handshake_state();

    let ret = handshake.read_message_vec(msg).map_err(|e| e.kind());
    println!("Decrypted message from {:?}", handshake.get_rs());
    ret
}

pub fn get_destination(msg: &[u8]) -> Option<Pubkey> {
    if msg.len() > 32 {
        msg[..32].try_into().ok()
    } else {
        None
    }
}
