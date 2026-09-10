# API 参考

## 概述

本节提供 MaìLang 的完整 API 参考文档，包括类型系统、运算符、内置函数和错误码。

## 目录

- [类型系统](/reference/types) - 所有类型的详细说明
- [运算符](/reference/operators) - 运算符优先级和行为
- [内置函数](/reference/builtins) - 标准库函数参考
- [错误码](/reference/errors) - 错误代码和消息

## 快速参考

### 基本类型

| 类型 | 大小 | 范围 | 示例 |
|------|------|------|------|
| `int` | 8 字节 | -2^63 ~ 2^63-1 | `42` |
| `float` | 8 字节 | IEEE 754 | `3.14` |
| `bool` | 1 字节 | true/false | `true` |
| `str` | 变长 | UTF-8 | `"hello"` |
| `char` | 4 字节 | Unicode | `'A'` |
| `null` | 0 字节 | - | `null` |

### 复合类型

| 类型 | 语法 | 示例 |
|------|------|------|
| 数组 | `[T]` | `[1, 2, 3]` |
| 字典 | `{K: V}` | `{"a": 1}` |
| 元组 | `(T1, T2)` | `(1, "a")` |
| Result | `Result<T, E>` | `Ok(42)` / `Err("msg")` |
| Option | `Option<T>` | `Some(42)` / `None` |

### 运算符优先级

| 优先级 | 运算符 | 结合性 |
|--------|--------|--------|
| 14 | `()` | - |
| 13 | `**` | 右 |
| 12 | `-` `!` `~` | 右 |
| 11 | `*` `/` `%` | 左 |
| 10 | `+` `-` | 左 |
| 9 | `<<` `>>` | 左 |
| 8 | `&` | 左 |
| 7 | `^` | 左 |
| 6 | `\|` | 左 |
| 5 | `==` `!=` `<` `<=` `>` `>=` | 左 |
| 4 | `&&` | 左 |
| 3 | `\|\|` | 左 |
| 2 | 赋值 | 右 |
| 1 | `,` | 左 |

### 控制流

```
if condition { ... } elif condition { ... } else { ... }
for item in iterable { ... }
while condition { ... }
match value { pattern => expr, ... }
return value
break
continue
```

### 函数

```
fn name(param: Type = default) -> ReturnType {
    // body
}
```

### 类

```
class Name extends Parent implements Trait {
    let property: Type
    fn init(param: Type) { ... }
    fn method() -> Type { ... }
}
```

### Trait

```
trait Name {
    fn required() -> Type
    fn default() -> Type { ... }
}
```
