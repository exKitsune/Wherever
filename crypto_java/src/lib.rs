use std::convert::TryInto;

use jni::objects::{JClass, JString, JValue};
use jni::sys::jbyteArray;

use jni::JNIEnv;

use wherever_crypto::{Key, Pubkey};
use wherever_crypto::{U8Array, DH, X25519};

#[no_mangle]
pub extern "system" fn Java_com_fruit_wherever_WhereverCrypto_encryptMessage(
    env: JNIEnv,
    _class: JClass,
    input: JString,
    client_key: jbyteArray,
    server_key: jbyteArray,
) -> jbyteArray {
    let input: String = env
        .get_string(input)
        .expect("Couldn't get java string")
        .into();

    let client_key = env.convert_byte_array(client_key).unwrap();
    let client_key = Key::from_slice(&client_key);
    let server_key = env
        .convert_byte_array(server_key)
        .unwrap()
        .try_into()
        .unwrap();

    let msg = wherever_crypto::encrypt_client_message(&input, client_key, server_key).unwrap();
    let output = env.byte_array_from_slice(&msg).unwrap();

    output
}

#[no_mangle]
pub extern "system" fn Java_com_fruit_wherever_WhereverCrypto_generateKey(env: JNIEnv, _class: JClass) -> jbyteArray {
    let key = X25519::genkey();
    println!("ur mom ");
    println!("{:?}", &*key);
    env.byte_array_from_slice(&*key).unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
