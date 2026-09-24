# Real Hardware (ESP32-C3 / Cortex-M4)

> **Links**: [GitHub](https://github.com/Maicarons/mailang) · [mailang.h](https://github.com/Maicarons/mailang/blob/master/crates/mailang-ffi/include/mailang.h) · [test_c.c](https://github.com/Maicarons/mailang/blob/master/ffi/tests/test_c.c) · [IoT Deployment](/en/guide/iot)

Embed **mailang-ffi** on a real MCU, wire `gpio_*` / `delay_ms` / `adc_read` to a real HAL via host functions, and ship precompiled `.mailangbc`.

## Prerequisite: SimulatedHal is host-only

Built-ins `gpio_write` / `gpio_read` / `delay_ms` / `adc_read` use `mailang_stdlib::hal::SimulatedHal` (`crates/mailang-stdlib/src/hal.rs`) and are **desktop/simulation only**:

- 32 virtual GPIO pins, 8 virtual ADC channels
- `delay_ms` only advances a simulated clock (no real sleep)
- no real peripherals

On a real MCU you must register host functions with `mailang_register_host_fn` and bind them to the vendor HAL. SimulatedHal will not blink an LED.

## 1. Build the static library

```bash
cargo build -p mailang-ffi --release
# target/release/libmailang_ffi.a
# header: crates/mailang-ffi/include/mailang.h

rustup target add riscv32imc-unknown-none-elf
cargo build -p mailang-ffi --release --target riscv32imc-unknown-none-elf   # ESP32-C3

rustup target add thumbv7em-none-eabihf
cargo build -p mailang-ffi --release --target thumbv7em-none-eabihf       # Cortex-M4
```

The C FFI layer expects `std`. For bare-metal Cortex-M4 prefer `mailang-vm` + `mailang-bytecode` (`no_std` + `alloc`) — see [IoT Deployment](/en/guide/iot).

Key C API (full list in `mailang.h`):

```c
MailangInterpreter *mailang_create(void);
void mailang_destroy(MailangInterpreter *interp);
int mailang_eval(MailangInterpreter *interp, const char *code, MailangResult *result);
int mailang_eval_file(MailangInterpreter *interp, const char *path, MailangResult *result);
int mailang_register_host_fn(MailangInterpreter *interp, const char *name,
                             MailangHostFn callback, void *user_data);
void mailang_free_string(char *ptr);
```

## 2. Bind real GPIO / Delay / ADC

Follow the `host_add` pattern in `ffi/tests/test_c.c`:

```c
#include "mailang.h"
extern void board_gpio_write(int pin, int level);
extern int  board_gpio_read(int pin);
extern void board_delay_ms(uint32_t ms);
extern int  board_adc_read(int channel);

static int host_gpio_write(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    if (argc != 2 || argv[0].tag != 2) return 1;
    board_gpio_write((int)argv[0].i, argv[1].i ? 1 : 0);
    out->tag = 0; out->i = 0; out->f = 0; out->s = NULL;
    return 0;
}

static int host_gpio_read(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    if (argc != 1 || argv[0].tag != 2) return 1;
    out->tag = 1; out->i = board_gpio_read((int)argv[0].i) ? 1 : 0;
    out->f = 0; out->s = NULL;
    return 0;
}

static int host_delay_ms(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    if (argc != 1 || argv[0].tag != 2 || argv[0].i < 0) return 1;
    board_delay_ms((uint32_t)argv[0].i);
    out->tag = 0; out->i = 0; out->f = 0; out->s = NULL;
    return 0;
}

static int host_adc_read(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    if (argc != 1 || argv[0].tag != 2) return 1;
    out->tag = 2; out->i = board_adc_read((int)argv[0].i);
    out->f = 0; out->s = NULL;
    return 0;
}

void mailang_bind_hal(MailangInterpreter *interp) {
    mailang_register_host_fn(interp, "gpio_write", host_gpio_write, NULL);
    mailang_register_host_fn(interp, "gpio_read",  host_gpio_read,  NULL);
    mailang_register_host_fn(interp, "delay_ms",   host_delay_ms,   NULL);
    mailang_register_host_fn(interp, "adc_read",   host_adc_read,   NULL);
}
```

Scripts keep calling `gpio_write(2, true)` / `delay_ms(50)`; the host registration takes over.

## 3. Load source or `.mailangbc`

### Source (dev)

```c
MailangInterpreter *interp = mailang_create();
mailang_bind_hal(interp);
MailangResult r;
if (mailang_eval(interp, "gpio_write(2, true)\ndelay_ms(100)", &r) == MAILANG_OK)
    mailang_free_string(r.output);
mailang_destroy(interp);
```

### Precompiled bytecode (production)

```bash
cargo run -p mailang-cli -- build app.mai -o app.mailangbc
```

Embed the blob (`xxd -i app.mailangbc > app_bc.h`) and run it through the C bytecode API:

```c
#include "mailang.h"

extern const unsigned char app_bc[];
extern const unsigned int app_bc_len;

char *out = NULL;
MailangStatus st = mailang_eval_bytecode(app_bc, app_bc_len, &out);
if (st == MAILANG_OK) {
    /* out holds program output */
} else {
    /* out holds an error message */
}
if (out) mailang_free_string(out);

/* Or load from a filesystem / flash-backed path */
st = mailang_load_bytecode_file("app.mailangbc", &out);
if (out) mailang_free_string(out);
```

- `mailang_eval_bytecode(data, len, &out)` — decode an in-memory `.mailangbc` blob and run it in a fresh interpreter.
- `mailang_load_bytecode_file(path, &out)` — read file → decode → run.
- Failure codes: `MAILANG_ERR_DECODE` (bad container), `MAILANG_ERR_IO` (read failed), `MAILANG_ERR_EVAL` (runtime).

These bytecode entry points run in a **fresh interpreter** and do not inherit host functions. For HAL callbacks, keep using `mailang_eval` on source, or a Rust entry (`run_bytecode` + `register_host_fn`).

## Checklist

1. Host: `mailang build` → `.mailangbc`
2. Cross-build `mailang-ffi` staticlib (or `mailang-vm` bare-metal)
3. `mailang_bind_hal()` → real GPIO/delay/ADC
4. `mailang_create()` → bind HAL → `mailang_eval`; or `mailang_eval_bytecode` / `mailang_load_bytecode_file` for precompiled bytecode
5. Free strings with `mailang_free_string`; check `mailang_last_error`

## References

- **End-to-end blink**: [ESP32 Blink](/en/guide/esp32-blink) (`examples/iot_blink.mai` + IDF-style C host)
- Footprint: [Footprint](/guide/footprint)
- Header: `crates/mailang-ffi/include/mailang.h`
- C smoke test: `ffi/tests/test_c.c`
- SimulatedHal: `crates/mailang-stdlib/src/hal.rs`
- Bytecode: `crates/mailang-bytecode` (`.mailangbc`)
- Demo scripts: `examples/hal_sim.mai`, `examples/iot_blink.mai`
