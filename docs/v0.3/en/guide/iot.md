# IoT Deployment

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [Cross.toml](https://github.com/Maicarons/mailang/blob/master/Cross.toml) · [rust-toolchain.toml](https://github.com/Maicarons/mailang/blob/master/rust-toolchain.toml)

## Overview

MaìLang supports devices from 64KB to GBs of memory through three-tier feature gating.

## Feature Levels

### Level 1: `no_std` (Minimal)
- Memory: 64KB+
- Devices: ARM Cortex-M0
- Features: Stack-only values, no heap allocation

### Level 2: `no_std + alloc` (Embedded)
- Memory: 128KB+
- Devices: RISC-V, ESP32
- Features: Reference counting, heap allocation

### Level 3: `std` (Standard)
- Memory: 256KB+
- Devices: Raspberry Pi, servers
- Features: Full functionality

## Cross Compilation

```bash
# ARM Cortex-M4
cargo build --release --target thumbv7em-none-eabihf

# RISC-V
cargo build --release --target riscv32imc-unknown-none-elf

# Linux ARM (Raspberry Pi)
cargo build --release --target aarch64-unknown-linux-gnu

# WebAssembly
cargo build --release --target wasm32-unknown-unknown
```

## Binary Size Optimization

```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

## Supported Architectures

| Target | Rust Target | Min Memory |
|--------|-------------|------------|
| ARM Cortex-M0 | `thumbv6m-none-eabi` | 64KB |
| ARM Cortex-M4 | `thumbv7em-none-eabihf` | 256KB |
| RISC-V | `riscv32imc-unknown-none-elf` | 128KB |
| ESP32 | `xtensa-esp32-none-elf` | 512KB |
| WASM | `wasm32-unknown-unknown` | N/A |
| Linux ARM | `aarch64-unknown-linux-gnu` | 16MB |

## Real Hardware (GPIO / ADC / delay)

`SimulatedHal` is host-only. To bind scripts to real ESP32-C3 / Cortex-M4 peripherals, see:

- **[Real Hardware](/v0.3/en/guide/hardware)** — build `mailang-ffi`, wire `mailang_register_host_fn` to a real HAL, ship `.mailangbc`
- **[ESP32 Blink End-to-End](/v0.3/en/guide/esp32-blink)** — copy-paste IDF-style C host + expected serial output

## Next Steps

- [Real Hardware](/v0.3/en/guide/hardware) - ESP32-C3 / Cortex-M4 embedding guide
- [ESP32 Blink](/v0.3/en/guide/esp32-blink) - End-to-end blink host
- [API Reference](/v0.3/en/reference/) - Complete API docs
