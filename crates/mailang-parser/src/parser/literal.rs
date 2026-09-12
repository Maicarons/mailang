//! Literal parsing for MaìLang (arrays, maps, lambdas, if/match expressions)

use super::Parser;
use crate::error::ParseError;
use mailang_ast::*;
use mailang_lexer::Token;

fn pattern_to_expr(p: &Pattern) -> Result<Expr, ParseError> {
    match p {
        Pattern::Literal(lit) => Ok(Expr::Literal(lit.clone())),
        Pattern::Identifier(name) => Ok(Expr::Identifier(name.clone())),
        _ => Err(ParseError::ExpectedExpression(
            "Cannot convert pattern to expression for range".to_string(),
        )),
    }
}

impl Parser {
    pub(crate) fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Token::Integer(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::Literal(Literal::Int(n)))
            }
            Token::Float(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::Literal(Literal::Float(n)))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                // Check for string interpolation: {expression}
                if s.contains('{') && s.contains('}') {
                    let parts = self.parse_string_interpolation(&s)?;
                    Ok(Expr::StringInterpolation(parts))
                } else {
                    Ok(Expr::Literal(Literal::Str(s)))
                }
            }
            Token::Char(c) => {
                let c = *c;
                self.advance();
                Ok(Expr::Literal(Literal::Char(c)))
            }
            Token::Bool(b) => {
                let b = *b;
                self.advance();
                Ok(Expr::Literal(Literal::Bool(b)))
            }
            Token::Null => {
                self.advance();
                Ok(Expr::Literal(Literal::Null))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Identifier(name))
            }
            Token::This => {
                self.advance();
                Ok(Expr::Identifier("this".to_string()))
            }
            Token::Super => {
                self.advance();
                Ok(Expr::Identifier("super".to_string()))
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(expr)
            }
            Token::LeftBracket => self.parse_array_literal(),
            Token::LeftBrace => self.parse_map_literal(),
            Token::Fn => self.parse_lambda(),
            Token::If => self.parse_if_expression(),
            Token::Match => self.parse_match_expression(),
            Token::Ok => {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let value = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(Expr::Ok(Box::new(value)))
            }
            Token::Err => {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let value = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(Expr::Err(Box::new(value)))
            }
            Token::Some => {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let value = self.parse_expression()?;
                self.expect(&Token::RightParen)?;
                Ok(Expr::Some(Box::new(value)))
            }
            Token::None => {
                self.advance();
                Ok(Expr::None)
            }
            token => Err(ParseError::ExpectedExpression(format!("{:?}", token))),
        }
    }

    fn parse_array_literal(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::LeftBracket)?;
        let mut elements = Vec::new();

        if self.peek() != &Token::RightBracket {
            elements.push(self.parse_expression()?);
            while self.peek() == &Token::Comma {
                self.advance();
                if self.peek() == &Token::RightBracket {
                    break;
                }
                elements.push(self.parse_expression()?);
            }
        }

        self.expect(&Token::RightBracket)?;
        Ok(Expr::Array(elements))
    }

    fn parse_map_literal(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::LeftBrace)?;
        let mut entries = Vec::new();

        if self.peek() != &Token::RightBrace {
            let key = self.parse_expression()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expression()?;
            entries.push((key, value));

            while self.peek() == &Token::Comma {
                self.advance();
                if self.peek() == &Token::RightBrace {
                    break;
                }
                let key = self.parse_expression()?;
                self.expect(&Token::Colon)?;
                let value = self.parse_expression()?;
                entries.push((key, value));
            }
        }

        self.expect(&Token::RightBrace)?;
        Ok(Expr::Map(entries))
    }

    fn parse_lambda(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::Fn)?;
        self.expect(&Token::LeftParen)?;
        let params = self.parse_parameter_list()?;
        self.expect(&Token::RightParen)?;
        if self.peek() == &Token::Arrow {
            self.advance();
            let body = self.parse_expression()?;
            Ok(Expr::Lambda {
                params,
                body: Box::new(body),
            })
        } else if self.peek() == &Token::LeftBrace {
            self.advance();
            let stmts = self.parse_block()?;
            self.expect(&Token::RightBrace)?;
            Ok(Expr::Lambda {
                params,
                body: Box::new(Expr::Block(stmts)),
            })
        } else {
            Err(ParseError::ExpectedExpression(format!("{:?}", self.peek())))
        }
    }

    fn parse_if_expression(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::If)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::LeftBrace)?;
        let then_branch = self.parse_expression()?;
        self.expect(&Token::RightBrace)?;

        let else_branch = if self.peek() == &Token::Else {
            self.advance();
            self.expect(&Token::LeftBrace)?;
            let else_expr = self.parse_expression()?;
            self.expect(&Token::RightBrace)?;
            Some(Box::new(else_expr))
        } else {
            None
        };

        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
        })
    }

    fn parse_match_expression(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::Match)?;
        let scrutinee = self.parse_expression()?;
        self.expect(&Token::LeftBrace)?;
        let mut arms = Vec::new();
        self.skip_newlines();

        while self.peek() != &Token::RightBrace {
            let pattern = self.parse_pattern()?;
            let guard = if self.peek() == &Token::If {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(&Token::FatArrow)?;
            let body = self.parse_expression()?;
            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });
            self.skip_newlines();
            // Optional comma between arms: `Ok(v) => x, Err(e) => y`
            if self.peek() == &Token::Comma {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(&Token::RightBrace)?;
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let first = self.parse_pattern_atom()?;

        // Check for range pattern: start..end or start..=end
        if self.peek() == &Token::DotDot {
            self.advance();
            let inclusive = if self.peek() == &Token::Assign {
                self.advance();
                true
            } else {
                false
            };
            let end = self.parse_pattern_atom()?;
            // Convert to range using expressions
            let start_expr = pattern_to_expr(&first)?;
            let end_expr = pattern_to_expr(&end)?;
            return Ok(Pattern::Range(
                Box::new(start_expr),
                Box::new(end_expr),
                inclusive,
            ));
        }

        // Check for or-pattern: a | b | c
        if self.peek() == &Token::Pipe {
            let mut patterns = vec![first];
            while self.peek() == &Token::Pipe {
                self.advance();
                patterns.push(self.parse_pattern_atom()?);
            }
            return Ok(Pattern::Or(patterns));
        }

        Ok(first)
    }

    fn parse_pattern_atom(&mut self) -> Result<Pattern, ParseError> {
        match self.peek() {
            Token::Integer(n) => {
                let n = *n;
                self.advance();
                Ok(Pattern::Literal(Literal::Int(n)))
            }
            Token::Float(n) => {
                let n = *n;
                self.advance();
                Ok(Pattern::Literal(Literal::Float(n)))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::Str(s)))
            }
            Token::Char(c) => {
                let c = *c;
                self.advance();
                Ok(Pattern::Literal(Literal::Char(c)))
            }
            Token::Bool(b) => {
                let b = *b;
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(b)))
            }
            Token::Null => {
                self.advance();
                Ok(Pattern::Literal(Literal::Null))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                if name == "_" {
                    Ok(Pattern::Wildcard)
                } else {
                    Ok(Pattern::Identifier(name))
                }
            }
            Token::Ok => {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let inner = self.parse_pattern()?;
                self.expect(&Token::RightParen)?;
                Ok(Pattern::Ok(Box::new(inner)))
            }
            Token::Err => {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let inner = self.parse_pattern()?;
                self.expect(&Token::RightParen)?;
                Ok(Pattern::Err(Box::new(inner)))
            }
            Token::Some => {
                self.advance();
                self.expect(&Token::LeftParen)?;
                let inner = self.parse_pattern()?;
                self.expect(&Token::RightParen)?;
                Ok(Pattern::Some(Box::new(inner)))
            }
            Token::None => {
                self.advance();
                Ok(Pattern::Literal(Literal::Null))
            }
            Token::LeftParen => {
                self.advance();
                let mut patterns = Vec::new();
                if self.peek() != &Token::RightParen {
                    patterns.push(self.parse_pattern()?);
                    while self.peek() == &Token::Comma {
                        self.advance();
                        patterns.push(self.parse_pattern()?);
                    }
                }
                self.expect(&Token::RightParen)?;
                Ok(Pattern::Tuple(patterns))
            }
            Token::LeftBracket => {
                self.advance();
                let mut patterns = Vec::new();
                if self.peek() != &Token::RightBracket {
                    patterns.push(self.parse_pattern()?);
                    while self.peek() == &Token::Comma {
                        self.advance();
                        patterns.push(self.parse_pattern()?);
                    }
                }
                self.expect(&Token::RightBracket)?;
                Ok(Pattern::Array(patterns))
            }
            token => Err(ParseError::ExpectedPattern(format!("{:?}", token))),
        }
    }

    pub(crate) fn parse_string_interpolation(
        &self,
        s: &str,
    ) -> Result<Vec<StringPart>, ParseError> {
        let mut parts = Vec::new();
        let mut current_text = String::new();
        let chars = s.chars();
        let mut depth = 0;
        let mut expr_text = String::new();

        for ch in chars {
            if ch == '{' && depth == 0 {
                // Start of expression
                if !current_text.is_empty() {
                    parts.push(StringPart::Text(current_text.clone()));
                    current_text.clear();
                }
                depth = 1;
                expr_text.clear();
            } else if ch == '{' && depth > 0 {
                depth += 1;
                expr_text.push(ch);
            } else if ch == '}' && depth > 0 {
                depth -= 1;
                if depth == 0 {
                    // End of expression - parse it
                    let mut expr_parser = Parser::new(&expr_text).map_err(|_| {
                        ParseError::ExpectedExpression(format!("Failed to parse: {}", expr_text))
                    })?;
                    let expr = expr_parser.parse_expression().map_err(|_| {
                        ParseError::ExpectedExpression(format!("Invalid expression: {}", expr_text))
                    })?;
                    parts.push(StringPart::Expr(expr));
                } else {
                    expr_text.push(ch);
                }
            } else if depth > 0 {
                expr_text.push(ch);
            } else {
                current_text.push(ch);
            }
        }

        if !current_text.is_empty() {
            parts.push(StringPart::Text(current_text));
        }

        if parts.is_empty() {
            parts.push(StringPart::Text(String::new()));
        }

        Ok(parts)
    }
}
