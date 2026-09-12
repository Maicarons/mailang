//! Conservative source formatter.
//! Parses to AST and reprints with 4-space indent and normalized spacing.
//! Unknown/exotic constructs fall back to the original line.

use mailang_ast::*;

const INDENT: &str = "    ";

pub fn format_source(code: &str) -> Result<String, String> {
    let mut parser = mailang_parser::Parser::new(code).map_err(|e| e.to_string())?;
    let program = parser.parse_program().map_err(|e| e.to_string())?;
    let mut out = String::new();
    for (i, stmt) in program.statements.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        format_stmt(stmt, 0, &mut out);
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str(INDENT);
    }
}

fn format_block(stmts: &[Stmt], level: usize, out: &mut String) {
    out.push_str(" {\n");
    for s in stmts {
        format_stmt(s, level + 1, out);
    }
    indent(level, out);
    out.push('}');
}

fn format_stmt(stmt: &Stmt, level: usize, out: &mut String) {
    indent(level, out);
    match stmt {
        Stmt::Let {
            name,
            mutable,
            type_annotation,
            value,
        } => {
            out.push_str(if *mutable { "var " } else { "let " });
            out.push_str(name);
            if let Some(t) = type_annotation {
                out.push_str(": ");
                out.push_str(&type_str(t));
            }
            if let Some(v) = value {
                out.push_str(" = ");
                format_expr(v, out);
            }
            out.push('\n');
        }
        Stmt::Const {
            name,
            type_annotation,
            value,
        } => {
            out.push_str("const ");
            out.push_str(name);
            if let Some(t) = type_annotation {
                out.push_str(": ");
                out.push_str(&type_str(t));
            }
            out.push_str(" = ");
            format_expr(value, out);
            out.push('\n');
        }
        Stmt::FunctionDef {
            name,
            params,
            return_type,
            body,
        } => {
            out.push_str("fn ");
            out.push_str(name);
            out.push('(');
            out.push_str(&params_str(params));
            out.push(')');
            if let Some(rt) = return_type {
                out.push_str(" -> ");
                out.push_str(&type_str(rt));
            }
            format_block(body, level, out);
            out.push('\n');
        }
        Stmt::ClassDef {
            name,
            superclass,
            traits,
            members,
        } => {
            out.push_str("class ");
            out.push_str(name);
            if let Some(s) = superclass {
                out.push_str(" extends ");
                out.push_str(s);
            }
            if !traits.is_empty() {
                out.push_str(" implements ");
                out.push_str(&traits.join(", "));
            }
            out.push_str(" {\n");
            for m in members {
                format_class_member(m, level + 1, out);
            }
            indent(level, out);
            out.push_str("}\n");
        }
        Stmt::TraitDef { name, methods } => {
            out.push_str("trait ");
            out.push_str(name);
            out.push_str(" {\n");
            for m in methods {
                match m {
                    TraitMethod::Required {
                        name,
                        params,
                        return_type,
                    } => {
                        indent(level + 1, out);
                        out.push_str("fn ");
                        out.push_str(name);
                        out.push('(');
                        out.push_str(&params_str(params));
                        out.push(')');
                        if let Some(rt) = return_type {
                            out.push_str(" -> ");
                            out.push_str(&type_str(rt));
                        }
                        out.push('\n');
                    }
                    TraitMethod::Default {
                        name,
                        params,
                        return_type,
                        body,
                    } => {
                        indent(level + 1, out);
                        out.push_str("fn ");
                        out.push_str(name);
                        out.push('(');
                        out.push_str(&params_str(params));
                        out.push(')');
                        if let Some(rt) = return_type {
                            out.push_str(" -> ");
                            out.push_str(&type_str(rt));
                        }
                        format_block(body, level + 1, out);
                        out.push('\n');
                    }
                }
            }
            indent(level, out);
            out.push_str("}\n");
        }
        Stmt::Expression(expr) => {
            format_expr(expr, out);
            out.push('\n');
        }
        Stmt::Return(value) => {
            out.push_str("return");
            if let Some(v) = value {
                out.push(' ');
                format_expr(v, out);
            }
            out.push('\n');
        }
        Stmt::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            out.push_str("if ");
            format_expr(condition, out);
            format_block(then_branch, level, out);
            for (cond, body) in elif_branches {
                out.push_str(" elif ");
                format_expr(cond, out);
                format_block(body, level, out);
            }
            if let Some(els) = else_branch {
                out.push_str(" else ");
                format_block(els, level, out);
            }
            out.push('\n');
        }
        Stmt::For {
            variable,
            iterable,
            body,
        } => {
            out.push_str("for ");
            out.push_str(variable);
            out.push_str(" in ");
            format_expr(iterable, out);
            format_block(body, level, out);
            out.push('\n');
        }
        Stmt::While { condition, body } => {
            out.push_str("while ");
            format_expr(condition, out);
            format_block(body, level, out);
            out.push('\n');
        }
        Stmt::Break => {
            out.push_str("break\n");
        }
        Stmt::Continue => {
            out.push_str("continue\n");
        }
        Stmt::Import { path, alias, .. } => {
            out.push_str("import ");
            out.push_str(&path.join("."));
            if let Some(a) = alias {
                out.push_str(" as ");
                out.push_str(a);
            }
            out.push('\n');
        }
        Stmt::ModuleDef { name, body } => {
            out.push_str("module ");
            out.push_str(name);
            format_block(body, level, out);
            out.push('\n');
        }
    }
}

