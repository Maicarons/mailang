//! Expression parsing for MaìLang (Pratt parsing)

use super::Parser;
use crate::error::ParseError;
use mailang_ast::*;
use mailang_lexer::Token;

impl Parser {
    pub(crate) fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_range()
    }

    fn parse_range(&mut self) -> Result<Expr, ParseError> {
        let start = self.parse_assignment()?;

        if self.peek() == &Token::DotDot {
            self.advance();
            let end = self.parse_assignment()?;
            Ok(Expr::Range {
                start: Box::new(start),
                end: Box::new(end),
            })
        } else {
            Ok(start)
        }
    }

    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_or()?;

        match self.peek() {
            Token::Assign => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::Assign {
                    target: Box::new(left),
                    value: Box::new(value),
                })
            }
            Token::PlusAssign => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::CompoundAssign {
                    op: BinaryOp::Add,
                    target: Box::new(left),
                    value: Box::new(value),
                })
            }
            Token::MinusAssign => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::CompoundAssign {
                    op: BinaryOp::Sub,
                    target: Box::new(left),
                    value: Box::new(value),
                })
            }
            Token::StarAssign => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::CompoundAssign {
                    op: BinaryOp::Mul,
                    target: Box::new(left),
                    value: Box::new(value),
                })
            }
            Token::SlashAssign => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::CompoundAssign {
                    op: BinaryOp::Div,
                    target: Box::new(left),
                    value: Box::new(value),
                })
            }
            Token::PercentAssign => {
                self.advance();
                let value = self.parse_assignment()?;
                Ok(Expr::CompoundAssign {
                    op: BinaryOp::Mod,
                    target: Box::new(left),
                    value: Box::new(value),
                })
            }
            _ => Ok(left),
        }
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;

        while self.peek() == &Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_bitwise_or()?;

        while self.peek() == &Token::And {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = Expr::BinaryOp {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_bitwise_xor()?;

        while self.peek() == &Token::Pipe {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = Expr::BinaryOp {
                op: BinaryOp::BitOr,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_bitwise_and()?;

        while self.peek() == &Token::Caret {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expr::BinaryOp {
                op: BinaryOp::BitXor,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;

        while self.peek() == &Token::Ampersand {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryOp {
                op: BinaryOp::BitAnd,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match self.peek() {
                Token::Equal => BinaryOp::Eq,
                Token::NotEqual => BinaryOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_shift()?;

        loop {
            let op = match self.peek() {
                Token::Less => BinaryOp::Lt,
                Token::LessEqual => BinaryOp::Le,
                Token::Greater => BinaryOp::Gt,
                Token::GreaterEqual => BinaryOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_shift()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.peek() {
                Token::LessLess => BinaryOp::Shl,
                Token::GreaterGreater => BinaryOp::Shr,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.peek() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_power()?;

        loop {
            let op = match self.peek() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, ParseError> {
        let base = self.parse_unary()?;

        if self.peek() == &Token::StarStar {
            self.advance();
            let exp = self.parse_power()?;
            Ok(Expr::BinaryOp {
                op: BinaryOp::Pow,
                left: Box::new(base),
                right: Box::new(exp),
            })
        } else {
            Ok(base)
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Token::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                })
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                })
            }
            Token::Tilde => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::BitNot,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Token::LeftParen => {
                    self.advance();
                    let args = self.parse_argument_list()?;
                    self.expect(&Token::RightParen)?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }
                Token::Dot => {
                    self.advance();
                    let property = self.expect_identifier()?;
                    if self.peek() == &Token::LeftParen {
                        self.advance();
                        let args = self.parse_argument_list()?;
                        self.expect(&Token::RightParen)?;
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: property,
                            args,
                        };
                    } else {
                        expr = Expr::PropertyAccess {
                            object: Box::new(expr),
                            property,
                        };
                    }
                }
                Token::LeftBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(&Token::RightBracket)?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    pub(crate) fn parse_argument_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();

        if self.peek() == &Token::RightParen {
            return Ok(args);
        }

        args.push(self.parse_expression()?);

        while self.peek() == &Token::Comma {
            self.advance();
            if self.peek() == &Token::RightParen {
                break;
            }
            args.push(self.parse_expression()?);
        }

        Ok(args)
    }
}
