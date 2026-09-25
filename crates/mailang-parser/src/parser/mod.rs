//! MaìLang parser module
//!
//! This module provides the recursive descent + Pratt parser for MaìLang.

mod expression;
mod literal;
mod statement;

use crate::error::ParseError;
use mailang_ast::*;
use mailang_lexer::{Lexer, Token};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Split token leftover when `>>` is consumed as two type-closers.
    pending: Option<Token>,
    /// Currently bound generic type parameter names (fn/class scopes).
    type_params: Vec<String>,
}

impl Parser {
    pub fn new(source: &str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().map_err(|e| ParseError::UnexpectedToken {
            expected: "valid token".to_string(),
            found: e.to_string(),
        })?;
        Ok(Self {
            tokens,
            pos: 0,
            pending: None,
            type_params: Vec::new(),
        })
    }

    fn peek(&self) -> &Token {
        if let Some(ref t) = self.pending {
            return t;
        }
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    #[allow(dead_code)]
    fn peek_at(&self, offset: usize) -> &Token {
        if let Some(ref t) = self.pending {
            return if offset == 0 {
                t
            } else {
                self.tokens
                    .get(self.pos + offset - 1)
                    .unwrap_or(&Token::Eof)
            };
        }
        self.tokens.get(self.pos + offset).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        if let Some(t) = self.pending.take() {
            return t;
        }
        let token = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        token
    }

    fn expect(&mut self, expected: &Token) -> Result<Token, ParseError> {
        let token = self.advance();
        if &token == expected {
            Ok(token)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", expected),
                found: format!("{:?}", token),
            })
        }
    }

    /// Consume a type-closing `>`. Splits `>>` into two `>` for nested generics.
    fn expect_greater(&mut self) -> Result<Token, ParseError> {
        match self.advance() {
            Token::Greater => Ok(Token::Greater),
            Token::GreaterGreater => {
                self.pending = Some(Token::Greater);
                Ok(Token::Greater)
            }
            other => Err(ParseError::UnexpectedToken {
                expected: "Greater".to_string(),
                found: format!("{:?}", other),
            }),
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Token::Identifier(name) => Ok(name),
            token => Err(ParseError::ExpectedIdentifier(format!("{:?}", token))),
        }
    }

    fn skip_newlines(&mut self) {
        while self.peek() == &Token::Newline {
            self.advance();
        }
    }

    /// True when `name` is a bound type parameter in the current fn/class scope.
    fn is_type_param(&self, name: &str) -> bool {
        self.type_params.iter().any(|p| p == name)
    }

    /// Parse `<T, U, ...>` after a fn/class name. Empty when absent.
    fn parse_type_parameter_list(&mut self) -> Result<Vec<String>, ParseError> {
        if self.peek() != &Token::Less {
            return Ok(Vec::new());
        }
        self.advance();
        let mut params = Vec::new();
        if self.peek() != &Token::Greater {
            params.push(self.expect_identifier()?);
            while self.peek() == &Token::Comma {
                self.advance();
                if self.peek() == &Token::Greater {
                    break;
                }
                params.push(self.expect_identifier()?);
            }
        }
        self.expect_greater()?;
        Ok(params)
    }

    /// Parse a comma-separated type-argument list (already past the opening `<`).
    fn parse_type_argument_list(&mut self) -> Result<Vec<TypeAnnotation>, ParseError> {
        let mut args = Vec::new();
        args.push(self.parse_type_annotation()?);
        while self.peek() == &Token::Comma {
            self.advance();
            if self.peek() == &Token::Greater {
                break;
            }
            args.push(self.parse_type_annotation()?);
        }
        Ok(args)
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while self.peek() != &Token::Eof {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        Ok(Program { statements })
    }

    /// Skip tokens until a statement boundary so parsing can resume after an error.
    ///
    /// Sync points: newline at brace-depth 0, statement keywords, or EOF.
    /// Bracket/brace nesting is tracked so we do not stop mid-block.
    pub fn recover_from_error(&mut self) {
        let mut brace = 0i32;
        let mut paren = 0i32;
        let mut bracket = 0i32;
        // Drop any half-consumed `>>` split leftover.
        self.pending = None;
        // Always make progress: skip the offending token first so we cannot loop.
        if self.peek() != &Token::Eof {
            self.advance();
        }

        loop {
            match self.peek().clone() {
                Token::Eof => return,
                Token::Newline if brace == 0 && paren == 0 && bracket == 0 => {
                    self.advance();
                    self.skip_newlines();
                    return;
                }
                Token::LeftBrace => {
                    brace += 1;
                    self.advance();
                }
                Token::RightBrace => {
                    if brace == 0 && paren == 0 && bracket == 0 {
                        // Closing the enclosing block — leave `}` for the caller.
                        return;
                    }
                    brace -= 1;
                    self.advance();
                    if brace == 0 && paren == 0 && bracket == 0 {
                        // Finished a nested block; next newline or keyword is a boundary.
                        self.skip_newlines();
                        return;
                    }
                }
                Token::LeftParen => {
                    paren += 1;
                    self.advance();
                }
                Token::RightParen => {
                    if paren > 0 {
                        paren -= 1;
                    }
                    self.advance();
                }
                Token::LeftBracket => {
                    bracket += 1;
                    self.advance();
                }
                Token::RightBracket => {
                    if bracket > 0 {
                        bracket -= 1;
                    }
                    self.advance();
                }
                Token::Let
                | Token::Var
                | Token::Const
                | Token::Fn
                | Token::Class
                | Token::Trait
                | Token::If
                | Token::While
                | Token::For
                | Token::Return
                | Token::Import
                | Token::Module
                    if brace == 0 && paren == 0 && bracket == 0 =>
                {
                    // Statement-start keyword at top level — stop before it.
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Parse a program collecting multiple syntax errors via token-sync recovery.
    ///
    /// Always returns a (possibly partial) `Program` plus every error encountered.
    pub fn parse_program_recovering(&mut self) -> (Program, Vec<ParseError>) {
        let mut statements = Vec::new();
        let mut errors = Vec::new();
        self.skip_newlines();

        while self.peek() != &Token::Eof {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    errors.push(err);
                    self.recover_from_error();
                }
            }
            self.skip_newlines();
        }

        (Program { statements }, errors)
    }
}
