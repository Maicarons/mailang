#!/bin/bash
# Release script for MaìLang

set -e

VERSION=${1:-""}

if [ -z "$VERSION" ]; then
    echo "Usage: ./scripts/release.sh <version>"
    echo "Example: ./scripts/release.sh 0.1.0"
    exit 1
fi

echo "Preparing release v${VERSION}..."

# Update version in Cargo.toml
sed -i "s/^version = .*/version = \"${VERSION}\"/" Cargo.toml

# Run tests
echo "Running tests..."
cargo test --workspace

# Run clippy
echo "Running clippy..."
cargo clippy --workspace -- -D warnings

# Build release
echo "Building release..."
cargo build --release

# Build WASM
echo "Building WASM..."
cargo build --release -p mailang-wasm --target wasm32-unknown-unknown

echo ""
echo "Release v${VERSION} prepared!"
echo "Next steps:"
echo "  1. git add -A && git commit -m 'Release v${VERSION}'"
echo "  2. git tag v${VERSION}"
echo "  3. git push origin main --tags"
