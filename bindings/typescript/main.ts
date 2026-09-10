/**
 * MaìLang TypeScript Binding Demo
 * Uses the Node.js FFI binding.
 *
 * Build: npx ts-node main.ts
 * Or:    tsc main.ts && node main.js
 * Note: Requires ffi-napi and the compiled FFI library
 */

interface MailangResult {
  code: number;
  output: string;
}

// This would use the actual FFI bindings in practice:
// import * as ffi from 'ffi-napi';
//
// const lib = ffi.Library('mailang_ffi', {
//   mailang_create: ['pointer', []],
//   mailang_destroy: ['void', ['pointer']],
//   mailang_eval: [{ code: 'int32', output: 'string' }, ['pointer', 'string']],
//   mailang_free_string: ['void', ['string']],
// });

console.log("=== MaìLang TypeScript Binding Demo ===\n");

// const interp = lib.mailang_create();
// const result: MailangResult = lib.mailang_eval(interp, '1 + 2');
// console.log(`1 + 2 = ${result.output}`);
// lib.mailang_destroy(interp);

console.log("Test: TypeScript binding structure demo");
console.log("  The binding uses ffi-napi to call the C FFI from Node.js/TypeScript");
console.log("  Install: npm install ffi-napi");
console.log("  Build: cargo build --release -p mailang-ffi");

console.log("\n=== Demo Complete ===");
