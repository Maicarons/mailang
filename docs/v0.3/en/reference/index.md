# API Reference

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [Source](https://github.com/Maicarons/mailang/tree/master/crates)

## Overview

This section provides the complete MaìLang API reference, including the type system, operators, built-in functions, and error codes.

## Contents

- [Type System](/v0.3/en/reference/types) — detailed description of every type
- [Operators](/v0.3/en/reference/operators) — operator precedence and behavior
- [Built-in Functions](/v0.3/en/reference/builtins) — standard library function reference
- [Error Codes](/v0.3/en/reference/errors) — error codes and messages

## Quick Reference

### Primitive Types

| Type | Size | Range | Example |
|------|------|-------|---------|
| `int` | 8 bytes | -2^63 ~ 2^63-1 | `42` |
| `float` | 8 bytes | IEEE 754 | `3.14` |
| `bool` | 1 byte | true/false | `true` |
| `str` | variable | UTF-8 | `"hello"` |
| `char` | 4 bytes | Unicode | `'A'` |
| `null` | 0 bytes | — | `null` |

### Composite Types

| Type | Syntax | Example |
|------|--------|---------|
| Array | `[T]` | `[1, 2, 3]` |
| Map | `{K: V}` | `{"a": 1}` |
| Tuple | `(T1, T2)` | `(1, "a")` |
| Result | `Result<T, E>` | `Ok(42)` / `Err("msg")` |
| Option | `Option<T>` | `Some(42)` / `None` |

### Operator Precedence

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 14 | `()` | — |
| 13 | `**` | right |
| 12 | `-` `!` `~` | right |
| 11 | `*` `/` `%` | left |
| 10 | `+` `-` | left |
| 9 | `<<` `>>` | left |
| 8 | `&` | left |
| 7 | `^` | left |
| 6 | `\|` | left |
| 5 | `==` `!=` `<` `<=` `>` `>=` | left |
| 4 | `&&` | left |
| 3 | `\|\|` | left |
| 2 | assignment | right |
| 1 | `,` | left |

### Control Flow

```
if condition { ... } elif condition { ... } else { ... }
for item in iterable { ... }
while condition { ... }
match value { pattern => expr, ... }
return value
break
continue
```

### Functions

```
fn name(param: Type = default) -> ReturnType {
    // body
}
```

### Classes

```
class Name extends Parent implements Trait {
    let property: Type
    fn init(param: Type) { ... }
    fn method() -> Type { ... }
}
```

### Traits

```
trait Name {
    fn required() -> Type
    fn default() -> Type { ... }
}
```
