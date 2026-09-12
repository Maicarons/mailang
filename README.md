# MaìLang (麦语)

[![CI](https://github.com/Maicarons/mailang/actions/workflows/ci.yml/badge.svg)](https://github.com/Maicarons/mailang/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.2.2-green.svg)](https://github.com/Maicarons/mailang/releases/tag/v0.2.2)

A modern programming language designed for IoT and cross-platform development, written in Rust.

**Repository**: [github.com/Maicarons/mailang](https://github.com/Maicarons/mailang)
**Documentation**: [Maicarons.github.io/mailang](https://maicarons.github.io/mailang/)
**Releases**: [v0.2.2](https://github.com/Maicarons/mailang/releases/tag/v0.2.2)

## Features

- **Object-Oriented Programming** - Classes, inheritance, constructors, `super()`, traits
- **Closures & Pattern Matching** - First-class functions; literals, ranges, `Ok`/`Err`/`Some`, `?` operator
- **Collections** - Array/Map/str methods (`push`, `keys`, `split`, …)
- **UTF-8 Native** - Full Unicode support for identifiers and strings
- **Stack-based Bytecode VM** - Slot locals, TCO, CallDirect, ~4× faster recursive fib vs 0.1
- **Embeddable** - C FFI host functions, no_std bytecode crate, `.mailangbc` artifacts
- **WebAssembly** - Run in browsers and Node
- **IoT Ready** - Simulated HAL builtins, embedded size CI
- **Tooling** - Analyzer, LSP, `mailang fmt`, module system v2

## Quick Start

```bash
# Install
cargo install mailang-cli

# Run a file
mailang run hello.mai

# REPL
mailang

# Evaluate inline code
mailang eval 'println("Hello, 世界!")'
```

## Example

```
// hello.mai
fn greet(name = "世界") -> str {
    return "你好，{name}！"
}

println(greet("MaìLang"))
```

## Project Structure

```
mailang/
├── crates/                    # Rust crates
│   ├── mailang-lexer/         # Lexer (Unicode-aware tokenizer)
│   ├── mailang-parser/        # Parser (recursive descent + Pratt)
│   ├── mailang-ast/           # AST definitions
│   ├── mailang-analyzer/      # Semantic analysis
│   ├── mailang-compiler/      # Bytecode compiler
│   ├── mailang-bytecode/      # Bytecode definitions
│   ├── mailang-vm/            # Virtual machine
│   ├── mailang-gc/            # Garbage collector
│   ├── mailang-stdlib/        # Standard library
│   ├── mailang-core/          # Core integration
│   ├── mailang-cli/           # CLI tool
│   ├── mailang-ffi/           # C FFI layer
│   ├── mailang-wasm/          # WebAssembly bindings
│   ├── mailang-lsp/           # Language Server Protocol
│   └── mailang-macros/        # Proc macros
├── bindings/                  # 12 language binding demos
├── docs/                      # VitePress documentation
├── examples/                  # Example programs
└── tests/                     # Integration tests
```

## Language Syntax

```
// Variables
let x = 42              // immutable
var y = 100             // mutable
const PI = 3.14159      // constant

// Functions
fn add(a: int, b: int) -> int {
    return a + b
}

// Classes
class Animal {
    let name: str
    fn init(name: str) {
        this.name = name
    }
    fn speak() -> str {
        return "{this.name} speaks"
    }
}

class Dog extends Animal {
    override fn speak() -> str {
        return "{this.name} barks!"
    }
}

// Traits
trait Printable {
    fn to_string() -> str
    fn print() {
        println(this.to_string())
    }
}

// Pattern matching
let result = match x {
    0 => "zero",
    1..10 => "one to nine",
    _ => "other"
}

// Error handling
fn divide(a: float, b: float) -> Result<float, str> {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}
```

## Building

```bash
# Default build
cargo build

# Release build
cargo build --release

# With specific features
cargo build --features "std,gc,io"

# Embedded (no_std)
cargo build --no-default-features --target thumbv7em-none-eabihf
```

## License

Licensed under [Apache License 2.0](LICENSE-APACHE).
