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
- **mailang-vm**: Two bytecode virtual machines: the default register (three-address) `RegisterVm` and the opt-in stack `Vm` with slot locals. Both execute the same `Bytecode` and return `Value`.
- **mailang-stdlib**: Built-in functions (`println`, `sqrt`, `len`, `parse_int`, etc.) operating on `mailang_bytecode::Value`.
- **mailang-core**: Glue crate. Re-exports all above. Contains `MailangInterpreter` which orchestrates Parser 鈫?Compiler 鈫?VM pipeline. This is the main integration point.
- **mailang-cli**: CLI binary. Uses `clap` for subcommands (`run`, `eval`) and provides an interactive REPL.
- **mailang-ffi**: C ABI layer (`extern "C"` functions). Uses `cbindgen` to generate `mailang.h`. Wraps `mailang-core`.
- **mailang-wasm**: WebAssembly bindings via `wasm-bindgen`. Wraps `mailang-core`.
- **mailang-lsp**: Language Server Protocol server (diagnostics/completions; multi-error syntax diagnostics via parser recovery). Uses `tower-lsp`.
- **mailang-gc**: Garbage collector (`collect_cycles()` edge-cut + `MarkSweepHeap` wired into both the stack VM and the register VM).
- **mailang-macros**: Procedural macros (passthrough stub).

### Key Design Decisions

- **Two interchangeable bytecode VMs over the same `Bytecode`** for predictable performance on IoT devices: the **register (three-address) VM is the default** (M5) and lowers the stack bytecode once per run via `StackToRegister`; the **stack VM + slot locals is opt-in** (`--vm=stack` / `MAILANG_VM=stack`). The compiler still emits the stack form — the register IR is a lowering, so `.mailangbc` remains backend-independent.
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

**Phase N + M5 (both VM backends agree on every shipped program; the register VM is the default)**. Runtime supports OOP (inherited constructors, `super()` ctor, `super.method()`, inherited methods), traits (`implements`, `trait extends`), pattern matching (literals/ranges/`Ok`/`Err`/`Some`/or/guards + array/tuple match destructure), `let`/`var` destructuring, default parameter values, postfix `expr match { }`, mutability enforcement (`let`/`const` reject assignment), `?`, TCO, collections/str methods, global `read_file`/`write_file`, `.mailangbc`, simulated HAL, C FFI, analyzer/LSP/fmt, module v2 with `mailang.toml` path deps (`mailang deps`), **filesystem/HTTP-static package registry** (`publish`/`install`/`search`/`yank`), **generic function monomorphization** (turbofish `id::<int>` → `id$int`; generic classes erase fields), **GC = Rc + `collect_cycles()` + `MarkSweepHeap`**, **parser error recovery** (`recover_from_error` / `parse_program_recovering`; LSP reports multiple syntax errors), **bundled Win32 host backends for the `tui`/`gui` packs** (`mailang run --host=sim|tui|gui`, Windows only), and a **stack VM whose `CallDirect` tail calls no longer overwrite the caller's operand slot** (M8b, Phase N onwards). Fib(30) on Windows release runs at roughly the same speed on both backends — **~1.0x**, the two backends always landing within a few percent of each other (two runs of `measure_fib.ps1` on the machine that measured M5: stack 119 / register 116 / default 116 ms, and under heavier load stack 188 / register 176 / default 174 ms, so the ratio ranged 0.94–0.97x; the register VM was ~1150 ms / ~10x slower before the Phase M optimisation). Gate: `benchmark/measure_fib.ps1` (≤2.0x). **137 integration tests (261 workspace tests), green both on the default backend — the register VM since M5 — and on `MAILANG_VM=stack`**, both enforced by the CI test matrix.

### Remaining gaps (honest)

