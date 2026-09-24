//! Statement parsing for MaìLang

use super::Parser;
use crate::error::ParseError;
use mailang_ast::*;
use mailang_lexer::Token;

impl Parser {
    pub(crate) fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        self.skip_newlines();

        match self.peek() {
            Token::Let => self.parse_let_statement(),
            Token::Var => self.parse_var_statement(),
            Token::Const => self.parse_const_statement(),
            Token::Fn => self.parse_function_definition(),
            Token::Class => self.parse_class_definition(),
            Token::Trait => self.parse_trait_definition(),
            Token::Module => self.parse_module_definition(),
            Token::Import => self.parse_import_statement(),
            Token::If => self.parse_if_statement(),
            Token::For => self.parse_for_statement(),
            Token::While => self.parse_while_statement(),
            Token::Return => self.parse_return_statement(),
            Token::Break => {
                self.advance();
                Ok(Stmt::Break)
            }
            Token::Continue => {
                self.advance();
                Ok(Stmt::Continue)
            }
            _ => self.parse_expression_statement(),
        }
    }

    pub(crate) fn parse_let_statement(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::Let)?;
        let mutable = if self.peek() == &Token::Var {
            self.advance();
            true
        } else {
            false
        };
        self.parse_binding(mutable)
    }

    fn parse_var_statement(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::Var)?;
        self.parse_binding(true)
    }

    /// Parse a let/var binding target: simple `name[: T] [= v]` or
    /// destructuring `(a, b) = v` / `[a, b] = v`.
    fn parse_binding(&mut self, mutable: bool) -> Result<Stmt, ParseError> {
        if matches!(self.peek(), Token::LeftParen | Token::LeftBracket) {
            let pattern = self.parse_pattern()?;
            self.expect(&Token::Assign)?;
            let value = self.parse_expression()?;
            return Ok(Stmt::Let {
                name: String::new(),
                mutable,
                type_annotation: None,
                value: Some(value),
                pattern: Some(pattern),
            });
        }
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
        Ok(Stmt::Let {
            name,
            mutable,
            type_annotation,
            value,
            pattern: None,
        })
    }

    fn parse_const_statement(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::Const)?;
        let name = self.expect_identifier()?;
        let type_annotation = if self.peek() == &Token::Colon {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect(&Token::Assign)?;
        let value = self.parse_expression()?;
        Ok(Stmt::Const {
            name,
            type_annotation,
            value,
        })
    }

    pub(crate) fn parse_function_definition(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::Fn)?;
        let name = self.expect_identifier()?;
        let type_params = self.parse_type_parameter_list()?;
        let saved = self.type_params.len();
        self.type_params.extend(type_params.iter().cloned());
        self.expect(&Token::LeftParen)?;
        let params = self.parse_parameter_list()?;
        self.expect(&Token::RightParen)?;
        let return_type = if self.peek() == &Token::Arrow {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect(&Token::LeftBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RightBrace)?;
        self.type_params.truncate(saved);
        Ok(Stmt::FunctionDef {
            name,
            type_params,
            params,
            return_type,
            body,
        })
    }

    pub(crate) fn parse_parameter_list(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();

        if self.peek() == &Token::RightParen {
            return Ok(params);
        }

        params.push(self.parse_parameter()?);

        while self.peek() == &Token::Comma {
            self.advance();
            if self.peek() == &Token::RightParen {
                break;
            }
            params.push(self.parse_parameter()?);
        }

        Ok(params)
    }

    fn parse_parameter(&mut self) -> Result<Param, ParseError> {
        let name = self.expect_identifier()?;
        let type_annotation = if self.peek() == &Token::Colon {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let default = if self.peek() == &Token::Assign {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Param {
            name,
            type_annotation,
            default,
        })
    }

    pub(crate) fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, ParseError> {
        match self.advance() {
            Token::Identifier(name) => match name.as_str() {
                "int" => Ok(TypeAnnotation::Int),
                "float" => Ok(TypeAnnotation::Float),
                "bool" => Ok(TypeAnnotation::Bool),
                "str" => Ok(TypeAnnotation::Str),
                "char" => Ok(TypeAnnotation::Char),
                "null" => Ok(TypeAnnotation::Option(Box::new(TypeAnnotation::Infer))),
                "Result" => {
                    // Result<T, E>
                    self.expect(&Token::Less)?;
                    let ok_type = self.parse_type_annotation()?;
                    self.expect(&Token::Comma)?;
                    let err_type = self.parse_type_annotation()?;
                    self.expect_greater()?;
                    Ok(TypeAnnotation::Result(
                        Box::new(ok_type),
                        Box::new(err_type),
                    ))
                }
                "Option" => {
                    // Option<T>
                    self.expect(&Token::Less)?;
                    let inner = self.parse_type_annotation()?;
                    self.expect_greater()?;
                    Ok(TypeAnnotation::Option(Box::new(inner)))
                }
                _ => {
                    // Type parameter in the current generic scope: `T` in `fn id<T>(x: T)`.
                    if self.is_type_param(&name) {
                        return Ok(TypeAnnotation::Param(name));
                    }
                    // Generic application: Name<T, ...>
                    if self.peek() == &Token::Less {
                        self.advance(); // consume <
                        let type_args = self.parse_type_argument_list()?;
                        self.expect_greater()?;
                        Ok(TypeAnnotation::Apply(name, type_args))
                    } else {
                        Ok(TypeAnnotation::Custom(name))
                    }
                }
            },
            Token::LeftBracket => {
                let inner = self.parse_type_annotation()?;
                self.expect(&Token::RightBracket)?;
                Ok(TypeAnnotation::Array(Box::new(inner)))
            }
            Token::LeftBrace => {
                let key = self.parse_type_annotation()?;
                self.expect(&Token::Colon)?;
                let value = self.parse_type_annotation()?;
                self.expect(&Token::RightBrace)?;
                Ok(TypeAnnotation::Map(Box::new(key), Box::new(value)))
            }
            Token::LeftParen => {
                let mut types = Vec::new();
                if self.peek() != &Token::RightParen {
                    types.push(self.parse_type_annotation()?);
                    while self.peek() == &Token::Comma {
                        self.advance();
                        types.push(self.parse_type_annotation()?);
                    }
                }
                self.expect(&Token::RightParen)?;
                Ok(TypeAnnotation::Tuple(types))
            }
            token => Err(ParseError::ExpectedTypeAnnotation(format!("{:?}", token))),
        }
    }

    fn parse_class_definition(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::Class)?;
        let name = self.expect_identifier()?;
        let type_params = self.parse_type_parameter_list()?;
        let saved = self.type_params.len();
        self.type_params.extend(type_params.iter().cloned());
        let superclass = if self.peek() == &Token::Extends {
            self.advance();
            Some(self.expect_identifier()?)
        } else {
            None
        };
        let traits = if self.peek() == &Token::Implements {
            self.advance();
            let mut traits = Vec::new();
            traits.push(self.expect_identifier()?);
            while self.peek() == &Token::Comma {
                self.advance();
                traits.push(self.expect_identifier()?);
            }
            traits
        } else {
            Vec::new()
        };
        self.expect(&Token::LeftBrace)?;
        let members = self.parse_class_members()?;
        self.expect(&Token::RightBrace)?;
        self.type_params.truncate(saved);
        Ok(Stmt::ClassDef {
            name,
            type_params,
            superclass,
            traits,
            members,
        })
    }

    fn parse_class_members(&mut self) -> Result<Vec<ClassMember>, ParseError> {
        let mut members = Vec::new();
        self.skip_newlines();

        while self.peek() != &Token::RightBrace {
            members.push(self.parse_class_member()?);
            self.skip_newlines();
        }

        Ok(members)
    }

    fn parse_class_member(&mut self) -> Result<ClassMember, ParseError> {
        match self.peek() {
            Token::Let | Token::Var => self.parse_property_member(),
            Token::Override | Token::Fn => self.parse_method_member(),
            _ => Err(ParseError::ExpectedClassMember(format!(
                "{:?}",
                self.peek()
            ))),
        }
    }

    fn parse_property_member(&mut self) -> Result<ClassMember, ParseError> {
        let mutable = if self.peek() == &Token::Var {
            self.advance();
            true
        } else {
            self.advance();
            false
        };
        let name = self.expect_identifier()?;
        let type_annotation = if self.peek() == &Token::Colon {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let default = if self.peek() == &Token::Assign {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(ClassMember::Property {
            name,
            mutable,
            type_annotation,
            default,
        })
    }

    fn parse_method_member(&mut self) -> Result<ClassMember, ParseError> {
        let is_override = if self.peek() == &Token::Override {
            self.advance();
            true
        } else {
            false
        };
        self.expect(&Token::Fn)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::LeftParen)?;
        let params = self.parse_parameter_list()?;
        self.expect(&Token::RightParen)?;
        let return_type = if self.peek() == &Token::Arrow {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect(&Token::LeftBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RightBrace)?;
        Ok(ClassMember::Method {
            name,
            is_override,
            params,
            return_type,
            body,
        })
    }

    fn parse_trait_definition(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::Trait)?;
        let name = self.expect_identifier()?;
        let supertraits = if self.peek() == &Token::Extends {
            self.advance();
            let mut supertraits = Vec::new();
            supertraits.push(self.expect_identifier()?);
            while self.peek() == &Token::Comma {
                self.advance();
                supertraits.push(self.expect_identifier()?);
            }
            supertraits
        } else {
            Vec::new()
        };
        self.expect(&Token::LeftBrace)?;
        let methods = self.parse_trait_methods()?;
        self.expect(&Token::RightBrace)?;
        Ok(Stmt::TraitDef {
            name,
            supertraits,
            methods,
        })
    }

    fn parse_trait_methods(&mut self) -> Result<Vec<TraitMethod>, ParseError> {
        let mut methods = Vec::new();
        self.skip_newlines();

        while self.peek() != &Token::RightBrace {
            methods.push(self.parse_trait_method()?);
            self.skip_newlines();
        }

        Ok(methods)
    }

    fn parse_trait_method(&mut self) -> Result<TraitMethod, ParseError> {
        self.expect(&Token::Fn)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::LeftParen)?;
        let params = self.parse_parameter_list()?;
        self.expect(&Token::RightParen)?;
        let return_type = if self.peek() == &Token::Arrow {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        if self.peek() == &Token::LeftBrace {
            self.advance();
            let body = self.parse_block()?;
            self.expect(&Token::RightBrace)?;
            Ok(TraitMethod::Default {
                name,
                params,
                return_type,
                body,
            })
        } else {
            Ok(TraitMethod::Required {
                name,
                params,
                return_type,
            })
        }
    }

    fn parse_module_definition(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::Module)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::LeftBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RightBrace)?;
        Ok(Stmt::ModuleDef { name, body })
    }

    fn parse_import_statement(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::Import)?;
        let mut path = Vec::new();

        // Support both identifier paths and string literal paths
        match self.peek() {
            Token::String(s) => {
                // String literal path: import "mai-json" or import "./utils"
                let path_str = s.clone();
                self.advance();
                // For relative paths, keep as single element
                if path_str.starts_with("./") || path_str.starts_with("../") {
                    path.push(path_str);
                } else {
                    // For named modules, use as-is
                    path.push(path_str);
                }
            }
            _ => {
                // Identifier path: import json or import json.utils
                path.push(self.expect_identifier()?);
                while self.peek() == &Token::Dot {
                    self.advance();
                    path.push(self.expect_identifier()?);
                }
            }
        }

        let alias = if self.peek() == &Token::Identifier("as".to_string()) {
            self.advance();
            Some(self.expect_identifier()?)
        } else {
            None
        };

        let items = if self.peek() == &Token::Dot {
            self.advance();
            if self.peek() == &Token::LeftBrace {
                self.advance();
                let mut items = Vec::new();
                items.push(self.expect_identifier()?);
                while self.peek() == &Token::Comma {
                    self.advance();
                    items.push(self.expect_identifier()?);
                }
                self.expect(&Token::RightBrace)?;
                Some(items)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Stmt::Import { path, alias, items })
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::If)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::LeftBrace)?;
        let then_branch = self.parse_block()?;
        self.expect(&Token::RightBrace)?;

        let mut elif_branches = Vec::new();
        while self.peek() == &Token::Elif {
            self.advance();
            let elif_condition = self.parse_expression()?;
            self.expect(&Token::LeftBrace)?;
            let elif_body = self.parse_block()?;
            self.expect(&Token::RightBrace)?;
            elif_branches.push((elif_condition, elif_body));
        }

        let else_branch = if self.peek() == &Token::Else {
            self.advance();
            self.expect(&Token::LeftBrace)?;
            let body = self.parse_block()?;
            self.expect(&Token::RightBrace)?;
            Some(body)
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
        })
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::For)?;
        let variable = self.expect_identifier()?;
        self.expect(&Token::In)?;
        let iterable = self.parse_expression()?;
        self.expect(&Token::LeftBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RightBrace)?;
        Ok(Stmt::For {
            variable,
            iterable,
            body,
        })
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::While)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::LeftBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RightBrace)?;
        Ok(Stmt::While { condition, body })
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, ParseError> {
        self.expect(&Token::Return)?;
        let value = if self.peek() == &Token::Newline || self.peek() == &Token::RightBrace {
            None
        } else {
            Some(self.parse_expression()?)
        };
        Ok(Stmt::Return(value))
    }

    pub(crate) fn parse_expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expression()?;
        Ok(Stmt::Expression(expr))
    }

    pub(crate) fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while self.peek() != &Token::RightBrace && self.peek() != &Token::Eof {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        Ok(statements)
    }
}
