use crate::error::AnalyzerError;
use mailang_ast::*;
use std::collections::HashMap;

/// Loose compatibility: Any/Unknown match anything; Int ⊂ Float.
/// Type parameters are parametric (match anything until substituted).
fn types_compatible(expected: &Type, found: &Type) -> bool {
    match (expected, found) {
        (Type::Any, _) | (_, Type::Any) => true,
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::TypeParam(_), _) | (_, Type::TypeParam(_)) => true,
        (Type::Float, Type::Int) => true,
        // `None` (Null / Option(Null)) is compatible with any Option.
        (Type::Option(_), Type::Null) => true,
        (Type::Option(_), Type::Option(inner)) if matches!(**inner, Type::Null) => true,
        (Type::Option(e), Type::Option(f)) => types_compatible(e, f),
        (Type::Result(eok, eerr), Type::Result(fok, ferr)) => {
            types_compatible(eok, fok) && types_compatible(eerr, ferr)
        }
        (Type::Array(e), Type::Array(f)) => types_compatible(e, f),
        (Type::Map(ek, ev), Type::Map(fk, fv)) => {
            types_compatible(ek, fk) && types_compatible(ev, fv)
        }
        (Type::Tuple(es), Type::Tuple(fs)) if es.len() == fs.len() => es
            .iter()
            .zip(fs.iter())
            .all(|(e, f)| types_compatible(e, f)),
        (Type::Apply(n1, a1), Type::Apply(n2, a2)) => {
            n1 == n2
                && a1.len() == a2.len()
                && a1
                    .iter()
                    .zip(a2.iter())
                    .all(|(e, f)| types_compatible(e, f))
        }
        (Type::Apply(n, _), Type::Class(m)) | (Type::Class(m), Type::Apply(n, _)) => n == m,
        (
            Type::Function {
                params: ep,
                return_type: er,
            },
            Type::Function {
                params: fp,
                return_type: fr,
            },
        ) => {
            ep.len() == fp.len()
                && ep
                    .iter()
                    .zip(fp.iter())
                    .all(|(e, f)| types_compatible(e, f))
                && types_compatible(er, fr)
        }
        _ => expected == found,
    }
}

/// Replace `Type::TypeParam` entries using `map` (call-site substitution).
fn substitute_type(ty: &Type, map: &HashMap<String, Type>) -> Type {
    match ty {
        Type::TypeParam(name) => map.get(name).cloned().unwrap_or(Type::Any),
        Type::Array(inner) => Type::Array(Box::new(substitute_type(inner, map))),
        Type::Map(k, v) => Type::Map(
            Box::new(substitute_type(k, map)),
            Box::new(substitute_type(v, map)),
        ),
        Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| substitute_type(t, map)).collect()),
        Type::Result(a, b) => Type::Result(
            Box::new(substitute_type(a, map)),
            Box::new(substitute_type(b, map)),
        ),
        Type::Option(i) => Type::Option(Box::new(substitute_type(i, map))),
        Type::Function {
            params,
            return_type,
        } => Type::Function {
            params: params.iter().map(|t| substitute_type(t, map)).collect(),
            return_type: Box::new(substitute_type(return_type, map)),
        },
        Type::Apply(n, args) => Type::Apply(
            n.clone(),
            args.iter().map(|t| substitute_type(t, map)).collect(),
        ),
        other => other.clone(),
    }
}

/// Wildcard / identifier (or an or-pattern containing one) is a catch-all.
fn pattern_is_catch_all(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Wildcard | Pattern::Identifier(_) => true,
        Pattern::Or(patterns) => patterns.iter().any(pattern_is_catch_all),
        Pattern::Guard(inner, _) => pattern_is_catch_all(inner),
        _ => false,
    }
}

/// True when arms alone cover Result (Ok+Err) or Option (Some+None).
fn arms_cover_result_or_option(arms: &[MatchArm]) -> bool {
    let has_ok = arms.iter().any(|a| matches!(a.pattern, Pattern::Ok(_)));
    let has_err = arms.iter().any(|a| matches!(a.pattern, Pattern::Err(_)));
    if has_ok && has_err {
        return true;
    }
    let has_some = arms.iter().any(|a| matches!(a.pattern, Pattern::Some(_)));
    let has_none = arms
        .iter()
        .any(|a| matches!(a.pattern, Pattern::Literal(Literal::Null)));
    has_some && has_none
}

/// Does any statement in `stmts` contain a `return` (nested blocks included)?
fn contains_return(stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::Return(_) => return true,
            Stmt::If {
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                if contains_return(then_branch) {
                    return true;
                }
                for (_, body) in elif_branches {
                    if contains_return(body) {
                        return true;
                    }
                }
                if let Some(body) = else_branch {
                    if contains_return(body) {
                        return true;
                    }
                }
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } => {
                if contains_return(body) {
                    return true;
                }
            }
            Stmt::Expression(Expr::Block(inner)) => {
                if contains_return(inner) {
                    return true;
                }
            }
            // Nested function returns do not count for the outer function.
            _ => {}
        }
    }
    false
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // richer type model used as the analyzer grows
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Char,
    Null,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Result(Box<Type>, Box<Type>),
    Option(Box<Type>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Class(String),
    Trait(String),
    /// Unsubstituted generic parameter (`T`).
    TypeParam(String),
    /// Generic instantiation (`Box<int>`).
    Apply(String, Vec<Type>),
    Any,
    Unknown,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::Str => write!(f, "str"),
            Type::Char => write!(f, "char"),
            Type::Null => write!(f, "null"),
            Type::Array(inner) => write!(f, "[{}]", inner),
            Type::Map(key, value) => write!(f, "{{{}: {}}}", key, value),
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::Option(inner) => write!(f, "Option<{}>", inner),
            Type::Function {
                params,
                return_type,
            } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", return_type)
            }
            Type::Class(name) => write!(f, "{}", name),
            Type::Trait(name) => write!(f, "{}", name),
            Type::TypeParam(name) => write!(f, "{}", name),
            Type::Apply(name, args) => {
                write!(f, "{}<", name)?;
                for (i, t) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ">")
            }
            Type::Any => write!(f, "any"),
            Type::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // reserved for typed analysis
