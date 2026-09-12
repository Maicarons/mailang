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
}

impl Parser {
    pub fn new(source: &str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().map_err(|e| ParseError::UnexpectedToken {
            expected: "valid token".to_string(),
            found: e.to_string(),
        })?;
        Ok(Self { tokens, pos: 0 })
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    #[allow(dead_code)]
    fn peek_at(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
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

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while self.peek() != &Token::Eof {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        Ok(Program { statements })
    }
}
