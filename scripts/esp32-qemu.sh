#!/usr/bin/env bash
# ESP32-C3 end-to-end under QEMU — the reproducible evidence path for
# docs/v0.4/guide/esp32-blink.md.
#
# What it does, end to end:
#   0. Bootstraps a docker volume (`mailang_esp_home`) with the esp rust fork
#      toolchain (rust-build release, linked via `rustup toolchain link`), a
#      local ESP-IDF v5.2.3 clone and the tool archives idf_tools.py needs —
#      so no step needs live GitHub access at build time.
#   1. Builds examples/esp32c3 (the full MaìLang interpreter as ESP-IDF
#      firmware) inside the espressif/idf-rust container.
#   2. elf2image + merge_bin into a 4 MB flash image.
#   3. Boots espressif's QEMU fork and prints the serial output — the blink
#      logic runs as MaìLang source inside the register VM on the emulated MCU.
#
# Requirements: docker, ~1 GB free for the volume, network for the first
# bootstrap only. Usage: ./scripts/esp32-qemu.sh [qemu-timeout-seconds]

set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${MAILANG_IDF_IMAGE:-mailang-esp32c3-dev}"
BASE_IMAGE="${MAILANG_IDF_BASE_IMAGE:-espressif/idf-rust:esp32c3_latest}"
VOL="${MAILANG_ESP_VOL:-mailang_esp_home}"
ESP_RUST_VER="${MAILANG_ESP_RUST_VER:-1.95.0.0}"
IDF_VER="v5.2.3"
QEMU_REL="${MAILANG_QEMU_REL:-esp-develop-9.2.2-20260417}"
QEMU_ASSET="qemu-riscv32-softmmu-esp_develop_9.2.2_20260417-x86_64-linux-gnu.tar.xz"
SECONDS_ARG="${1:-25}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

dl() { curl -fL --ssl-no-revoke --retry 3 -sS -o "$2" "$1"; } # --ssl-no-revoke: schannel

# ---------------------------------------------------------------------------
echo "== [0/4] bootstrap tool volume '$VOL' (first run downloads ~700 MB) =="
docker volume create "$VOL" >/dev/null

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  docker build -t "$IMAGE" -f "$REPO/examples/esp32c3/Dockerfile.dev" "$REPO/examples/esp32c3"
fi

docker run --rm -v "$VOL:/home/esp-build" -v "$TMP:/tmp/seeds" -u root "$IMAGE" bash -s <<BOOTSTRAP
set -e
TC=/home/esp-build/toolchains/esp\$ESP_RUST_VER
if [ ! -f "\$TC/bin/rustc" ]; then
  echo "-- downloading esp rust fork \$ESP_RUST_VER"
  curl -fL --retry 3 -sS -o /tmp/rust.tar.xz \
    "https://github.com/esp-rs/rust-build/releases/download/v\$ESP_RUST_VER/rust-\$ESP_RUST_VER-x86_64-unknown-linux-gnu.tar.xz"
  curl -fL --retry 3 -sS -o /tmp/src.tar.xz \
    "https://github.com/esp-rs/rust-build/releases/download/v\$ESP_RUST_VER/rust-src-\$ESP_RUST_VER.tar.xz"
  mkdir -p "\$TC"
  tar xf /tmp/rust.tar.xz -C "\$TC" --strip-components=1
  tar xf /tmp/src.tar.xz -C "\$TC" --strip-components=2
  for c in cargo rustc clippy-preview rustfmt-preview rust-std-x86_64-unknown-linux-gnu; do
    if [ -d "\$TC/\$c" ]; then cp -a "\$TC/\$c/." "\$TC/"; rm -rf "\$TC/\$c"; fi
  done
  chown -R \$(id -un) /home/esp-build
fi
export HOME=/home/esp-build
rustup toolchain link "esp\$ESP_RUST_VER" "\$TC" 2>/dev/null || true
if [ ! -d /home/esp-build/idf/v5.2.3 ]; then
  echo "-- cloning ESP-IDF \$IDF_VER into the volume"
  git clone --recursive --depth 1 --shallow-submodules --branch \$IDF_VER \
      https://github.com/espressif/esp-idf.git /home/esp-build/idf/v5.2.3