struct Variable {
    name: String,
    ty: Type,
    mutable: bool,
    is_const: bool,
    used: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // reserved for typed analysis
struct FunctionInfo {
    name: String,
    /// Generic type parameters (empty for monomorphic functions).
    type_params: Vec<String>,
    params: Vec<(String, Type)>,
    /// Number of leading params without defaults (minimum arity).
    required: usize,
    return_type: Type,
    /// False for host/stdlib builtins (arity is dynamic).
    declared: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // reserved for typed analysis
struct ClassInfo {
    name: String,
    /// Generic type parameters (empty for monomorphic classes).
    type_params: Vec<String>,
    superclass: Option<String>,
    traits: Vec<String>,
    methods: HashMap<String, FunctionInfo>,
    properties: HashMap<String, (Type, bool)>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // reserved for typed analysis
struct TraitInfo {
    name: String,
    methods: HashMap<String, FunctionInfo>,
}

struct Scope {
    variables: HashMap<String, Variable>,
    parent: Option<usize>,
}

pub struct Analyzer {
    scopes: Vec<Scope>,
    current_scope: usize,
    functions: HashMap<String, FunctionInfo>,
    classes: HashMap<String, ClassInfo>,
    traits: HashMap<String, TraitInfo>,
    current_class: Option<String>,
    /// Stack of declared return types for enclosing functions (None = unannotated).
    return_stack: Vec<Option<Type>>,
    /// Currently bound generic type parameter names (fn/class scopes).
    type_param_scopes: Vec<Vec<String>>,
    errors: Vec<AnalyzerError>,
}

impl Analyzer {
    pub fn new() -> Self {
        let global_scope = Scope {
            variables: HashMap::new(),
            parent: None,
        };
        let mut s = Self {
            scopes: vec![global_scope],
            current_scope: 0,
            functions: HashMap::new(),
            classes: HashMap::new(),
            traits: HashMap::new(),
            current_class: None,
            return_stack: Vec::new(),
            type_param_scopes: Vec::new(),
            errors: Vec::new(),
        };
        s.register_stdlib_builtins();
        s
    }

    /// Mark a name as a known global (builtin or host-registered function).
    pub fn register_builtin(&mut self, name: &str) {
        self.define_variable(name.to_string(), Type::Any, true);
        self.functions.insert(
            name.to_string(),
            FunctionInfo {
                name: name.to_string(),
                type_params: Vec::new(),
                params: Vec::new(),
                required: 0,
                return_type: Type::Any,
                declared: false,
            },
        );
    }

    fn register_stdlib_builtins(&mut self) {
        const BUILTINS: &[&str] = &[
            "println",
            "print",
            "input",
            "sqrt",
            "abs",
            "sin",
            "cos",
            "floor",
            "ceil",
            "round",
            "min",
            "max",
            "len",
            "to_string",
            "parse_int",
            "parse_float",
            "time_now",
            "time_now_secs",
            "time_year",
            "time_month",
            "time_day",
            "time_hour",
            "time_minute",
            "time_second",
            "time_date",
            "time_datetime",
            "time_elapsed",
            "time_sleep",
            // simulated HAL
            "gpio_write",
            "gpio_read",
            "delay_ms",
            "adc_read",
            "pwm_write",
            "pwm_freq",
            "uart_write",
            "uart_read",
            "i2c_xfer",
            "spi_xfer",
            "read_file",
            "write_file",
            "env",
            "process_exit",
            "json_parse",
            "json_stringify",
        ];
        for name in BUILTINS {
            self.register_builtin(name);
        }
    }

    fn push_scope(&mut self) {
        let scope = Scope {
            variables: HashMap::new(),
            parent: Some(self.current_scope),
        };
        self.scopes.push(scope);
        self.current_scope = self.scopes.len() - 1;
    }

    fn pop_scope(&mut self) {
        let scope = &self.scopes[self.current_scope];
        for var in scope.variables.values() {
            if !var.used && !var.name.starts_with('_') {
                self.errors
                    .push(AnalyzerError::UnusedVariable(var.name.clone()));
            }
        }
        if let Some(parent) = scope.parent {
            self.current_scope = parent;
        }
    }

    fn define_variable(&mut self, name: String, ty: Type, mutable: bool) {
        self.insert_variable(name, ty, mutable, false);
    }

    fn define_constant(&mut self, name: String, ty: Type) {
        self.insert_variable(name, ty, false, true);
    }

    fn insert_variable(&mut self, name: String, ty: Type, mutable: bool, is_const: bool) {
        let scope = &mut self.scopes[self.current_scope];
        scope.variables.insert(
            name.clone(),
            Variable {
                name,
                ty,
                mutable,
                is_const,
                used: false,
            },
        );
    }

    fn lookup_variable(&mut self, name: &str) -> Option<&Variable> {
        let mut scope_index = self.current_scope;
        loop {
            if let Some(var) = self.scopes[scope_index].variables.get(name) {
                return Some(var);
            }
            if let Some(parent) = self.scopes[scope_index].parent {
                scope_index = parent;
            } else {
                break;
            }
        }
        None
    }

