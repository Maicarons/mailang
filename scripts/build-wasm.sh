#!/bin/bash
# Build MaìLang WASM package

set -e

echo "Building MaìLang WASM..."
cargo build --release -p mailang-wasm --target wasm32-unknown-unknown

echo "WASM build complete!"
echo "Output: target/wasm32-unknown-unknown/release/mailang_wasm.wasm"
