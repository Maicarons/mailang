# Changelog

All notable changes to MaìLang will be documented in this file.

Project links: [GitHub](https://github.com/Maicarons/mailang) · [Releases](https://github.com/Maicarons/mailang/releases) · [Tags](https://github.com/Maicarons/mailang/tags)

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
