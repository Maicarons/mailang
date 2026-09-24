# Syntax Guide

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [v0.1.0-parser](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-parser) · [v0.1.0-lexer](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-lexer)

## Overview

MaìLang uses a hybrid syntax style combining the best features of Rust, Python, and JavaScript.

## Comments

```
// Single-line comment

/* 
   Multi-line comment
   Can span multiple lines
*/

/* /* Supports nesting */ */
```

## Variables

### Immutable `let`

```
let x = 42
let name: str = "MaìLang"
```

### Mutable `var`

```
var count = 0
count = count + 1  // OK
```

### Constants `const`

```
const PI = 3.14159
```

## Basic Types

| Type | Description | Example |
|------|-------------|---------|
| `int` | 64-bit signed integer | `42` |
| `float` | 64-bit float | `3.14` |
| `bool` | Boolean | `true`, `false` |
| `str` | UTF-8 string | `"hello"` |
| `char` | Unicode character | `'A'` |
| `null` | Null value | `null` |

## String Interpolation

```
let name = "MaìLang"
let greeting = "Hello, {name}!"
let expr = "Result: {1 + 2}"
```

## Operators

### Arithmetic

| Operator | Description | Example |
|----------|-------------|---------|
| `+` | Addition | `1 + 2 = 3` |
| `-` | Subtraction | `5 - 3 = 2` |
| `*` | Multiplication | `4 * 3 = 12` |
| `/` | Division | `10 / 3 = 3` |
| `%` | Modulo | `10 % 3 = 1` |
| `**` | Power | `2 ** 3 = 8` |

### Comparison

| Operator | Description | Example |
|----------|-------------|---------|
| `==` | Equal | `1 == 1` → `true` |
| `!=` | Not equal | `1 != 2` → `true` |
| `<` | Less than | `1 < 2` → `true` |
| `>` | Greater than | `2 > 1` → `true` |

### Logical

| Operator | Description | Example |
|----------|-------------|---------|
| `&&` | And | `true && false` → `false` |
| `\|\|` | Or | `true \|\| false` → `true` |
| `!` | Not | `!true` → `false` |

## Control Flow

### if-elif-else

```
if condition {
    // ...
} elif other {
    // ...
} else {
    // ...
}
```

### for loop

```
for item in [1, 2, 3] {
    println(item)
}

for i in 0..10 {
    println(i)
}
```

### while loop

```
var count = 0
while count < 10 {
    println(count)
    count += 1
}
```

### match expression

```
let result = match x {
    0 => "zero",
    1..10 => "one to nine",
    _ => "other"
}
```

## Functions

```
fn add(a: int, b: int) -> int {
    return a + b
}

// Default parameters are not implemented yet (in progress)
fn greet(name: str) -> str {
    return "Hello, {name}!"
}

// Lambda
let square = fn(x) -> x * x
```

## OOP

```
class Animal {
    let name: str
    
    fn init(name: str) {
        this.name = name
    }
    
    fn speak() -> str {
        return "{this.name} speaks"
    }
}

class Dog extends Animal {
    override fn speak() -> str {
        return "{this.name} barks!"
    }
}

trait Printable {
    fn to_string() -> str
    fn print() {
        println(this.to_string())
    }
}
```

## Error Handling

```
fn divide(a: float, b: float) -> Result<float, str> {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}

// Note: postfix `expr match { ... }` is not implemented yet (in progress)
let value = match divide(10.0, 3.0) {
    Ok(v) => v,
    Err(e) => {
        println("Error: {e}")
        0.0
    }
}
```

## Next Steps

- [OOP](/en/guide/oop) - Classes, inheritance, traits
- [Standard Library](/en/guide/stdlib) - Built-in modules
- [FFI](/en/guide/ffi) - Language integration
