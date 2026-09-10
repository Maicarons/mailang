use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum CompilerError {
    #[error("Undefined variable '{0}'")]
    UndefinedVariable(String),

    #[error("Too many constants (max 65536)")]
    TooManyConstants,

    #[error("Too many locals (max 256)")]
    TooManyLocals,

    #[error("Too many upvalues (max 256)")]
    TooManyUpvalues,

    #[error("Invalid assignment target")]
    InvalidAssignmentTarget,

    #[error("Break outside of loop")]
    BreakOutsideLoop,

    #[error("Continue outside of loop")]
    ContinueOutsideLoop,

    #[error("Return outside of function")]
    ReturnOutsideFunction,

    #[error("Duplicate function name '{0}'")]
    DuplicateFunction(String),

    #[error("Duplicate class name '{0}'")]
    DuplicateClass(String),

    #[error("Duplicate trait name '{0}'")]
    DuplicateTrait(String),

    #[error("Internal compiler error: {0}")]
    Internal(String),
}
