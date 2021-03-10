use std::convert::TryInto;

use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};

pub use noise_protocol;
pub use noise_rust_crypto;

use noise_protocol::patterns::{noise_x, noise_xn, noise_xx};
use noise_protocol::HandshakeStateBuilder;
use noise_protocol::{HandshakeState, DH};
use noise_rust_crypto::{Blake2b, ChaCha20Poly1305, X25519};

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
    let client_key = handshake.get_rs().ok_or(noise_protocol::ErrorKind::DH)?;
    // this is probably the wrong error but whatever
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

pub fn discovery_receiver_handshake(
    static_key: Key,
    words: &str,
) -> HandshakeState<X25519, ChaCha20Poly1305, Blake2b> {
    let mut handshake = HandshakeStateBuilder::new();
    handshake
        .set_pattern(noise_xx())
        .set_is_initiator(true)
        .set_prologue(words.as_bytes())
        .set_s(static_key);
    handshake.build_handshake_state()
}

pub fn discovery_sender_handshake(
    static_key: Key,
    words: &str,
) -> HandshakeState<X25519, ChaCha20Poly1305, Blake2b> {
    let mut handshake = HandshakeStateBuilder::new();
    handshake
        .set_pattern(noise_xx())
        .set_is_initiator(false)
        .set_prologue(words.as_bytes())
        .set_s(static_key);
    handshake.build_handshake_state()
}

pub struct Initiator;
pub struct Responder;
pub struct DiscoveryProtocol<Side> {
    handshake: HandshakeState<X25519, ChaCha20Poly1305, Blake2b>,
    _side: Side,
}

impl DiscoveryProtocol<Initiator> {
    pub fn initiator(static_key: Key, words: &str) -> (Self, Vec<u8>) {
        let mut handshake = HandshakeStateBuilder::new();
        handshake
            .set_pattern(noise_xx())
            .set_is_initiator(true)
            .set_prologue(words.as_bytes())
            .set_s(static_key);
        let mut handshake = handshake.build_handshake_state();
        let msg = handshake.write_message_vec(&[]).unwrap();
        (
            Self {
                handshake,
                _side: Initiator,
            },
            msg,
        )
    }
    pub fn read_message(mut self, msg: &[u8]) -> Option<(Pubkey, Vec<u8>)> {
        self.handshake.read_message_vec(msg).ok()?;
        let msg = self.handshake.write_message_vec(&[]).ok()?;
        Some((self.handshake.get_rs()?, msg))
    }
}

impl DiscoveryProtocol<Responder> {
    pub fn responder(static_key: Key, words: &str, msg: &[u8]) -> Option<(Self, Vec<u8>)> {
        let mut handshake = HandshakeStateBuilder::new();
        handshake
            .set_pattern(noise_xx())
            .set_is_initiator(false)
            .set_prologue(words.as_bytes())
            .set_s(static_key);
        let mut handshake = handshake.build_handshake_state();
        handshake.read_message_vec(msg).ok()?;
        let msg = handshake.write_message_vec(&[]).unwrap();
        Some((
            Self {
                handshake,
                _side: Responder,
            },
            msg,
        ))
    }
    pub fn read_message(mut self, msg: &[u8]) -> Option<Pubkey> {
        self.handshake.read_message_vec(msg).ok()?;
        Some(self.handshake.get_rs()?)
    }
}
