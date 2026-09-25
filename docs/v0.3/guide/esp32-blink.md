# ESP32 端到端：点灯（IDF 风格宿主）

> **相关**：[IoT 部署](/v0.3/guide/iot) · [真实硬件部署](/v0.3/guide/hardware) · 示例脚本 [`examples/iot_blink.mai`](https://github.com/Maicarons/mailang/blob/master/examples/iot_blink.mai)

本页给出一份**可复制粘贴**的 ESP-IDF 风格 C 宿主：用 `mailang_register_host_fn` 注册真实 `gpio_write` / `delay_ms`，然后跑 `iot_blink.mai`（或预编译 `.mailangbc`）。

## 1. 脚本：`examples/iot_blink.mai`

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

> 桌面仿真（SimulatedHal）也能跑通打印，但 **不会点亮真实 LED**。真实板子必须走下面的宿主注册。

## 2. 构建 `mailang-ffi` 静态库

```bash
# 主机（或交叉到 ESP32 的 host 工具链）
cargo build -p mailang-ffi --release

# 产物示例
#   target/release/libmailang_ffi.a
# 头文件
#   crates/mailang-ffi/include/mailang.h
```

> `mailang-ffi` 依赖 `std`，适合 **ESP-IDF（有 OS/heap）**。裸机 Cortex-M4 请改用 `mailang-vm` + `no_std + alloc`（见 [IoT 部署](/v0.3/guide/iot)）。

## 3. IDF 风格 C 宿主（可复制）

把下面内容保存为 `main/mailang_blink_host.c`（或合并进你的 `app_main`）。GPIO 用 ESP-IDF `driver/gpio.h`，延时用 `vTaskDelay`。

```c
/* mailang_blink_host.c — ESP-IDF style host for examples/iot_blink.mai */
#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/gpio.h"
#include "esp_log.h"
#include "mailang.h"

#define BLINK_GPIO 2          /* 与脚本里的 led 一致 */
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

    /* 方式 A：跑源码文件（开发期，需要文件系统 / 嵌入字符串）。
       若 SPIFFS 挂载到 /spiffs，可：
       mailang_eval_file(interp, "/spiffs/iot_blink.mai", &r);
       下面用内嵌源码演示，便于复制粘贴。 */
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

### 用 `.mailangbc`（生产推荐）

主机上先编译，避免在 MCU 上跑 parser/compiler：

```bash
cargo run -p mailang-cli -- build examples/iot_blink.mai -o iot_blink.mailangbc
```

当前 C API（`mailang.h`）提供 `mailang_eval` / `mailang_eval_file`（源码）。`.mailangbc` 解码在 Rust 侧：

```rust
let bytes = std::fs::read("iot_blink.mailangbc")?; // 或 include_bytes!
let bc = mailang_core::bytecode::decode(&bytes)?;
let mut interp = mailang_core::MailangInterpreter::new();
// 先 register_host_fn 绑定真实 GPIO/delay，再：
interp.run_bytecode(bc)?;
```

C 固件常见做法：嵌入源码用 `mailang_eval`，或用 Rust 入口跑 `.mailangbc`、C 只做板级初始化（详见 [真实硬件部署](/v0.3/guide/hardware)）。

## 4. 预期串口输出

接好 LED（GPIO2）后，`idf.py flash monitor` 应看到类似：

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

要点：

- `println` 输出进 `MailangResult.output`（或宿主 stdout），**不会**自动打到 UART——上面 C 代码用 `printf` 打出。
- `gpio_write` 的 ESP_LOG 行来自宿主回调，证明真实 HAL 已接管。
- 若只看到 `println`、没有 `gpio_write(...)` 日志且灯不亮，说明仍在用 SimulatedHal。

## 5. 故障排查

| 现象 | 原因 |
|------|------|
| 灯不亮 | 未 `mailang_register_host_fn`，或 pin 与脚本不一致 |
| `eval failed` | 脚本语法错误；看 `mailang_last_error` |
| 链接缺符号 | 未链 `libmailang_ffi.a` / 未加 `include/` |
| delay 卡住 | 回调里应 `vTaskDelay`，不要忙等看门狗 |

## 参考

- 示例：`examples/iot_blink.mai`、`examples/hal_sim.mai`
- 头文件：`crates/mailang-ffi/include/mailang.h`
- C 冒烟：`ffi/tests/test_c.c`
- [真实硬件部署](/v0.3/guide/hardware) · [IoT 部署](/v0.3/guide/iot)
