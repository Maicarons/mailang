use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum VmError {
    #[error("Stack overflow")]
    StackOverflow,

    #[error("Stack underflow")]
    StackUnderflow,

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Undefined variable '{0}'")]
    UndefinedVariable(String),

    #[error("Undefined function '{0}'")]
    UndefinedFunction(String),

    #[error("Undefined property '{0}'")]
    UndefinedProperty(String),

    #[error("Wrong number of arguments: expected {expected}, found {found}")]
    WrongArgumentCount { expected: usize, found: usize },

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Index out of bounds: index {index}, length {length}")]
    IndexOutOfBounds { index: i64, length: usize },

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Internal VM error: {0}")]
    Internal(String),
}