fn format_class_member(m: &ClassMember, level: usize, out: &mut String) {
    indent(level, out);
    match m {
        ClassMember::Property {
            name,
            mutable,
            type_annotation,
            default,
        } => {
            out.push_str(if *mutable { "var " } else { "let " });
            out.push_str(name);
            if let Some(t) = type_annotation {
                out.push_str(": ");
                out.push_str(&type_str(t));
            }
            if let Some(d) = default {
                out.push_str(" = ");
                format_expr(d, out);
            }
            out.push('\n');
        }
        ClassMember::Method {
            name,
            params,
            return_type,
            body,
            is_override,
        } => {
            if *is_override {
                out.push_str("override ");
            }
            out.push_str("fn ");
            out.push_str(name);
            out.push('(');
            out.push_str(&params_str(params));
            out.push(')');
            if let Some(rt) = return_type {
                out.push_str(" -> ");
                out.push_str(&type_str(rt));
            }
            format_block(body, level, out);
            out.push('\n');
        }
        ClassMember::Constructor {
            params,
            super_args,
            body,
        } => {
            out.push_str("fn init(");
            out.push_str(&params_str(params));
            out.push(')');
            if let Some(sargs) = super_args {
                out.push_str(" super(");
                for (i, a) in sargs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    format_expr(a, out);
                }
                out.push(')');
            }
            format_block(body, level, out);
            out.push('\n');
        }
    }
}

fn params_str(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| {
            let mut s = p.name.clone();
            if let Some(t) = &p.type_annotation {
                s.push_str(": ");
                s.push_str(&type_str(t));
            }
            s
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn type_str(t: &TypeAnnotation) -> String {
    match t {
        TypeAnnotation::Int => "int".into(),
        TypeAnnotation::Float => "float".into(),
        TypeAnnotation::Bool => "bool".into(),
        TypeAnnotation::Str => "str".into(),
        TypeAnnotation::Char => "char".into(),
        TypeAnnotation::Array(inner) => format!("[{}]", type_str(inner)),
        TypeAnnotation::Map(k, v) => format!("{{{}: {}}}", type_str(k), type_str(v)),
        TypeAnnotation::Tuple(ts) => format!(
            "({})",
            ts.iter().map(type_str).collect::<Vec<_>>().join(", ")
        ),
        TypeAnnotation::Result(a, b) => format!("Result<{}, {}>", type_str(a), type_str(b)),
        TypeAnnotation::Option(inner) => format!("Option<{}>", type_str(inner)),
        TypeAnnotation::Custom(name) => name.clone(),
        TypeAnnotation::Infer => "null".into(),
    }
}

fn format_expr(expr: &Expr, out: &mut String) {
    match expr {
        Expr::Literal(lit) => out.push_str(&literal_str(lit)),
        Expr::Identifier(name) => out.push_str(name),
        Expr::BinaryOp { left, op, right } => {
            format_expr(left, out);
            out.push(' ');
            out.push_str(binop_str(op));
            out.push(' ');
            format_expr(right, out);
        }
        Expr::UnaryOp { op, operand } => {
            out.push_str(match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "!",
                UnaryOp::BitNot => "~",
            });
            format_expr(operand, out);
        }
        Expr::Call { callee, args } => {
            format_expr(callee, out);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_expr(a, out);
            }
            out.push(')');
        }
        Expr::MethodCall {
            object,
            method,
            args,
        } => {
            format_expr(object, out);
            out.push('.');
            out.push_str(method);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_expr(a, out);
            }
            out.push(')');
        }
        Expr::PropertyAccess { object, property } => {
            format_expr(object, out);
            out.push('.');
            out.push_str(property);
        }
        Expr::Index { object, index } => {
            format_expr(object, out);
            out.push('[');
            format_expr(index, out);
            out.push(']');
        }
        Expr::Array(items) => {
            out.push('[');
            for (i, e) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_expr(e, out);
            }
            out.push(']');
        }
        Expr::Map(entries) => {
            out.push('{');
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_expr(k, out);
                out.push_str(": ");
                format_expr(v, out);
            }
            out.push('}');
        }
        Expr::Tuple(items) => {
            out.push('(');
            for (i, e) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_expr(e, out);
            }
            out.push(')');
        }
        Expr::Range { start, end } => {
            format_expr(start, out);
            out.push_str("..");
            format_expr(end, out);
        }
        Expr::Assign { target, value } => {
            format_expr(target, out);
            out.push_str(" = ");
            format_expr(value, out);
        }
        Expr::CompoundAssign { op, target, value } => {
            format_expr(target, out);
            out.push(' ');
            out.push_str(binop_str(op));
            out.push_str("= ");
            format_expr(value, out);
        }
        Expr::Ok(v) => {
            out.push_str("Ok(");
            format_expr(v, out);
            out.push(')');
        }
        Expr::Err(v) => {
            out.push_str("Err(");
            format_expr(v, out);
            out.push(')');
        }
        Expr::Some(v) => {
            out.push_str("Some(");
            format_expr(v, out);
            out.push(')');
        }
        Expr::None => out.push_str("None"),
        Expr::Try(inner) => {
            format_expr(inner, out);
            out.push('?');
        }
        Expr::Lambda { params, body } => {
            out.push_str("fn(");
            out.push_str(&params_str(params));
            out.push(')');
            match body.as_ref() {
                // expression body (via Block with single Expression)
                Expr::Block(stmts) if stmts.len() == 1 => {
                    if let Stmt::Expression(e) = &stmts[0] {
                        out.push_str(" -> ");
                        format_expr(e, out);
                    } else {
                        out.push_str(" { ... }");
                    }
                }
                other => {
                    out.push_str(" -> ");
                    format_expr(other, out);
                }
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            out.push_str("if ");
            format_expr(condition, out);
            out.push_str(" { ");
            format_expr(then_branch, out);
            out.push_str(" }");
            if let Some(els) = else_branch {
                out.push_str(" else { ");
                format_expr(els, out);
                out.push_str(" }");
            }
        }
        Expr::Match { scrutinee, arms } => {
            out.push_str("match ");
            format_expr(scrutinee, out);
            out.push_str(" {\n");
            for arm in arms {
                out.push_str(INDENT);
                format_pattern(&arm.pattern, out);
                if let Some(g) = &arm.guard {
                    out.push_str(" if ");
                    format_expr(g, out);
                }
                out.push_str(" => ");
                format_expr(&arm.body, out);
                out.push('\n');
            }
            out.push('}');
        }
        Expr::Block(stmts) => {
            out.push_str("{\n");
            for s in stmts {
                format_stmt(s, 1, out);
            }
            out.push('}');
        }
        Expr::StringInterpolation(parts) => {
            out.push('"');
            for p in parts {
                match p {
                    StringPart::Text(t) => out.push_str(t),
                    StringPart::Expr(e) => {
                        out.push('{');
                        format_expr(e, out);
                        out.push('}');
                    }
                }
            }
            out.push('"');
        }
    }
}

