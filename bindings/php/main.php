<?php
/**
 * MaìLang PHP Binding Demo
 * Uses ext-ffi to call the C FFI.
 *
 * Requirements:
 *   - PHP 8.0+ with FFI extension enabled
 *   - mailang_ffi.dll/so in the library path
 *
 * Run: php main.php
 */

echo "=== MaìLang PHP Binding Demo ===\n\n";

try {
    // Load the FFI library
    $ffi = FFI::cdef('
        typedef struct MailangInterpreter MailangInterpreter;
        typedef struct {
            int32_t code;
            char* output;
        } MailangResult;

        MailangInterpreter* mailang_create(void);
        void mailang_destroy(MailangInterpreter* interp);
        int mailang_eval(MailangInterpreter* interp, const char* code, MailangResult* result);
        void mailang_free_string(char* ptr);
    ', 'mailang_ffi.dll');

    echo "Test: Interpreter creation\n";
    $interp = $ffi->mailang_create();
    if ($interp === null) {
        echo "  ERROR: Failed to create interpreter\n";
        exit(1);
    }
    echo "  Interpreter created successfully\n";

    // Clean up
    $ffi->mailang_destroy($interp);
    echo "\n=== Demo Complete ===\n";

} catch (FFI\Exception $e) {
    echo "FFI Error: " . $e->getMessage() . "\n";
    echo "Make sure mailang_ffi.dll is in the library path.\n";
}
