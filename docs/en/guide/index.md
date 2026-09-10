# Introduction

MaìLang is a modern programming language designed for **IoT and cross-platform development**, written in Rust. It combines the best features of multiple languages to provide a simple, efficient, and safe programming experience.

## Design Philosophy

### Simple and Easy to Learn
MaìLang uses a hybrid syntax style, borrowing from Rust, Python, and JavaScript:
- Rust-like variable declarations and type system
- Python-like concise function syntax
- JavaScript-like object operations

### High Performance
Uses a **register-based bytecode virtual machine**, faster than tree-walk interpreters (like Rhai) and more efficient than stack-based VMs (like Rune):
- Bytecode can be serialized and cached for faster startup
- Reduced memory access, suitable for IoT scenarios
- Register allocation optimization

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
| Execution Model | Register VM | Tree Walk | Stack VM | Stack VM |
| OOP Support | ✅ Full | ❌ Limited | ❌ Limited | ✅ Full |
| Embedded Support | ✅ 64KB+ | ✅ | ❌ | ❌ |
| FFI | ✅ 12 Languages | ✅ Rust | ❌ | ✅ Python |
| Type System | Hybrid | Dynamic | Dynamic | Dynamic |
| Pattern Matching | ✅ | ❌ | ✅ | ❌ |
| Async Support | Planned | ❌ | ✅ | ❌ |

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

- [Getting Started](/en/guide/getting-started) - Installation and first program
- [Syntax Guide](/en/guide/syntax) - Complete syntax reference
- [OOP](/en/guide/oop) - Classes, inheritance, traits
- [Standard Library](/en/guide/stdlib) - Built-in modules
- [FFI](/en/guide/ffi) - Language integration guide
- [IoT](/en/guide/iot) - Embedded compilation and deployment
