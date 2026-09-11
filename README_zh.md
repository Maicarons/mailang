# MaìLang (麦语)

[![CI](https://github.com/Maicarons/mailang/actions/workflows/ci.yml/badge.svg)](https://github.com/Maicarons/mailang/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.1.0-green.svg)](https://github.com/Maicarons/mailang/releases/tag/v0.1.0)

一个为 IoT 和跨平台开发设计的现代编程语言，使用 Rust 编写。

**仓库地址**：[github.com/Maicarons/mailang](https://github.com/Maicarons/mailang)
**在线文档**：[Maicarons.github.io/mailang](https://maicarons.github.io/mailang/)
**版本发布**：[v0.1.0](https://github.com/Maicarons/mailang/releases/tag/v0.1.0)

## 特性

- **面向对象编程** - 类、继承、构造函数、`super()` 调用
- **闭包** - 一等公民函数，支持 upvalue 捕获
- **模式匹配** - 字面量、通配符、or-pattern、守卫表达式
- **UTF-8 原生支持** - 完整 Unicode 支持，标识符和字符串可使用任何文字
- **栈式字节码 VM** - 基于 slot 的高性能执行
- **12 种语言 FFI** - 从 C、Python、JavaScript、Java、Go 等调用 MaìLang
- **WebAssembly** - 在浏览器和边缘环境中运行
- **IoT 就绪** - 三级特性门控，支持嵌入式设备

## 项目标签

| 标签 | 说明 |
|------|------|
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
