# 解析器详解

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [解析器源码](https://github.com/Maicarons/mailang/tree/master/crates/mailang-parser) · [v0.1.0-parser](https://github.com/Maicarons/mailang/releases/tag/v0.1.0-parser)

## 概述

MaìLang 的解析器采用**递归下降**（Recursive Descent）方法，表达式部分使用 **Pratt 解析**（也称为 Top-Down Operator Precedence）。本文档详细解析器的工作原理。

## 解析策略

### 递归下降

递归下降是一种自顶向下的语法分析方法，每个语法规则对应一个解析函数：

```
Program → Statement*
Statement → LetStatement | IfStatement | ForStatement | ...
Expression → Assignment | Range | Or | And | ...
```

```rust
// 伪代码示例
fn parse_statement() -> Stmt {
    match peek() {
        Let => parse_let_statement(),
        If => parse_if_statement(),
        For => parse_for_statement(),
        _ => parse_expression_statement(),
    }
}
```

### Pratt 解析

Pratt 解析用于处理表达式的优先级和结合性。每个运算符有两个属性：
- **绑定力 (Binding Power)**: 运算符优先级，数值越大优先级越高
- **结合性**: 左结合或右结合

```rust
// 运算符优先级表
const PRECEDENCE: &[(&Token, u8)] = &[
    (Token::Assign, 1),      // =
    (Token::PlusAssign, 1),  // +=
    (Token::Or, 2),          // ||
    (Token::And, 3),         // &&
    (Token::Pipe, 4),        // |
    (Token::Caret, 5),       // ^
    (Token::Ampersand, 6),   // &
    (Token::Equal, 7),       // ==
    (Token::Less, 8),        // <
    (Token::LessLess, 9),    // <<
    (Token::Plus, 10),       // +
    (Token::Star, 11),       // *
    (Token::StarStar, 12),   // **
];
```

## 语法规则

### 完整语法（EBNF）

```ebnf
Program = Statement*

Statement = LetStatement
          | VarStatement
          | ConstStatement
          | FunctionDef
          | ClassDef
          | TraitDef
          | ModuleDef
          | ImportStatement
          | IfStatement
          | ForStatement
          | WhileStatement
          | ReturnStatement
          | BreakStatement
          | ContinueStatement
          | ExpressionStatement

LetStatement = "let" ["var"] Identifier [":" Type] ["=" Expression]
VarStatement = "var" Identifier [":" Type] ["=" Expression]
ConstStatement = "const" Identifier [":" Type] "=" Expression

FunctionDef = "fn" Identifier "(" ParameterList ")" ["->" Type] Block
ClassDef = "class" Identifier ["extends" Identifier] ["implements" IdentifierList] "{" ClassMember* "}"
TraitDef = "trait" Identifier "{" TraitMethod* "}"

IfStatement = "if" Expression Block {"elif" Expression Block} ["else" Block]
ForStatement = "for" Identifier "in" Expression Block
WhileStatement = "while" Expression Block

Expression = Assignment
Assignment = Range (("=" | "+=" | "-=" | "*=" | "/=" | "%=") Assignment)?
Range = Or (".." Or)?
Or = And ("||" And)*
And = BitwiseOr ("&&" BitwiseOr)*
BitwiseOr = BitwiseXor ("|" BitwiseXor)*
BitwiseXor = BitwiseAnd ("^" BitwiseAnd)*
BitwiseAnd = Equality ("&" Equality)*
Equality = Comparison (("==" | "!=") Comparison)*
Comparison = Shift (("<" | ">" | "<=" | ">=") Shift)*
Shift = Additive (("<<" | ">>") Additive)*
Additive = Multiplicative (("+" | "-") Multiplicative)*
Multiplicative = Power (("*" | "/" | "%") Power)*
Power = Unary ("**" Power)?  // 右结合
Unary = ("-" | "!" | "~") Unary | Call
Call = Primary ("(" ArgumentList ")" | "." Identifier | "[" Expression "]")*
Primary = Integer | Float | String | Char | Bool | Null
        | Identifier | "this" | "super"
        | "(" Expression ")"
        | ArrayLiteral | MapLiteral
        | Lambda | IfExpression | MatchExpression
        | "Ok" "(" Expression ")"
        | "Err" "(" Expression ")"
        | "Some" "(" Expression ")"
        | "None"
```

## 解析流程

### 1. 程序解析

```rust
fn parse_program(&mut self) -> Result<Program, ParseError> {
    let mut statements = Vec::new();
    self.skip_newlines();

    while self.peek() != &Token::Eof {
        statements.push(self.parse_statement()?);
        self.skip_newlines();
    }

    Ok(Program { statements })
}
```

### 2. 语句解析

#### let 语句

```
源代码: let x: int = 42

解析流程:
1. 匹配 "let" 关键字
2. 检查是否有 "var"（可变标记）
3. 解析标识符 "x"
4. 检查是否有 ":"（类型标注）
5. 解析类型 "int"
6. 检查是否有 "="（初始值）
7. 解析表达式 "42"
```

```rust
fn parse_let_statement(&mut self) -> Result<Stmt, ParseError> {
    self.expect(&Token::Let)?;
    let mutable = if self.peek() == &Token::Var {
        self.advance();
        true
    } else {
        false
    };
    let name = self.expect_identifier()?;
    let type_annotation = if self.peek() == &Token::Colon {
        self.advance();
        Some(self.parse_type_annotation()?)
    } else {
        None
    };
    let value = if self.peek() == &Token::Assign {
        self.advance();
        Some(self.parse_expression()?)
    } else {
        None
    };
    Ok(Stmt::Let { name, mutable, type_annotation, value })
}
```

#### if 语句

```
源代码: if x > 10 { ... } elif x > 5 { ... } else { ... }

解析流程:
1. 匹配 "if" 关键字
2. 解析条件表达式 "x > 10"
3. 解析 then 分支块
4. 循环检查 "elif" 分支
5. 检查 "else" 分支
```

#### for 语句

```
源代码: for i in 0..10 { ... }

解析流程:
1. 匹配 "for" 关键字
2. 解析循环变量 "i"
3. 匹配 "in" 关键字
4. 解析可迭代表达式 "0..10"（范围表达式）
5. 解析循环体块
```

### 3. 表达式解析

#### Prat 解析核心

```rust
fn parse_expression_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
    // 解析前缀（一元运算符、字面量、标识符等）
    let mut lhs = self.parse_prefix()?;

    loop {
        let op = match self.peek() {
            Token::Plus => BinaryOp::Add,
            Token::Star => BinaryOp::Mul,
            // ...
            _ => break,
        };

        let (l_bp, r_bp) = self.infix_binding_power(&op);
        if l_bp < min_bp {
            break;
        }

        self.advance();
        let rhs = self.parse_expression_bp(r_bp)?;
        lhs = Expr::BinaryOp {
            op,
            left: Box::new(lhs),
            right: Box::new(rhs),
        };
    }

    Ok(lhs)
}
```

#### 函数调用

```
源代码: add(1, 2 + 3)

解析流程:
1. 解析主表达式 "add" → Identifier("add")
2. 匹配 "("
3. 解析参数列表:
   a. 解析表达式 "1" → Literal(1)
   b. 匹配 ","
   c. 解析表达式 "2 + 3" → BinaryOp(Add, 2, 3)
4. 匹配 ")"
5. 构建 Call 节点
```

#### 属性访问

```
源代码: obj.method(arg)

解析流程:
1. 解析主表达式 "obj" → Identifier("obj")
2. 匹配 "."
3. 解析标识符 "method"
4. 匹配 "("
5. 解析参数 "arg"
6. 匹配 ")"
7. 构建 MethodCall 节点
```

### 4. 模式解析

```
源代码: match x { 0 => "zero", _ => "other" }

解析流程:
1. 匹配 "match" 关键字
2. 解析匹配表达式 "x"
3. 匹配 "{"
4. 循环解析匹配分支:
   a. 解析模式 "0" → Literal(0)
   b. 匹配 "=>"
   c. 解析分支体 "zero" → Literal("zero")
5. 匹配 "}"
```

## 错误处理

### 错误类型

```rust
pub enum ParseError {
    UnexpectedToken { expected, found },
    ExpectedIdentifier(String),
    ExpectedExpression(String),
    ExpectedPattern(String),
    ExpectedClassMember(String),
    // ...
}
```

### 错误恢复

解析器在遇到错误时尝试恢复：

```rust
fn recover_from_error(&mut self) {
    // 跳过当前 token，直到找到同步点
    while !self.is_at_sync_point() {
        self.advance();
    }
}

fn is_at_sync_point(&self) -> bool {
    matches!(self.peek(),
        Token::Newline | Token::Semicolon | Token::RightBrace | Token::Eof
    )
}
```

## 运算符优先级表

| 优先级 | 运算符 | 结合性 | 说明 |
|--------|--------|--------|------|
| 1 | `=` `+=` `-=` `*=` `/=` `%=` | 右 | 赋值 |
| 2 | `..` | 左 | 范围 |
| 3 | `\|\|` | 左 | 逻辑或 |
| 4 | `&&` | 左 | 逻辑与 |
| 5 | `\|` | 左 | 按位或 |
| 6 | `^` | 左 | 按位异或 |
| 7 | `&` | 左 | 按位与 |
| 8 | `==` `!=` | 左 | 相等 |
| 9 | `<` `>` `<=` `>=` | 左 | 比较 |
| 10 | `<<` `>>` | 左 | 移位 |
| 11 | `+` `-` | 左 | 加减 |
| 12 | `*` `/` `%` | 左 | 乘除模 |
| 13 | `**` | 右 | 幂 |
| 14 | `-` `!` `~` | 右 | 一元 |

## 下一步

- [编译原理](/reference/compiler) - 整体编译流程
- [类型系统](/reference/types) - 类型定义
- [错误码](/reference/errors) - 错误代码
