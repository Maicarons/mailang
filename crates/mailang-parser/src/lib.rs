mod error;
mod parser;

pub use error::ParseError;
pub use parser::Parser;

#[cfg(test)]
mod tests {
    use super::*;
    use mailang_ast::*;

    fn parse(src: &str) -> Program {
        Parser::new(src)
            .expect("lex")
            .parse_program()
            .expect("parse")
    }

    #[test]
    fn postfix_match_expression() {
        let prog = parse("x match {\n1 => 2\n_ => 3\n}\n");
        assert_eq!(prog.statements.len(), 1);
        match &prog.statements[0] {
            Stmt::Expression(Expr::Match { scrutinee, arms }) => {
                assert!(matches!(&**scrutinee, Expr::Identifier(n) if n == "x"));
                assert_eq!(arms.len(), 2);
                assert!(matches!(arms[0].pattern, Pattern::Literal(Literal::Int(1))));
                assert!(matches!(arms[1].pattern, Pattern::Wildcard));
            }
            other => panic!("expected match expression, got {:?}", other),
        }
    }

    #[test]
    fn postfix_match_after_call() {
        let prog = parse("f(1) match {\nOk(v) => v\nErr(e) => 0\n}\n");
        match &prog.statements[0] {
            Stmt::Expression(Expr::Match { scrutinee, arms }) => {
                assert!(matches!(&**scrutinee, Expr::Call { .. }));
                assert_eq!(arms.len(), 2);
            }
            other => panic!("expected match expression, got {:?}", other),
        }
    }

    #[test]
    fn let_destructure_tuple() {
        let prog = parse("let (a, b) = pair\n");
        match &prog.statements[0] {
            Stmt::Let {
                name,
                mutable,
                pattern: Some(Pattern::Tuple(items)),
                value: Some(Expr::Identifier(v)),
                ..
            } => {
                assert!(name.is_empty());
                assert!(!*mutable);
                assert_eq!(items.len(), 2);
                assert!(matches!(&items[0], Pattern::Identifier(n) if n == "a"));
                assert!(matches!(&items[1], Pattern::Identifier(n) if n == "b"));
                assert_eq!(v, "pair");
            }
            other => panic!("expected let destructure, got {:?}", other),
        }
    }

    #[test]
    fn let_destructure_array_and_var() {
        let prog = parse("var [x, y] = arr\n");
        match &prog.statements[0] {
            Stmt::Let {
                mutable,
                pattern: Some(Pattern::Array(items)),
                ..
            } => {
                assert!(*mutable);
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected array destructure, got {:?}", other),
        }
    }

    #[test]
    fn let_simple_has_no_pattern() {
        let prog = parse("let x = 1\n");
        match &prog.statements[0] {
            Stmt::Let {
                name,
                mutable,
                pattern,
                ..
            } => {
                assert_eq!(name, "x");
                assert!(!*mutable);
                assert!(pattern.is_none());
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn trait_extends_supertraits() {
        let prog = parse("trait T extends A, B {\nfn f() -> int\n}\n");
        match &prog.statements[0] {
            Stmt::TraitDef {
                name,
                supertraits,
                methods,
            } => {
                assert_eq!(name, "T");
                assert_eq!(supertraits, &vec!["A".to_string(), "B".to_string()]);
                assert_eq!(methods.len(), 1);
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn trait_without_extends() {
        let prog = parse("trait T {\nfn f() -> int\n}\n");
        match &prog.statements[0] {
            Stmt::TraitDef { supertraits, .. } => {
                assert!(supertraits.is_empty());
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn fn_type_params() {
        let prog = parse("fn id<T>(x: T) -> T {\nreturn x\n}\n");
        match &prog.statements[0] {
            Stmt::FunctionDef {
                name,
                type_params,
                params,
                return_type,
                ..
            } => {
                assert_eq!(name, "id");
                assert_eq!(type_params, &vec!["T".to_string()]);
                assert_eq!(params.len(), 1);
                assert!(matches!(
                    params[0].type_annotation,
                    Some(TypeAnnotation::Param(ref n)) if n == "T"
                ));
                assert!(matches!(
                    return_type,
                    Some(TypeAnnotation::Param(ref n)) if n == "T"
                ));
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn fn_two_type_params() {
        let prog = parse("fn pair<A, B>(a: A, b: B) -> (A, B) {\nreturn (a, b)\n}\n");
        match &prog.statements[0] {
            Stmt::FunctionDef {
                type_params,
                params,
                ..
            } => {
                assert_eq!(type_params, &vec!["A".to_string(), "B".to_string()]);
                assert_eq!(params.len(), 2);
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn class_type_params_and_apply_annotation() {
        let prog = parse("class Box<T> {\nlet val: T\n}\nlet b: Box<int>\n");
        match &prog.statements[0] {
            Stmt::ClassDef {
                name,
                type_params,
                members,
                ..
            } => {
                assert_eq!(name, "Box");
                assert_eq!(type_params, &vec!["T".to_string()]);
                match &members[0] {
                    ClassMember::Property {
                        type_annotation, ..
                    } => {
                        assert!(matches!(
                            type_annotation,
                            Some(TypeAnnotation::Param(ref n)) if n == "T"
                        ));
                    }
                    other => panic!("{:?}", other),
                }
            }
            other => panic!("{:?}", other),
        }
        match &prog.statements[1] {
            Stmt::Let {
                type_annotation, ..
            } => {
                assert!(matches!(
                    type_annotation,
                    Some(TypeAnnotation::Apply(ref n, ref args))
                        if n == "Box" && args.len() == 1 && args[0] == TypeAnnotation::Int
                ));
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn turbofish_call() {
        let prog = parse("id::<int>(3)\n");
        match &prog.statements[0] {
            Stmt::Expression(Expr::Call {
                callee,
                args,
                type_args,
            }) => {
                assert!(matches!(&**callee, Expr::Identifier(n) if n == "id"));
                assert_eq!(args.len(), 1);
                assert_eq!(type_args, &vec![TypeAnnotation::Int]);
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn turbofish_two_type_args() {
        let prog = parse("pair::<int, str>(1, \"a\")\n");
        match &prog.statements[0] {
            Stmt::Expression(Expr::Call { type_args, .. }) => {
                assert_eq!(type_args, &vec![TypeAnnotation::Int, TypeAnnotation::Str]);
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn nested_generic_type_splits_shift() {
        // `Box<Box<int>>` must not be eaten as a `>>` shift token.
        let prog = parse("let x: Box<Box<int>>\n");
        match &prog.statements[0] {
            Stmt::Let {
                type_annotation, ..
            } => {
                assert!(matches!(
                    type_annotation,
                    Some(TypeAnnotation::Apply(n, args))
                        if n == "Box"
                            && args.len() == 1
                            && matches!(&args[0], TypeAnnotation::Apply(inner, _)
                                if inner == "Box")
                ));
            }
            other => panic!("{:?}", other),
        }
    }
}