    fn mark_variable_used(&mut self, name: &str) {
        let mut scope_index = self.current_scope;
        loop {
            if let Some(var) = self.scopes[scope_index].variables.get_mut(name) {
                var.used = true;
                return;
            }
            if let Some(parent) = self.scopes[scope_index].parent {
                scope_index = parent;
            } else {
                break;
            }
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<AnalyzerError>> {
        for stmt in &program.statements {
            self.analyze_statement(stmt);
        }

        // Drop unused-variable / missing-return noise from the hard-error list; they are hints.
        let hard: Vec<AnalyzerError> = self
            .errors
            .iter()
            .filter(|e| {
                !matches!(
                    e,
                    AnalyzerError::UnusedVariable(_) | AnalyzerError::MissingReturn(_)
                )
            })
            .cloned()
            .collect();

        if hard.is_empty() {
            Ok(())
        } else {
            Err(hard)
        }
    }

    /// All diagnostics including unused-variable hints.
    pub fn diagnostics(&self) -> Vec<AnalyzerError> {
        self.errors.clone()
    }

    fn analyze_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                type_annotation,
                value,
                pattern,
            } => {
                let ty = if let Some(ta) = type_annotation {
                    self.resolve_type(ta)
                } else if let Some(val) = value {
                    self.infer_type(val)
                } else {
                    Type::Unknown
                };
                if let Some(pat) = pattern {
                    self.define_pattern_bindings(pat, *mutable, false);
                } else {
                    self.define_variable(name.clone(), ty, *mutable);
                }
                if let Some(val) = value {
                    self.analyze_expression(val);
                }
            }
            Stmt::Const {
                name,
                type_annotation,
                value,
            } => {
                let ty = if let Some(ta) = type_annotation {
                    self.resolve_type(ta)
                } else {
                    self.infer_type(value)
                };
                self.define_constant(name.clone(), ty);
                self.analyze_expression(value);
            }
            Stmt::FunctionDef {
                name,
                type_params,
                params,
                return_type,
                body,
            } => {
                let saved = self.type_param_scopes.len();
                self.type_param_scopes.push(type_params.clone());
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        p.type_annotation
                            .as_ref()
                            .map(|ta| self.resolve_type(ta))
                            .unwrap_or(Type::Any)
                    })
                    .collect();
                let ret_type = return_type
                    .as_ref()
                    .map(|ta| self.resolve_type(ta))
                    .unwrap_or(Type::Null);

                self.functions.insert(
                    name.clone(),
                    FunctionInfo {
                        name: name.clone(),
                        type_params: type_params.clone(),
                        params: params
                            .iter()
                            .zip(param_types.iter())
                            .map(|(p, t)| (p.name.clone(), t.clone()))
                            .collect(),
                        required: params.iter().filter(|p| p.default.is_none()).count(),
                        return_type: ret_type.clone(),
                        declared: true,
                    },
                );

