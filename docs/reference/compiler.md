# 编译原理

## 概述

MaìLang 的编译器采用经典的流水线架构，将源代码转换为字节码并在虚拟机中执行。

```
源代码 (.mai)
    │
    ▼
┌─────────────┐
│   词法分析器  │  Lexer
│  (Lexer)     │  源代码 → Token 流
└─────┬───────┘
      │
      ▼
┌─────────────┐
│   语法分析器  │  Parser
│  (Parser)    │  Token 流 → AST
└─────┬───────┘
      │
      ▼
┌─────────────┐
│   语义分析器  │  Analyzer
│  (Analyzer)  │  AST → 类型检查
└─────┬───────┘
      │
      ▼
┌─────────────┐
│   字节码编译器 │  Compiler
│  (Compiler)  │  AST → Bytecode
└─────┬───────┘
      │
      ▼
┌─────────────┐
│   虚拟机     │  VM
│  (VM)        │  执行 Bytecode
└─────────────┘
```

## Crate 结构

编译器分为多个 crate，每个负责一个阶段：

| Crate | 职责 | 输入 | 输出 |
|-------|------|------|------|
| `mailang-lexer` | 词法分析 | 源代码字符串 | `Vec<Token>` |
| `mailang-ast` | AST 定义 | - | 数据结构 |
| `mailang-parser` | 语法分析 | `Vec<Token>` | `Program` (AST) |
| `mailang-analyzer` | 语义分析 | `Program` | 类型检查结果 |
| `mailang-bytecode` | 字节码定义 | - | 数据结构 |
| `mailang-compiler` | 字节码编译 | `Program` | `Bytecode` |
| `mailang-vm` | 虚拟机执行 | `Bytecode` | `Value` |
| `mailang-stdlib` | 标准库 | - | 内置函数 |
| `mailang-core` | 集成 | - | 统一接口 |

## 词法分析 (Lexer)

### Token 类型

```rust
pub enum Token {
    // 字面量
    Integer(i64),      // 42
    Float(f64),        // 3.14
    String(String),    // "hello"
    Char(char),        // 'a'
    Bool(bool),        // true/false
    Null,              // null

    // 标识符和关键字
    Identifier(String), // myVar
    Let, Var, Const,
    Fn, Class, Trait,
    If, Elif, Else,
    For, While, In,
    Return, Break, Continue,
    Match, Import, Module,
    // ...

    // 运算符
    Plus, Minus, Star, Slash,  // + - * /
    Equal, NotEqual,           // == !=
    Less, Greater,             // < >
    And, Or, Not,              // && || !
    // ...

    // 分隔符
    LeftParen, RightParen,    // ( )
    LeftBrace, RightBrace,    // { }
    LeftBracket, RightBracket, // [ ]
    Comma, Dot, Colon, Semicolon,
    Arrow, FatArrow,          // -> =>
    // ...
}
```

### Unicode 支持

MaìLang 使用 `unicode-xid` crate 支持 Unicode 标识符：

```rust
fn read_identifier(&mut self) -> String {
    let mut ident = String::new();
    while let Some(ch) = self.peek() {
        if ch.is_xid_continue() || ch == '_' {
            ident.push(ch);
            self.advance();
        } else {
            break;
        }
    }
    ident
}
```

支持的标识符示例：
- `中文变量`
- `مرحبا`
- `🔥`
- `my_var_123`

## 语法分析 (Parser)

### 递归下降 + Pratt 解析

MaìLang 使用递归下降解析器，表达式部分使用 Pratt 解析：

```rust
// 语句解析（递归下降）
fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
    match self.peek() {
        Token::Let => self.parse_let_statement(),
        Token::Fn => self.parse_function_definition(),
        Token::If => self.parse_if_statement(),
        Token::For => self.parse_for_statement(),
        _ => self.parse_expression_statement(),
    }
}

// 表达式解析（Pratt）
fn parse_expression(&mut self) -> Result<Expr, ParseError> {
    self.parse_range()
}

fn parse_range(&mut self) -> Result<Expr, ParseError> {
    let start = self.parse_assignment()?;
    if self.peek() == &Token::DotDot {
        self.advance();
        let end = self.parse_assignment()?;
        Ok(Expr::Range { start, end })
    } else {
        Ok(start)
    }
}

// 运算符优先级（从低到高）
// 1. 赋值 (=, +=, -=, ...)
// 2. 范围 (..)
// 3. 逻辑或 (||)
// 4. 逻辑与 (&&)
// 5. 按位或 (|)
// 6. 按位异或 (^)
// 7. 按位与 (&)
// 8. 相等 (==, !=)
// 9. 比较 (<, >, <=, >=)
// 10. 移位 (<<, >>)
// 11. 加减 (+, -)
// 12. 乘除 (*, /, %)
// 13. 幂 (**)
// 14. 一元 (-, !, ~)
// 15. 调用 (.func(), .prop, [index])
```

### AST 节点

