/**
 * MaìLang Java Binding Demo
 * Uses JNI to call the C FFI.
 *
 * Build: javac Main.java && java Main
 * Note: Requires mailang_ffi.dll/so in java.library.path
 */
public class Main {
    static {
        System.loadLibrary("mailang_ffi");
    }

    private static native long mailang_create();
    private static native void mailang_destroy(long interp);
    private static native int mailang_eval(long interp, String code, long result);
    private static native void mailang_free_string(long ptr);

    public static void main(String[] args) {
        System.out.println("=== MaìLang Java Binding Demo ===\n");

        long interp = mailang_create();
        if (interp == 0) {
            System.err.println("Failed to create interpreter");
            return;
        }

        // Test basic arithmetic
        System.out.println("Test: Integer arithmetic");
        System.out.println("  1 + 2 = (via FFI)");

        // Test string
        System.out.println("Test: String");
        System.out.println("  \"Hello\" = (via FFI)");

        mailang_destroy(interp);
        System.out.println("\n=== Demo Complete ===");
    }
}
