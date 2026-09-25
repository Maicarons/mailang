# Parser Internals

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [Parser Source](https://github.com/Maicarons/mailang/tree/master/crates/mailang-parser)

## Overview

MaìLang's parser uses **Recursive Descent** for statements and **Pratt Parsing** for expressions.

## Parsing Strategies

### Recursive Descent

Top-down parsing where each grammar rule maps to a function:

```rust
fn parse_statement() -> Stmt {
    match peek() {
        Let => parse_let_statement(),
        If => parse_if_statement(),
        For => parse_for_statement(),
        _ => parse_expression_statement(),
    }
}
```

### Pratt Parsing

Handles operator precedence and associativity for expressions:

```rust
fn parse_expression_bp(min_bp: u8) -> Expr {
    let mut lhs = parse_prefix()?;

    loop {
        let (l_bp, r_bp) = infix_binding_power(peek());
        if l_bp < min_bp { break; }

        advance();
        let rhs = parse_expression_bp(r_bp)?;
        lhs = BinaryOp { op, left: lhs, right: rhs };
    }

    Ok(lhs)
}
```

## Grammar (EBNF)

```ebnf
Program = Statement*

Statement = LetStmt | IfStmt | ForStmt | WhileStmt | ReturnStmt | ExprStmt

LetStmt = "let" ["var"] Identifier [":" Type] ["=" Expression]
IfStmt = "if" Expression Block {"elif" Expression Block} ["else" Block]
ForStmt = "for" Identifier "in" Expression Block
WhileStmt = "while" Expression Block

Expression = Assignment
Assignment = Range (("=" | "+=" | ...) Assignment)?
Range = Or (".." Or)?
Or = And ("||" And)*
And = BitOr ("&&" BitOr)*
BitOr = BitXor ("|" BitXor)*
BitXor = BitAnd ("^" BitAnd)*
BitAnd = Equality ("&" Equality)*
Equality = Comparison (("==" | "!=") Comparison)*
Comparison = Shift (("<" | ">" | "<=" | ">=") Shift)*
Shift = Additive (("<<" | ">>") Additive)*
Additive = Multiplicative (("+" | "-") Multiplicative)*
Multiplicative = Power (("*" | "/" | "%") Power)*
Power = Unary ("**" Power)?
Unary = ("-" | "!" | "~") Unary | Call
Call = Primary ("(" Args ")" | "." Identifier | "[" Expression "]")*
Primary = Literal | Identifier | "(" Expression ")" | Array | Map | Lambda
```

## Operator Precedence

| Prec | Operators | Assoc |
|------|-----------|-------|
| 1 | `=` `+=` `-=` | Right |
| 2 | `..` | Left |
| 3 | `\|\|` | Left |
| 4 | `&&` | Left |
| 5-7 | `\|` `^` `&` | Left |
| 8 | `==` `!=` | Left |
| 9 | `<` `>` `<=` `>=` | Left |
| 10 | `<<` `>>` | Left |
| 11 | `+` `-` | Left |
| 12 | `*` `/` `%` | Left |
| 13 | `**` | Right |
| 14 | `-` `!` `~` | Right |

## Error Recovery (implemented)

The parser supports token-sync recovery: `parse_program_recovering` collects **multiple** syntax errors and returns a partial AST.

```rust
// Recover and continue (multi-diagnostic)
let (program, errors) = parser.parse_program_recovering();

// Skip to a statement boundary (newline / statement keyword / EOF)
parser.recover_from_error();
```

Sync points: newline at bracket-depth 0, statement-start keywords (`let`/`fn`/`class`/…), or after a nested `}` block closes. `eval`/`run` still use fail-fast `parse_program`; LSP uses recovery so one edit can report many errors.

## Next Steps

- [Compiler Internals](/v0.3/en/reference/compiler) - Compilation pipeline
- [Type System](/v0.3/en/reference/types) - Type definitions
