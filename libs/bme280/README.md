# bme280 — BME280 driver pack

**Status: host-fn contract + pure-Mai simulation.** No real I2C until the host
registers transport. Desktop tests run without hardware.

## Host-function contract

Real hardware must supply **one** of the following via
`mailang_register_host_fn` (C FFI) or `Vm::register_host_fn` (Rust).
Host registration **overrides** the stub hooks in `lib.mai` at call time.

### Option A — cooked sensor hooks (simplest)

| Host function | Returns | Meaning |
|---------------|---------|---------|
| `bme280_temp_c()` | float | temperature °C |
| `bme280_humidity_pct()` | float | relative humidity %RH |
| `bme280_pressure_pa()` | float | pressure Pa |

Use default mode `"sim"` (the public API calls these hook names).

### Option B — raw I2C (matches `SimulatedHal` shape)

| Host function | Signature | Returns |
|---------------|-----------|---------|
| `i2c_xfer` | `(addr:int, w_hex:str, rlen:int)` | hex string of `rlen` response bytes |

```mai
bme280.set_mode("i2c")
bme280.use_sim_bus(false)   // route to host i2c_xfer
bme280.begin(0x76)          // probes chip id 0xD0 == 0x60
```

C sketch (see `docs/guide/hardware.md` for the full pattern):

```c
mailang_register_host_fn(interp, "i2c_xfer", host_i2c_xfer, NULL);
```

## Pure-Mai simulation (default)

No host registration. Deterministic values for scripts and tests:

| Function | Returns | Default sim |
|----------|---------|-------------|
| `begin(addr)` | `Ok(true)` / `Err` | `Ok(true)` |
| `chip_id()` | int | `96` (`0x60`) |
| `read_temp()` | float | `25.0` °C |
| `read_humidity()` | float | `40.0` % |
| `read_pressure()` | float | `101325.0` Pa |
| `read_all()` | map | `{temp, humidity, pressure}` |
| `set_sim_values(t, h, p)` | null | overrides sim physical values |
| `set_mode(m)` / `mode()` | `"sim"` \| `"i2c"` | `"sim"` |
| `use_sim_bus(flag)` | null | `i2c` mode: virtual device vs host `i2c_xfer` |

`use_sim_bus(true)` (default) runs the **same register protocol** against a
pure-Mai virtual BME280 (chip id `0x60`) so desktop tests exercise the driver
path without hardware.

## Layout

```
bme280/
├── mailib.ini
├── lib.mai              # contract + sim driver
├── example/basic.mai
└── test/test_bme280.mai
```

## Usage

```mai
import "bme280"

bme280.begin(0x76)
println("temp={bme280.read_temp()}C rh={bme280.read_humidity()}%")
```

Resolve the module with `libs/` next to the CLI, or a project `mailang.toml`
`[dependencies]` path entry (see `docs/MODULE_SPEC.md`).

## Tests

```bash
# from repo root
cargo run -p mailang-cli --release -- run libs/bme280/test/test_bme280.mai
```

Exit code `0` on success, `1` on any failure.

## Honest limits

- **Real hardware was not validated in this repo.** You must register host fns
  and run on a device with a real BME280.
- Compensation in `i2c` mode is **simplified scaling** (raw/5120, raw/1024,
  raw as Pa) — **not** the full Bosch datasheet formula (no `dig_T/P/H`).
  For calibrated accuracy use Option A hooks with host-side calibration, or
  extend `lib.mai` with datasheet compensation.
- `SimulatedHal`'s `i2c_xfer` is a deterministic **echo** device, not a BME280.
  `begin()` on that bus returns `Err` (chip id ≠ `0x60`) — by design.
- No I2C bus scan, no SPI variant, no filtering/oversampling config API yet.
