/**
 * MaìLang Swift Binding Demo
 * Uses C bridging to call the C FFI.
 *
 * Build: swiftc Main.swift -lmailang_ffi -L../../target/release -o demo
 * Note: Requires libmailang_ffi.dylib/so in library path
 */

import Foundation

// C function declarations (bridged via mailang.h)
// In practice, you would import the C module:
//   import CMaìLang
// For this demo, we show the concept.

print("=== MaìLang Swift Binding Demo ===\n")

// The actual FFI calls would work like this:
// let interp = mailang_create()
// defer { mailang_destroy(interp) }
//
// var result = MailangResult()
// let ret = mailang_eval(interp, "1 + 2", &result)
// if ret == 0 && result.code == 0 {
//     let output = String(cString: result.output!)
//     print("1 + 2 = \(output)")
//     mailang_free_string(result.output)
// }

print("Test: Swift binding structure demo")
print("  The C FFI can be called from Swift using bridging headers")
print("  or by importing the C module directly")

print("\n=== Demo Complete ===")
