# Standard Library

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [stdlib Source](https://github.com/Maicarons/mailang/tree/master/crates/mailang-stdlib) · [v0.1.0-stdlib](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-stdlib)

## Overview

MaìLang's standard library provides commonly used modules out of the box.

## io Module

```
println("Hello!")
print("Enter: ")
let name = input("Name: ")
```

## math Module

```
math.sqrt(16.0)    // 4.0
math.abs(-5)       // 5
math.sin(0.0)      // 0.0
math.floor(3.7)    // 3
math.ceil(3.2)     // 4
math.round(3.5)    // 4
math.random()      // 0.0 ~ 1.0
math.PI            // 3.14159...
```

## String Functions

```
"hello".len()          // 5
"hello".upper()        // "HELLO"
"hello".contains("ll") // true
"a,b,c".split(",")     // ["a", "b", "c"]
```

## Array Functions

```
var arr = [3, 1, 2]
arr.sort()             // [1, 2, 3]
arr.push(4)            // [1, 2, 3, 4]
arr.map(fn(x) -> x * 2) // [2, 4, 6, 8]
arr.filter(fn(x) -> x > 2) // [3, 4]
```

## Next Steps

- [FFI](/en/guide/ffi) - Language integration
- [IoT](/en/guide/iot) - Embedded deployment