```rust
// 表达式节点
pub enum Expr {
    Literal(Literal),          // 42, "hello", true
    Identifier(String),        // myVar
    BinaryOp { op, left, right }, // a + b
    UnaryOp { op, operand },   // -a, !a
    Call { callee, args },     // func(a, b)
    MethodCall { object, method, args }, // obj.method()
    PropertyAccess { object, property }, // obj.prop
    Index { object, index },   // arr[0]
    Array(Vec<Expr>),          // [1, 2, 3]
    Map(Vec<(Expr, Expr)>),    // {"a": 1}
    Range { start, end },      // 0..10
    Lambda { params, body },   // fn(x) -> x * 1
    If { condition, then_branch, else_branch },
    Match { scrutinee, arms },
    Block(Vec<Stmt>),
    Assign { target, value },
    // ...
}

// 语句节点
pub enum Stmt {
    Let { name, mutable, type_annotation, value },
    Const { name, type_annotation, value },
    FunctionDef { name, params, return_type, body },
    ClassDef { name, superclass, traits, members },
    TraitDef { name, methods },
    If { condition, then_branch, elif_branches, else_branch },
    For { variable, iterable, body },
    While { condition, body },
    Return(Option<Expr>),
    Break,
    Continue,
    Expression(Expr),
    // ...
}
```

## 字节码编译 (Compiler)

### 字节码格式

```rust
pub struct Bytecode {
    pub chunks: Vec<Chunk>,  // 函数代码块
    pub main_chunk: usize,   // 主函数索引
}

pub struct Chunk {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub name: String,
}

pub struct Instruction {
    pub opcode: Opcode,
    pub operand: Option<u32>,
    pub line: usize,
}
```

### 操作码 (Opcodes)

```rust
pub enum Opcode {
    // 栈操作
    Push,       // 压入常量
    Pop,        // 弹出栈顶
    Dup,        // 复制栈顶

    // 变量操作
    LoadLocal,  // 加载局部变量
    StoreLocal, // 存储局部变量
    LoadGlobal, // 加载全局变量
    StoreGlobal,// 存储全局变量

    // 算术运算
    Add, Sub, Mul, Div, Mod, Pow, Neg,

    // 比较运算
    Eq, Ne, Lt, Le, Gt, Ge,

    // 逻辑运算
    And, Or, Not,

    // 控制流
    Jump,       // 无条件跳转
    JumpIfFalse,// 条件跳转（假）
    JumpIfTrue, // 条件跳转（真）
    Call,       // 函数调用
    Return,     // 函数返回

    // 对象操作
    GetProperty,// 获取属性
    SetProperty,// 设置属性
    Invoke,     // 方法调用

    // 集合操作
    BuildArray, // 构建数组
    BuildMap,   // 构建字典
    IndexGet,   // 索引读取
    IndexSet,   // 索引写入

    // 特殊
    Halt,       // 停止执行
}
```

### 编译示例

源代码：
```
let x = 1 + 2
println(x)
```

编译后的字节码：
```
Chunk "main":
  0: Push 1        // 压入常量 1
  1: Push 2        // 压入常量 2
  2: Add           // 弹出 2 和 1，压入 3
  3: StoreLocal 0  // 存储到局部变量 0 (x)
  4: LoadGlobal "println"  // 加载 println
  5: LoadLocal 0   // 加载 x
  6: Call 1        // 调用 println(1 个参数)
  7: Pop           // 弹出返回值
  8: Halt          // 停止
```

### 函数编译

每个函数编译为独立的 Chunk：

```
fn add(a, b) {
    return a + b
}

Chunk "add":
  0: LoadLocal 0   // 加载参数 a
  1: LoadLocal 1   // 加载参数 b
  2: Add           // a + b
  3: Return         // 返回结果
```

## 虚拟机 (VM)

### 寄存器式 VM

MaìLang 使用基于寄存器的虚拟机（实际上是基于栈的，但通过局部变量实现寄存器语义）：

```rust
pub struct Vm {
    bytecode: Bytecode,
    stack: Vec<Value>,           // 操作数栈
    globals: HashMap<String, Value>, // 全局变量
    call_stack: Vec<CallFrame>,  // 调用栈
    ip: usize,                   // 指令指针
    chunk_index: usize,          // 当前 Chunk
}

struct CallFrame {
    chunk_index: usize,  // 调用者的 Chunk
    ip: usize,           // 调用者的返回地址
    stack_base: usize,   // 局部变量基址
}
```

### 执行循环

```rust
loop {
    let instruction = self.current_instruction();
    self.ip += 1;

    match instruction.opcode {
        Opcode::Push => {
            let value = self.get_constant(instruction.operand);
            self.push(value);
        }
        Opcode::Add => {
            let right = self.pop();
            let left = self.pop();
            self.push(self.add_values(left, right)?);
        }
        Opcode::Call => {
            let arg_count = instruction.operand;
            let func = self.peek_n(arg_count + 1);
            // 设置调用帧，跳转到函数
        }
        Opcode::Halt => {
            return self.pop();
        }
        // ...
    }
}
```

### 值类型

```rust
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Char(char),
    Array(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Function { name, arity, chunk_index },
    Closure { function_index, upvalues },
    Class { name, methods },
    Instance { class_index, fields },
    Ok(Box<Value>),
    Err(Box<Value>),
    Some(Box<Value>),
    Builtin { name, arity },
}
```

## 优化技术

### 常量折叠

编译时计算常量表达式：

```
// 源代码
const X = 1 + 2 * 3

// 编译后
Push 7  // 直接编译为 7
```

### 局部变量优化

局部变量通过栈索引访问，避免哈希查找：

```
// 局部变量 - O(1) 栈索引访问
LoadLocal 0

// 全局变量 - 哈希表查找
LoadGlobal "x"
```

### 函数内联（计划中）

小型函数可内联展开，减少调用开销。

## 下一步

- [解析器详解](/reference/parser) - 语法分析细节
- [类型系统](/reference/types) - 类型定义
- [字节码参考](/reference/bytecode) - 完整操作码列表
