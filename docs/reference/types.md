# 类型系统

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [AST 源码](https://github.com/Maicarons/mailang/blob/master/crates/mailang-ast/src/lib.rs)

## 概述

MaìLang 采用混合类型系统，支持类型推导和显式类型标注。默认使用类型推导，也可以显式指定类型。

## 基本类型

### int（整数）

64 位有符号整数。

```
let a = 42
let b = -10
let c = 0
let d: int = 100
```

**范围**: -9,223,372,036,854,775,808 ~ 9,223,372,036,854,775,807

**字面量格式**:
- 十进制: `42`, `1_000_000`
- 十六进制: `0xFF`, `0x00`
- 八进制: `0o77`, `0o0`
- 二进制: `0b1010`, `0b0`

### float（浮点数）

64 位 IEEE 754 双精度浮点数。

```
let a = 3.14
let b = -0.5
let c = 1.0
let d: float = 2.5
```

**范围**: ±2.2250738585072014 × 10^-308 ~ ±1.7976931348623157 × 10^308

### bool（布尔）

布尔值，`true` 或 `false`。

```
let a = true
let b = false
let c: bool = true
```

### str（字符串）

UTF-8 编码的字符串。

```
let a = "hello"
let b = "你好"
let c = "🔥"
let d: str = "MaìLang"
```

**转义字符**:
- `\n` - 换行
- `\t` - 制表符
- `\\` - 反斜杠
- `\"` - 双引号
- `\0` - 空字符
- `\u&lbrace;XXXX&rbrace;` - Unicode 转义

**字符串插值**:
```
let name = "MaìLang"
let greeting = "Hello, {name}!"
let expr = "Result: {1 + 2}"
```

### char（字符）

单个 Unicode 字符。

```
let a = 'A'
let b = '中'
let c = '🔥'
let d: char = '\n'
```

### null（空值）

表示空值。

```
let a = null
let b: null = null
```

## 复合类型

### 数组 `[T]`

同类型元素的有序集合。

```
let nums = [1, 2, 3, 4, 5]
let strs = ["a", "b", "c"]
let empty: [int] = []
let mixed: [any] = [1, "two", 3.0]
```

**操作**:
```
arr[0]          // 访问
arr.len         // 长度
arr.push(x)     // 添加
arr.pop()       // 删除末尾
arr.insert(i, x)// 插入
arr.contains(x) // 是否包含
arr.join(sep)   // 连接为字符串
arr.reverse()   // 反转
arr.clear()     // 清空
```

### 字典 `&lbrace;K: V&rbrace;`

键值对集合。

```
let map = {
    "name": "MaìLang",
    "version": "0.1.0"
}
let empty: {str: int} = {}
```

**操作**:
```
map["key"]      // 访问
map["key"] = v  // 设置
map.has(k)      // 检查键
map.remove(k)   // 删除
map.keys()      // 所有键
map.values()    // 所有值
map.clear()     // 清空
```

### 元组 `(T1, T2, ...)`

不同类型元素的有序集合。

```
let point = (3.0, 4.0)
let person = ("张三", 25, true)
let nested = (1, (2, 3))
```

**访问**:
```
let (x, y) = point  // 解构
let first = point[0] // 索引
```

### Result&lt;T, E&gt;

用于错误处理的类型。泛型参数仅为标注（annotation-only），不做单态化或真正的泛型检查。

```
fn divide(a: float, b: float) -> Result<float, str> {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}

// 后缀写法 `expr match { ... }` 尚未实现（开发中）
let result = match divide(10.0, 3.0) {
    Ok(v) => v,
    Err(e) => 0.0
}
```

### Option&lt;T&gt;

用于表示可选值的类型。泛型参数仅为标注（annotation-only），不做单态化。

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

## 类型推导

MaìLang 支持自动类型推导：

```
let a = 42          // 推导为 int
let b = 3.14        // 推导为 float
let c = true        // 推导为 bool
let d = "hello"     // 推导为 str
let e = [1, 2, 3]   // 推导为 [int]
let f = {"a": 1}    // 推导为 {str: int}
```

## 类型转换

```
// 显式转换
let i = 42
let f = i as float   // 42.0
let s = i as str     // "42"

// 隐式转换（在某些上下文中）
let a: float = 42    // int 自动转为 float
```

## any 类型

`any` 类型可以持有任何类型的值：

```
let value: any = 42
value = "hello"
value = [1, 2, 3]
```

## 类型检查

当前没有 `is_int` / `is_float` / `is_str` 等类型判定内置函数。可用做法：

```
// Result / Option 模式匹配
match value {
    Ok(v) => ...,
    Err(e) => ...
}
```

## 下一步

- [运算符](/reference/operators) - 运算符详细说明
- [内置函数](/reference/builtins) - 标准库函数
