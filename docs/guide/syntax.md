# 语法指南

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [v0.1.0-parser](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-parser) · [v0.1.0-lexer](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-lexer)

## 概述

MaìLang 采用混合式语法风格，结合了 Rust、Python、JavaScript 等语言的优点。语法设计遵循以下原则：

- **一致性**：相似的操作使用相似的语法
- **简洁性**：减少不必要的样板代码
- **可读性**：代码即文档

## 注释

```
// 单行注释

/* 
   多行注释
   可以跨越多行
*/

/* /* 支持嵌套 */ */
```

## 变量与常量

### 不可变变量 `let`

```
let x = 42
let name: str = "MaìLang"
let nums: [int] = [1, 2, 3]
```

`let` 声明的变量不可重新赋值：

```
let x = 10
x = 20  // 错误！x 是不可变的
```

### 可变变量 `var`

```
var count = 0
count = count + 1  // 正确
```

### 常量 `const`

```
const PI = 3.14159
const MAX_SIZE = 1024
```

常量必须在声明时初始化，且不可修改。

### 类型标注

类型标注是可选的，编译器会自动推导：

```
let a = 42          // 推导为 int
let b = 3.14        // 推导为 float
let c = true        // 推导为 bool
let d = "hello"     // 推导为 str
let e: int = 42     // 显式标注
```

## 基本类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `int` | 64 位有符号整数 | `42`, `-10`, `0` |
| `float` | 64 位浮点数 | `3.14`, `-0.5`, `1.0` |
| `bool` | 布尔值 | `true`, `false` |
| `str` | 字符串 | `"hello"`, `"你好"` |
| `char` | Unicode 字符 | `'A'`, `'中'`, `'🔥'` |
| `null` | 空值 | `null` |

### 数字字面量

```
let decimal = 42
let hex = 0xFF
let octal = 0o77
let binary = 0b1010
let float = 3.14
let with_sep = 1_000_000  // 下划线分隔
```

### 字符串

```
let simple = "hello"
let multiline = "
  多行
  字符串
"

// 转义字符
let escaped = "换行\n\t制表符"

// Unicode 转义
let unicode = "\u{4f60}\u{597d}"  // "你好"

// 字符串插值
let name = "MaìLang"
let greeting = "你好，{name}！"
let expr = "结果：{1 + 2}"
```

### 字符

```
let letter = 'A'
let chinese = '中'
let emoji = '🔥'
let escaped = '\n'
```

## 复合类型

### 数组 `[T]`

```
let nums = [1, 2, 3, 4, 5]
let mixed = [1, "two", 3.0]  // 类型推导为 [any]

// 访问
let first = nums[0]
let len = nums.len

// 修改（需要 var）
var arr = [1, 2, 3]
arr[0] = 10
```

### 字典 `&lbrace;K: V&rbrace;`

```
let person = {
    "name": "张三",
    "age": 25,
    "city": "北京"
}

// 访问
let name = person["name"]
let age = person["age"]

// 修改（需要 var）
var config = {"debug": false}
config["debug"] = true
```

### 元组 `(T1, T2, ...)`

```
let point = (3.0, 4.0)
let person = ("张三", 25, "北京")

// 访问
let x = point[0]
let y = point[1]
```

## 运算符

### 算术运算符

| 运算符 | 说明 | 示例 |
|--------|------|------|
| `+` | 加法 | `1 + 2 = 3` |
| `-` | 减法 | `5 - 3 = 2` |
| `*` | 乘法 | `4 * 3 = 12` |
| `/` | 除法 | `10 / 3 = 3`（整数除法） |
| `%` | 取模 | `10 % 3 = 1` |
| `**` | 幂运算 | `2 ** 3 = 8` |
| `-` | 取负 | `-x` |

### 比较运算符

| 运算符 | 说明 | 示例 |
|--------|------|------|
| `==` | 等于 | `1 == 1` → `true` |
| `!=` | 不等于 | `1 != 2` → `true` |
| `<` | 小于 | `1 < 2` → `true` |
| `<=` | 小于等于 | `1 <= 1` → `true` |
| `>` | 大于 | `2 > 1` → `true` |
| `>=` | 大于等于 | `2 >= 2` → `true` |

### 逻辑运算符

| 运算符 | 说明 | 示例 |
|--------|------|------|
| `&&` | 逻辑与 | `true && false` → `false` |
| `\|\|` | 逻辑或 | `true \|\| false` → `true` |
| `!` | 逻辑非 | `!true` → `false` |

### 位运算符