fi
mkdir -p /home/esp-build/dist
echo "-- fetching the tool archives idf_tools.py would download (dist cache)"
dl() { curl -fL --retry 3 -sS -o "/home/esp-build/dist/\$2" "\$1"; }
dl https://github.com/espressif/crosstool-NG/releases/download/esp-13.2.0_20230928/riscv32-esp-elf-13.2.0_20230928-x86_64-linux-gnu.tar.xz riscv32-esp-elf-13.2.0_20230928-x86_64-linux-gnu.tar.xz
dl https://github.com/espressif/llvm-project/releases/download/esp-16.0.0-20230516/llvm-esp-16.0.0-20230516-linux-amd64.tar.xz llvm-esp-16.0.0-20230516-linux-amd64.tar.xz
dl https://github.com/ninja-build/ninja/releases/download/v1.11.1/ninja-linux.zip ninja-linux-v1.11.1.zip
dl https://github.com/Kitware/CMake/releases/download/v3.24.0/cmake-3.24.0-linux-x86_64.tar.gz cmake-3.24.0-linux-x86_64.tar.gz
dl https://github.com/espressif/esp-rom-elfs/releases/download/20230320/esp-rom-elfs-20230320.tar.gz esp-rom-elfs-20230320.tar.gz
mkdir -p "$REPO/examples/esp32c3/.embuild/espressif/dist"
cp /home/esp-build/dist/* "$REPO/examples/esp32c3/.embuild/espressif/dist/"
BOOTSTRAP

# ---------------------------------------------------------------------------
echo "== [1/4] building examples/esp32c3 =="
docker run --rm -v "$REPO:/project" -v "$VOL:/home/esp-build" \
    -w /project/examples/esp32c3 -e HOME=/home/esp-build \
    -e RUSTUP_TOOLCHAIN="esp$ESP_RUST_VER" \
    -e IDF_PATH=/home/esp-build/idf/v5.2.3 \
    -u root "$IMAGE" cargo build --release

# ---------------------------------------------------------------------------
echo "== [2/4] merging flash image =="
docker run --rm -v "$REPO:/project" -v "$VOL:/home/esp-build" -u root "$IMAGE" bash -s <<MERGE
set -e
R=/project/examples/esp32c3/target/riscv32imc-esp-espidf/release
PY=/project/examples/esp32c3/.embuild/espressif/python_env/idf5.2_py3.11_env/bin/python
cd "\$R"
\$PY -m esptool --chip esp32c3 elf2image --flash_mode dio --flash_freq 40m --flash_size 4MB \\
    -o mailang-esp32c3-blink.bin mailang-esp32c3-blink
\$PY -m esptool --chip esp32c3 merge_bin -o /project/esp32c3-flash.bin \\
    --flash_mode dio --flash_size 4MB \\
    0x0 bootloader.bin 0x8000 partition-table.bin 0x10000 mailang-esp32c3-blink.bin
truncate -s 4M /project/esp32c3-flash.bin   # QEMU wants an exact 4 MB image
MERGE

# ---------------------------------------------------------------------------
echo "== [3/4] booting in QEMU (serial stdout) =="
docker run --rm -v "$VOL:/home/esp-build" -v "$REPO:/project" -u root "$IMAGE" bash -s <<QEMU
set -e
apt-get update -qq >/dev/null 2>&1
apt-get install -y -qq libpixman-1-0 libglib2.0-0 libsdl2-2.0-0 libslirp0 >/dev/null 2>&1
Q=\$(find /home/esp-build/qemu -name 'qemu-system-riscv32' -type f 2>/dev/null | head -1) || true
if [ -z "\$Q" ]; then
  echo "-- downloading espressif QEMU fork"
  curl -fL --retry 3 -sS -o /tmp/qemu.tar.xz \
    "https://github.com/espressif/qemu/releases/download/$QEMU_REL/$QEMU_ASSET"
  mkdir -p /home/esp-build/qemu && tar xf /tmp/qemu.tar.xz -C /home/esp-build/qemu
  Q=\$(find /home/esp-build/qemu -name 'qemu-system-riscv32' -type f | head -1)
fi
chmod +x "\$Q"
timeout $SECONDS_ARG "\$Q" -machine esp32c3 \
    -drive file=/project/esp32c3-flash.bin,if=mtd,format=raw \
    -nographic -serial mon:stdio 2>&1 | sed -e 's/\x1b\[[0-9;]*m//g' | grep -aE 'iot_blink|mailang|ON|OFF' || true
QEMU

echo "== done — expected: 5 ON/OFF cycles then 'iot_blink: done after 5 cycles' =="
