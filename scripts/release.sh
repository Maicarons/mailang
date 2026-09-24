#!/usr/bin/env bash
# Release script for MaìLang
#
# Bumps the workspace package version and every in-tree
# `mailang-* = { path = "...", version = "X.Y.Z" }` dependency pin so
# all crates stay on one version.

set -euo pipefail

VERSION=${1:-}

if [ -z "$VERSION" ]; then
    echo "Usage: ./scripts/release.sh <version>"
    echo "Example: ./scripts/release.sh 0.2.7"
    exit 1
fi

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "error: version must look like X.Y.Z (got '$VERSION')"
    exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Prefer python3; fall back to python (Windows).
if command -v python3 >/dev/null 2>&1; then
    PY=python3
elif command -v python >/dev/null 2>&1; then
    PY=python
else
    echo "error: python3 (or python) is required for version bumping"
    exit 1
fi

echo "Preparing release v${VERSION}..."

bump_toml() {
    $PY - "$1" "$VERSION" <<'PY'
import pathlib, re, sys

path = pathlib.Path(sys.argv[1])
version = sys.argv[2]
text = path.read_text(encoding="utf-8")

# workspace.package version (root Cargo.toml)
text, n1 = re.subn(
    r'(?m)^(version\s*=\s*")[^"]+(")',
    lambda m: m.group(1) + version + m.group(2),
    text,
    count=1,
)

# mailang-* path dependency version pins:
#   mailang-core = { path = "../mailang-core", version = "0.2.6" }
#   mailang-stdlib = { path = "...", version = "0.2.6", features = [...] }
text, n2 = re.subn(
    r'(mailang-[a-z0-9-]+\s*=\s*\{[^}]*?version\s*=\s*")[^"]+(")',
    lambda m: m.group(1) + version + m.group(2),
    text,
)

path.write_text(text, encoding="utf-8")
print(f"  {path}: workspace-version={n1} dep-pins={n2}")
PY
}

echo "Bumping versions..."
bump_toml Cargo.toml
for f in crates/*/Cargo.toml; do
    bump_toml "$f"
done

echo "Verifying version pins are consistent..."
$PY - "$VERSION" <<'PY'
import pathlib, re, sys
version = sys.argv[1]
root = pathlib.Path(".")
text = (root / "Cargo.toml").read_text(encoding="utf-8")
m = re.search(r'(?m)^version\s*=\s*"([^"]+)"', text)
assert m and m.group(1) == version, f"root workspace version is {m.group(1) if m else None}, expected {version}"
bad = []
for p in sorted((root / "crates").glob("*/Cargo.toml")):
    body = p.read_text(encoding="utf-8")
    for pin in re.finditer(r'(mailang-[a-z0-9-]+\s*=\s*\{[^}]*?version\s*=\s*")([^"]+)(")', body):
        if pin.group(2) != version:
            bad.append(f"{p}: {pin.group(0)}")
if bad:
    print("inconsistent pins:")
    print("\n".join(bad))
    sys.exit(1)
print("  all mailang-* pins match", version)
PY

# Run tests
echo "Running tests..."
cargo test --workspace

# Run clippy
echo "Running clippy..."
cargo clippy --workspace -- -D warnings

# Build release
echo "Building release..."
cargo build --release

# Build WASM (and sync docs/playground copies)
echo "Building WASM..."
./scripts/build-wasm.sh

echo ""
echo "Release v${VERSION} prepared!"
echo "Next steps:"
echo "  1. git add -A && git commit -m 'Release v${VERSION}'"
echo "  2. git tag v${VERSION}"
echo "  3. git push origin main --tags"