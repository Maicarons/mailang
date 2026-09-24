# 快速开�?
> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [v0.2.6](https://github.com/Maicarons/mailang/releases/tag/v0.2.6) · [Playground](https://maicarons.github.io/mailang/playground)

## 系统要求

- **操作系统**：Windows、macOS、Linux
- **Rust** 1.70+（用于从源码构建�?- **内存**：至�?256MB

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
# 输出：mailang 0.2.6
```

## Hello World

创建 `hello.mai`�?
```
// 这是 MaìLang �?Hello World
println("你好，MaìLang�?)
```

运行�?
```bash
mailang run hello.mai
# 输出：你好，MaìLang�?```

## CLI 子命�?
| 命令 | 说明 |
|------|------|
| `mailang` | 启动 REPL |
| `mailang run file.mai` | 运行源文�?|
| `mailang run file.mailangbc` | 运行编译后的字节�?|
| `mailang eval '1+2'` | 执行内联代码 |
| `mailang build file.mai` | 编译�?`.mailangbc` |
| `mailang fmt file.mai` | 格式化源文件（`--check` 仅检查） |
| `mailang lsp` | 启动 Language Server |

## REPL 交互模式

```bash
mailang
```

```
MaìLang REPL v0.2.6
Type 'exit' or 'quit' to exit.
> 1 + 2
3
> let name = "MaìLang"
> println("Hello, {name}!")
Hello, MaìLang!
> exit
```

## 内联代码执行

```bash
mailang eval 'println(42 * 2)'
# 输出�?4
```

## 编译为字节码

```bash
mailang build hello.mai -o hello.mailangbc
mailang run hello.mailangbc
```

## 基本语法速览

### 变量

```
let x = 42              // 不可变变�?var y = 100             // 可变变量
const PI = 3.14159      // 常量
let name: str = "麦语"  // 显式类型
```

### 函数

```
fn add(a: int, b: int) -> int {
    return a + b
}
```

### 控制�?
```
if x > 10 {
    println("大于10")
} elif x > 5 {
    println("大于5")
} else {
    println("小于等于5")
}

for i in 0..10 {
    println(i)
}
```

### 面向对象�?Trait

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
- [语法指南](/guide/syntax) - 完整语法参�?- [面向对象](/guide/oop) - 类、继承、trait
- [标准库](/guide/stdlib) - 内置模块
