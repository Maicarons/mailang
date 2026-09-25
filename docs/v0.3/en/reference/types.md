# Type System

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [AST Source](https://github.com/Maicarons/mailang/blob/master/crates/mailang-ast/src/lib.rs)

## Overview

MaìLang uses a hybrid type system with type inference and explicit type annotations. Inference is the default; you can also annotate types explicitly.

## Primitive Types

### int (integer)

64-bit signed integer.

```
let a = 42
let b = -10
let c = 0
let d: int = 100
```

**Range**: -9,223,372,036,854,775,808 ~ 9,223,372,036,854,775,807

**Literal formats**:
- Decimal: `42`, `1_000_000`
- Hexadecimal: `0xFF`, `0x00`
- Octal: `0o77`, `0o0`
- Binary: `0b1010`, `0b0`

### float (floating-point)

64-bit IEEE 754 double-precision floating-point number.

```
let a = 3.14
let b = -0.5
let c = 1.0
let d: float = 2.5
```

**Range**: ±2.2250738585072014 × 10^-308 ~ ±1.7976931348623157 × 10^308

### bool (boolean)

Boolean value, either `true` or `false`.

```
let a = true
let b = false
let c: bool = true
```

### str (string)

UTF-8 encoded string.

```
let a = "hello"
let b = "你好"
let c = "🔥"
let d: str = "MaìLang"
```

**Escape sequences**:
- `\n` — newline
- `\t` — tab
- `\\` — backslash
- `\"` — double quote
- `\0` — null character
- `\u&lbrace;XXXX&rbrace;` — Unicode escape

**String interpolation**:
```
let name = "MaìLang"
let greeting = "Hello, {name}!"
let expr = "Result: {1 + 2}"
```

### char (character)

A single Unicode character.

```
let a = 'A'
let b = '中'
let c = '🔥'
let d: char = '\n'
```

### null (null value)

Represents the absence of a value.

```
let a = null
let b: null = null
```

## Composite Types

### Array `[T]`

An ordered collection of same-type elements.

```
let nums = [1, 2, 3, 4, 5]
let strs = ["a", "b", "c"]
let empty: [int] = []
let mixed: [any] = [1, "two", 3.0]
```

**Operations**:
```
arr[0]          // access
arr.len         // length
arr.push(x)     // append
arr.pop()       // remove last
arr.insert(i, x)// insert
arr.contains(x) // contains?
arr.join(sep)   // join to string
arr.reverse()   // reverse
arr.clear()     // clear
```

### Map `&lbrace;K: V&rbrace;`

A key-value collection.

```
let map = {
    "name": "MaìLang",
    "version": "0.1.0"
}
let empty: {str: int} = {}
```

**Operations**:
```
map["key"]      // access
map["key"] = v  // set
map.has(k)      // has key?
map.remove(k)   // remove
map.keys()      // all keys
map.values()    // all values
map.clear()     // clear
```

### Tuple `(T1, T2, ...)`

An ordered collection of mixed-type elements.

```
let point = (3.0, 4.0)
let person = ("张三", 25, true)
let nested = (1, (2, 3))
```

**Access**:
```
let (x, y) = point  // destructure
let first = point[0] // index
```

### Result&lt;T, E&gt;

A type for error handling. Generic parameters participate in monomorphization (e.g. `id::<int>`); generic class fields use an erased layout and are not fully generically checked.

```
fn divide(a: float, b: float) -> Result<float, str> {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}

// Postfix `expr match { ... }` is supported
let result = match divide(10.0, 3.0) {
    Ok(v) => v,
    Err(e) => 0.0
}
```

### Option&lt;T&gt;

A type representing an optional value. Generic parameters participate in monomorphization; generic class fields use an erased layout.

```
fn find(arr: [int], target: int) -> Option<int> {
    for item in arr {
        if item == target {
            return Some(item)
        }
    }
    return None
}

let value = match find([1, 2, 3], 2) {
    Some(v) => v,
    None => 0
}
```

## Type Inference

MaìLang supports automatic type inference:

```
let a = 42          // inferred as int
let b = 3.14        // inferred as float
let c = true        // inferred as bool
let d = "hello"     // inferred as str
let e = [1, 2, 3]   // inferred as [int]
let f = {"a": 1}    // inferred as {str: int}
```

## Type Conversion

```
// Explicit conversion
let i = 42
let f = i as float   // 42.0
let s = i as str     // "42"

// Implicit conversion (in some contexts)
let a: float = 42    // int automatically converted to float
```

## The any Type

The `any` type can hold a value of any type:

```
let value: any = 42
value = "hello"
value = [1, 2, 3]
```

## Type Checking

There are currently no type-test builtins such as `is_int` / `is_float` / `is_str`. What you can do:

```
// Result / Option pattern matching
match value {
    Ok(v) => ...,
    Err(e) => ...
}
```

## Next Steps

- [Operators](/v0.3/en/reference/operators) — detailed operator reference
- [Built-in Functions](/v0.3/en/reference/builtins) — standard library functions
