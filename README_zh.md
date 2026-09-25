# MaìLang (麦语)

[![CI](https://github.com/Maicarons/mailang/actions/workflows/ci.yml/badge.svg)](https://github.com/Maicarons/mailang/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.3.0-green.svg)](https://github.com/Maicarons/mailang/releases/tag/v0.3.0)

一个为 IoT 和跨平台开发设计的现代编程语言，使用 Rust 编写。

**仓库地址**：[github.com/Maicarons/mailang](https://github.com/Maicarons/mailang)
**在线文档**：[Maicarons.github.io/mailang](https://maicarons.github.io/mailang/)
**版本发布**：[v0.3.0](https://github.com/Maicarons/mailang/releases/tag/v0.3.0)

## 特性

- **面向对象编程** - 类、继承、构造函数、`super()` / `super.method()`、Trait（`implements`、`trait extends`）
- **闭包与模式匹配** - 一等函数；字面量、范围、`Ok`/`Err`/`Some`、或模式、守卫、数组/元组解构、后缀 `expr match { }`、`?` 操作符
- **集合** - 数组/字典/字符串方法（`push`、`keys`、`split` 等）
- **泛型** - 泛型函数 monomorphization（turbofish `id::<int>` → `id$int`）；`Result<T,E>` / `Option<T>` 标注
- **UTF-8 原生支持** - 完整 Unicode 支持
- **栈式字节码 VM** - slot 局部变量、TCO、CallDirect，递归 fib 约为 0.1 的 4 倍；另有特性对齐的可选寄存器 VM
- **GC** - Rc + `collect_cycles()` + mark-sweep 堆（`gc_stats()`）
- **可嵌入** - C FFI 宿主函数、no_std 字节码 crate、`.mailangbc` 产物
- **WebAssembly** - 浏览器与 Node
- **IoT 就绪** - 模拟 HAL 内置、嵌入式体积 CI
- **包管理** - 文件系统 / 静态 HTTP 注册表：`mailang publish/install/search/yank/registry`
- **工具链** - Analyzer、LSP（多错误诊断）、`mailang fmt`、模块系统 v2

## 快速开始

```bash
# 安装
cargo install mailang-cli

# 运行文件
mailang run hello.mai

# REPL 交互
mailang

# 内联代码执行
mailang eval 'println("你好，世界！")'
```

## 示例

```
// hello.mai
fn greet(name: str) -> str {
    return "你好，{name}！"
}

println(greet("麦语"))
```

## 项目结构

```
mailang/
├── crates/                    # Rust 代码
│   ├── mailang-lexer/         # 词法分析器
│   ├── mailang-parser/        # 语法分析器
│   ├── mailang-ast/           # AST 定义
│   ├── mailang-analyzer/      # 语义分析
│   ├── mailang-compiler/      # 字节码编译器
│   ├── mailang-bytecode/      # 字节码定义
│   ├── mailang-vm/            # 虚拟机
│   ├── mailang-gc/            # 垃圾回收器
│   ├── mailang-stdlib/        # 标准库
│   ├── mailang-core/          # 核心集成
│   ├── mailang-cli/           # 命令行工具
│   ├── mailang-ffi/           # C FFI 层
│   ├── mailang-wasm/          # WebAssembly 绑定
│   ├── mailang-lsp/           # Language Server Protocol
│   └── mailang-macros/        # 过程宏
├── bindings/                  # 12 种语言绑定示例
├── docs/                      # VitePress 文档
├── examples/                  # 示例程序
└── tests/                     # 集成测试
```

## 构建

```bash
# 默认构建
cargo build

# 发布构建
cargo build --release

# 嵌入式构建
cargo build --no-default-features --target thumbv7em-none-eabihf
```

## 测试

集成测试：栈式 VM（默认）与寄存器 VM（`MAILANG_VM=register` / `--vm=register`）均为 **102/102**。

## 当前限制

- **包注册表为本地/静态 HTTP** — 支持 `mailang publish/install/search/yank`（文件系统注册表或静态 HTTP 镜像）；无公共托管注册表服务。
- **泛型函数可单态化；泛型类字段擦除** — turbofish `id::<int>(x)` 会特化（`id$int`）；`Result<T,E>` / `Option<T>` 标注可用。泛型类共用一份擦除布局（字段仍动态）。
- **不支持 `async`** — 无 async/await 运行时或语法。
- **GC 为 Rc + mark-sweep** — 引用计数、`collect_cycles()` 断环，以及接入栈式 VM 的 `MarkSweepHeap`（见 `gc_stats()`）。尚无增量/并发 GC。
- **IoT HAL 默认为模拟实现** — `gpio_*` / `adc_read` / `delay_ms` 在未通过 FFI 注册真实 HAL 时使用宿主模拟；不声称真实硬件验证。
- **寄存器 VM 为可选后端** — 默认仍为栈式 VM；`MAILANG_VM=register` / `--vm=register` 通过同一套 102 项集成测试（特性对齐）。
- **`mailang-macros` 仍为 passthrough 桩** — 尚无真实过程宏。
- **crates.io 发布需要 `CARGO_REGISTRY_TOKEN`** — secret 未配置时 CI 跳过 crates.io（本地/HTTP 注册表可离线使用）。

## 许可证

Apache 2.0 许可证。
