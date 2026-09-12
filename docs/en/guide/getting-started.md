# Getting Started

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [v0.2.2](https://github.com/Maicarons/mailang/releases/tag/v0.2.2) · [Playground](https://maicarons.github.io/mailang/playground)

## System Requirements

- **OS**: Windows, macOS, Linux
- **Rust**: 1.70+ (for building from source)
- **Memory**: Minimum 256MB

## Installation

### Option 1: Install from crates.io

```bash
cargo install mailang-cli
```

### Option 2: Build from source

```bash
git clone https://github.com/Maicarons/mailang.git
cd mailang
cargo build --release
```

The binary will be at `target/release/mailang`.

### Option 3: Download pre-built binary

Download from [GitHub Releases](https://github.com/Maicarons/mailang/releases).

## Verify Installation

```bash
mailang --version
# Output: mailang 0.2.2
```

## Hello World

Create `hello.mai`:

```
// This is MaìLang's Hello World
println("Hello, MaìLang! 🌍")
```

Run:

```bash
mailang run hello.mai
# Output: Hello, MaìLang! 🌍
```

## CLI commands

| Command | Description |
|---------|-------------|
| `mailang` | Start REPL |
| `mailang run file.mai` | Run a source file |
| `mailang run file.mailangbc` | Run compiled bytecode |
| `mailang eval '1+2'` | Evaluate inline code |
| `mailang build file.mai` | Compile to `.mailangbc` |
| `mailang fmt file.mai` | Format sources (`--check` to lint only) |
| `mailang lsp` | Start the language server |

## REPL

REPL (Read-Eval-Print Loop) is the best way to learn and debug:

```bash
mailang
```

```
MaìLang REPL v0.2.2
Type 'exit' or 'quit' to exit.
> 1 + 2
3
> let name = "MaìLang"
> println("Hello, {name}!")
Hello, MaìLang!
> exit
```

## Inline Execution

Use the `eval` subcommand to execute code directly:

```bash
mailang eval 'println(42 * 2)'
# Output: 84
```

## Basic Syntax

### Variables

```
let x = 42              // immutable
var y = 100             // mutable
const PI = 3.14159      // constant
let name: str = "MaìLang" // explicit type
```

### Functions

```
fn add(a: int, b: int) -> int {
    return a + b
}

fn greet(name = "World") -> str {
    return "Hello, {name}!"
}
```

### Control Flow

```
if x > 10 {
    println("Greater than 10")
} elif x > 5 {
    println("Greater than 5")
} else {
    println("Less than or equal to 5")
}

for i in 0..10 {
    println(i)
}
```

### OOP

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

let dog = Dog("Rex")
println(dog.speak())  // Rex barks!
```

### Error Handling

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

## Next Steps

- [Syntax Guide](/en/guide/syntax) - Complete syntax reference
- [OOP](/en/guide/oop) - Classes, inheritance, traits
- [Standard Library](/en/guide/stdlib) - Built-in modules
