# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

MaìLang (麦语) is a programming language for IoT and cross-platform development, implemented in Rust. The language supports OOP (classes, traits), pattern matching, error handling (Result/Option), string interpolation, and UTF-8 identifiers. File extension: `.mai`.

## Build & Test Commands

```bash
# Build the workspace
cargo build

# Run the CLI (REPL mode if no arguments)
cargo run -p mailang-cli

# Run a .mai file
cargo run -p mailang-cli -- run examples/hello.mai

# Evaluate inline code
cargo run -p mailang-cli -- eval '1 + 2'

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p mailang-lexer
cargo test -p mailang-parser

# Run a single test by name
cargo test test_basic_arithmetic

# Clippy and formatting
cargo clippy --workspace
cargo fmt --workspace

# Build WASM target
cargo build -p mailang-wasm --target wasm32-unknown-unknown

# Cross-compile for embedded targets (requires `cross` tool)
cross build -p mailang-cli --target thumbv7em-none-eabihf
cross build -p mailang-cli --target riscv32imc-unknown-none-elf
```

## Architecture

The interpreter follows a classic pipeline, split across dedicated crates:

```
Source (.mai) → Lexer → Parser → Compiler → VM
```

### Crate Dependency Flow

- **mailang-lexer**: Unicode-aware tokenizer. Source → `Vec<Token>`. Exports `Token`, `Lexer`, `LexerError`.
- **mailang-ast**: Shared AST type definitions (`Program`, `Stmt`, `Expr`, `Pattern`, `Literal`, etc.). No logic, just data structures with serde support.
- **mailang-parser**: Recursive descent + Pratt parsing. Tokens → `Program` (AST). Depends on mailang-ast.
- **mailang-analyzer**: Semantic analysis and type checking (stub/in-progress). Depends on mailang-ast.
- **mailang-compiler**: Bytecode compiler. AST (`Program`) → `Bytecode`. Depends on mailang-ast, mailang-bytecode.
- **mailang-bytecode**: Bytecode IR definitions (`Opcode`, `Value`, `Instruction`, `Chunk`, `Bytecode`). Shared between compiler and VM.
- **mailang-vm**: Stack-based bytecode virtual machine with slot locals. Executes `Bytecode`, returns `Value`.
- **mailang-stdlib**: Built-in functions (`println`, `sqrt`, `len`, `parse_int`, etc.) operating on `mailang_bytecode::Value`.
- **mailang-core**: Glue crate. Re-exports all above. Contains `MailangInterpreter` which orchestrates Parser → Compiler → VM pipeline. This is the main integration point.
- **mailang-cli**: CLI binary. Uses `clap` for subcommands (`run`, `eval`) and provides an interactive REPL.
- **mailang-ffi**: C ABI layer (`extern "C"` functions). Uses `cbindgen` to generate `mailang.h`. Wraps `mailang-core`.
- **mailang-wasm**: WebAssembly bindings via `wasm-bindgen`. Wraps `mailang-core`.
- **mailang-lsp**: Language Server Protocol server (diagnostics/completions; multi-error syntax diagnostics via parser recovery). Uses `tower-lsp`.
- **mailang-gc**: Garbage collector (`collect_cycles()` edge-cut + `MarkSweepHeap` wired into the stack VM).
- **mailang-macros**: Procedural macros (passthrough stub).

### Key Design Decisions

- **Stack-based bytecode VM + slot locals** (not a register VM) for predictable performance on IoT devices.
- **Value enum** in mailang-bytecode is the universal runtime type: `Null`, `Bool`, `Int(i64)`, `Float(f64)`, `Str`, `Char`, `Array`, `Map`, `Tuple`, `Function`, `Closure`, `Class`, `Instance`, `Ok`, `Err`, `Some`.
- **Local vs global variable resolution**: Compiler resolves locals by stack index, globals by constant-pool name. Closures capture via upvalues.
- **Chunk-based bytecode**: Each function gets its own `Chunk` with independent instruction and constant lists. Main code is chunk 0.

### Cross-Compilation Targets

Defined in `rust-toolchain.toml`:
- `wasm32-unknown-unknown` (browser/edge)
- `thumbv7em-none-eabihf` (ARM Cortex-M4 MCU)
- `riscv32imc-unknown-none-elf` (ESP32-C3, RISC-V IoT)

`Cross.toml` configures Docker images for `cross` on Linux ARM/RISC-V targets.

### FFI Header Generation

`cbindgen.toml` configures auto-generation of C headers. Run `cbindgen` to produce `mailang.h` from the `mailang-ffi` crate. Feature flags `std`/`gc` map to C defines `MAILANG_STD`/`MAILANG_GC`.

## Current Status

**Phase L (both VMs pass the full suite; docs/release prep)**. Runtime supports OOP (inherited constructors, `super()` ctor, `super.method()`, inherited methods), traits (`implements`, `trait extends`), pattern matching (literals/ranges/`Ok`/`Err`/`Some`/or/guards + array/tuple match destructure), `let`/`var` destructuring, default parameter values, postfix `expr match { }`, mutability enforcement (`let`/`const` reject assignment), `?`, TCO, collections/str methods, global `read_file`/`write_file`, `.mailangbc`, simulated HAL, C FFI, analyzer/LSP/fmt, module v2 with `mailang.toml` path deps (`mailang deps`), **filesystem/HTTP-static package registry** (`publish`/`install`/`search`/`yank`), **generic function monomorphization** (turbofish `id::<int>` → `id$int`; generic classes erase fields), **GC = Rc + `collect_cycles()` + `MarkSweepHeap`**, and **parser error recovery** (`recover_from_error` / `parse_program_recovering`; LSP reports multiple syntax errors). Fib(30) ~137 ms (~4.25× baseline). **102 integration tests on the stack VM (default) and 102/102 on the register VM** (`MAILANG_VM=register` / `--vm=register`) — feature parity.

### Remaining gaps (honest)

- No public hosted package registry (local/`http(s)` static mirror only).
- Generic classes are field-erased (no per-type layout monomorphization).
- `async` not supported.
- IoT HAL is simulated by default until a real HAL is registered via FFI (no real-hardware validation claimed).
- Register VM (`MAILANG_VM=register` / `--vm=register`) is opt-in and feature-parity with the stack VM, but the stack VM remains the default and primary backend.
- `mailang-macros` is still a passthrough stub.
- VM has no `set_debug` / `memory_stats` APIs.
