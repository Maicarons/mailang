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

## Real transports for the driver packs (mqtt / bme280)

The standard-library driver packs (`libs/mqtt`, `libs/bme280`) share one
contract: the pure-Mai simulation is the default, and the host overrides it by
registering the hook names (`mqtt_connect` / `bme280_temp_c` / ...). The
`mailang` CLI ships two real backends so the real-data path can be exercised
end to end without embedded hardware:

**MQTT real transport (standard MQTT 3.1.1, QoS 0)**

```bash
# terminal 1: the built-in dev broker (QoS 0, not a production broker)
mailang mqtt-broker --port 1883

# terminal 2: the script talks real TCP — mosquitto / hivemq work too
mailang run --mqtt app.mai
```

`mqtt.poll(timeout_ms)` returns `Ok({topic, payload})` or `Err(...)` — a
timeout is a MaìLang value, not a host error, so the script just `match`es it.
The real-transport suite `libs/mqtt/test/host_mqtt.mai` runs against the dev
broker in CI.

**bme280 feed backend (CSV replay)**

```bash
mailang run --bme280-feed=libs/bme280/test/feed.csv libs/bme280/test/host_bme280.mai
```

`--bme280-feed` overrides the three cooked hooks with rows recorded in a CSV
file — a **replay**, not a live sensor. Real readings still need an ESP-IDF /
MCU host implementing the same hook names over I2C.

## Real Hardware (GPIO / ADC / delay)

`SimulatedHal` is host-only. To bind scripts to real ESP32-C3 / Cortex-M4 peripherals, see:

- **[Real Hardware](/v0.4/en/guide/hardware)** — build `mailang-ffi`, wire `mailang_register_host_fn` to a real HAL, ship `.mailangbc`
- **[ESP32 Blink End-to-End](/v0.4/en/guide/esp32-blink)** — copy-paste IDF-style C host + expected serial output

## Next Steps

- [Real Hardware](/v0.4/en/guide/hardware) - ESP32-C3 / Cortex-M4 embedding guide
- [ESP32 Blink](/v0.4/en/guide/esp32-blink) - End-to-end blink host
- [API Reference](/v0.4/en/reference/) - Complete API docs
