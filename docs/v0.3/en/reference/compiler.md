# Compiler Internals

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [Compiler Source](https://github.com/Maicarons/mailang/tree/master/crates/mailang-compiler) · [VM Source](https://github.com/Maicarons/mailang/tree/master/crates/mailang-vm)

## Overview

MaìLang uses a classic compilation pipeline to transform source code into bytecode executed by a stack-based virtual machine.

```
Source Code (.mai)
    │
    ▼
┌─────────────┐
│    Lexer     │  Source → Tokens
└─────┬───────┘
      │
      ▼
┌─────────────┐
│    Parser    │  Tokens → AST
└─────┬───────┘
      │
      ▼
┌─────────────┐
│   Compiler   │  AST → Bytecode
└─────┬───────┘
      │
      ▼
┌─────────────┐
│     VM       │  Execute Bytecode
└─────────────┘
```

## Crate Structure

| Crate | Responsibility | Input | Output |
|-------|---------------|-------|--------|
| `mailang-lexer` | Tokenization | Source string | `Vec&lt;Token&gt;` |
| `mailang-ast` | AST types | - | Data structures |
| `mailang-parser` | Parsing | `Vec&lt;Token&gt;` | `Program` |
| `mailang-bytecode` | Bytecode types | - | Data structures |
| `mailang-compiler` | Compilation | `Program` | `Bytecode` |
| `mailang-vm` | Execution | `Bytecode` | `Value` |
| `mailang-stdlib` | Built-ins | - | Functions |
| `mailang-core` | Integration | - | Unified API |

## Lexer

### Token Types

```rust
pub enum Token {
    Integer(i64), Float(f64), String(String), Char(char), Bool(bool), Null,
    Identifier(String),
    Let, Var, Const, Fn, Class, Trait, If, Elif, Else, For, While, ...
    Plus, Minus, Star, Slash, Equal, NotEqual, Less, Greater, ...
    LeftParen, RightParen, LeftBrace, RightBrace, ...
}
```

### Unicode Support

Identifiers support Unicode via `unicode-xid`:
- `中文变量`, `مرحبا`, `🔥`, `my_var`

## Parser

### Recursive Descent + Pratt Parsing

Statements use recursive descent; expressions use Pratt parsing for operator precedence.

```rust
// Statement parsing (recursive descent)
fn parse_statement() -> Stmt {
    match peek() {
        Let => parse_let_statement(),
        If => parse_if_statement(),
        _ => parse_expression_statement(),
    }
}

// Expression parsing (Pratt)
fn parse_expression() -> Expr {
    parse_range()  // Lowest precedence
}

// Precedence chain (low → high):
// assignment → range → or → and → bitwise_or → ... → unary → call → primary
```

### Operator Precedence

| Prec | Operators | Assoc | Description |
|------|-----------|-------|-------------|
| 1 | `=` `+=` `-=` ... | Right | Assignment |
| 2 | `..` | Left | Range |
| 3 | `\|\|` | Left | Logical OR |
| 4 | `&&` | Left | Logical AND |
| 5-7 | `\|` `^` `&` | Left | Bitwise |
| 8 | `==` `!=` | Left | Equality |
| 9 | `<` `>` `<=` `>=` | Left | Comparison |
| 10 | `<<` `>>` | Left | Shift |
| 11 | `+` `-` | Left | Additive |
| 12 | `*` `/` `%` | Left | Multiplicative |
| 13 | `**` | Right | Power |
| 14 | `-` `!` `~` | Right | Unary |

## Compiler

### Bytecode Format

```rust
pub struct Bytecode {
    pub chunks: Vec<Chunk>,  // Function code blocks
    pub main_chunk: usize,   // Main function index
}

pub struct Chunk {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub name: String,
}
```

### Opcodes

| Opcode | Description |
|--------|-------------|
| `Push` | Push constant |
| `Pop` | Pop top |
| `LoadLocal` | Load local variable |
| `StoreLocal` | Store local variable |
| `LoadGlobal` | Load global variable |
| `StoreGlobal` | Store global variable |
| `Add` `Sub` `Mul` `Div` | Arithmetic |
| `Eq` `Ne` `Lt` `Gt` | Comparison |
| `And` `Or` `Not` | Logical |
| `Jump` `JumpIfFalse` | Control flow |
| `Call` `Return` | Function |
| `BuildArray` `IndexGet` | Collection |
| `Halt` | Stop execution |

### Compilation Example

```
Source: let x = 1 + 2

Bytecode:
  0: Push const[1]   // Push 1
  1: Push const[2]   // Push 2
  2: Add             // 1 + 2 = 3
  3: StoreLocal 0    // Store to x
```

## VM

### Stack-Based Execution

The default VM is **stack-based** with slot-indexed locals (an opt-in register backend is available via `--vm=register`, feature-parity with the stack VM):

```rust
pub struct Vm {
    bytecode: Bytecode,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    call_stack: Vec<CallFrame>,
    ip: usize,
}
```

### Value Types

```rust
pub enum Value {
    Null, Bool(bool), Int(i64), Float(f64), Str(String),
    Array(Vec<Value>), Map(Vec<(Value, Value)>),
    Function { name, arity, chunk_index },
    Class { name, methods },
    Instance { class_index, fields },
    Builtin { name, arity },
}
```

## Optimization

### Constant Folding

Compile-time evaluation of constant expressions:
```
const X = 1 + 2 * 3  →  Push 7
```

### Local Variable Access

Locals use stack index (O(1)) vs globals using hash lookup (O(n)).

## Next Steps

- [Parser Internals](/v0.3/en/reference/parser) - Parsing details
- [Type System](/v0.3/en/reference/types) - Type definitions
