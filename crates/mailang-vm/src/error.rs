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

    #[error("Call depth exceeded (limit {limit})")]
    CallDepthExceeded { limit: usize },

    #[error("Fuel exhausted (execution budget spent)")]
    FuelExhausted,

    /// Error decorated with source location and a call-stack trace.
    #[error("{message}\n  at {location}{trace}")]
    Located {
        message: String,
        location: String,
        trace: String,
    },
}

impl VmError {
    /// Wrap `self` with a source location and call-stack trace (if not already located).
    pub fn located(self, location: impl Into<String>, frames: Vec<String>) -> Self {
        if matches!(self, VmError::Located { .. }) {
            return self;
        }
        let message = self.to_string();
        let trace = if frames.is_empty() {
            String::new()
        } else {
            let mut s = String::new();
            for f in frames {
                s.push_str("\n    ");
                s.push_str(&f);
            }
            s
        };
        VmError::Located {
            message,
            location: location.into(),
            trace,
        }
    }
}
