# Error Codes

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [VM Error Source](https://github.com/Maicarons/mailang/blob/master/crates/mailang-vm/src/error.rs)

## Overview

MaìLang uses the Result type for error handling and does not throw exceptions. Every fallible operation returns `Result&lt;T, E&gt;` or `Option&lt;T&gt;`.

## Runtime Errors

### VmError

| Error code | Description | Example |
|------------|-------------|---------|
| `StackOverflow` | Stack overflow | Recursion too deep |
| `StackUnderflow` | Stack underflow | Pop from an empty stack |
| `TypeError` | Type error | `"a" - 1` |
| `UndefinedVariable` | Undefined variable | `println(x)` |
| `UndefinedFunction` | Undefined function | `foo()` |
| `UndefinedProperty` | Undefined property | `obj.bar` |
| `WrongArgumentCount` | Wrong argument count | `add(1)` |
| `DivisionByZero` | Division by zero | `1 / 0` |
| `IndexOutOfBounds` | Index out of bounds | `arr[100]` |
| `InvalidOperation` | Invalid operation | `null + 1` |
| `RuntimeError` | Runtime error | Generic error |

### CompilerError

| Error code | Description |
|------------|-------------|
| `UndefinedVariable` | Undefined variable |
| `TooManyConstants` | Too many constants (over 65536) |
| `TooManyLocals` | Too many locals (over 256) |
| `TooManyUpvalues` | Too many upvalues (over 256) |
| `InvalidAssignmentTarget` | Invalid assignment target |
| `BreakOutsideLoop` | `break` outside a loop |
| `ContinueOutsideLoop` | `continue` outside a loop |
| `ReturnOutsideFunction` | `return` outside a function |
| `DuplicateFunction` | Duplicate function name |
| `DuplicateClass` | Duplicate class name |
| `DuplicateTrait` | Duplicate trait name |

### ParseError

| Error code | Description |
|------------|-------------|
| `UnexpectedToken` | Unexpected token |
| `UnexpectedEof` | Unexpected end of file |
| `InvalidExpression` | Invalid expression |
| `InvalidStatement` | Invalid statement |
| `ExpectedIdentifier` | Expected identifier |
| `ExpectedTypeAnnotation` | Expected type annotation |
| `ExpectedExpression` | Expected expression |
| `ExpectedPattern` | Expected pattern |
| `ExpectedBlock` | Expected block |
| `InvalidAssignmentTarget` | Invalid assignment target |
| `InvalidFunctionDefinition` | Invalid function definition |
| `InvalidClassDefinition` | Invalid class definition |
| `InvalidPattern` | Invalid pattern |

### LexerError

| Error code | Description |
|------------|-------------|
| `UnexpectedCharacter` | Unexpected character |
| `UnterminatedString` | Unterminated string |
| `UnterminatedChar` | Unterminated char |
| `InvalidEscapeSequence` | Invalid escape sequence |
| `InvalidNumber` | Invalid number format |
| `InvalidUnicodeEscape` | Invalid Unicode escape |

## AnalyzerError

| Error code | Description |
|------------|-------------|
| `UndefinedVariable` | Undefined variable |
| `UndefinedFunction` | Undefined function |
| `UndefinedClass` | Undefined class |
| `UndefinedTrait` | Undefined trait |
| `TypeMismatch` | Type mismatch |
| `ImmutableAssignment` | Assignment to an immutable variable |
| `ConstantAssignment` | Assignment to a constant |
| `WrongArgumentCount` | Wrong argument count |
| `MethodNotFound` | Method not found |
| `PropertyNotFound` | Property not found |
| `MissingTraitMethod` | Unimplemented trait method |
| `CircularInheritance` | Circular inheritance |
| `ThisOutsideMethod` | `this` used outside a method |
| `SuperOutsideSubclass` | `super` used outside a subclass |
| `DuplicateDefinition` | Duplicate definition |
| `UnusedVariable` | Unused variable |

## Error Handling Best Practices

### Handle errors with match

```
let result = match divide(10.0, 0.0) {
    Ok(v) => println("结果: {v}"),
    Err(e) => println("错误: {e}")
}
```

### Use the ? operator

`?` is supported: it unwraps `Ok`/`Some` and early-returns `Err`/`None` from the current function.

```
fn process() -> Result<str, str> {
    let value = divide(10.0, 0.0)?  // early-return on error
    Ok("处理完成")
}
```

### Custom error types

> The `enum` keyword is **not supported yet**. For now, use string error codes or a class to carry error information:

```
fn do_something() -> Result<str, str> {
    // ...
    return Err("NotFound: file.txt")
}
```

## Next Steps

- [Type System](/v0.3/en/reference/types) — type details
- [Operators](/v0.3/en/reference/operators) — operator reference
- [Built-in Functions](/v0.3/en/reference/builtins) — standard library functions
