# Introduction

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [Online Playground](https://maicarons.github.io/mailang/playground) · [Issue Tracker](https://github.com/Maicarons/mailang/issues)

MaìLang is a modern programming language designed for **IoT and cross-platform development**, written in Rust. It combines the best features of multiple languages to provide a simple, efficient, and safe programming experience.

## Key Features

| Feature | Description | Status |
|---------|-------------|--------|
| OOP | Classes, inheritance, constructors, `super()` / `super.method()` | ✅ Implemented |
| Traits | `implements`, default methods, `trait extends` | ✅ Implemented |
| Closures | Upvalue capture, first-class functions | ✅ Implemented |
| Pattern Matching | Literals, ranges, or-patterns, guards, array/tuple destructure, postfix `expr match { }` | ✅ Implemented |
| Error Handling | Result/Option + `?` propagation | ✅ Implemented |
| Default Params / Destructuring | `fn f(a, b = 10)`; `let (x, y) = …` | ✅ Implemented |
| Generics | Generic-function monomorphization (turbofish); field-erased generic classes | ✅ Implemented |
| UTF-8 Native | Identifiers support any Unicode script | ✅ Implemented |
| Stack-based Bytecode VM | Slot locals, high-performance execution | ✅ Implemented |
| Register VM (opt-in) | `--vm=register`, feature-parity with the stack VM | ✅ Implemented |
| GC | Rc + `collect_cycles()` + mark-sweep heap | ✅ Implemented |
| Package Registry | Filesystem / static HTTP: `publish`/`install`/`search`/`yank` | ✅ Implemented |
| C FFI | 12-language binding support | ✅ Implemented |
| WebAssembly | Browser/edge runtime | ✅ Implemented |
| CLI + REPL | Command-line tool | ✅ Implemented |

## Project Resources

- **Source Code**: [github.com/Maicarons/mailang](https://github.com/Maicarons/mailang)
- **Releases**: [Releases](https://github.com/Maicarons/mailang/releases)
- **Tags**: [Tags](https://github.com/Maicarons/mailang/tags)
- **Contributing**: [CONTRIBUTING.md](https://github.com/Maicarons/mailang/blob/master/CONTRIBUTING.md)
- **Roadmap**: [ROADMAP.md](https://github.com/Maicarons/mailang/blob/master/ROADMAP.md)
- **Report**: [REPORT.md](https://github.com/Maicarons/mailang/blob/master/REPORT.md)

## Design Philosophy

### Simple and Easy to Learn
MaìLang uses a hybrid syntax style, borrowing from Rust, Python, and JavaScript:
- Rust-like variable declarations and type system
- Python-like concise function syntax
- JavaScript-like object operations

### High Performance
Uses a **stack-based bytecode virtual machine** with slot locals, faster than tree-walk interpreters (like Rhai):
- Bytecode can be serialized and cached for faster startup
- Locals use slot indices (no hash lookup), suitable for IoT scenarios
- Hot-path specialization and CallDirect calls

### Universal Platform Support
Through three-tier feature gating, MaìLang can run on all platforms from 64KB IoT chips to desktop servers:

| Level | Memory | Typical Devices | Features |
|-------|--------|-----------------|----------|
| `no_std` | 64KB | ARM Cortex-M0 | Stack-only values |
| `no_std + alloc` | 128KB | RISC-V, ESP32 | Reference counting, no GC |
| `std` | 256KB+ | Raspberry Pi, Servers | Full features |

### UTF-8 Native Support
MaìLang enforces UTF-8 encoding, identifiers support any Unicode script:
```
let 中文变量 = "支持中文"
let مرحبا = "支持阿拉伯文"
let 🔥 = "支持emoji"
```

### Safe and Reliable
- Strong type system with optional type inference
- Result/Option error handling, no exceptions
- Optional ownership system

## Comparison with Other Languages

| Feature | MaìLang | Rhai | Rune | RustPython |
|---------|---------|------|------|------------|
| Execution Model | Stack VM + slot locals | Tree Walk | Stack VM | Stack VM |
| OOP Support | ✅ Full | ❌ Limited | ❌ Limited | ✅ Full |
| Embedded Support | ✅ 64KB+ | ✅ | ❌ | ❌ |
| FFI | ✅ 12 Languages | ✅ Rust | ❌ | ✅ Python |
| Type System | Hybrid | Dynamic | Dynamic | Dynamic |
| Pattern Matching | ✅ | ❌ | ✅ | ❌ |
| Async Support | Not supported (no async runtime) | ❌ | ✅ | ❌ |

## Use Cases

### IoT and Embedded
- Sensor data processing
- Device configuration scripts
- Edge computing logic

### Game Development
- Game logic scripting
- AI behavior trees
- Configuration file parsing

### Automation
- Data processing pipelines
- System administration scripts
- Test automation

### Education
- Programming introduction
- Language design learning
- Compiler theory practice

## Quick Start

```bash
# Install
cargo install mailang-cli

# REPL
mailang

# Run a file
mailang run hello.mai

# Inline execution
mailang eval 'println("Hello, MaìLang!")'
```

## Next Steps

- [Getting Started](/v0.3/en/guide/getting-started) - Installation and first program
- [Syntax Guide](/v0.3/en/guide/syntax) - Complete syntax reference
- [OOP](/v0.3/en/guide/oop) - Classes, inheritance, traits
- [Standard Library](/v0.3/en/guide/stdlib) - Built-in modules
- [Package Registry](/v0.3/en/guide/registry) - Publish, install, search packages
- [FFI](/v0.3/en/guide/ffi) - Language integration guide
- [WASM](/v0.3/en/guide/wasm) - Browser and edge integration
- [IoT](/v0.3/en/guide/iot) - Embedded compilation and deployment
- [Real Hardware](/v0.3/en/guide/hardware) - ESP32-C3 / Cortex-M4
- [ESP32 Blink](/v0.3/en/guide/esp32-blink) - First firmware walkthrough
- [Footprint](/v0.3/en/guide/footprint) - Measuring artifact sizes
- [Playground](/v0.3/en/guide/playground) - Run MaìLang in the browser
