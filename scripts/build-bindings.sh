#!/bin/bash
# Build bindings for all languages

set -e

echo "Building MaìLang C library..."
cargo build --release -p mailang-ffi

echo "Generating C header..."
# cbindgen would generate mailang.h here

echo "Building WASM..."
cargo build --release -p mailang-wasm --target wasm32-unknown-unknown

echo "Done!"