                self.push_scope();
                for (param, ty) in params.iter().zip(param_types.iter()) {
                    self.define_variable(param.name.clone(), ty.clone(), true);
                }
                self.return_stack
                    .push(return_type.as_ref().map(|_| ret_type.clone()));
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.return_stack.pop();
                // Hint: non-null return type with a path that falls off the end.
                if return_type.is_some()
                    && !matches!(ret_type, Type::Null | Type::Any | Type::Unknown)
                    && !matches!(ret_type, Type::TypeParam(_))
                {
                    let last_is_return = matches!(body.last(), Some(Stmt::Return(_)));
                    if !last_is_return && !contains_return(body) {
                        self.errors.push(AnalyzerError::MissingReturn(name.clone()));
                    }
                }
                self.pop_scope();
                self.type_param_scopes.truncate(saved);
            }
            Stmt::ClassDef {
                name,
                type_params,
                superclass,
                traits,
                members,
            } => {
                self.analyze_class(name, type_params, superclass, traits, members);
            }
            Stmt::TraitDef {
                name,
                supertraits: _,
                methods,
            } => {
                self.analyze_trait(name, methods);
            }
            Stmt::ModuleDef { name: _, body } => {
                self.push_scope();
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.pop_scope();
            }
            Stmt::Import { .. } => {}
            Stmt::Expression(expr) => {
                self.analyze_expression(expr);
            }
            Stmt::Return(value) => {
                if let Some(val) = value {
                    self.analyze_expression(val);
                    if let Some(Some(expected)) = self.return_stack.last().cloned() {
                        let found = self.infer_type(val);
                        if !types_compatible(&expected, &found) {
                            self.errors.push(AnalyzerError::TypeMismatch {
                                expected: expected.to_string(),
                                found: found.to_string(),
                            });
                        }
                    }
                } else if let Some(Some(expected)) = self.return_stack.last().cloned() {
                    // Bare `return` in a function with a non-null declared return type.
                    if !types_compatible(&expected, &Type::Null) {
                        self.errors.push(AnalyzerError::TypeMismatch {
                            expected: expected.to_string(),
                            found: Type::Null.to_string(),
                        });
                    }
                }
            }
            Stmt::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                self.analyze_expression(condition);
                self.push_scope();
                for stmt in then_branch {
                    self.analyze_statement(stmt);
                }
                self.pop_scope();
                for (cond, body) in elif_branches {
                    self.analyze_expression(cond);
                    self.push_scope();
                    for stmt in body {
                        self.analyze_statement(stmt);
                    }
                    self.pop_scope();
                }
                if let Some(body) = else_branch {
                    self.push_scope();
                    for stmt in body {
                        self.analyze_statement(stmt);
                    }
                    self.pop_scope();
                }
            }
            Stmt::For {
                variable,
                iterable,
                body,
            } => {
                self.analyze_expression(iterable);
                self.push_scope();
                self.define_variable(variable.clone(), Type::Any, true);
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.pop_scope();
            }
            Stmt::While { condition, body } => {
                self.analyze_expression(condition);
                self.push_scope();
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.pop_scope();
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn analyze_class(
        &mut self,
        name: &str,
        type_params: &[String],
        superclass: &Option<String>,
        traits: &[String],
        members: &[ClassMember],
    ) {
        let saved = self.type_param_scopes.len();
        self.type_param_scopes.push(type_params.to_vec());
        let mut class_info = ClassInfo {
            name: name.to_string(),
            type_params: type_params.to_vec(),
            superclass: superclass.clone(),
            traits: traits.to_vec(),
            methods: HashMap::new(),
            properties: HashMap::new(),
        };

        for member in members {
            match member {
                ClassMember::Property {
                    name,
                    mutable,
                    type_annotation,
                    ..
                } => {
                    let ty = type_annotation
                        .as_ref()
                        .map(|ta| self.resolve_type(ta))
                        .unwrap_or(Type::Any);
                    class_info.properties.insert(name.clone(), (ty, *mutable));
                }
                ClassMember::Method {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let param_types: Vec<Type> = params
                        .iter()
                        .map(|p| {
                            p.type_annotation
                                .as_ref()
                                .map(|ta| self.resolve_type(ta))
                                .unwrap_or(Type::Any)
                        })
                        .collect();
                    let ret_type = return_type
                        .as_ref()
                        .map(|ta| self.resolve_type(ta))
                        .unwrap_or(Type::Null);
                    class_info.methods.insert(
                        name.clone(),
                        FunctionInfo {
                            name: name.clone(),
                            type_params: Vec::new(),
                            params: params
                                .iter()
                                .zip(param_types.iter())
                                .map(|(p, t)| (p.name.clone(), t.clone()))
                                .collect(),
                            required: params.iter().filter(|p| p.default.is_none()).count(),
                            return_type: ret_type,
                            declared: true,
                        },
                    );
                }
                ClassMember::Constructor { params, body, .. } => {
                    self.push_scope();
                    self.define_variable("this".to_string(), Type::Class(name.to_string()), true);
                    for param in params {
                        let ty = param
                            .type_annotation
                            .as_ref()
                            .map(|ta| self.resolve_type(ta))
                            .unwrap_or(Type::Any);
                        self.define_variable(param.name.clone(), ty, true);
                    }
                    for stmt in body {
                        self.analyze_statement(stmt);
                    }
                    self.pop_scope();
                }
            }
        }

        self.classes.insert(name.to_string(), class_info);

        self.current_class = Some(name.to_string());
        self.push_scope();
        self.define_variable("this".to_string(), Type::Class(name.to_string()), true);
        for member in members {
            if let ClassMember::Method { params, body, .. } = member {
                self.push_scope();
                self.define_variable("this".to_string(), Type::Class(name.to_string()), true);
                for param in params {
                    let ty = param
                        .type_annotation
                        .as_ref()
                        .map(|ta| self.resolve_type(ta))
                        .unwrap_or(Type::Any);
                    self.define_variable(param.name.clone(), ty, true);
                }
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.pop_scope();
            }
        }
        self.pop_scope();
        self.current_class = None;
        self.type_param_scopes.truncate(saved);
    }

    fn analyze_trait(&mut self, name: &str, methods: &[TraitMethod]) {
        let mut trait_info = TraitInfo {
            name: name.to_string(),
            methods: HashMap::new(),
        };

        for method in methods {
            match method {
                TraitMethod::Required {
                    name,
                    params,
                    return_type,
                } => {
                    let param_types: Vec<Type> = params
                        .iter()
                        .map(|p| {
                            p.type_annotation
                                .as_ref()
                                .map(|ta| self.resolve_type(ta))
                                .unwrap_or(Type::Any)
                        })
                        .collect();
                    let ret_type = return_type
                        .as_ref()
                        .map(|ta| self.resolve_type(ta))
                        .unwrap_or(Type::Null);
                    trait_info.methods.insert(
                        name.clone(),
                        FunctionInfo {
                            name: name.clone(),
                            type_params: Vec::new(),
                            params: params
                                .iter()
                                .zip(param_types.iter())
                                .map(|(p, t)| (p.name.clone(), t.clone()))
                                .collect(),
                            required: params.iter().filter(|p| p.default.is_none()).count(),
                            return_type: ret_type,
                            declared: true,
                        },
                    );
                }
                TraitMethod::Default {
                    name,
                    params,
                    return_type,
                    body,
                } => {
                    let param_types: Vec<Type> = params
                        .iter()
                        .map(|p| {
                            p.type_annotation
                                .as_ref()
                                .map(|ta| self.resolve_type(ta))
                                .unwrap_or(Type::Any)
                        })
                        .collect();
                    let ret_type = return_type
                        .as_ref()
                        .map(|ta| self.resolve_type(ta))
                        .unwrap_or(Type::Null);
                    trait_info.methods.insert(
                        name.clone(),
                        FunctionInfo {
                            name: name.clone(),
                            type_params: Vec::new(),
                            params: params
                                .iter()
                                .zip(param_types.iter())
                                .map(|(p, t)| (p.name.clone(), t.clone()))
                                .collect(),
                            required: params.iter().filter(|p| p.default.is_none()).count(),
                            return_type: ret_type,
                            declared: true,
                        },
                    );

                    self.push_scope();
                    // Default methods have an implicit receiver.
                    self.define_variable("this".to_string(), Type::Any, true);
                    for (param, ty) in params.iter().zip(param_types.iter()) {
                        self.define_variable(param.name.clone(), ty.clone(), true);
                    }
                    for stmt in body {
                        self.analyze_statement(stmt);
                    }
                    self.pop_scope();
                }
            }
        }

        self.traits.insert(name.to_string(), trait_info);
    }

    fn analyze_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(_) => {}
            Expr::Identifier(name) => {
                if name == "super" || name == "this" {
                    // Context-sensitive keywords checked by the compiler/VM.
                    return;
                }
                if self.lookup_variable(name).is_some() {
                    self.mark_variable_used(name);
                } else if self.functions.contains_key(name)
                    || self.classes.contains_key(name)
                    || self.traits.contains_key(name)
                {
                    // known function/class/trait name used as a value
                    self.mark_variable_used(name);
                } else {
                    self.errors
                        .push(AnalyzerError::UndefinedVariable(name.clone()));
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.analyze_expression(left);
                self.analyze_expression(right);
            }
            Expr::UnaryOp { operand, .. } => {
                self.analyze_expression(operand);
            }
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                self.analyze_expression(callee);
                for arg in args {
                    self.analyze_expression(arg);
                }
                // Arity / simple type check for named user functions with annotations.
                if let Expr::Identifier(fname) = &**callee {
                    if let Some(info) = self.functions.get(fname).cloned() {
                        if info.declared {
                            // Turbofish type arguments must match the generic arity.
                            if !type_args.is_empty()
                                && type_args.len() != info.type_params.len()
                                && !info.type_params.is_empty()
                            {
                                self.errors.push(AnalyzerError::WrongTypeArgumentCount {
                                    name: fname.clone(),
                                    expected: info.type_params.len(),
                                    found: type_args.len(),
                                });
                            } else if !type_args.is_empty() && info.type_params.is_empty() {
                                self.errors.push(AnalyzerError::WrongTypeArgumentCount {
                                    name: fname.clone(),
                                    expected: 0,
                                    found: type_args.len(),
                                });
                            }
                            // Substitute type args (or leave params parametric).
                            let subst: HashMap<String, Type> = if !type_args.is_empty() {
                                info.type_params
                                    .iter()
                                    .zip(type_args.iter())
                                    .map(|(p, ta)| (p.clone(), self.resolve_type(ta)))
                                    .collect()
                            } else {
                                HashMap::new()
                            };
                            let min = info.required;
                            let max = info.params.len();
                            if args.len() < min || args.len() > max {
                                self.errors.push(AnalyzerError::WrongArgumentCount {
                                    name: fname.clone(),
                                    expected: max,
                                    found: args.len(),
                                });
                            } else {
                                for ((pname, pty), arg) in info.params.iter().zip(args.iter()) {
                                    let expected_ty = substitute_type(pty, &subst);
                                    let aty = self.infer_type(arg);
                                    if !types_compatible(&expected_ty, &aty) {
                                        self.errors.push(AnalyzerError::TypeMismatch {
                                            expected: format!("{}: {}", pname, expected_ty),
                                            found: format!("{}", aty),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Expr::MethodCall {
                object,
                method: _,
                args,
            } => {
                self.analyze_expression(object);
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            Expr::PropertyAccess {
                object,
                property: _,
            } => {
                self.analyze_expression(object);
            }
            Expr::Index { object, index } => {
                self.analyze_expression(object);
                self.analyze_expression(index);
            }
            Expr::Array(elements) => {
                for elem in elements {
                    self.analyze_expression(elem);
                }
            }
            Expr::Range { start, end } => {
                self.analyze_expression(start);
                self.analyze_expression(end);
            }
            Expr::Map(entries) => {
                for (key, value) in entries {
                    self.analyze_expression(key);
                    self.analyze_expression(value);
                }
            }
            Expr::Tuple(elements) => {
                for elem in elements {
                    self.analyze_expression(elem);
                }
            }
            Expr::Lambda { params, body } => {
                self.push_scope();
                for param in params {
                    let ty = param
                        .type_annotation
                        .as_ref()
                        .map(|ta| self.resolve_type(ta))
                        .unwrap_or(Type::Any);
                    self.define_variable(param.name.clone(), ty, true);
                }
                self.analyze_expression(body);
                self.pop_scope();
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.analyze_expression(condition);
                self.analyze_expression(then_branch);
                if let Some(else_expr) = else_branch {
                    self.analyze_expression(else_expr);
                }
            }
            Expr::Match { scrutinee, arms } => {
                self.analyze_expression(scrutinee);
                // Exhaustiveness: hard error when there is no catch-all arm and the
                // patterns cannot cover Result/Option constructors either.
                let has_catch_all = arms.iter().any(|a| pattern_is_catch_all(&a.pattern));
                if !has_catch_all && !arms_cover_result_or_option(arms) {
                    self.errors.push(AnalyzerError::NonExhaustiveMatch);
                }
                for arm in arms {
                    self.push_scope();
                    self.analyze_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.analyze_expression(guard);
                    }
                    self.analyze_expression(&arm.body);
                    self.pop_scope();
                }
            }
            Expr::Block(stmts) => {
                self.push_scope();
                for stmt in stmts {
                    self.analyze_statement(stmt);
                }
                self.pop_scope();
            }
            Expr::Assign { target, value } => {
                self.check_assignment_target(target);
                self.analyze_expression(target);
                self.analyze_expression(value);
            }
            Expr::CompoundAssign { target, value, .. } => {
                self.check_assignment_target(target);
                self.analyze_expression(target);
                self.analyze_expression(value);
            }
            Expr::Ok(value) | Expr::Err(value) | Expr::Some(value) => {
                self.analyze_expression(value);
            }
            Expr::None => {}
            Expr::Try(inner) => {
                self.analyze_expression(inner);
            }
            Expr::StringInterpolation(parts) => {
                for part in parts {
                    if let StringPart::Expr(expr) = part {
                        self.analyze_expression(expr);
                    }
                }
            }
        }
    }

    fn analyze_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Literal(_) | Pattern::Wildcard => {}
            Pattern::Identifier(name) => {
                self.define_variable(name.clone(), Type::Any, true);
            }
            Pattern::Or(patterns) | Pattern::Tuple(patterns) | Pattern::Array(patterns) => {
                for p in patterns {
                    self.analyze_pattern(p);
                }
            }
            Pattern::Range(start, end, _) => {
                self.analyze_expression(start);
                self.analyze_expression(end);
            }
            Pattern::Guard(inner, guard) => {
                self.analyze_pattern(inner);
                self.analyze_expression(guard);
            }
            Pattern::Ok(inner) | Pattern::Err(inner) | Pattern::Some(inner) => {
                self.analyze_pattern(inner);
            }
        }
    }

    /// Define variables bound by a let/var destructuring pattern.
    fn define_pattern_bindings(&mut self, pattern: &Pattern, mutable: bool, is_const: bool) {
        match pattern {
            Pattern::Literal(_) | Pattern::Wildcard | Pattern::Range(_, _, _) => {}
            Pattern::Identifier(name) => {
                self.insert_variable(name.clone(), Type::Any, mutable, is_const);
            }
            Pattern::Or(patterns) | Pattern::Tuple(patterns) | Pattern::Array(patterns) => {
                for p in patterns {
                    self.define_pattern_bindings(p, mutable, is_const);
                }
            }
            Pattern::Guard(inner, _) => {
                self.define_pattern_bindings(inner, mutable, is_const);
            }
            Pattern::Ok(inner) | Pattern::Err(inner) | Pattern::Some(inner) => {
                self.define_pattern_bindings(inner, mutable, is_const);
            }
        }
    }

    /// Reject assignment to immutable bindings and `const` names.
    fn check_assignment_target(&mut self, target: &Expr) {
        let Expr::Identifier(name) = target else {
            return;
        };
        if name == "this" || name == "super" {
            return;
        }
        let info = self.lookup_variable(name).map(|v| (v.mutable, v.is_const));
        match info {
            Some((_, true)) => {
                self.errors
                    .push(AnalyzerError::ConstantAssignment(name.clone()));
            }
            Some((false, false)) => {
                self.errors
                    .push(AnalyzerError::ImmutableAssignment(name.clone()));
            }
            _ => {}
        }
    }

    fn resolve_type(&self, annotation: &TypeAnnotation) -> Type {
        match annotation {
            TypeAnnotation::Int => Type::Int,
            TypeAnnotation::Float => Type::Float,
            TypeAnnotation::Bool => Type::Bool,
            TypeAnnotation::Str => Type::Str,
            TypeAnnotation::Char => Type::Char,
            TypeAnnotation::Array(inner) => Type::Array(Box::new(self.resolve_type(inner))),
            TypeAnnotation::Map(key, value) => Type::Map(
                Box::new(self.resolve_type(key)),
                Box::new(self.resolve_type(value)),
            ),
            TypeAnnotation::Tuple(types) => {
                Type::Tuple(types.iter().map(|t| self.resolve_type(t)).collect())
            }
            TypeAnnotation::Result(ok, err) => Type::Result(
                Box::new(self.resolve_type(ok)),
                Box::new(self.resolve_type(err)),
            ),
            TypeAnnotation::Option(inner) => Type::Option(Box::new(self.resolve_type(inner))),
            TypeAnnotation::Custom(name) => {
                if self.classes.contains_key(name) {
                    Type::Class(name.clone())
                } else if self.traits.contains_key(name) {
                    Type::Trait(name.clone())
                } else {
                    Type::Unknown
                }
            }
            TypeAnnotation::Param(name) => {
                if self
                    .type_param_scopes
                    .iter()
                    .any(|scope| scope.iter().any(|p| p == name))
                {
                    Type::TypeParam(name.clone())
                } else {
                    Type::Unknown
                }
            }
            TypeAnnotation::Apply(name, args) => {
                let resolved: Vec<Type> = args.iter().map(|a| self.resolve_type(a)).collect();
                Type::Apply(name.clone(), resolved)
            }
            TypeAnnotation::Infer => Type::Any,
        }
    }

    fn infer_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => Type::Int,
                Literal::Float(_) => Type::Float,
                Literal::Bool(_) => Type::Bool,
                Literal::Str(_) => Type::Str,
                Literal::Char(_) => Type::Char,
                Literal::Null => Type::Null,
            },
            Expr::Array(elements) => {
                if let Some(first) = elements.first() {
                    Type::Array(Box::new(self.infer_type(first)))
                } else {
                    Type::Array(Box::new(Type::Any))
                }
            }
            Expr::Ok(value) => Type::Result(Box::new(self.infer_type(value)), Box::new(Type::Any)),
            Expr::Err(value) => Type::Result(Box::new(Type::Any), Box::new(self.infer_type(value))),
            Expr::Some(value) => Type::Option(Box::new(self.infer_type(value))),
            Expr::None => Type::Option(Box::new(Type::Null)),
            Expr::StringInterpolation(_) => Type::Str,
            Expr::Call {
                callee,
                args: _,
                type_args,
            } => {
                if let Expr::Identifier(fname) = &**callee {
                    if let Some(info) = self.functions.get(fname) {
                        let ret = info.return_type.clone();
                        if !type_args.is_empty() && !info.type_params.is_empty() {
                            let subst: HashMap<String, Type> = info
                                .type_params
                                .iter()
                                .zip(type_args.iter())
                                .map(|(p, ta)| (p.clone(), self.resolve_type(ta)))
                                .collect();
                            return substitute_type(&ret, &subst);
                        }
                        return ret;
                    }
                }
                Type::Any
            }
            _ => Type::Any,
        }
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn let_stmt(name: &str, mutable: bool, value: Expr) -> Stmt {
        Stmt::Let {
            name: name.to_string(),
            mutable,
            type_annotation: None,
            value: Some(value),
            pattern: None,
        }
    }

    fn assign(name: &str, value: Expr) -> Stmt {
        Stmt::Expression(Expr::Assign {
            target: Box::new(Expr::Identifier(name.to_string())),
            value: Box::new(value),
        })
    }

    fn compound_assign(name: &str) -> Stmt {
        Stmt::Expression(Expr::CompoundAssign {
            op: BinaryOp::Add,
            target: Box::new(Expr::Identifier(name.to_string())),
            value: Box::new(Expr::Literal(Literal::Int(1))),
        })
    }

    #[test]
    fn immutable_assignment_rejected() {
        let program = Program {
            statements: vec![
                let_stmt("x", false, Expr::Literal(Literal::Int(1))),
                assign("x", Expr::Literal(Literal::Int(2))),
            ],
        };
        let mut a = Analyzer::new();
        let errs = a.analyze(&program).expect_err("should reject");
        assert!(errs
            .iter()
            .any(|e| matches!(e, AnalyzerError::ImmutableAssignment(n) if n == "x")));
    }

    #[test]
    fn immutable_compound_assign_rejected() {
        let program = Program {
            statements: vec![
                let_stmt("x", false, Expr::Literal(Literal::Int(1))),
                compound_assign("x"),
            ],
        };
        let mut a = Analyzer::new();
        let errs = a.analyze(&program).expect_err("should reject");
        assert!(errs
            .iter()
            .any(|e| matches!(e, AnalyzerError::ImmutableAssignment(n) if n == "x")));
    }

    #[test]
    fn mutable_assignment_allowed() {
        let program = Program {
            statements: vec![
                let_stmt("x", true, Expr::Literal(Literal::Int(1))),
                assign("x", Expr::Literal(Literal::Int(2))),
            ],
        };
        let mut a = Analyzer::new();
        a.analyze(&program).expect("var x should be assignable");
    }

    #[test]
    fn const_assignment_rejected() {
        let program = Program {
            statements: vec![
                Stmt::Const {
                    name: "c".to_string(),
                    type_annotation: None,
                    value: Expr::Literal(Literal::Int(1)),
                },
                assign("c", Expr::Literal(Literal::Int(2))),
            ],
        };
        let mut a = Analyzer::new();
        let errs = a.analyze(&program).expect_err("should reject");
        assert!(errs
            .iter()
            .any(|e| matches!(e, AnalyzerError::ConstantAssignment(n) if n == "c")));
    }

    #[test]
    fn let_pattern_defines_names_immutable() {
        let program = Program {
            statements: vec![
                Stmt::Let {
                    name: String::new(),
                    mutable: false,
                    type_annotation: None,
                    value: Some(Expr::Tuple(vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Int(2)),
                    ])),
                    pattern: Some(Pattern::Tuple(vec![
                        Pattern::Identifier("a".to_string()),
                        Pattern::Identifier("b".to_string()),
                    ])),
                },
                assign("a", Expr::Literal(Literal::Int(9))),
            ],
        };
        let mut a = Analyzer::new();
        let errs = a.analyze(&program).expect_err("should reject assign");
        assert!(errs
            .iter()
            .any(|e| matches!(e, AnalyzerError::ImmutableAssignment(n) if n == "a")));
        assert!(!errs
            .iter()
            .any(|e| matches!(e, AnalyzerError::UndefinedVariable(_))));
    }

    #[test]
    fn var_pattern_is_mutable() {
        let program = Program {
            statements: vec![
                Stmt::Let {
                    name: String::new(),
                    mutable: true,
                    type_annotation: None,
                    value: Some(Expr::Array(vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Int(2)),
                    ])),
                    pattern: Some(Pattern::Array(vec![
                        Pattern::Identifier("x".to_string()),
                        Pattern::Identifier("y".to_string()),
                    ])),
                },
                assign("x", Expr::Literal(Literal::Int(9))),
            ],
        };
        let mut a = Analyzer::new();
        a.analyze(&program)
            .expect("var pattern bindings are mutable");
    }

