package com.fruit.wherever;

public class WhereverCrypto {
    private static native byte[] encryptMessage(String input, byte[] client_key, byte[] serverKey);
    private static native byte[] generateKey();

    static {
        System.loadLibrary("wherever_crypto_java");
    }

    public static byte[] genKey() {
        return generateKey();
    }
    public static byte[] encMsg(String input, byte[] client_key, byte[] serverKey) { return encryptMessage(input, client_key, serverKey); }
}
