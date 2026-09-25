# ESP32 End-to-End: LED Blink (IDF-style host)

> **See also**: [IoT Deployment](/v0.3/en/guide/iot) · [Real Hardware](/v0.3/en/guide/hardware) · script [`examples/iot_blink.mai`](https://github.com/Maicarons/mailang/blob/master/examples/iot_blink.mai)

This page is a **copy-pasteable** ESP-IDF-style C host that registers real `gpio_write` / `delay_ms` via `mailang_register_host_fn`, then runs `iot_blink.mai` (or a precompiled `.mailangbc`).

## 1. Script: `examples/iot_blink.mai`

```mai
// IoT blink demo — host must supply real gpio_write / delay_ms
let led = 2
var count = 0

println("iot_blink: LED pin {led}")

while count < 5 {
    gpio_write(led, true)
    println("ON  #{count}")
    delay_ms(500)
    gpio_write(led, false)
    println("OFF #{count}")
    delay_ms(500)
    count = count + 1
}

gpio_write(led, false)
println("iot_blink: done after {count} cycles")
```

> Desktop `SimulatedHal` can print the same lines but **will not light a real LED**. A real board needs the host registration below.

## 2. Build `mailang-ffi` staticlib

```bash
cargo build -p mailang-ffi --release

# Artifacts (example)
#   target/release/libmailang_ffi.a
# Header
#   crates/mailang-ffi/include/mailang.h
```

> `mailang-ffi` needs `std` (ESP-IDF with OS/heap). For bare-metal Cortex-M4 use `mailang-vm` + `no_std + alloc` instead (see [IoT Deployment](/v0.3/en/guide/iot)).

## 3. IDF-style C host (copy-paste)

Save as `main/mailang_blink_host.c` (or merge into `app_main`). GPIO uses ESP-IDF `driver/gpio.h`; delay uses `vTaskDelay`.

```c
/* mailang_blink_host.c — ESP-IDF style host for examples/iot_blink.mai */
#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/gpio.h"
#include "esp_log.h"
#include "mailang.h"

#define BLINK_GPIO 2          /* matches `led` in the script */
static const char *TAG = "mailang_blink";

/* --- host HAL callbacks (MailangHostFn) --- */
/* tag: 0=null, 1=bool, 2=int, 3=float, 4=str */

static int host_gpio_write(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    if (argc != 2 || argv[0].tag != 2) return 1;
    int pin = (int)argv[0].i;
    int level = 0;
    if (argv[1].tag == 1) level = argv[1].i ? 1 : 0;
    else if (argv[1].tag == 2) level = argv[1].i ? 1 : 0;
    else return 1;

    gpio_set_level((gpio_num_t)pin, level);
    ESP_LOGI(TAG, "gpio_write(%d, %d)", pin, level);

    out->tag = 0; out->i = 0; out->f = 0; out->s = NULL;
    return 0;
}

static int host_delay_ms(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    if (argc != 1 || argv[0].tag != 2 || argv[0].i < 0) return 1;
    uint32_t ms = (uint32_t)argv[0].i;
    vTaskDelay(pdMS_TO_TICKS(ms));
    out->tag = 0; out->i = 0; out->f = 0; out->s = NULL;
    return 0;
}

static void mailang_bind_hal(MailangInterpreter *interp) {
    mailang_register_host_fn(interp, "gpio_write", host_gpio_write, NULL);
    mailang_register_host_fn(interp, "delay_ms",   host_delay_ms,   NULL);
}

static void board_init_led(void) {
    gpio_config_t io = {
        .pin_bit_mask = 1ULL << BLINK_GPIO,
        .mode = GPIO_MODE_OUTPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    gpio_config(&io);
}

void app_main(void) {
    board_init_led();

    MailangInterpreter *interp = mailang_create();
    if (!interp) {
        ESP_LOGE(TAG, "mailang_create failed");
        return;
    }
    mailang_bind_hal(interp);

    /* Path A: eval source (dev). With SPIFFS at /spiffs you can use
       mailang_eval_file(interp, "/spiffs/iot_blink.mai", &r);
       Below embeds the same source so the sample is copy-pasteable. */
    const char *src =
        "let led = 2\n"
        "var count = 0\n"
        "println(\"iot_blink: LED pin {led}\")\n"
        "while count < 5 {\n"
        "  gpio_write(led, true)\n"
        "  println(\"ON  #{count}\")\n"
        "  delay_ms(500)\n"
        "  gpio_write(led, false)\n"
        "  println(\"OFF #{count}\")\n"
        "  delay_ms(500)\n"
        "  count = count + 1\n"
        "}\n"
        "gpio_write(led, false)\n"
        "println(\"iot_blink: done after {count} cycles\")\n";

    MailangResult r;
    int rc = mailang_eval(interp, src, &r);
    if (rc == MAILANG_OK) {
        if (r.output) {
            printf("%s", r.output);
            mailang_free_string(r.output);
        }
    } else {
        ESP_LOGE(TAG, "eval failed rc=%d err=%s", rc, mailang_last_error(interp));
    }

    mailang_destroy(interp);
}
```

### Use `.mailangbc` (recommended for production)

Compile on the host so the MCU skips parse/compile:

```bash
cargo run -p mailang-cli -- build examples/iot_blink.mai -o iot_blink.mailangbc
```

The C API (`mailang.h`) exposes `mailang_eval` / `mailang_eval_file` (source). `.mailangbc` decode lives on the Rust side:

```rust
let bytes = std::fs::read("iot_blink.mailangbc")?; // or include_bytes!
let bc = mailang_core::bytecode::decode(&bytes)?;
let mut interp = mailang_core::MailangInterpreter::new();
// register_host_fn for real GPIO/delay first, then:
interp.run_bytecode(bc)?;
```

Typical C firmware options: embed source with `mailang_eval`, or run `.mailangbc` from a Rust entry while C only does board init (see [Real Hardware](/v0.3/en/guide/hardware)).

## 4. Expected serial output

With an LED on GPIO2, `idf.py flash monitor` should show:

```text
I (xxx) mailang_blink: gpio_write(2, 1)
iot_blink: LED pin 2
iot_blink: ON  #0
I (xxx) mailang_blink: gpio_write(2, 1)
I (xxx) mailang_blink: gpio_write(2, 0)
iot_blink: OFF #0
...
iot_blink: done after 5 cycles
```

Notes:

- `println` lands in `MailangResult.output` (or host stdout); it does **not** auto-print to UART — the C sample `printf`s it.
- The `gpio_write(...)` log lines come from the host callback, proving the real HAL took over.
- If you only see `println` and the LED never moves, you are still on SimulatedHal.

## 5. Troubleshooting

| Symptom | Cause |
|---------|-------|
| LED stays dark | Missing `mailang_register_host_fn`, or pin ≠ script `led` |
| `eval failed` | Script syntax; read `mailang_last_error` |
| Undefined symbols | Not linking `libmailang_ffi.a` / missing `include/` |
| Hang in delay | Callback should `vTaskDelay`, not busy-wait (WDT) |

## References

- Examples: `examples/iot_blink.mai`, `examples/hal_sim.mai`
- Header: `crates/mailang-ffi/include/mailang.h`
- C smoke test: `ffi/tests/test_c.c`
- [Real Hardware](/v0.3/en/guide/hardware) · [IoT Deployment](/v0.3/en/guide/iot)
