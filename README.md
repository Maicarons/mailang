# MaìLang (麦语)

[![CI](https://github.com/Maicarons/mailang/actions/workflows/ci.yml/badge.svg)](https://github.com/Maicarons/mailang/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.3.0-green.svg)](https://github.com/Maicarons/mailang/releases/tag/v0.3.0)

A modern programming language designed for IoT and cross-platform development, written in Rust.

**Repository**: [github.com/Maicarons/mailang](https://github.com/Maicarons/mailang)
**Documentation**: [Maicarons.github.io/mailang](https://maicarons.github.io/mailang/)
**Releases**: [v0.3.0](https://github.com/Maicarons/mailang/releases/tag/v0.3.0)

## Features

- **Object-Oriented Programming** - Classes, inheritance, constructors, `super()` / `super.method()`, traits (`implements`, `trait extends`)
- **Closures & Pattern Matching** - First-class functions; literals, ranges, `Ok`/`Err`/`Some`, or-patterns, guards, array/tuple destructure, postfix `expr match { }`, `?` operator
- **Collections** - Array/Map/str methods (`push`, `keys`, `split`, …)
- **Generics** - Generic function monomorphization via turbofish (`id::<int>` → `id$int`); `Result<T,E>` / `Option<T>` annotations
- **UTF-8 Native** - Full Unicode support for identifiers and strings
- **Stack-based Bytecode VM** - Slot locals, TCO, CallDirect, ~4× faster recursive fib vs 0.1; optional register VM with feature parity
- **GC** - Rc + `collect_cycles()` + mark-sweep heap (`gc_stats()`)
- **Embeddable** - C FFI host functions, no_std bytecode crate, `.mailangbc` artifacts
- **WebAssembly** - Run in browsers and Node
- **IoT Ready** - Simulated HAL builtins, embedded size CI
- **Packages** - Filesystem / static-HTTP registry: `mailang publish/install/search/yank/registry`
- **Tooling** - Analyzer, LSP (multi-error diagnostics), `mailang fmt`, module system v2

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
fn greet(name: str) -> str {
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

## Testing

Integration suite: **102/102** on both the stack VM (default) and the register VM (`MAILANG_VM=register` / `--vm=register`).

## Current limitations

- **Package registry is local/HTTP-static** — `mailang publish/install/search/yank` against a filesystem registry (or static HTTP mirror); no public hosted registry service.
- **Generic functions monomorphize; generic classes erase fields** — turbofish `id::<int>(x)` specializes (`id$int`); `Result<T,E>` / `Option<T>` annotations work. Generic classes use one erased layout (fields stay dynamic).
- **`async` is not supported** — no async/await runtime or syntax.
- **GC is Rc + mark-sweep** — reference counting, `collect_cycles()` edge-cut, and a `MarkSweepHeap` wired into the stack VM (see `gc_stats()`). Incremental/concurrent GC not implemented.
- **IoT HAL is simulated by default** — `gpio_*` / `adc_read` / `delay_ms` use a host simulation unless a real HAL is registered via FFI. No real-hardware validation claimed.
- **Register VM is opt-in** — stack VM remains the default; `MAILANG_VM=register` / `--vm=register` passes the same 102 integration tests (feature parity).
- **`mailang-macros` is a passthrough stub** — no real procedural macros yet.
- **crates.io publish requires `CARGO_REGISTRY_TOKEN`** — CI skips crates.io when the secret is unset (local/HTTP registry works offline).

## License

Licensed under [Apache License 2.0](LICENSE-APACHE).
