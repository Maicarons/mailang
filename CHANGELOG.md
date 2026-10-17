# Changelog

All notable changes to MaìLang will be documented in this file.

Project links: [GitHub](https://github.com/Maicarons/mailang) · [Releases](https://github.com/Maicarons/mailang/releases) · [Tags](https://github.com/Maicarons/mailang/tags)

## [Unreleased]

### Added
- **Bundled Win32 backends for the UI packs.** `mailang run --host=sim|tui|gui` (Windows only, `sim` is the default and installs nothing):
  - `--host=tui` replaces `tui_write` / `tui_term_size` / `tui_is_tty` / `tui_read_key` with real console calls — stdout writes, `GetConsoleScreenBufferInfo` for size, and non-blocking `ReadConsoleInputW` key decoding into the escape sequences the pack expects (`↑`→`ESC [ A`, Enter→`\r`, …).
  - `--host=gui` replaces the `gui_*` hooks with a real window: `CreateWindowExW` (never shown, so it also works on a build agent without a visible desktop), a 32-bit top-down DIB per window, GDI drawing (`FillRect`, `TextOutW`, `MoveToEx`/`LineTo`, `Ellipse`), `BitBlet` + `UpdateWindow` on present, and `PeekMessageW` event polling.
  - Backends live in `crates/mailang-cli/src/hostwin.rs`, gated by `cfg(windows)` plus a target-specific `windows-sys` dependency; other targets are unaffected.
  - Verified on Windows: `libs/tui/test/host_tui.mai` 7/7 and `libs/gui/test/host_gui.mai` 7/7, plus both example programs end-to-end.
- **`libs/gui`** — graphical UI pack (window / canvas / widgets / events). Host-function contract (`gui_create_window`, `gui_draw_rect`, `gui_draw_text`, `gui_draw_line`, `gui_draw_circle`, `gui_poll_event`, `gui_present`, `gui_load_font`) plus a pure-Mai simulation that records draw calls into an inspectable display list and serves events from an injected queue. Ships `example/dashboard.mai` and `test/test_gui.mai` (98 assertions, all passing on the stack VM).
- **`libs/tui`** — terminal UI pack, renamed from the earlier `libs/ui` draft: ANSI sequences, text layout, progress/table/box widgets, terminal size and key input. Host contract `tui_write` / `tui_term_size` / `tui_is_tty` / `tui_read_key` plus simulation; 78 assertions passing on the stack VM.
- CI job `host-packs` (windows-latest) that builds the CLI and runs each pack suite against the backend it asserts — `test_tui.mai` / `test_gui.mai` under `--host=sim` (they assert the simulation), `host_tui.mai` / `host_gui.mai` under `--host=tui` / `--host=gui` (they assert the real backend took over).

### Fixed
- **Host functions can now actually override a pack's stubs.** Both VMs resolve `host_fns` by `FunctionObj.name` at call time (`Call`, `TailCall`, and the register VM's `call_value`). Previously the hooks were installed as globals *before* the module body ran, so a module's own `fn` definitions overwrote them at runtime and the documented "host overrides stub" contract silently did not hold.
- `VmError` now implements `From<String>`, so a host function can report failure with `?`.

### Changed
- A registered host function reports "nothing available" (no key pending, no event, unknown window) as a **MaìLang `Err` value** rather than a host error, matching the packs' stub behaviour — the script can `match` on it instead of the program aborting.

### Fixed
- **Register VM**: `Int`↔`Float` mixed arithmetic and comparison. `bin_apply` / `cmp_apply` only covered `Int`op`Int` and `Float`op`Float`, so `Float * Int`, `Int - Float`, `2.5 / 10`, `7.5 % 2.0`, `t > 20.0` and friends failed with `Type error: Cannot apply binary op` / `Cannot add these types`, while the stack VM handled them. Mixed pairs now promote to `f64` (with the same division-by-zero errors and integer semantics for `Int`op`Int`); string repetition and array concatenation were added to match the stack VM as well. This is what `examples/wasm_demo.mai` and `libs/time/example/basic.mai` were hitting.
- **Register VM**: heap nodes (arrays / maps / instances / builtin results) are now registered with the mark-sweep census. Previously `maybe_auto_collect` was defined but never called, so reference cycles were never collected on this backend.
- **Register VM**: `TailCall` now reuses the current frame instead of pushing a new register window, so a tail-recursive loop runs in constant register footprint instead of growing the register file (and no longer hits the call-depth limit).
- **Register VM**: `?` (`TryQ`) early return releases the callee register window like `Ret` does.

