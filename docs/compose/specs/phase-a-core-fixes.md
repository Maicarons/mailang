---
feature: phase-a-core-fixes
status: delivered
updated: 2026-06-17
branch: master
commits: 8fe3e55..HEAD
---

# Phase A: Core Fixes — Make the Language Actually Work

## Report

**What was built** — Fixed the MaìLang interpreter so that OOP (classes, constructors, method calls, inheritance with `super()`), closures (upvalue capture), and pattern matching (literal, wildcard, identifier binding, or-patterns, guards) actually work end-to-end. Fixed 40 integration tests that now pass. Fixed multiple P0 bugs: `GetProperty`/`IndexGet` `continue`→`break`, UTF-8 `.len` using `chars().count()`, `CreateClass` reading from constants instead of stack, parser emitting `MethodCall` instead of `Call(PropertyAccess)`, and property assignment compile order.

**Verification** — `cargo test -p mailang-core` → 40/40 pass. `examples/oop_demo.mai` → "小黑 汪汪叫！". `examples/fibonacci.mai` → correct output. `examples/hello.mai` → correct output.

**Journey log**:
1. Class system subagent failed (unknown error); implemented manually instead.
2. Closures/match subagent succeeded in one shot with 40/40 tests.
3. `CreateClass` originally popped from stack but compiler emitted constant index — fixed to read from constants.
4. Parser originally emitted `Call(PropertyAccess)` for `obj.method()`; changed to `MethodCall` for proper `Invoke` dispatch.
5. Property assignment stack order was wrong (value before object); fixed compiler to push object first.

## [S1] Problem

MaìLang has a working lexer→parser→compiler→VM pipeline for basic procedural code, but all advanced language features (OOP, closures, pattern matching) are broken stubs. Tests don't run. The analyzer is never invoked. Documentation overstates maturity.

## [S2] Design

### S2.1 Test Infrastructure
- Move `tests/basic.rs` into `crates/mailang-core/tests/`
- Add integration tests covering all working language features
- `cargo test -p mailang-core` must actually run 40+ tests

### S2.2 Class System
- `compile_class` compiles method bodies into separate chunks
- VM `Call` on `Value::Class` creates instance and calls `init`
- VM `Invoke` dispatches method calls with `this` binding
- Constructor (`init`) support with `super()` parent constructor call
- `this.x = value` property assignment works in methods

### S2.3 Closures
- VM `LoadUpvalue`/`StoreUpvalue` read/write captured variables via `upvalue_store`
- Compiler emits `MakeClosure` with upvalue captures
- Closures capture by reference (shared mutability)

### S2.4 Pattern Matching
- Compiler emits pattern test bytecode (no VM `MatchPattern` dependency)
- Wildcard `_` always matches
- Literal patterns compare correctly
- Or-patterns `1 | 2 | 3` with short-circuit
- Guard patterns `x if x > 10`

### S2.5 Analyzer Integration
- Not completed in this phase (deferred)

### S2.6 Bug Fixes
- `GetProperty` on Instance: fixed `continue`→`break`
- `IndexGet` on Map: fixed `continue`→`break`
- UTF-8 `.len` uses `chars().count()` not `s.len()`
- `CreateClass` reads from constants not stack
- Parser emits `MethodCall` for `obj.method()`
- Property assignment compile order fixed

## [S3] Out of Scope
- Value→Rc performance optimization (Phase B)
- no_std / IoT targets (Phase C)
- LSP / formatter (Phase E)
- GC implementation (Phase B)
- Analyzer pipeline integration (deferred)
- Hex/octal/binary literals (deferred)
- Block comment `*/` terminator (deferred)

## Tasks
- [x] T1: Fix test infrastructure — acceptance: `cargo test -p mailang-core` runs 40 tests, all pass (covers: S2.1)
- [x] T2: Implement class system — acceptance: `examples/oop_demo.mai` prints "小黑 汪汪叫！" (covers: S2.2; depends: T1)
- [x] T3: Implement closures — acceptance: closure capturing outer variable returns correct value (covers: S2.3; depends: T1)
- [x] T4: Implement pattern matching — acceptance: wildcard, literal, or-patterns work in VM (covers: S2.4; depends: T1)
- [ ] T5: Wire analyzer into pipeline — acceptance: undefined variable produces error message (covers: S2.5; depends: T1)
- [x] T6: Fix P0 bugs — acceptance: all listed bugs verified fixed with tests (covers: S2.6; depends: T1)