    fn fn_def(name: &str, return_type: Option<TypeAnnotation>, body: Vec<Stmt>) -> Stmt {
        Stmt::FunctionDef {
            name: name.to_string(),
            type_params: Vec::new(),
            params: Vec::new(),
            return_type,
            body,
        }
    }

    fn parse_prog(src: &str) -> Program {
        let mut p = mailang_parser::Parser::new(src).expect("lex");
        p.parse_program().expect("parse")
    }

    #[test]
    fn generic_id_accepts_any_type() {
        // Parametric: T matches any argument without turbofish.
        let program = Program {
            statements: vec![
                Stmt::FunctionDef {
                    name: "id".to_string(),
                    type_params: vec!["T".to_string()],
                    params: vec![Param {
                        name: "x".to_string(),
                        type_annotation: Some(TypeAnnotation::Param("T".to_string())),
                        default: None,
                    }],
                    return_type: Some(TypeAnnotation::Param("T".to_string())),
                    body: vec![ret(Expr::Identifier("x".to_string()))],
                },
                Stmt::Expression(Expr::Call {
                    callee: Box::new(Expr::Identifier("id".to_string())),
                    args: vec![Expr::Literal(Literal::Int(3))],
                    type_args: Vec::new(),
                }),
            ],
        };
        let mut a = Analyzer::new();
        a.analyze(&program).expect("generic id accepts int");
    }

