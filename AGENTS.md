# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

Ma矛Lang (楹﹁) is a programming language for IoT and cross-platform development, implemented in Rust. The language supports OOP (classes, traits), pattern matching, error handling (Result/Option), string interpolation, and UTF-8 identifiers. File extension: `.mai`.

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

**Platform-gated code needs a cross-target check.** `cargo check`, `clippy` and `test` only compile
the branch that matches the host, so anything behind `#[cfg(not(windows))]` (or a `#[cfg(windows)]`
one on Linux) can be broken while every local command is green. That is exactly how a missing
`PartialEq` on the CLI's `--host` enum passed here on Windows and failed the `Check`/`Clippy`/`Test`
jobs on `ubuntu-latest` with `E0369`. Before pushing a change that touches such a branch, compile
for the other platform too:

```bash
cargo check --workspace --target x86_64-unknown-linux-gnu
cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
```

## Architecture

The interpreter follows a classic pipeline, split across dedicated crates:

```
Source (.mai) 鈫?Lexer 鈫?Parser 鈫?Compiler 鈫?VM
```

### Crate Dependency Flow

- **mailang-lexer**: Unicode-aware tokenizer. Source 鈫?`Vec<Token>`. Exports `Token`, `Lexer`, `LexerError`.
- **mailang-ast**: Shared AST type definitions (`Program`, `Stmt`, `Expr`, `Pattern`, `Literal`, etc.). No logic, just data structures with serde support.
- **mailang-parser**: Recursive descent + Pratt parsing. Tokens 鈫?`Program` (AST). Depends on mailang-ast.
- **mailang-analyzer**: Semantic analysis and type checking (stub/in-progress). Depends on mailang-ast.
- **mailang-compiler**: Bytecode compiler. AST (`Program`) 鈫?`Bytecode`. Depends on mailang-ast, mailang-bytecode.
- **mailang-bytecode**: Bytecode IR definitions (`Opcode`, `Value`, `Instruction`, `Chunk`, `Bytecode`). Shared between compiler and VM.
- **mailang-vm**: Stack-based bytecode virtual machine with slot locals. Executes `Bytecode`, returns `Value`.
- **mailang-stdlib**: Built-in functions (`println`, `sqrt`, `len`, `parse_int`, etc.) operating on `mailang_bytecode::Value`.
- **mailang-core**: Glue crate. Re-exports all above. Contains `MailangInterpreter` which orchestrates Parser 鈫?Compiler 鈫?VM pipeline. This is the main integration point.
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

**Phase N (both VM backends agree on every shipped program)**. Runtime supports OOP (inherited constructors, `super()` ctor, `super.method()`, inherited methods), traits (`implements`, `trait extends`), pattern matching (literals/ranges/`Ok`/`Err`/`Some`/or/guards + array/tuple match destructure), `let`/`var` destructuring, default parameter values, postfix `expr match { }`, mutability enforcement (`let`/`const` reject assignment), `?`, TCO, collections/str methods, global `read_file`/`write_file`, `.mailangbc`, simulated HAL, C FFI, analyzer/LSP/fmt, module v2 with `mailang.toml` path deps (`mailang deps`), **filesystem/HTTP-static package registry** (`publish`/`install`/`search`/`yank`), **generic function monomorphization** (turbofish `id::<int>` 鈫?`id$int`; generic classes erase fields), **GC = Rc + `collect_cycles()` + `MarkSweepHeap`**, **parser error recovery** (`recover_from_error` / `parse_program_recovering`; LSP reports multiple syntax errors), and **bundled Win32 host backends for the `tui`/`gui` packs** (`mailang run --host=sim|tui|gui`, Windows only). Fib(30) on Windows release runs at roughly the same speed on both backends — **~1.0x** (stack and register within a few percent, both ~140–170 ms depending on machine load; the register VM was ~1150 ms / ~10x slower before the Phase M optimisation). Gate: `benchmark/measure_fib.ps1` (≤2.0x). **125 integration tests on the stack VM (default) and 125/125 on the register VM** (`MAILANG_VM=register` / `--vm=register`), both enforced by the CI test matrix.

### Remaining gaps (honest)

- No public hosted package registry (local/`http(s)` static mirror only).
- Generic classes are field-erased (no per-type layout monomorphization).
- `async` not supported.
- IoT HAL is simulated by default. A real HAL can be registered through FFI/host functions, but no real-hardware validation is claimed for it.
- Register VM (`MAILANG_VM=register` / `--vm=register`) is opt-in; it matches the stack VM on the whole test suite (125/125, CI matrix) and on `benchmark/compare_backends.ps1` (64/64 identical), and it is on par for `bench_fib30` (~1.0x, well within the ≤2.0x gate), but the stack VM remains the default backend.
- **Backend/platform conclusions must be judged with `benchmark/compare_backends.ps1`, not `cargo test` alone** (the suite does not exercise every shape — e.g. it missed the host-override gap below), and a CLI-side conclusion is only valid after `cargo build`: during the Phase N research a stale binary produced a wrong "register VM reads function locals as `null`" finding.
- `libs/tui` and `libs/gui` ship with real Win32 backends (`--host=tui` / `--host=gui`). Their **simulation** suites (`test_tui.mai` 78/78, `test_gui.mai` 98/98) now pass on **both** backends (CI matrix), but the **host** suites have 1–2 failing checks under `--vm=register`: `gui.draw_rect` / `gui.draw_text` return the stub's `Ok("sim")` instead of the host's `"win32"`, and `tui.write` returns the stub value instead of `"tty"` — the register backend does not take over those host hooks. Pre-existing (reproduced at `a063f14`), tracked with Phase M M8. Other platforms have no bundled backend yet (the host contract is platform-independent; only Windows is implemented).
- `match` now follows the expression value protocol on both backends (Phase N N2): a bound pattern stores into its slot and every arm leaves its value on the operand stack, so `let r = match … { … }` reads the arm's value back. The remaining tail-value gap is a bare **`if`** as a block's last expression (a bare `match` tail is fixed). Root cause and history in `ROADMAP.md` Phase M 13.3 / Phase N. `benchmark/compare_backends.ps1` compares 64 `.mai` programs (66 minus one wall-clock printer and one non-terminating event loop) and **all 64 agree**; `benchmark/known_backend_mismatches.txt` is empty.
- **Heap memory in allocation-heavy loops is bounded** (Phase N N4): the census threshold is now `max(MIN, live * 2)` (was an unbounded `×1.5`) and garbage is released on the next collection, so the `while`-loop probe peaks at **4.9 MB at 2M and 4M iterations** (stack) / **5.1 MB** (register) — it used to be 151/333 MB. Gate: `benchmark/measure_memory.ps1`. `libs/gui/example/interactive.mai` no longer aborts at 128 MiB.
- **Parameter handling still has shape-dependent gaps** (pre-existing, reproduced at `a063f14`, tracked as Phase M M8): on the **stack** VM, `y + h - 1` inside a 5-parameter function raises `Cannot add these types`, forwarding a parameter into a deeper call (`to_rgba(color)`) raises `len expects a string, array, map, or tuple`, and `println(f(x))` where `f` ends in `return g(a)` (a tail call) raises `Internal VM error: Stack underflow in Call`; the **register** VM computes all of those correctly. No shipped program has these shapes, so `compare_backends.ps1` stays green.
- `mailang-macros` is still a passthrough stub.
- VM has no `set_debug` / `memory_stats` APIs.
