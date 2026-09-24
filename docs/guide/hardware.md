# 真实硬件部署（ESP32-C3 / Cortex-M4）

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [mailang.h](https://github.com/Maicarons/mailang/blob/master/crates/mailang-ffi/include/mailang.h) · [test_c.c](https://github.com/Maicarons/mailang/blob/master/ffi/tests/test_c.c) · [IoT 部署](/guide/iot)

本页说明如何把 **mailang-ffi** 嵌进真实 MCU（ESP32-C3 / ARM Cortex-M4），用宿主函数把脚本里的 `gpio_*` / `delay_ms` / `adc_read` 接到真实 HAL，并部署预编译的 `.mailangbc`。

## 重要前提：SimulatedHal 仅用于主机

标准库内置的 `gpio_write` / `gpio_read` / `delay_ms` / `adc_read` 走 `mailang_stdlib::hal::SimulatedHal`（见 `crates/mailang-stdlib/src/hal.rs`），**只适合桌面/仿真**：

- 32 个虚拟 GPIO、8 路虚拟 ADC
- `delay_ms` 只推进模拟时钟，**不阻塞真实时间**
- 不会碰任何真实外设

真实 MCU 上必须在 **宿主侧** 用 `mailang_register_host_fn` 注册同名（或业务名）函数，绑定到厂商 HAL / GPIO 驱动。不要期望 SimulatedHal 能点亮 LED。

## 1. 构建静态库

在主机交叉编译 `mailang-ffi` 为 staticlib（C ABI），头文件使用 `crates/mailang-ffi/include/mailang.h`：

```bash
# 主机（开发/链接进固件工具链）
cargo build -p mailang-ffi --release

# 产物（Linux 示例）
# target/release/libmailang_ffi.a
# target/release/libmailang_ffi.so  (若 cdylib)

# 交叉到 ESP32-C3（RISC-V，需 nightly / cross 或 rust-xtensa/riscv 工具链）
rustup target add riscv32imc-unknown-none-elf
cargo build -p mailang-ffi --release --target riscv32imc-unknown-none-elf
# 注意：FFI 层依赖 std，完整解释器通常跑在带 OS 的 ESP32（IDF）或主机上；
# 裸机 Cortex-M4 请用 mailang-vm + mailang-bytecode（no_std + alloc），见 /guide/iot。

# 交叉到 Cortex-M4 目标 triple（同上，按你的 link 脚本打包）
rustup target add thumbv7em-none-eabihf
cargo build -p mailang-ffi --release --target thumbv7em-none-eabihf
```

链接示例（与 `ffi/tests/test_c.c` 相同模式）：

```bash
# 主机冒烟
gcc -I crates/mailang-ffi/include -L target/release \
    -o firmware_host_stub main.c -lmailang_ffi

# 固件侧（以 ESP-IDF / arm-none-eabi-gcc 为例）
xtensa-esp32s3-elf-gcc -I crates/mailang-ffi/include \
    -L target/riscv32imc-unknown-none-elf/release \
    -o mailang_embed.elf main.c -lmailang_ffi
```

头文件关键 API（完整定义见 `mailang.h`）：

```c
MailangInterpreter *mailang_create(void);
void mailang_destroy(MailangInterpreter *interp);

int mailang_eval(MailangInterpreter *interp, const char *code, MailangResult *result);
int mailang_eval_file(MailangInterpreter *interp, const char *path, MailangResult *result);

int mailang_register_host_fn(
    MailangInterpreter *interp,
    const char *name,
    MailangHostFn callback,
    void *user_data);

const char *mailang_last_error(MailangInterpreter *interp);
void mailang_free_string(char *ptr);
```

`MailangHostFn` 签名：

```c
typedef int (*MailangHostFn)(
    void *user_data,
    int32_t argc,
    const MailangValue *argv,
    MailangValue *out);
```

## 2. 用宿主函数接真实 GPIO / Delay / ADC

模式对齐 `ffi/tests/test_c.c` 中的 `host_add`：检查 `argc`/`tag`，写 `out`，返回 0 表示成功。

```c
#include "mailang.h"
/* 替换为你的 MCU HAL 头文件，例如 ESP-IDF driver/gpio.h、driver/adc.h、
   或 STM32 HAL GPIO / ADC。下面用占位声明。 */
extern void board_gpio_write(int pin, int level);
extern int  board_gpio_read(int pin);
extern void board_delay_ms(uint32_t ms);
extern int  board_adc_read(int channel); /* raw */

static int host_gpio_write(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    if (argc != 2 || argv[0].tag != 2) return 1;
    int pin = (int)argv[0].i;
    int level = 0;
    if (argv[1].tag == 1) level = argv[1].i ? 1 : 0;
    else if (argv[1].tag == 2) level = argv[1].i ? 1 : 0;
    else return 1;
    board_gpio_write(pin, level);
    out->tag = 0; out->i = 0; out->f = 0; out->s = NULL;
    return 0;
}

static int host_gpio_read(void *ud, int32_t argc, const MailangValue *argv, MailangValue *out) {
    (void)ud;
    if (argc != 1 || argv[0].tag != 2) return 1;
    out->tag = 1;
    out->i = board_gpio_read((int)argv[0].i) ? 1 : 0;
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
    out->tag = 2;
    out->i = board_adc_read((int)argv[0].i);
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

> 脚本侧仍可写 `gpio_write(5, true)` / `delay_ms(50)`。宿主注册的同名函数会接管调用（见 `test_c.c` 里 `host_add` / `host_greet` 的用法）。

## 3. 加载脚本与 `.mailangbc`

### 3.1 源码求值（开发期）

与 `test_c.c` 相同：

```c
MailangInterpreter *interp = mailang_create();
mailang_bind_hal(interp);

MailangResult r;
if (mailang_eval(interp, "gpio_write(2, true)\ndelay_ms(100)\ngpio_read(2)", &r) == MAILANG_OK) {
    /* use r.output */
    mailang_free_string(r.output);
}
mailang_destroy(interp);
```

### 3.2 预编译 `.mailangbc`（生产推荐）

主机上先编译，避免在 MCU 上跑 parser/compiler：

```bash
cargo run -p mailang-cli -- build app.mai -o app.mailangbc
```

固件中把字节码嵌入只读存储（示例：`xxd -i app.mailangbc > app_bc.h`），再通过 C API 解码并执行：

```c
#include "mailang.h"

/* 来自 xxd -i app.mailangbc，或直接放 .rodata */
extern const unsigned char app_bc[];
extern const unsigned int app_bc_len;

char *out = NULL;
MailangStatus st = mailang_eval_bytecode(app_bc, app_bc_len, &out);
if (st == MAILANG_OK) {
    /* out 为程序输出 */
} else {
    /* out 为错误信息 */
}
if (out) mailang_free_string(out);

/* 也可从文件系统 / flash 路径加载 */
st = mailang_load_bytecode_file("app.mailangbc", &out);
if (out) mailang_free_string(out);
```

- `mailang_eval_bytecode(data, len, &out)`：解码内存中的 `.mailangbc` 并在全新解释器中执行。
- `mailang_load_bytecode_file(path, &out)`：读文件 → 解码 → 执行。
- 失败码：`MAILANG_ERR_DECODE`（格式错误）、`MAILANG_ERR_IO`（读文件失败）、`MAILANG_ERR_EVAL`（运行错误）。

两条字节码入口在**全新解释器**中执行，不继承宿主已注册的 host function。需要 HAL 回调时，请继续用 `mailang_eval` 跑源码，或用 Rust 入口 `run_bytecode` + `register_host_fn`。

`mailang build` 与 `mailang run --bytecode` 在主机上可验证字节码链路。

## 4. 端到端清单（ESP32-C3）

1. 主机：`cargo run -p mailang-cli -- build firmware.mai` → `firmware.mailangbc`
2. 交叉编译 `mailang-ffi` 为 staticlib，或改用 `mailang-vm` + `no_std + alloc`（见 [IoT 部署](/guide/iot)）
3. `mailang_bind_hal()` 注册 `gpio_write` / `gpio_read` / `delay_ms` / `adc_read` → 真实 IDF/HAL
4. `mailang_create()` → 绑定 HAL → `mailang_eval`；或直接 `mailang_eval_bytecode` / `mailang_load_bytecode_file` 跑预编译字节码
5. 用 `mailang_last_error` / `MailangResult.code` 处理错误；字符串结果用 `mailang_free_string` 释放

## 5. Cortex-M4 差异

| 点 | ESP32-C3 | Cortex-M4 |
|----|----------|-----------|
| 典型运行时 | ESP-IDF（有 OS/heap） | 裸机 `thumbv7em-none-eabihf` |
| 解释器形态 | `mailang-ffi` staticlib | `mailang-vm` + `mailang-bytecode`（`no_std` + `alloc`） |
| HAL | IDF `driver/gpio.h` 等 | STM32/nRF HAL 或寄存器层 |
| 字节码 | 嵌入 flash / NVS | 嵌入 `.rodata`（`include_bytes!`） |
| 预编译 | `mailang build` → `.mailangbc` | 同左 |

裸机路径请参考 [IoT 部署](/guide/iot) 的 feature gating；C ABI 完整列表见 [FFI 接入](/guide/ffi)。

## 参考

- **端到端点灯**：[ESP32 点灯](/guide/esp32-blink)（`examples/iot_blink.mai` + IDF 风格 C 宿主）
- 体积：[体积仪表盘](/guide/footprint)
- 头文件：`crates/mailang-ffi/include/mailang.h`
- C 冒烟测试（host fn / global / eval）：`ffi/tests/test_c.c`
- SimulatedHal 实现：`crates/mailang-stdlib/src/hal.rs`
- 字节码格式：`crates/mailang-bytecode`（`.mailangbc`）
- 示例脚本：`examples/hal_sim.mai`、`examples/iot_blink.mai`
