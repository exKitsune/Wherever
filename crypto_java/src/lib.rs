use std::convert::TryInto;

use jni::objects::{JClass, JString, JValue};
use jni::sys::{jbyteArray, jlong};

use jni::JNIEnv;

use wherever_crypto::noise_protocol::{U8Array, DH};
use wherever_crypto::noise_rust_crypto::X25519;
use wherever_crypto::{Key, Pubkey};

// These function definitions come from running javah WhereverCrypto

#[no_mangle]
pub extern "system" fn Java_com_fruit_wherever_WhereverCrypto_encryptMessage(
    env: JNIEnv,
    _class: JClass,
    input: JString,
    client_key: jbyteArray,
    server_key: jbyteArray,
    seq: jlong,
) -> jbyteArray {
    let input: String = env
        .get_string(input)
        .expect("Couldn't get java string")
        .into();
    let seq = seq as u64;

    let client_key = env.convert_byte_array(client_key).unwrap();
    let client_key = Key::from_slice(&client_key);
    let server_key = env
        .convert_byte_array(server_key)
        .unwrap()
        .try_into()
        .unwrap();

    let msg = wherever_crypto::encrypt_client_message(&input, client_key, server_key, seq).unwrap();
    let output = env.byte_array_from_slice(&msg).unwrap();

    output
}

#[no_mangle]
pub extern "system" fn Java_com_fruit_wherever_WhereverCrypto_generateKey(
    env: JNIEnv,
    _class: JClass,
) -> jbyteArray {
    let key = X25519::genkey();
    env.byte_array_from_slice(&*key).unwrap()
}

#[no_mangle]
pub extern "system" fn Java_com_fruit_wherever_WhereverCrypto_getPubkey(
    env: JNIEnv,
    _class: JClass,
    client_key: jbyteArray,
) -> jbyteArray {
    let client_key = env.convert_byte_array(client_key).unwrap();
    let client_key = Key::from_slice(&client_key);
    let pubkey = X25519::pubkey(&client_key);
    env.byte_array_from_slice(&pubkey).unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
