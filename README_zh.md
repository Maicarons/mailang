# MaìLang (麦语)

[![CI](https://github.com/Maicarons/mailang/actions/workflows/ci.yml/badge.svg)](https://github.com/Maicarons/mailang/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.2.0-green.svg)](https://github.com/Maicarons/mailang/releases/tag/v0.2.0)

一个为 IoT 和跨平台开发设计的现代编程语言，使用 Rust 编写。

**仓库地址**：[github.com/Maicarons/mailang](https://github.com/Maicarons/mailang)
**在线文档**：[Maicarons.github.io/mailang](https://maicarons.github.io/mailang/)
**版本发布**：[v0.2.0](https://github.com/Maicarons/mailang/releases/tag/v0.2.0)

## 特性

- **面向对象编程** - 类、继承、构造函数、`super()`、Trait
- **闭包与模式匹配** - 一等函数；字面量、范围、`Ok`/`Err`/`Some`
- **UTF-8 原生支持** - 完整 Unicode 支持
- **栈式字节码 VM** - slot 局部变量、TCO、CallDirect，递归 fib 约为 0.1 的 4 倍
- **可嵌入** - C FFI 宿主函数、no_std 字节码 crate、`.mailangbc` 产物
- **WebAssembly** - 浏览器与 Node
- **IoT 就绪** - 模拟 HAL 内置、嵌入式体积 CI
- **工具链** - Analyzer、LSP、`mailang fmt`、模块系统 v2

## 项目标签

| 标签 | 说明 |
|------|------|
| [`v0.2.0`](https://github.com/Maicarons/mailang/releases/tag/v0.2.0) | Phase B–F（语言 / IoT / FFI / 工具 / 性能） |
| [`v0.1.0`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0) | Phase 1 完整发布 |
| [`v0.1.0-lexer`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-lexer) | Unicode-aware 词法分析器 |
| [`v0.1.0-parser`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-parser) | 递归下降 + Pratt 语法分析器 |
| [`v0.1.0-vm`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-vm) | 栈式字节码虚拟机 |
| [`v0.1.0-oop`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-oop) | 类系统与继承 |
| [`v0.1.0-closures`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-closures) | 闭包 upvalue 捕获 |
| [`v0.1.0-pattern-match`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-pattern-match) | 模式匹配引擎 |
| [`v0.1.0-ffi`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-ffi) | C FFI 层 |
| [`v0.1.0-wasm`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-wasm) | WebAssembly 绑定 |
| [`v0.1.0-stdlib`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-stdlib) | 标准库 |
| [`v0.1.0-cli`](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-cli) | CLI 与 REPL |
| [`phase-a-complete`](https://github.com/Maicarons/mailang/releases/tag/phase-a-complete) | Phase A 里程碑 |

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
fn greet(name = "世界") -> str {
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

## 许可证

Apache 2.0 许可证。