    #[test]
    fn turbofish_type_arg_mismatch_rejected() {
        // id::<str>(3): T=str but the argument is int.
        let program = Program {
            statements: vec![
                Stmt::FunctionDef {
                    name: "id".to_string(),
                    type_params: vec!["T".to_string()],
                    params: vec![Param {
                        name: "x".to_string(),
                        type_annotation: Some(TypeAnnotation::Param("T".to_string())),
                        default: None,
                    }],
                    return_type: Some(TypeAnnotation::Param("T".to_string())),
                    body: vec![ret(Expr::Identifier("x".to_string()))],
                },
                Stmt::Expression(Expr::Call {
                    callee: Box::new(Expr::Identifier("id".to_string())),
                    args: vec![Expr::Literal(Literal::Int(3))],
                    type_args: vec![TypeAnnotation::Str],
                }),
            ],
        };
        let mut a = Analyzer::new();
        let errs = a
            .analyze(&program)
            .expect_err("id::<str>(3) should be rejected");
        assert!(errs
            .iter()
            .any(|e| matches!(e, AnalyzerError::TypeMismatch { .. })));
    }

    #[test]
    fn turbofish_wrong_type_arg_count_rejected() {
        let program = Program {
            statements: vec![
                Stmt::FunctionDef {
                    name: "id".to_string(),
                    type_params: vec!["T".to_string()],
                    params: vec![Param {
                        name: "x".to_string(),
                        type_annotation: Some(TypeAnnotation::Param("T".to_string())),
                        default: None,
                    }],
                    return_type: Some(TypeAnnotation::Param("T".to_string())),
                    body: vec![ret(Expr::Identifier("x".to_string()))],
                },
                Stmt::Expression(Expr::Call {
                    callee: Box::new(Expr::Identifier("id".to_string())),
                    args: vec![Expr::Literal(Literal::Int(3))],
                    type_args: vec![TypeAnnotation::Int, TypeAnnotation::Str],
                }),
            ],
        };
        let mut a = Analyzer::new();
        let errs = a
            .analyze(&program)
            .expect_err("id::<int,str>(3) should be rejected");
        assert!(errs.iter().any(|e| matches!(
            e,
            AnalyzerError::WrongTypeArgumentCount {
                expected: 1,
                found: 2,
                ..
            }
        )));
    }

