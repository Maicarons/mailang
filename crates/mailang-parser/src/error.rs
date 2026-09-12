use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("Expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String },

    #[error("Unexpected end of file")]
    UnexpectedEof,

    #[error("Invalid expression at line {0}, column {1}")]
    InvalidExpression(usize, usize),

    #[error("Invalid statement at line {0}, column {1}")]
    InvalidStatement(usize, usize),

    #[error("Expected identifier, found {0}")]
    ExpectedIdentifier(String),

    #[error("Expected type annotation, found {0}")]
    ExpectedTypeAnnotation(String),

    #[error("Expected expression, found {0}")]
    ExpectedExpression(String),

    #[error("Expected pattern, found {0}")]
    ExpectedPattern(String),

    #[error("Expected statement, found {0}")]
    ExpectedStatement(String),

    #[error("Expected block, found {0}")]
    ExpectedBlock(String),

    #[error("Expected parameter, found {0}")]
    ExpectedParameter(String),

    #[error("Expected class member, found {0}")]
    ExpectedClassMember(String),

    #[error("Expected trait method, found {0}")]
    ExpectedTraitMethod(String),

    #[error("Expected import path, found {0}")]
    ExpectedImportPath(String),

    #[error("Invalid assignment target at line {0}, column {1}")]
    InvalidAssignmentTarget(usize, usize),

    #[error("Invalid function definition at line {0}, column {1}")]
    InvalidFunctionDefinition(usize, usize),

    #[error("Invalid class definition at line {0}, column {1}")]
    InvalidClassDefinition(usize, usize),

    #[error("Invalid trait definition at line {0}, column {1}")]
    InvalidTraitDefinition(usize, usize),

    #[error("Invalid module definition at line {0}, column {1}")]
    InvalidModuleDefinition(usize, usize),

    #[error("Invalid import statement at line {0}, column {1}")]
    InvalidImportStatement(usize, usize),

    #[error("Invalid pattern at line {0}, column {1}")]
    InvalidPattern(usize, usize),

    #[error("Invalid match arm at line {0}, column {1}")]
    InvalidMatchArm(usize, usize),

    #[error("Invalid for loop at line {0}, column {1}")]
    InvalidForLoop(usize, usize),

    #[error("Invalid while loop at line {0}, column {1}")]
    InvalidWhileLoop(usize, usize),

    #[error("Invalid if statement at line {0}, column {1}")]
    InvalidIfStatement(usize, usize),

    #[error("Invalid return statement at line {0}, column {1}")]
    InvalidReturnStatement(usize, usize),
}
