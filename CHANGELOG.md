# Changelog

All notable changes to MaìLang will be documented in this file.

Project links: [GitHub](https://github.com/Maicarons/mailang) · [Releases](https://github.com/Maicarons/mailang/releases) · [Tags](https://github.com/Maicarons/mailang/tags)

## [Unreleased]

## [0.3.0] - Phase H–L

Phases H–L: language completeness, IoT surface, dual-VM work, parser recovery, and docs/release honesty.

### Added — Language correctness
- **Array/tuple match destructure** — `[a, b]` / `(a, b)` patterns check length and bind elements
- **Default parameter values** — `fn f(a, b = 10)`; defaults may reference earlier params; analyzer allows `min..=max` arity
- **Match bindings are arm-local** — no longer leak to globals
- **`let` / `var` destructuring** — `let (a, b) = …`, `let [x, y] = …`
- **Postfix match** — `expr match { … }`
- **Trait extends** — `trait B extends A { … }` (inherits required methods and defaults)
- **`super.method(...)`** — call the superclass implementation
- **Inherited methods** — subclass flattens parent methods (not only `init`)
- **`self` as first parameter** — alias for the implicit receiver (not an extra argument)
- **Non-literal class property defaults** — `let val = f()` applied in `init`
- **Mutability enforcement** — `let`/`const` reject assignment; `var` / `let var` stay mutable

### Fixed
- Default-parameter prologue stack balance (provided vs missing paths)
- Analyzer arity checks now respect default parameters

### Added — Parser recovery (Phase L)
- **`Parser::recover_from_error`** — token-sync skip to statement boundary
- **`Parser::parse_program_recovering`** — multi-error parse returning partial AST + `Vec<ParseError>`
- LSP reports **multiple syntax errors** per change (`eval`/`run` stay fail-fast)

### Added — GC
- **Mark-sweep heap** (`mailang-gc::MarkSweepHeap`) wired into the stack VM — tracked node census, periodic/auto collect, `gc_stats()` = `(tracked, collections, freed)`
- `collect_cycles()` now runs classic edge-cut **and** a real mark-sweep pass over stack + globals + upvalues
- Integration: `test_mark_sweep_frees_cycles`

### Added — Register VM
- **Three-address register IR** (`mailang-bytecode::register`: `RegOp` / `RegChunk`) + stack→register lowering (`StackToRegister`)
- **`RegisterVm`** — flat per-frame register file, shared builtins / host-fns / globals with the stack VM
- Fixed **`Ret` / `TryQ` `ret_dst` backfill**, **TailCall arg base**, and **expression-stack vs local overlap**
- Register builtins mirrored with the stack VM (io/math/string/collections/json/time/HAL/sys)
- `MailangInterpreter::set_vm_backend(VmBackend::{Stack,Register})` — `eval` / `run_bytecode` honour the choice; default remains **stack**
- CLI: `mailang run --vm=register` / `mailang eval --vm=register` (also `MAILANG_VM=register`)

### Added — Packages
- **`mailang.toml`** project dependencies + `mailang deps`
- **Filesystem / static-HTTP package registry**: `mailang publish` / `install` / `search` / `yank` / `registry`

### Added — IoT / docs
- ESP32 end-to-end: `examples/iot_blink.mai`, `docs/guide/esp32-blink.md` + en (copy-paste IDF-style C host)
- Footprint dashboard (`benchmark/measure_size.py`, `docs/guide/footprint.md`)
- Driver pack stubs with host-fn contracts + sim: `libs/bme280/`, `libs/mqtt/`
- Real-hardware embedding guide (`docs/guide/hardware.md`); playground examples; WASM build script
- CI: FFI smoke tests + `cargo publish --dry-run`; multi-crate `release.sh`

### Testing
- 102 integration tests (stack VM: **102/102**)
- `MAILANG_VM=register` re-run of the same suite: **102/102** — register VM reached feature parity with the stack VM. Stack VM stays the default.

### Changed
- Docs honesty pass: stack-VM wording everywhere, registry / generics / GC / parser recovery reflected, version **0.3.0**
- Removed undocumented `vm.set_debug` / `memory_stats` from the IoT guide
- Workspace crates versioned **0.3.0**

## [0.2.6] - 2026-09-12

### Fixed
- Publish script: include mailang-module, skip already-published, retry on 429 rate limit

## [0.2.5] - 2026-09-12

### Fixed
- Path dependencies now carry `version` so `cargo publish` can verify manifests
- CI publishes `mailang-lsp` (required by `mailang-cli`)

## [0.2.4] - 2026-09-12

Phase G: language completeness and DX.

### Added
- **`?` operator** — `Ok`/`Some` unwrap, `Err`/`None` early-return from the enclosing function
- **Array methods**: `push` `pop` `insert` `contains` `join` `reverse` `clear`
- **Map methods**: `keys` `values` `has` `remove` `clear`
- **String methods**: `trim` `split` `replace` `starts_with` `ends_with` `contains` `to_upper` `to_lower` `repeat`
- **File builtins**: `read_file(path)` `write_file(path, contents)` (host FS)
- Analyzer: arity mismatch and simple type mismatch for declared functions with annotations

### Fixed
- Subclasses without `init` inherit the parent constructor (`Dog("Rex")` → `Rex barks!`)

## [0.2.2] - 2026-09-12

### Fixed
- Subclasses without their own `init` now inherit the parent constructor (`Dog("Rex")` sets `name`)
- CI triggers on `master`; workspace `--exclude` requires `--workspace`
- Release packaging uses `--target` so archives include the `mailang` binary
- docs/playground npm security overrides (vite/esbuild/nanoid/postcss)

### Changed
- README drops the Project Tags table; docs present **v0.2.2** as the current release

## [0.2.0] - 2026-09-12

Phases B–F: language completeness, IoT surface, embeddability, tooling, and performance.

### Added — Language
- Trait system with `implements` and default method injection
- Generic type annotations (`Result<T, E>`, `Option<T>`)
- Match: `Ok`/`Err`/`Some`/`None` patterns, ranges (`1..10`, `1..=10`), comma-separated arms
- Hex / octal / binary integer literals (`0x10`, `0o17`, `0b1010`)
- Block comments `/* */`
- Self tail-call optimization (deep recursion without stack overflow)

### Added — IoT & Embedded
- `mailang-bytecode` is `no_std` + `alloc` (serde optional; host only)
- Versioned `.mailangbc` binary format (magic `MAILBC01`, little-endian)
- CLI: `mailang build` / `mailang run file.mailangbc`
- Simulated HAL: `gpio_write`, `gpio_read`, `delay_ms`, `adc_read`
- Embedded size reporting (`benchmark/measure_size.py`, CI no_std jobs)

### Added — FFI & WASM
- C API: `mailang_register_host_fn`, global get/set (int/str), structured error codes
- `catch_unwind` on all `extern "C"` entry points
- Header: `crates/mailang-ffi/include/mailang.h`
- Verified bindings: C (MinGW), Python (ctypes), Go (cgo), Node.js (WASM)
- Host callbacks/globals survive `eval` VM rebuilds

### Added — Tooling
- Semantic analyzer wired into `eval`/`compile` (undefined variables fail early)
- Analyzer diagnostics with source line/col (`mailang_analyzer::diagnose`)
- LSP: diagnostics, completions, go-to-definition, hover (`mailang lsp`)
- Formatter: `mailang fmt` / `mailang fmt --check`
- Module system v2: independent parse/analyze, export tables, bytecode linking (no AST injection)

### Added — Performance & GC
- Value heap payloads wrapped in `Rc` (Function/Closure/Class)
- Hot-path int/compare/call specialization; fused `*Imm` opcodes
- `CallDirect` for known top-level functions
- Release profile: LTO + `panic = "abort"`
- **Fibonacci(30) ~137 ms (~4.25× Phase A baseline)**
- Rc cycle collection (`Vm::collect_cycles`)

### Testing
- 72+ integration tests covering Phase B–F features

### Security
- Docs/playground npm overrides: vite ≥ 6.4.3, esbuild ≥ 0.25, nanoid ≥ 3.3.18, postcss ≥ 8.5.23, brace-expansion ≥ 2.1.4

## [0.1.0] - 2026-06-17

### Added — Core Language
- Lexer with full Unicode support (unicode-xid)
- Parser with recursive descent + Pratt parsing
- AST definitions for all language constructs with serde support
- Bytecode compiler with 50+ opcodes
- Stack-based bytecode VM with slot-based locals
- Standard library (io, math, string, collections)
- CLI with REPL and file execution

### Added — OOP
- Class definitions with properties and methods
- Constructor (`init`) support
- Inheritance with `extends` and `super()` calls
- Method dispatch via `Invoke` opcode
- `this` binding in methods

### Added — Closures
- Upvalue capture for closures
- `MakeClosure` opcode
- Shared mutability across closure calls
- Block-bodied anonymous functions

### Added — Pattern Matching
- Literal pattern matching
- Wildcard `_` pattern
- Identifier binding patterns
- Or-patterns `1 | 2 | 3`
- Guard expressions `x if x > 10`

### Added — Infrastructure
- 40 integration tests (all passing)
- C FFI layer with 12-language binding support
- WebAssembly bindings
- VitePress documentation (Chinese + English)
- GitHub Actions CI/CD
- 12 project tags for component tracking

### Fixed
- `GetProperty` on Instance: `continue` → `break`
- `IndexGet` on Map: `continue` → `break`
- UTF-8 `.len` now uses `chars().count()`
- `CreateClass` reads from constants not stack
- Parser emits `MethodCall` for `obj.method()`
- Property assignment compile order

### Known Limitations
- `let`/`var` mutability not enforced at compile time
- Default parameter values not compiled
- Hex/octal/binary literals not implemented
- Block comment uses `**` terminator (not `*/`)
- Analyzer not wired into execution pipeline
- GC is a stub (not integrated with VM)
- LSP is a skeleton (initialize only)