    #[test]
    fn pair_two_type_params_checked() {
        // pair::<int,str>(1,"a") ok; pair::<int,str>("a",1) mismatch.
        let ok = Program {
            statements: vec![
                Stmt::FunctionDef {
                    name: "pair".to_string(),
                    type_params: vec!["A".to_string(), "B".to_string()],
                    params: vec![
                        Param {
                            name: "a".to_string(),
                            type_annotation: Some(TypeAnnotation::Param("A".to_string())),
                            default: None,
                        },
                        Param {
                            name: "b".to_string(),
                            type_annotation: Some(TypeAnnotation::Param("B".to_string())),
                            default: None,
                        },
                    ],
                    return_type: Some(TypeAnnotation::Tuple(vec![
                        TypeAnnotation::Param("A".to_string()),
                        TypeAnnotation::Param("B".to_string()),
                    ])),
                    body: vec![ret(Expr::Tuple(vec![
                        Expr::Identifier("a".to_string()),
                        Expr::Identifier("b".to_string()),
                    ]))],
                },
                Stmt::Expression(Expr::Call {
                    callee: Box::new(Expr::Identifier("pair".to_string())),
                    args: vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Str("a".to_string())),
                    ],
                    type_args: vec![TypeAnnotation::Int, TypeAnnotation::Str],
                }),
            ],
        };
        let mut a = Analyzer::new();
        a.analyze(&ok).expect("pair::<int,str>(1,\"a\") ok");

