# 快速开�?

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [v0.1.0](https://github.com/Maicarons/mailang/releases/tag/v0.1.0) · [Playground](https://maicarons.github.io/mailang/playground)

## 系统要求

- **操作系统**：Windows、macOS、Linux
- **Rust**�?.70+（用于从源码构建�?
- **内存**：最�?256MB

## 安装

### 方式一：从 crates.io 安装

```bash
cargo install mailang-cli
```

### 方式二：从源码构�?

```bash
git clone https://github.com/Maicarons/mailang.git
cd mailang
cargo build --release
```

构建完成后，二进制文件位�?`target/release/mailang`�?

### 方式三：下载预编译二进制

�?[GitHub Releases](https://github.com/Maicarons/mailang/releases) 下载对应平台的二进制文件�?

## 验证安装

```bash
mailang --version
# 输出：mailang 0.1.0
```

## Hello World

### 创建文件

创建 `hello.mai`�?

```
// 这是 MaìLang �?Hello World
println("你好，MaìLang！�?)
```

### 运行

```bash
mailang run hello.mai
# 输出：你好，MaìLang！�?
```

## REPL 交互模式

REPL（Read-Eval-Print Loop）是学习和调试的最佳方式：

```bash
mailang
```

进入 REPL 后，可以逐行输入代码�?

```
MaìLang REPL v0.1.0
Type 'exit' or 'quit' to exit.
> 1 + 2
3
> let name = "MaìLang"
> println("Hello, {name}!")
Hello, MaìLang!
> exit
```

## 内联代码执行

使用 `eval` 子命令直接执行代码：

```bash
mailang eval 'println(42 * 2)'
# 输出�?4
```

## 基本语法速览

### 变量

```
let x = 42              // 不可变变�?
var y = 100             // 可变变量
const PI = 3.14159      // 常量
let name: str = "麦语"  // 显式类型
```

### 函数

```
fn add(a: int, b: int) -> int {
    return a + b
}

// 默认参数
fn greet(name = "世界") -> str {
    return "你好，{name}�?
}
```

### 控制�?

```
// if-elif-else
if x > 10 {
    println("大于10")
} elif x > 5 {
    println("大于5")
} else {
    println("小于等于5")
}

// for 循环
for i in 0..10 {
    println(i)
}

// while 循环
while condition {
    // ...
}
```

### 面向对象

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

let dog = Dog("小黑")
println(dog.speak())  // 小黑 barks!
```

### 错误处理

```
fn divide(a: float, b: float) -> Result<float, str> {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}

match divide(10.0, 3.0) {
    Ok(v) => println("Result: {v}"),
    Err(e) => println("Error: {e}")
}
```

## 下一�?

- [语法指南](/guide/syntax) - 完整语法参�?
- [面向对象](/guide/oop) - 类、继承、trait
- [标准库](/guide/stdlib) - 内置模块