| 运算符 | 说明 | 示例 |
|--------|------|------|
| `&` | 按位与 | `0b1010 & 0b1100` → `0b1000` |
| `\|` | 按位或 | `0b1010 \| 0b1100` → `0b1110` |
| `^` | 按位异或 | `0b1010 ^ 0b1100` → `0b0110` |
| `~` | 按位取反 | `~0b1010` |
| `<<` | 左移 | `1 << 3` → `8` |
| `>>` | 右移 | `8 >> 2` → `2` |

### 赋值运算符

```
var x = 10
x += 5   // x = x + 5
x -= 3   // x = x - 3
x *= 2   // x = x * 2
x /= 4   // x = x / 4
x %= 3   // x = x % 3
x &= 0xFF // x = x & 0xFF
x |= 0x01 // x = x | 0x01
x ^= 0xFF // x = x ^ 0xFF
x <<= 1  // x = x << 1
x >>= 1  // x = x >> 1
```

### 运算符优先级

从高到低：

1. `()` 括号
2. `**` 幂运算
3. `-`（一元）, `!`, `~`
4. `*`, `/`, `%`
5. `+`, `-`
6. `<<`, `>>`
7. `&`
8. `^`
9. `|`
10. `==`, `!=`, `<`, `<=`, `>`, `>=`
11. `&&`
12. `||`
13. 赋值运算符

## 控制流

### if-elif-else

```
if condition {
    // ...
} elif other_condition {
    // ...
} else {
    // ...
}
```

### for 循环

```
// 遍历数组
for item in [1, 2, 3] {
    println(item)
}

// 遍历范围
for i in 0..10 {
    println(i)  // 0, 1, 2, ..., 9
}

// 带步长（需要 while）
var i = 0
while i < 10 {
    println(i)
    i += 2
}
```

### while 循环

```
var count = 0
while count < 10 {
    println(count)
    count += 1
}
```

### break 和 continue

```
for i in 0..100 {
    if i == 5 {
        continue  // 跳过 5
    }
    if i == 10 {
        break  // 在 10 停止
    }
    println(i)
}
```

### match 表达式

```
let x = 42

// 基本匹配
let result = match x {
    0 => "零",
    1 => "一",
    _ => "其他"
}

// 范围匹配
let category = match x {
    0 => "零",
    1..10 => "个位数",
    10..100 => "两位数",
    _ => "更大的数"
}

// 模式匹配
let value = match some_value {
    Ok(v) => v,
    Err(e) => {
        println("错误: {e}")
        0
    }
}
```

## 函数

### 基本函数

```
fn add(a: int, b: int) -> int {
    return a + b
}

let result = add(1, 2)  // 3
```

### 默认参数

```
fn greet(name = "世界") -> str {
    return "你好，{name}！"
}

greet()          // "你好，世界！"
greet("MaìLang") // "你好，MaìLang！"
```

### Lambda / 闭包

```
let square = fn(x) -> x * x
let add = fn(a, b) -> a + b

// 在高阶函数中使用
let nums = [1, 2, 3, 4, 5]
let doubled = nums.map(fn(x) -> x * 2)
```

## 错误处理

### Result 类型

```
fn divide(a: float, b: float) -> Result<float, str> {
    if b == 0.0 {
        return Err("除数不能为零")
    }
    return Ok(a / b)
}

// 使用 match 处理
let result = divide(10.0, 3.0) match {
    Ok(v) => v,
    Err(e) => {
        println("错误: {e}")
        0.0
    }
}
```

### Option 类型

```
fn find(arr: [int], target: int) -> Option<int> {
    for item in arr {
        if item == target {
            return Some(item)
        }
    }
    return None
}

let found = find([1, 2, 3], 2) match {
    Some(v) => "找到了: {v}",
    None => "未找到"
}
```

## 模块系统

### 导入

```
import math.{sqrt, PI}
import utils as u

// 使用
let hypotenuse = sqrt(3.0 ** 2 + 4.0 ** 2)
```

### 定义模块

```
module geometry {
    pub fn area_circle(radius: float) -> float {
        return PI * radius * radius
    }
    
    pub fn area_rect(width: float, height: float) -> float {
        return width * height
    }
}

// 使用
let area = geometry.area_circle(5.0)
```

## UTF-8 支持

MaìLang 强制使用 UTF-8 编码，标识符支持任何 Unicode 文字：

```
let 中文变量 = "支持中文标识符"
let مرحبا = "支持阿拉伯文"
let 🔥 = "支持emoji"
let 日本語変数 = "日本語もOK"

fn 你好() {
    println("你好世界")
}
```

## 下一步

- [面向对象](/guide/oop) - 类、继承、trait
- [标准库](/guide/stdlib) - 内置模块
- [FFI 接入](/guide/ffi) - 各语言接入指南
