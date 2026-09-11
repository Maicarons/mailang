# WASM Integration Guide

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [WASM Source](https://github.com/Maicarons/mailang/tree/master/crates/mailang-wasm) · [Playground](https://maicarons.github.io/mailang/playground)

## Overview

MaìLang supports compilation to WebAssembly (WASM), enabling execution in browsers and Node.js. This guide covers building, using, and optimizing the WASM version.

## Building WASM

### Prerequisites

```bash
# Install Rust WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

### Build Commands

```bash
# Build for browser
wasm-pack build --target web --release -p mailang-wasm

# Build for Node.js
wasm-pack build --target nodejs --release -p mailang-wasm

# Build for bundlers (Vite/Webpack)
wasm-pack build --target bundler --release -p mailang-wasm
```

Output is in `crates/mailang-wasm/pkg/`.

## Browser Usage

### Basic Usage

```html
<!DOCTYPE html>
<html>
<body>
    <script type="module">
        import init, { WasmInterpreter } from './pkg/mailang_wasm.js';

        async function main() {
            await init();
            const interp = new WasmInterpreter();
            
            const result = interp.eval('1 + 2');
            console.log(result);  // "3"
            
            interp.free();
        }
        main();
    </script>
</body>
</html>
```

### Structured Results

```javascript
const jsonStr = interp.eval_json('1 + 2');
const result = JSON.parse(jsonStr);

if (result.ok) {
    console.log('Output:', result.output);
} else {
    console.error('Error:', result.error);
}
```

## Node.js Usage

```javascript
const { WasmInterpreter } = require('./pkg/mailang_wasm.js');

const interp = new WasmInterpreter();
console.log(interp.eval('println("Hello!")'));
interp.free();
```

## Vue 3 Integration

```vue
<script setup>
import { ref, onMounted } from 'vue';
import init, { WasmInterpreter } from './pkg/mailang_wasm.js';

const output = ref('');
let interp = null;

onMounted(async () => {
    await init();
    interp = new WasmInterpreter();
});

function runCode(code) {
    output.value = interp.eval(code);
}
</script>
```

## API Reference

### WasmInterpreter

| Method | Description |
|--------|-------------|
| `new WasmInterpreter()` | Create interpreter |
| `eval(code)` | Execute code, return result string |
| `eval_json(code)` | Execute code, return JSON result |
| `reset()` | Reset interpreter state |
| `free()` | Free WASM memory |
| `WasmInterpreter.version()` | Get version string |
| `WasmInterpreter.features()` | Get features list |

## Playground

MaìLang includes a Vue + WASM playground:

```bash
cd playground
npm install
npm run build:wasm
npm run dev
```

Features:
- CodeMirror editor with syntax highlighting
- Real-time execution
- Built-in examples
- Error display

## Size Optimization

```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
strip = "symbols"
```

Expected sizes:
- Full: ~2MB
- Minimal: ~800KB

## Limitations

- No filesystem access
- No network requests
- No `input()` function
- Memory limited (default 256MB WASM limit)

## Next Steps

- [IoT Deployment](/en/guide/iot) - Embedded devices
- [FFI](/en/guide/ffi) - Other language bindings
