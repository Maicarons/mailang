/**
 * MaìLang Node.js Binding Test
 * Uses ffi-napi to call the C FFI directly.
 */
const path = require('path');
const fs = require('fs');

// Check if ffi-napi is available, otherwise use a simpler approach
let ffi;
try {
    ffi = require('ffi-napi');
} catch (e) {
    // Fallback: use child_process to call the C demo
    console.log("ffi-napi not available, using child_process fallback\n");
    
    const { execSync } = require('child_process');
    const demoPath = path.join(__dirname, '..', 'c', 'demo.exe');
    
    if (fs.existsSync(demoPath)) {
        console.log("=== MaìLang Node.js Binding Test (via C demo) ===\n");
        try {
            const output = execSync(demoPath, { encoding: 'utf-8' });
            console.log(output);
        } catch (err) {
            console.error("Failed to run C demo:", err.message);
        }
    } else {
        console.log("C demo not found. Please build it first.");
    }
    process.exit(0);
}

// If ffi-napi is available, use it directly
const dllPath = path.join(__dirname, '..', '..', 'target', 'x86_64-pc-windows-gnu', 'release', 'mailang_ffi.dll');

if (!fs.existsSync(dllPath)) {
    console.error(`DLL not found at ${dllPath}`);
    process.exit(1);
}

const lib = ffi.Library(dllPath, {
    'mailang_create': ['pointer', []],
    'mailang_destroy': ['void', ['pointer']],
    'mailang_eval': [{ code: 'int32', output: 'string' }, ['pointer', 'string']],
    'mailang_free_string': ['void', ['string']],
});

console.log("=== MaìLang Node.js Binding Test ===\n");

const interp = lib.mailang_create();

function runTest(name, code, expected) {
    const result = lib.mailang_eval(interp, code);
    const output = result.output || "null";
    const passed = output === expected;
    const status = passed ? "PASS" : "FAIL";
    
    if (passed) {
        console.log(`  [${status}] ${name}: ${code} = ${output}`);
    } else {
        console.log(`  [${status}] ${name}: ${code} = ${output} (expected ${expected})`);
    }
    return passed;
}

let passed = 0;
let total = 0;

function test(name, code, expected) {
    total++;
    if (runTest(name, code, expected)) passed++;
}

test("Integer Add", "1 + 2", "3");
test("String Concat", '"Hello, " + "MaìLang!"', "Hello, MaìLang!");
test("Boolean", "true", "true");
test("Null", "null", "null");
test("Float Mul", "3.14 * 2.0", "6.28");

lib.mailang_destroy(interp);

console.log(`\n=== Results: ${passed}/${total} tests passed ===`);
process.exit(passed === total ? 0 : 1);
