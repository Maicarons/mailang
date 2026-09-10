# MaìLang (麦语)

A modern programming language designed for IoT and cross-platform development, written in Rust.

## Features

- **Object-Oriented Programming** - Classes, inheritance, traits, and polymorphism
- **UTF-8 Native** - Full Unicode support for identifiers and strings
- **Register-based Bytecode VM** - Fast execution with register-based virtual machine
- **IoT Ready** - Three-tier feature gating for embedded devices (64KB+)
- **12 Language FFI** - Call MaìLang from C, Python, JavaScript, Java, Go, and more
- **Pattern Matching** - Powerful match expressions
- **Error Handling** - Result/Option types for safe error handling
- **Lambda/Closures** - First-class functions

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