- No public hosted package registry (local/`http(s)` static mirror only).
- Generic classes are field-erased (no per-type layout monomorphization).
- `async` not supported.
- IoT HAL is simulated by default. A real HAL can be registered through FFI/host functions, but no real-hardware validation is claimed for it.
- **The register VM is the default since M5** (`--vm=register` is now a no-op alias of the default; `--vm=stack` / `MAILANG_VM=stack` opt out). Its prerequisites were met before the switch: it matches the stack VM on the whole test suite (137/137, CI matrix) and on `benchmark/compare_backends.ps1` (64/64 identical), it is on par for `bench_fib30` (~1.0x, well within the ≤2.0x gate), and it no longer loses the `host_*` hooks (M8a). Costs of the switch: the stack→register lowering runs once per `run`/`eval` (it did before too, on every `--vm=register` run), and `.mailangbc` is untouched because both backends read the same stack bytecode. Binary size is essentially unchanged (+4 bytes on the `wasm32` release artifact) since `RegisterVm` was always compiled in.
- **`MAILANG_VM` only existed in the test harness until M5.** `CHANGELOG.md` has claimed `MAILANG_VM=register` works for the CLI since v0.3.0, but the CLI never read it — the variable only switched the integration-test factory. The CLI now resolves the backend as **`--vm` flag > `MAILANG_VM` env > default (register)**, treating empty/unknown values as "unset" so a stray variable cannot break a run.
- **Backend/platform conclusions must be judged with `benchmark/compare_backends.ps1`, not `cargo test` alone**, and a CLI-side conclusion is only valid after `cargo build`: during the Phase N research a stale binary produced a wrong "register VM reads function locals as `null`" finding. A host-backed feature additionally needs its `host_*` pack suite — that is what caught the register VM's tail-call path bypassing host functions (fixed after Phase N).
- `libs/tui` and `libs/gui` ship with real Win32 backends (`--host=tui` / `--host=gui`). Their **simulation** suites (`test_tui.mai` 78/78, `test_gui.mai` 98/98) and their **host** suites (`host_tui.mai` / `host_gui.mai`, 7/7 each) all pass on **both** backends (CI matrix). Other platforms have no bundled backend yet (the host contract is platform-independent; only Windows is implemented).
- `match` now follows the expression value protocol on both backends (Phase N N2): a bound pattern stores into its slot and every arm leaves its value on the operand stack, so `let r = match … { … }` reads the arm's value back. A bare **`if`/`elif`/`else`** as a body's last statement now also yields the selected branch's value, and an `elif` branch without a `return` no longer jumps to instruction 0 (fixed after N7). Root cause and history in `ROADMAP.md` Phase M 13.3 / 13.7. `benchmark/compare_backends.ps1` compares 64 `.mai` programs (66 minus one wall-clock printer and one non-terminating event loop) and **all 64 agree**; `benchmark/known_backend_mismatches.txt` is empty.
- **Heap memory in allocation-heavy loops is bounded** (Phase N N4): the census threshold is now `max(MIN, live * 2)` (was an unbounded `×1.5`) and garbage is released on the next collection, so the `while`-loop probe peaks at **4.9 MB at 2M and 4M iterations** (stack) / **5.1 MB** (register) — it used to be 151/333 MB. Gate: `benchmark/measure_memory.ps1`. `libs/gui/example/interactive.mai` no longer aborts at 128 MiB.
- **The stack VM's `CallDirect` tail calls no longer corrupt the caller** (M8b, fixed after Phase N). The frame-reuse shuffle in `Opcode::TailCall` — and the builtin tail-call unwind — assumed every frame owns a callee slot one below its first argument, so a tail call made from a `CallDirect` frame (which has none) moved/truncated `stack_base - 1`, i.e. a live slot of the **caller's** operand stack. On the stack VM that surfaced as `Cannot add these types` on `y + h - 1` inside a 5-parameter function, `len expects a string, array, map, or tuple` after a forwarded parameter (`to_rgba(color)`), `Internal VM error: Stack underflow in Call` on `println(f(x))` where `f` ends in a tail call, and two different values for one shape across the two backends (all reproduced at `a063f14`; the register VM was always correct). `frame_result_base` + `Vm::shuffle_tail_args` now keep a direct frame's shape, with 5 regression tests in `integration.rs`; root cause in `ROADMAP.md` 13.9.
- `mailang-macros` is still a passthrough stub.
- VM has no `set_debug` / `memory_stats` APIs.
