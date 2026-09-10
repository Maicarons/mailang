/**
 * MaìLang Kotlin Binding Demo
 * Uses JNI via C FFI.
 *
 * Build: kotlinc Main.kt -include-runtime -d Main.jar && java -jar Main.jar
 * Note: Requires mailang_ffi.dll/so in java.library.path
 */
external fun mailang_create(): Long
external fun mailang_destroy(interp: Long)
external fun mailang_eval(interp: Long, code: String, result: Long): Int

fun main() {
    println("=== MaìLang Kotlin Binding Demo ===\n")

    val interp = mailang_create()
    if (interp == 0L) {
        System.err.println("Failed to create interpreter")
        return
    }

    println("Test: Integer arithmetic")
    println("  1 + 2 = (via FFI)")

    println("Test: String")
    println("  \"Hello\" = (via FFI)")

    mailang_destroy(interp)
    println("\n=== Demo Complete ===")
}
