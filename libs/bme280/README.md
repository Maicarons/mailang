# bme280 — BME280 driver pack (stub)

**Status: stub / simulation only.**

This pack exposes a small MaìLang API shaped like a BME280 temperature/humidity
sensor driver. It **does not talk to hardware**. Readings are fixed sim values
so scripts and tests can run on desktop.

The **host** must supply a real transport (I2C/SPI register read/write) via
`mailang_register_host_fn` (or a future HAL binding) before these numbers
mean anything physical.

## API (stub)

| Function | Returns | Stub behavior |
|----------|---------|---------------|
| `read_temp()` | float | `25.0` (°C) |
| `read_humidity()` | float | `40.0` (%) |

## Layout

```
bme280/
├── mailib.ini      # module manifest
├── lib.mai         # stub implementation
├── example/basic.mai
└── test/test_bme280.mai
```

## Usage

```mai
import "bme280"

println("temp={bme280.read_temp()}C rh={bme280.read_humidity()}%")
```

Resolve the module with `libs/` next to the CLI, or a project `mailang.toml`
`[dependencies]` path entry (see `docs/MODULE_SPEC.md`).

## Honest limits

- No I2C/SPI driver, no calibration coefficients, no compensation formula.
- Sim values only — do not claim sensor accuracy from this stub.
- Real hardware: implement host fns and replace `lib.mai` bodies, or call
  those fns from here once registered.
