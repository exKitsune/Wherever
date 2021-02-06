import java.util.Arrays;

class WhereverCrypto {
    private static native byte[] encryptMessage(String input, byte[] client_key, byte[] serverKey);
    private static native byte[] generateKey();

    static {
        System.loadLibrary("wherever_crypto_java");
    }

    public static void main(String[] args) {
        byte[] output = WhereverCrypto.generateKey();
        System.out.println(Arrays.toString(output));
    }
}