### Changed
- **Register VM performance (ROADMAP M1 met)**: `bench_fib30` went from ~1150 ms to ~150 ms on Windows release — from ~10.4x slower than the stack VM to ~1.2x. Main changes: instruction pointer / chunk / frame base cached in loop locals instead of re-read from the frame stack per instruction; arguments taken over in place as the callee window (no per-call `resize` + argument clone); `RegFrame` shrunk (dead `n_regs` field removed, 72 B -> 40 B); `CallChunk` hot path inlined; GC tracking moved from every register write to heap-allocation points; `Ret`/`TryQ` no longer truncate the register file (bounded by `max_call_depth`), which removes ~19 drop-glue runs per return. Details and per-round measurements in `benchmark/REPORT.md`.
- **CI**: the test job now runs the whole suite on both backends (`MAILANG_VM` matrix: `stack`, `register`), so the dual-VM equivalence claim is machine-checked instead of manual.
- `RegChunk` carries `start_sp` (parameter/`this` slot count) so the VM can tell real parameters from locals for calls that carry no `FunctionObj`.

### Added
- `benchmark/measure_fib.ps1` — measures both backends on `bench_fib30.mai` and prints the ratio (the M1 gate).
- `benchmark/compare_backends.ps1` — runs every `.mai` program under `examples/` and `libs/` on both backends and compares output byte for byte. Programs known to differ are listed in `benchmark/known_backend_mismatches.txt`; a mismatch outside that list fails the run. Of 66 programs it compares 64 (2 cannot be compared: one prints wall-clock time, one is an interactive loop that never terminates), 56 match and 8 differ — 4 `match` compiler defects and 4 UI-pack programs that hit the parameter defects below. Both numbers are higher than the earlier 52/5 because `libs/tui` and `libs/gui` are now included.
- Regression tests for mixed `Int`/`Float` arithmetic and comparison, and a baseline test pinning range-pattern behaviour.
- CI job that times `bench_fib30` on both backends and prints the numbers.

### Known issues (pre-existing, not introduced here)
- **Language / VM defects that shape what a library can do.** Both VMs miscompute expressions whose operands come from function parameters, and the register VM can lose the type of a parameter (so a list literal arrives as something else). Consequences: arithmetic on ≥4 parameters fails, so `libs/gui` has no `draw_rect_outline` / `progress_bar` helper (callers emit the rects themselves), and `libs/gui` only accepts a colour *name* or an *[r, g, b] array* because the language has no `type_of` and every type probe raises instead of returning false. Both libraries pass fully on the stack VM but hit these defects under `--vm=register`.
- `match` mis-compiles in two ways, both reproduced by `examples/match_demo.mai`, `examples/phase_b_check.mai` and `examples/iot_sensor.mai`:
  1. `compile_bindings_from_stack`'s `Identifier` arm declares the slot but emits no `StoreLocal`, relying on "the bound value already sits at its slot index". The value actually sits one slot lower (on top of the operand stack), so any local declared before the `match` makes the arm read an uninitialised slot.
  2. An arm's value is parked in a local by `end_scope_keeping_result` and never pushed back, so the enclosing `let m = match {...}` picks up leftover stack content.
  Fixing both requires changing the compiler *and* `mailang-bytecode/src/register.rs`'s `compute_heights` dataflow contract together — patching the compiler alone regresses the register VM's array-pattern length check. See ROADMAP Phase M 13.3 for the bytecode-level evidence.
- A function or block whose last statement is a bare `match` / `if` expression yields `null` instead of that expression's value.
- **Heap memory is not reclaimed in allocation-heavy loops** (both backends): allocating one array per `while` iteration retains ~80 bytes per iteration with no plateau — 159 MB peak at 2M iterations and 336 MB at 4M, on the stack VM and the register VM alike, while a loop with no heap allocation stays at ~5 MB over 20M iterations. The mark-sweep census never frees them; the exact line is not identified yet. `libs/gui/example/interactive.mai` aborts at 128 MiB because of it, which is why that program is excluded from the backend comparison. Evidence in ROADMAP Phase M 13.6.

### Removed
- `tests/basic.rs`: an orphan test file at the workspace root, which has no `[package]` and was therefore never compiled by `cargo test`. Its five cases were already covered by `crates/mailang-core/tests/integration.rs`.

## [0.3.1] - Docs site, CI green, register VM perf

### Changed
- **Register VM** faster on call-heavy integer code: truncate callee registers on Ret (~2x), Int fast paths for Bin/Cmp, skip `burn_fuel` when unset. fib still ~10x stack (target 2x) — stack remains default.
- **Docs site is versioned** under docs/v0.3/ with a nav version switcher (`versions.json`); language-version prose removed from doc bodies.
- **i18n**: Chinese and English guide+reference pages aligned (20 each), including registry, footprint, types, operators, builtins, errors.

### Fixed
- CI: clippy -D warnings and rustfmt; mailang-bytecode no_std alloc imports and jump backfill (thumbv7em).
- VitePress: escaped CJK labels in config, srcExclude for internal specs, playground WASM loads from public base URL, root playground redirects to guide.

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
