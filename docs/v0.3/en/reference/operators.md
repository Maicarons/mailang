# Operators

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [Compiler Source](https://github.com/Maicarons/mailang/blob/master/crates/mailang-compiler/src/compiler.rs)

## Overview

MaìLang provides a rich set of operators, including arithmetic, comparison, logical, bitwise, and assignment operators.

## Arithmetic Operators

| Operator | Name | Description | Example |
|----------|------|-------------|---------|
| `+` | Addition | Add two numbers or concatenate strings | `1 + 2 = 3`, `"a" + "b" = "ab"` |
| `-` | Subtraction | Subtract two numbers | `5 - 3 = 2` |
| `*` | Multiplication | Multiply two numbers or repeat a string | `4 * 3 = 12`, `"ha" * 3 = "hahaha"` |
| `/` | Division | Divide two numbers | `10 / 3 = 3` (integers), `10.0 / 3.0 = 3.333...` |
| `%` | Modulo | Remainder | `10 % 3 = 1` |
| `**` | Exponentiation | Power | `2 ** 3 = 8` |
| `-` | Negation | Unary operator | `-5` |

### Type Behavior

```
int + int = int       // 1 + 2 = 3
int + float = float   // 1 + 2.0 = 3.0
float + float = float // 1.0 + 2.0 = 3.0
str + str = str       // "a" + "b" = "ab"
str + any = str       // "a" + 42 = "a42"
int * int = int       // 3 * 4 = 12
str * int = str       // "ha" * 3 = "hahaha"
```

## Comparison Operators

| Operator | Name | Description | Example |
|----------|------|-------------|---------|
| `==` | Equal | Values are equal | `1 == 1` → `true` |
| `!=` | Not equal | Values are not equal | `1 != 2` → `true` |
| `<` | Less than | Left is less than right | `1 < 2` → `true` |
| `<=` | Less than or equal | Left is less than or equal to right | `1 <= 1` → `true` |
| `>` | Greater than | Left is greater than right | `2 > 1` → `true` |
| `>=` | Greater than or equal | Left is greater than or equal to right | `2 >= 2` → `true` |

### Comparison Rules

```
int vs int        // compared by value
float vs float    // compared by value
int vs float      // int is converted to float, then compared
str vs str        // lexicographic comparison
bool vs bool      // true > false
null vs null      // always equal
```

## Logical Operators

| Operator | Name | Description | Example |
|----------|------|-------------|---------|
| `&&` | Logical AND | Both are true | `true && false` → `false` |
| `\|\|` | Logical OR | Either is true | `true \|\| false` → `true` |
| `!` | Logical NOT | Negation | `!true` → `false` |

### Short-Circuit Evaluation

```
// && short-circuits: if the left side is false, the right side does not run
false && expensive_function()  // expensive_function does not run

// || short-circuits: if the left side is true, the right side does not run
true || expensive_function()   // expensive_function does not run
```

### Truth Table

```
&& | true | false
---|------|------
true  | true  | false
false | false | false

|| | true | false
---|------|------
true  | true  | true
false | true  | false
```

## Bitwise Operators

| Operator | Name | Description | Example |
|----------|------|-------------|---------|
| `&` | Bitwise AND | 1 only when both bits are 1 | `0b1010 & 0b1100` → `0b1000` |
| `\|` | Bitwise OR | 1 when either bit is 1 | `0b1010 \| 0b1100` → `0b1110` |
| `^` | Bitwise XOR | 1 when the bits differ | `0b1010 ^ 0b1100` → `0b0110` |
| `~` | Bitwise NOT | Invert all bits | `~0b1010` → `...11110101` |
| `<<` | Left shift | Shift left by the given count | `1 << 3` → `8` |
| `>>` | Right shift | Shift right by the given count | `8 >> 2` → `2` |

### Examples

```
// Set bits
let flags = 0b0000
flags = flags | 0b0001  // set bit 0
flags = flags | 0b0010  // set bit 1

// Clear bits
flags = flags & ~0b0001  // clear bit 0

// Toggle bits
flags = flags ^ 0b0001  // toggle bit 0

// Test bits
if flags & 0b0001 != 0 {
    // bit 0 is set
}
```

## Assignment Operators

| Operator | Equivalent | Example |
|----------|------------|---------|
| `=` | — | `x = 10` |
| `+=` | `x = x + y` | `x += 5` |
| `-=` | `x = x - y` | `x -= 3` |
| `*=` | `x = x * y` | `x *= 2` |
| `/=` | `x = x / y` | `x /= 4` |
| `%=` | `x = x % y` | `x %= 3` |
| `&=` | `x = x & y` | `x &= 0xFF` |
| `\|=` | `x = x \| y` | `x \|= 0x01` |
| `^=` | `x = x ^ y` | `x ^= 0xFF` |
| `<<=` | `x = x << y` | `x <<= 1` |
| `>>=` | `x = x >> y` | `x >>= 1` |

## Operator Precedence

From highest to lowest:

| Precedence | Operators | Associativity | Description |
|------------|-----------|---------------|-------------|
| 15 | `()` | — | Parentheses |
| 14 | `.` `[]` `()` | left | Member access, indexing, function call |
| 13 | `**` | right | Exponentiation |
| 12 | `-` `!` `~` | right | Unary operators |
| 11 | `*` `/` `%` | left | Multiplicative |
| 10 | `+` `-` | left | Additive |
| 9 | `<<` `>>` | left | Shift |
| 8 | `&` | left | Bitwise AND |
| 7 | `^` | left | Bitwise XOR |
| 6 | `\|` | left | Bitwise OR |
| 5 | `==` `!=` `<` `<=` `>` `>=` | left | Comparison |
| 4 | `&&` | left | Logical AND |
| 3 | `\|\|` | left | Logical OR |
| 2 | `=` `+=` `-=` etc. | right | Assignment |
| 1 | `,` | left | Comma |

## Next Steps

- [Built-in Functions](/v0.3/en/reference/builtins) — standard library functions
- [Error Codes](/v0.3/en/reference/errors) — error codes
