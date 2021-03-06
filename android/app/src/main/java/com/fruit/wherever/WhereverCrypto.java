package com.fruit.wherever;

public class WhereverCrypto {
    private static native byte[] encryptMessage(String input, byte[] client_key, byte[] serverKey, long sequence);
    private static native byte[] generateKey();
    private static native byte[] getPubkey(byte[] client_key);
    //https://stackoverflow.com/questions/24357687/how-to-include-so-library-in-android-studio
    static {
        System.loadLibrary("wherever_crypto_java");
    }

    public static byte[] genKey() {
        return generateKey();
    }
    public static byte[] encMsg(String input, byte[] client_key, byte[] serverKey, long sequence) { return encryptMessage(input, client_key, serverKey, sequence); }
    public static byte[] getPub(byte[] client_key) {
        return getPubkey(client_key);
    }
}
