use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AnalyzerError {
    #[error("Undefined variable '{0}'")]
    UndefinedVariable(String),

    #[error("Undefined function '{0}'")]
    UndefinedFunction(String),

    #[error("Undefined class '{0}'")]
    UndefinedClass(String),

    #[error("Undefined trait '{0}'")]
    UndefinedTrait(String),

    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("Cannot assign to immutable variable '{0}'")]
    ImmutableAssignment(String),

    #[error("Cannot assign to constant '{0}'")]
    ConstantAssignment(String),

    #[error("Wrong number of arguments for function '{name}': expected {expected}, found {found}")]
    WrongArgumentCount {
        name: String,
        expected: usize,
        found: usize,
    },

    #[error("Wrong number of type arguments for '{name}': expected {expected}, found {found}")]
    WrongTypeArgumentCount {
        name: String,
        expected: usize,
        found: usize,
    },

    #[error("Method '{method}' not found on type '{type_name}'")]
    MethodNotFound { method: String, type_name: String },

    #[error("Property '{property}' not found on type '{type_name}'")]
    PropertyNotFound { property: String, type_name: String },

    #[error(
        "Class '{class}' does not implement required method '{method}' from trait '{trait_name}'"
    )]
    MissingTraitMethod {
        class: String,
        method: String,
        trait_name: String,
    },

    #[error("Circular inheritance detected for class '{0}'")]
    CircularInheritance(String),

    #[error("Cannot use 'this' outside of a method")]
    ThisOutsideMethod,

    #[error("Cannot use 'super' outside of a subclass method")]
    SuperOutsideSubclass,

    #[error("Invalid pattern in match expression")]
    InvalidPattern,

    #[error("Unreachable code detected")]
    UnreachableCode,

    #[error("Unused variable '{0}'")]
    UnusedVariable(String),

    #[error("Duplicate definition of '{0}'")]
    DuplicateDefinition(String),

    #[error(
        "Non-exhaustive match: no wildcard `_` / catch-all arm and patterns do not cover all cases"
    )]
    NonExhaustiveMatch,

    #[error(
        "Function '{0}' declares a non-null return type but falls off the end without a return"
    )]
    MissingReturn(String),
}
