#!/usr/bin/env bash
# Build MaìLang WASM package and sync one source of truth to all consumers:
#   playground/public/wasm/
#   docs/wasm/
#   docs/.vitepress/public/wasm/
#
# Prefer wasm-pack (same entry as playground `npm run build:wasm`).
# Fallback: raw `cargo build --release -p mailang-wasm --target wasm32-unknown-unknown`
# (wasm-bindgen JS glue is then missing — playground/docs need the wasm-pack pkg).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

WASM_CRATE=crates/mailang-wasm
OUT_DIR="$WASM_CRATE/pkg"
FALLBACK_WASM="target/wasm32-unknown-unknown/release/mailang_wasm.wasm"

DESTS=(
  "playground/public/wasm"
  "docs/wasm"
  "docs/.vitepress/public/wasm"
)

echo "Building MaìLang WASM..."

if command -v wasm-pack >/dev/null 2>&1; then
    echo "Using wasm-pack (via playground build:wasm entry)..."
    # Same command as playground/package.json "build:wasm"
    (cd "$WASM_CRATE" && wasm-pack build --target web --release)
    ARTIFACTS=(mailang_wasm.js mailang_wasm_bg.wasm mailang_wasm.d.ts mailang_wasm_bg.wasm.d.ts package.json)
else
    echo "wasm-pack not found; falling back to cargo build (raw cdylib only)."
    echo "NOTE: without wasm-pack, JS glue (mailang_wasm.js) is NOT regenerated."
    echo "      Install wasm-pack for playground/docs: cargo install wasm-pack"
    rustup target add wasm32-unknown-unknown
    cargo build --release -p mailang-wasm --target wasm32-unknown-unknown
    ARTIFACTS=()
fi

for dest in "${DESTS[@]}"; do
    mkdir -p "$dest"
    if [ ${#ARTIFACTS[@]} -gt 0 ]; then
        for f in "${ARTIFACTS[@]}"; do
            if [ -f "$OUT_DIR/$f" ]; then
                cp -f "$OUT_DIR/$f" "$dest/$f"
            fi
        done
        echo "Copied wasm-pack artifacts -> $dest"
    else
        if [ -f "$FALLBACK_WASM" ]; then
            cp -f "$FALLBACK_WASM" "$dest/mailang_wasm_bg.wasm"
        fi
        echo "Copied raw .wasm only -> $dest (JS glue unchanged)"
    fi
done

echo "WASM build complete!"
if [ ${#ARTIFACTS[@]} -gt 0 ]; then
    echo "Source of truth: $OUT_DIR"
    echo "Synced to: ${DESTS[*]}"
else
    echo "Raw wasm: $FALLBACK_WASM (limitation: no wasm-bindgen glue)"
fi