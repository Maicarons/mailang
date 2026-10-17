# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
cargo fmt --all

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
- **mailang-analyzer**: Semantic analysis and type checking (wired into the pipeline; `analyze()` + diagnostics). Depends on mailang-ast.
- **mailang-compiler**: Bytecode compiler. AST (`Program`) → `Bytecode`. Depends on mailang-ast, mailang-bytecode.
- **mailang-bytecode**: Bytecode IR definitions (`Opcode`, `Value`, `Instruction`, `Chunk`, `Bytecode`) plus the register IR (`RegChunk`, `RegOp`) and the `StackToRegister` translator. Shared between compiler and VM.
- **mailang-vm**: Two interchangeable backends over the same `Bytecode`: the register (three-address) `RegisterVm` (the default since M5, stack bytecode is lowered via `StackToRegister`) and the stack `Vm` (opt-in via `--vm=stack` / `MAILANG_VM=stack`). Executes and returns `Value`.
- **mailang-stdlib**: Built-in functions (`println`, `sqrt`, `len`, `parse_int`, etc.) operating on `mailang_bytecode::Value`.
- **mailang-core**: Glue crate. Re-exports all above. Contains `MailangInterpreter` which orchestrates Parser → Compiler → VM pipeline, plus the `mailang fmt` formatter. This is the main integration point.
- **mailang-cli**: CLI binary. Uses `clap` for subcommands (`run`, `eval`, `build`, `fmt`, `deps`, `publish`, `install`, `registry`, `lsp`) and provides an interactive REPL. On Windows it also bundles real UI backends selected with `--host=sim|tui|gui` (`hostwin.rs`): `tui` drives the console (stdout, `GetConsoleScreenBufferInfo`, non-blocking `ReadConsoleInputW`), `gui` opens a real window and paints with GDI. `sim` is the default and installs nothing.
- **mailang-ffi**: C ABI layer (`extern "C"` functions). Uses `cbindgen` to generate `mailang.h`. Wraps `mailang-core`.
- **mailang-wasm**: WebAssembly bindings via `wasm-bindgen`. Wraps `mailang-core`.
- **mailang-lsp**: Language Server Protocol server (diagnostics via parser error recovery, completions). Uses `tower-lsp`.
- **mailang-gc**: Garbage collector: `collect_cycles()` edge-cut plus `MarkSweepHeap`, used by both VM backends.
- **mailang-module**: Package registry (filesystem / HTTP static mirror) and `mailang.toml` path dependencies.
- **mailang-macros**: Procedural macros (passthrough stub, no call sites yet).

### Key Design Decisions

- **Register VM by default, stack VM as the opt-out (M5)**: both execute the same `Bytecode` (the register VM via `StackToRegister` lowering) and pass the same 137 integration tests. `MAILANG_VM` / `--vm` selects the backend with the flag winning over the environment; `stack` is the only supported opt-out value, everything else (including unset/empty) means the register default. CI runs the whole suite both ways. `benchmark/compare_backends.ps1` is the byte-for-byte equivalence gate over every `.mai` program in `examples/` + `libs/` (currently 64/64 identical).
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

v0.3.1 (Phase N + M5 shipped). The pipeline is complete: OOP with inheritance/`super`, traits, pattern matching, closures, error handling (`Result`/`Option`/`?`), TCO, generics with turbofish monomorphization, a filesystem/HTTP package registry, bundled Win32 `--host=tui|gui` backends for the `libs/tui` + `libs/gui` packs, and dual stack/register VMs that agree on every shipped program (137 integration tests, 261 workspace tests, green both ways, plus `compare_backends.ps1` 64/64). **The register VM is the default backend**; `--vm=stack` / `MAILANG_VM=stack` opt back out.

Known gaps are listed in `AGENTS.md` ("Remaining gaps") and tracked in `ROADMAP.md` Phase M: generic *classes* erase their fields, `async` is not supported, the IoT HAL is simulated by default, `mailang-macros` is a passthrough stub, and allocation-heavy loops keep a bounded working set rather than being fully reclaimed between collections (`benchmark\measure_memory.ps1` gates it).
