use std::convert::TryInto;

use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};

use noise_protocol::patterns::{noise_x, noise_xn};
use noise_protocol::HandshakeStateBuilder;
pub use noise_protocol::{HandshakeState, U8Array, DH};
pub use noise_rust_crypto::{Blake2b, ChaCha20Poly1305, X25519};

pub type Key = <X25519 as DH>::Key;
pub type Pubkey = <X25519 as DH>::Pubkey;

pub fn encrypt_client_message(
    msg: &str,
    client_key: Key,
    server_key: Pubkey,
    seq_num: u64,
) -> Result<Vec<u8>, noise_protocol::Error> {
    let mut msg_seq = vec![];
    msg_seq.write_u64::<NetworkEndian>(seq_num).unwrap();
    msg_seq.extend_from_slice(msg.as_bytes());
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
    let mut msg_crypt = handshake.write_message_vec(&msg_seq)?;
    msg_out.append(&mut msg_crypt);
    Ok(msg_out)
}

pub fn decrypt_client_message(
    msg: &[u8],
    server_key: Key,
) -> Result<(Pubkey, u64, Vec<u8>), noise_protocol::ErrorKind> {
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

    let mut ret = handshake.read_message_vec(msg).map_err(|e| e.kind())?;
    let seq = NetworkEndian::read_u64(&ret);
    let ret = ret.split_off(8);
    let client_key = handshake.get_rs().unwrap();
    Ok((client_key, seq, ret))
}

pub fn get_destination(msg: &[u8]) -> Option<Pubkey> {
    if msg.len() > 32 {
        msg[..32].try_into().ok()
    } else {
        None
    }
}

pub fn relay_client_handshake(
    static_key: Key,
) -> HandshakeState<X25519, ChaCha20Poly1305, Blake2b> {
    let mut handshake = HandshakeStateBuilder::new();
    handshake
        .set_pattern(noise_xn())
        .set_is_initiator(true)
        .set_s(static_key)
        .set_prologue(&[]);
    handshake.build_handshake_state()
}

pub fn relay_server_handshake() -> HandshakeState<X25519, ChaCha20Poly1305, Blake2b> {
    let mut handshake = HandshakeStateBuilder::new();
    handshake
        .set_pattern(noise_xn())
        .set_is_initiator(false)
        .set_prologue(&[]);
    handshake.build_handshake_state()
}
