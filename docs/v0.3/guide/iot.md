# IoT 部署指南

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [Cross.toml](https://github.com/Maicarons/mailang/blob/master/Cross.toml) · [rust-toolchain.toml](https://github.com/Maicarons/mailang/blob/master/rust-toolchain.toml)

## 概述

MaìLang 通过三级特性门控（Feature Gating）支持从 64KB 到数 GB 内存的各类设备。本指南介绍如何为嵌入式设备编译和部署 MaìLang。

## 三级特性门控

### Level 1: `no_std`（最小模式）

- **内存需求**: 64KB+
- **典型设备**: ARM Cortex-M0, 小型 MCU
- **特性**: 仅栈上值类型，无堆分配，无 GC

```toml
[dependencies]
mailang-vm = { path = "../crates/mailang-vm", default-features = false }
```

### Level 2: `no_std + alloc`（嵌入式模式）

- **内存需求**: 128KB+
- **典型设备**: RISC-V, ESP32, ARM Cortex-M4
- **特性**: 引用计数，无 GC，支持堆分配

```toml
[dependencies]
mailang-vm = { path = "../crates/mailang-vm", default-features = false, features = ["alloc"] }
```

### Level 3: `std`（标准模式）

- **内存需求**: 256KB+
- **典型设备**: 树莓派, 服务器, 桌面
- **特性**: 完整功能，GC，文件/网络 I/O

```toml
[dependencies]
mailang-vm = { path = "../crates/mailang-vm", features = ["std", "gc"] }
```

## 目标架构

| 架构 | Rust Target | 最低内存 | 示例设备 |
|------|-------------|---------|---------|
| ARM Cortex-M0 | `thumbv6m-none-eabi` | 64KB | STM32F0, nRF51 |
| ARM Cortex-M4 | `thumbv7em-none-eabihf` | 256KB | STM32F4, nRF52 |
| RISC-V | `riscv32imc-unknown-none-elf` | 128KB | ESP32-C3, GD32VF103 |
| ESP32 (Xtensa) | `xtensa-esp32-none-elf` | 512KB | ESP32, ESP32-S2 |
| WASM | `wasm32-unknown-unknown` | N/A | 浏览器, 边缘计算 |
| Linux ARM | `aarch64-unknown-linux-gnu` | 16MB | 树莓派 3/4 |
| Linux MIPS | `mipsel-unknown-linux-gnu` | 16MB | 路由器, OpenWrt |

## 编译配置

### 交叉编译工具链

```bash
# 安装目标
rustup target add thumbv7em-none-eabihf
rustup target add riscv32imc-unknown-none-elf
rustup target add aarch64-unknown-linux-gnu

# 安装交叉编译工具
cargo install cross
```

### 编译命令

```bash
# ARM Cortex-M4 (STM32F4)
cargo build --release \
    --target thumbv7em-none-eabihf \
    --no-default-features \
    --features "alloc"

# RISC-V (ESP32-C3)
cargo build --release \
    --target riscv32imc-unknown-none-elf \
    --no-default-features \
    --features "alloc"

# Linux ARM (树莓派)
cargo build --release \
    --target aarch64-unknown-linux-gnu

# WebAssembly
cargo build --release \
    --target wasm32-unknown-unknown \
    -p mailang-wasm
```

### 使用 Cross

```bash
# 安装 cross
cargo install cross

# ARM 交叉编译
cross build --release \
    --target aarch64-unknown-linux-gnu \
    -p mailang-cli

# RISC-V 交叉编译
cross build --release \
    --target riscv32imc-unknown-none-elf \
    -p mailang-cli
```

## 二进制体积优化

### Cargo.toml 配置

```toml
[profile.release]
opt-level = "z"        # 优化体积
lto = "fat"            # 链接时优化
codegen-units = 1      # 单编译单元
panic = "abort"        # panic 时直接 abort
strip = "symbols"      # 去除符号表
```

### 进一步优化

```toml
[profile.release]
# 启用 nightly 优化
# cargo +nightly build --release -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort
```

### 预期体积

| 配置 | x86_64 | ARM | RISC-V |
|------|--------|-----|--------|
| `std` (完整) | ~2MB | ~1.5MB | ~1.5MB |
| `alloc` (嵌入式) | ~500KB | ~400KB | ~400KB |
| `no_std` (最小) | ~100KB | ~80KB | ~80KB |

## 嵌入式示例

### ARM Cortex-M4 (STM32F4)

```rust
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use mailang_vm::Vm;
use mailang_bytecode::Bytecode;

#[entry]
fn main() -> ! {
    // 初始化硬件
    let peripherals = cortex_m::Peripherals::take().unwrap();
    
    // 创建字节码
    let bytecode = Bytecode::new();
    
    // 创建 VM（无 GC，仅引用计数）
    let mut vm = Vm::new(bytecode);
    
    // 执行代码
    match vm.run() {
        Ok(result) => {
            // 处理结果
        }
        Err(e) => {
            // 错误处理
        }
    }
    
    loop {}
}
```

### ESP32 (Xtensa)

```rust
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;
use hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, timer::TimerGroup};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    
    // 初始化
    println!("MaìLang ESP32 Demo");
    
    // 创建解释器
    let mut vm = Vm::new(Bytecode::new());
    
    loop {
        // 传感器读取逻辑
        // 数据处理逻辑
    }
}
```

### 树莓派 (Linux ARM)

```rust
use mailang_core::MailangInterpreter;
use std::fs;

fn main() {
    let mut interp = MailangInterpreter::new();
    
    // 读取配置
    let config = fs::read_to_string("config.mai").unwrap();
    interp.eval(&config).unwrap();
    
    // 执行主程序
    let result = interp.eval_file("main.mai").unwrap();
    println!("{}", result);
}
```

## 内存管理策略

### 引用计数（嵌入式）

在嵌入式模式下，MaìLang 使用引用计数管理内存：

```rust
// 值类型
enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<String>),      // 引用计数字符串
    Array(Rc<Vec<Value>>), // 引用计数数组
    // ...
}
```

### GC（标准模式）

内存管理基于 `Rc` 引用计数。引用计数本身无法回收循环引用，因此提供**循环打破**（cycle breaking）：

```rust
// Host API: break Rc cycles and return how many were broken
let broken = interpreter.collect_cycles();
```

- 不是 mark-sweep，也没有增量 GC。
- 没有 `GcConfig { threshold, incremental }` 之类的自动 GC 配置。
- 嵌入式 `alloc` 模式仅有引用计数，无循环收集。

## 性能优化

### 字节码缓存

```rust
use std::fs;

// 编译并缓存字节码
fn compile_and_cache(source: &str, cache_path: &str) -> Bytecode {
    let bytecode = compile(source);
    let serialized = bincode::serialize(&bytecode).unwrap();
    fs::write(cache_path, serialized).unwrap();
    bytecode
}

// 加载缓存的字节码
fn load_cached(cache_path: &str) -> Bytecode {
    let data = fs::read(cache_path).unwrap();
    bincode::deserialize(&data).unwrap()
}
```

### 常量折叠

编译器会自动进行常量折叠优化：

```
// 编译时计算
const SIZE = 1024 * 1024  // 编译为 1048576
const PI_SQ = 3.14159 * 3.14159  // 编译为 9.8695877281
```

## 调试

### 日志输出

```rust
use mailang_vm::Vm;

let mut vm = Vm::new(bytecode);

// 执行
match vm.run() {
    Ok(result) => println!("Result: {:?}", result),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

> 注：VM 没有 `set_debug` / `memory_stats` API。调试请在宿主侧打印错误与 `Value`。

### 内存监控

VM 不提供 `memory_stats()`。嵌入式侧请用链接器 map / 平台 heap 统计估算占用。

## 最佳实践

1. **选择合适的特性级别**: 根据设备内存选择 `no_std`/`alloc`/`std`
2. **预编译字节码**: 避免在设备上运行编译器
3. **限制递归深度**: 防止栈溢出
4. **监控内存使用**: 使用引用计数避免内存泄漏
5. **错误处理**: 嵌入式环境下避免 panic
6. **测试**: 在目标设备上充分测试

## 真实硬件（GPIO / ADC / 延时）

`SimulatedHal` 仅用于主机仿真。要把脚本接到真实 ESP32-C3 / Cortex-M4 的 GPIO、延时与 ADC，请阅读：

- **[真实硬件部署](/v0.3/guide/hardware)** — 构建 `mailang-ffi` 静态库、用 `mailang_register_host_fn` 绑定真实 HAL、部署 `.mailangbc`
- **[ESP32 点灯端到端](/v0.3/guide/esp32-blink)** — 可复制的 IDF 风格 C 宿主 + 预期串口输出
- **[体积仪表盘](/v0.3/guide/footprint)** — 用 `benchmark/measure_size.py` 看 host CLI / bytecode rlib 体积

## 下一步

- [真实硬件部署](/v0.3/guide/hardware) - ESP32-C3 / Cortex-M4 嵌入指南
- [ESP32 点灯](/v0.3/guide/esp32-blink) - 端到端点灯宿主
- [体积仪表盘](/v0.3/guide/footprint) - 产物体积与对比注意点
- [API 参考](/v0.3/reference/) - 完整 API 文档
- [类型系统](/v0.3/reference/types) - 类型详解
- [内置函数](/v0.3/reference/builtins) - 内置函数参考