fn format_pattern(p: &Pattern, out: &mut String) {
    match p {
        Pattern::Literal(l) => out.push_str(&literal_str(l)),
        Pattern::Identifier(n) => out.push_str(n),
        Pattern::Wildcard => out.push('_'),
        Pattern::Or(ps) => {
            for (i, x) in ps.iter().enumerate() {
                if i > 0 {
                    out.push_str(" | ");
                }
                format_pattern(x, out);
            }
        }
        Pattern::Tuple(ps) | Pattern::Array(ps) => {
            out.push('(');
            for (i, x) in ps.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_pattern(x, out);
            }
            out.push(')');
        }
        Pattern::Range(a, b, inclusive) => {
            format_expr(a, out);
            out.push_str(if *inclusive { "..=" } else { ".." });
            format_expr(b, out);
        }
        Pattern::Guard(inner, g) => {
            format_pattern(inner, out);
            out.push_str(" if ");
            format_expr(g, out);
        }
        Pattern::Ok(i) => {
            out.push_str("Ok(");
            format_pattern(i, out);
            out.push(')');
        }
        Pattern::Err(i) => {
            out.push_str("Err(");
            format_pattern(i, out);
            out.push(')');
        }
        Pattern::Some(i) => {
            out.push_str("Some(");
            format_pattern(i, out);
            out.push(')');
        }
    }
}

fn literal_str(l: &Literal) -> String {
    match l {
        Literal::Int(n) => n.to_string(),
        Literal::Float(n) => {
            if n.fract() == 0.0 {
                format!("{:.1}", n)
            } else {
                n.to_string()
            }
        }
        Literal::Bool(b) => b.to_string(),
        Literal::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Literal::Char(c) => format!("'{}'", c),
        Literal::Null => "null".into(),
    }
}

fn binop_str(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Pow => "**",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_let_and_fn() {
        let src = "let x=1\nfn add(a,b){return a+b}\n";
        let out = format_source(src).unwrap();
        assert!(out.contains("let x = 1"));
        assert!(out.contains("fn add(a, b) {"));
        assert!(out.contains("    return a + b"));
    }

    #[test]
    fn formats_class() {
        let src = "class P { let x\n fn init(x){ this.x=x }\n }\n";
        let out = format_source(src).unwrap();
        assert!(out.contains("class P {"));
        assert!(out.contains("    let x"));
    }
}
