use crate::error::AnalyzerError;
use mailang_ast::*;
use std::collections::HashMap;

/// Loose compatibility: Any/Unknown match anything; Int ⊂ Float.
fn types_compatible(expected: &Type, found: &Type) -> bool {
    match (expected, found) {
        (Type::Any, _) | (_, Type::Any) => true,
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::Float, Type::Int) => true,
        _ => expected == found,
    }
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
    used: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // reserved for typed analysis
struct FunctionInfo {
    name: String,
    params: Vec<(String, Type)>,
    return_type: Type,
    /// False for host/stdlib builtins (arity is dynamic).
    declared: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // reserved for typed analysis
struct ClassInfo {
    name: String,
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
                params: Vec::new(),
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
            "read_file",
            "write_file",
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
        let scope = &mut self.scopes[self.current_scope];
        scope.variables.insert(
            name.clone(),
            Variable {
                name,
                ty,
                mutable,
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

        // Drop unused-variable noise from the hard-error list; they are hints.
        let hard: Vec<AnalyzerError> = self
            .errors
            .iter()
            .filter(|e| !matches!(e, AnalyzerError::UnusedVariable(_)))
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
            } => {
                let ty = if let Some(ta) = type_annotation {
                    self.resolve_type(ta)
                } else if let Some(val) = value {
                    self.infer_type(val)
                } else {
                    Type::Unknown
                };
                self.define_variable(name.clone(), ty, *mutable);
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
                self.define_variable(name.clone(), ty, false);
                self.analyze_expression(value);
            }
            Stmt::FunctionDef {
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

                self.functions.insert(
                    name.clone(),
                    FunctionInfo {
                        name: name.clone(),
                        params: params
                            .iter()
                            .zip(param_types.iter())
                            .map(|(p, t)| (p.name.clone(), t.clone()))
                            .collect(),
                        return_type: ret_type,
                        declared: true,
                    },
                );

                self.push_scope();
                for (param, ty) in params.iter().zip(param_types.iter()) {
                    self.define_variable(param.name.clone(), ty.clone(), true);
                }
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.pop_scope();
            }
            Stmt::ClassDef {
                name,
                superclass,
                traits,
                members,
            } => {
                self.analyze_class(name, superclass, traits, members);
            }
            Stmt::TraitDef { name, methods } => {
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
        superclass: &Option<String>,
        traits: &[String],
        members: &[ClassMember],
    ) {
        let mut class_info = ClassInfo {
            name: name.to_string(),
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
                            params: params
                                .iter()
                                .zip(param_types.iter())
                                .map(|(p, t)| (p.name.clone(), t.clone()))
                                .collect(),
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
                            params: params
                                .iter()
                                .zip(param_types.iter())
                                .map(|(p, t)| (p.name.clone(), t.clone()))
                                .collect(),
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
                            params: params
                                .iter()
                                .zip(param_types.iter())
                                .map(|(p, t)| (p.name.clone(), t.clone()))
                                .collect(),
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
            Expr::Call { callee, args } => {
                self.analyze_expression(callee);
                for arg in args {
                    self.analyze_expression(arg);
                }
                // Arity / simple type check for named user functions with annotations.
                if let Expr::Identifier(fname) = &**callee {
                    if let Some(info) = self.functions.get(fname) {
                        if info.declared {
                            let expected = info.params.len();
                            if expected != args.len() {
                                self.errors.push(AnalyzerError::WrongArgumentCount {
                                    name: fname.clone(),
                                    expected,
                                    found: args.len(),
                                });
                            } else {
                                for ((pname, pty), arg) in info.params.iter().zip(args.iter()) {
                                    let aty = self.infer_type(arg);
                                    if !types_compatible(pty, &aty) {
                                        self.errors.push(AnalyzerError::TypeMismatch {
                                            expected: format!("{}: {}", pname, pty),
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
                self.analyze_expression(target);
                self.analyze_expression(value);
            }
            Expr::CompoundAssign { target, value, .. } => {
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
            _ => Type::Any,
        }
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}
