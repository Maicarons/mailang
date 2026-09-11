# 错误码

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [VM 错误源码](https://github.com/Maicarons/mailang/blob/master/crates/mailang-vm/src/error.rs)

## 概述

MaìLang 使用 Result 类型进行错误处理，不抛出异常。所有可能失败的操作都返回 `Result<T, E>` 或 `Option<T>`。

## 运行时错误

### VmError

| 错误代码 | 说明 | 示例 |
|----------|------|------|
| `StackOverflow` | 栈溢出 | 递归过深 |
| `StackUnderflow` | 栈下溢 | 弹出空栈 |
| `TypeError` | 类型错误 | `"a" - 1` |
| `UndefinedVariable` | 未定义变量 | `println(x)` |
| `UndefinedFunction` | 未定义函数 | `foo()` |
| `UndefinedProperty` | 未定义属性 | `obj.bar` |
| `WrongArgumentCount` | 参数数量错误 | `add(1)` |
| `DivisionByZero` | 除以零 | `1 / 0` |
| `IndexOutOfBounds` | 索引越界 | `arr[100]` |
| `InvalidOperation` | 无效操作 | `null + 1` |
| `RuntimeError` | 运行时错误 | 通用错误 |

### CompilerError

| 错误代码 | 说明 |
|----------|------|
| `UndefinedVariable` | 未定义的变量 |
| `TooManyConstants` | 常量过多（超过 65536） |
| `TooManyLocals` | 局部变量过多（超过 256） |
| `TooManyUpvalues` | 闭包变量过多（超过 256） |
| `InvalidAssignmentTarget` | 无效的赋值目标 |
| `BreakOutsideLoop` | break 在循环外 |
| `ContinueOutsideLoop` | continue 在循环外 |
| `ReturnOutsideFunction` | return 在函数外 |
| `DuplicateFunction` | 重复的函数名 |
| `DuplicateClass` | 重复的类名 |
| `DuplicateTrait` | 重复的 trait 名 |

### ParseError

| 错误代码 | 说明 |
|----------|------|
| `UnexpectedToken` | 意外的 token |
| `UnexpectedEof` | 意外的文件结束 |
| `InvalidExpression` | 无效的表达式 |
| `InvalidStatement` | 无效的语句 |
| `ExpectedIdentifier` | 期望标识符 |
| `ExpectedTypeAnnotation` | 期望类型标注 |
| `ExpectedExpression` | 期望表达式 |
| `ExpectedPattern` | 期望模式 |
| `ExpectedBlock` | 期望代码块 |
| `InvalidAssignmentTarget` | 无效的赋值目标 |
| `InvalidFunctionDefinition` | 无效的函数定义 |
| `InvalidClassDefinition` | 无效的类定义 |
| `InvalidPattern` | 无效的模式 |

### LexerError

| 错误代码 | 说明 |
|----------|------|
| `UnexpectedCharacter` | 意外的字符 |
| `UnterminatedString` | 未结束的字符串 |
| `UnterminatedChar` | 未结束的字符 |
| `InvalidEscapeSequence` | 无效的转义序列 |
| `InvalidNumber` | 无效的数字格式 |
| `InvalidUnicodeEscape` | 无效的 Unicode 转义 |

## AnalyzerError

| 错误代码 | 说明 |
|----------|------|
| `UndefinedVariable` | 未定义的变量 |
| `UndefinedFunction` | 未定义的函数 |
| `UndefinedClass` | 未定义的类 |
| `UndefinedTrait` | 未定义的 trait |
| `TypeMismatch` | 类型不匹配 |
| `ImmutableAssignment` | 对不可变变量赋值 |
| `ConstantAssignment` | 对常量赋值 |
| `WrongArgumentCount` | 参数数量错误 |
| `MethodNotFound` | 方法未找到 |
| `PropertyNotFound` | 属性未找到 |
| `MissingTraitMethod` | 未实现 trait 方法 |
| `CircularInheritance` | 循环继承 |
| `ThisOutsideMethod` | this 在方法外使用 |
| `SuperOutsideSubclass` | super 在子类外使用 |
| `DuplicateDefinition` | 重复定义 |
| `UnusedVariable` | 未使用的变量 |

## 错误处理最佳实践

### 使用 match 处理错误

```
let result = divide(10.0, 0.0) match {
    Ok(v) => println("结果: {v}"),
    Err(e) => println("错误: {e}")
}
```

### 使用 ? 操作符（计划中）

```
fn process() -> Result<str, str> {
    let value = divide(10.0, 0.0)?  // 如果错误，提前返回
    Ok("处理完成")
}
```

### 自定义错误类型

```
enum MyError {
    NotFound { resource: str },
    PermissionDenied { action: str },
    InvalidInput { field: str, reason: str },
}

fn do_something() -> Result<str, MyError> {
    // ...
    return Err(MyError::NotFound { resource: "file.txt" })
}
```

## 下一步

- [类型系统](/reference/types) - 类型详解
- [运算符](/reference/operators) - 运算符参考
- [内置函数](/reference/builtins) - 标准库函数