        let bad = Program {
            statements: vec![
                ok.statements[0].clone(),
                Stmt::Expression(Expr::Call {
                    callee: Box::new(Expr::Identifier("pair".to_string())),
                    args: vec![
                        Expr::Literal(Literal::Str("a".to_string())),
                        Expr::Literal(Literal::Int(1)),
                    ],
                    type_args: vec![TypeAnnotation::Int, TypeAnnotation::Str],
                }),
            ],
        };
        let mut a = Analyzer::new();
        let errs = a
            .analyze(&bad)
            .expect_err("pair::<int,str>(\"a\",1) rejected");
        assert!(errs
            .iter()
            .any(|e| matches!(e, AnalyzerError::TypeMismatch { .. })));
    }

    #[test]
    fn box_generic_class_annotation() {
        // class Box<T> { let val: T } + let b: Box<int> = ...
        let program = Program {
            statements: vec![
                Stmt::ClassDef {
                    name: "Box".to_string(),
                    type_params: vec!["T".to_string()],
                    superclass: None,
                    traits: vec![],
                    members: vec![ClassMember::Property {
                        name: "val".to_string(),
                        mutable: false,
                        type_annotation: Some(TypeAnnotation::Param("T".to_string())),
                        default: None,
                    }],
                },
                Stmt::Let {
                    name: "b".to_string(),
                    mutable: false,
                    type_annotation: Some(TypeAnnotation::Apply(
                        "Box".to_string(),
                        vec![TypeAnnotation::Int],
                    )),
                    value: None,
                    pattern: None,
                },
            ],
        };
        let mut a = Analyzer::new();
        a.analyze(&program).expect("Box<int> annotation accepted");
    }

    #[test]
    fn source_generics_parse_and_analyze() {
        let src = "fn id<T>(x: T) -> T {\nreturn x\n}\nlet a = id::<int>(3)\n";
        let program = parse_prog(src);
        let mut a = Analyzer::new();
        a.analyze(&program).expect("source generics analyze");
    }

    fn ret(expr: Expr) -> Stmt {
        Stmt::Return(Some(expr))
    }

    #[test]
    fn return_type_mismatch_rejected() {
        let program = Program {
            statements: vec![fn_def(
                "f",
                Some(TypeAnnotation::Int),
                vec![ret(Expr::Literal(Literal::Str("hi".to_string())))],
            )],
        };
        let mut a = Analyzer::new();
        let errs = a.analyze(&program).expect_err("str return vs int decl");
        assert!(errs.iter().any(|e| matches!(
            e,
            AnalyzerError::TypeMismatch { expected, found }
                if expected.contains("int") && found.contains("str")
        )));
    }

    #[test]
    fn return_int_into_float_ok() {
        // Int ⊂ Float is compatible.
        let program = Program {
            statements: vec![fn_def(
                "f",
                Some(TypeAnnotation::Float),
                vec![ret(Expr::Literal(Literal::Int(1)))],
            )],
        };
        let mut a = Analyzer::new();
        a.analyze(&program).expect("int return fits float decl");
    }

    #[test]
    fn missing_return_is_hint() {
        let program = Program {
            statements: vec![fn_def(
                "f",
                Some(TypeAnnotation::Int),
                vec![let_stmt("x", true, Expr::Literal(Literal::Int(1)))],
            )],
        };
        let mut a = Analyzer::new();
        // Warn-level only: analyze still succeeds.
        a.analyze(&program).expect("missing return is a hint");
        let diags = a.diagnostics();
        assert!(
            diags
                .iter()
                .any(|e| matches!(e, AnalyzerError::MissingReturn(n) if n == "f")),
            "expected MissingReturn hint, got {diags:?}"
        );
    }

    #[test]
    fn non_exhaustive_match_is_hard_error() {
        // No wildcard/identifier arm and patterns cannot cover Result/Option.
        let program = Program {
            statements: vec![Stmt::Expression(Expr::Match {
                scrutinee: Box::new(Expr::Literal(Literal::Int(1))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::Int(2)),
                        guard: None,
                        body: Expr::Literal(Literal::Str("a".to_string())),
                    },
                    MatchArm {
                        pattern: Pattern::Literal(Literal::Int(3)),
                        guard: None,
                        body: Expr::Literal(Literal::Str("b".to_string())),
                    },
                ],
            })],
        };
        let mut a = Analyzer::new();
        let errs = a.analyze(&program).expect_err("non-exhaustive match");
        assert!(errs
            .iter()
            .any(|e| matches!(e, AnalyzerError::NonExhaustiveMatch)));
    }

    #[test]
    fn exhaustive_match_wildcard_ok() {
        let program = Program {
            statements: vec![Stmt::Expression(Expr::Match {
                scrutinee: Box::new(Expr::Literal(Literal::Int(1))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::Int(2)),
                        guard: None,
                        body: Expr::Literal(Literal::Str("a".to_string())),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Expr::Literal(Literal::Str("b".to_string())),
                    },
                ],
            })],
        };
        let mut a = Analyzer::new();
        a.analyze(&program).expect("wildcard arm is catch-all");
    }

    #[test]
    fn result_ok_err_arms_are_exhaustive() {
        let program = Program {
            statements: vec![Stmt::Expression(Expr::Match {
                scrutinee: Box::new(Expr::Ok(Box::new(Expr::Literal(Literal::Int(1))))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Ok(Box::new(Pattern::Identifier("v".to_string()))),
                        guard: None,
                        body: Expr::Identifier("v".to_string()),
                    },
                    MatchArm {
                        pattern: Pattern::Err(Box::new(Pattern::Wildcard)),
                        guard: None,
                        body: Expr::Literal(Literal::Int(0)),
                    },
                ],
            })],
        };
        let mut a = Analyzer::new();
        a.analyze(&program).expect("Ok+Err covers Result");
    }
}
