use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum LexerError {
    #[error("Unexpected character '{0}' at line {1}, column {2}")]
    UnexpectedCharacter(char, usize, usize),

    #[error("Unterminated string at line {0}, column {1}")]
    UnterminatedString(usize, usize),

    #[error("Unterminated char at line {0}, column {1}")]
    UnterminatedChar(usize, usize),

    #[error("Invalid escape sequence '\\{0}' at line {1}, column {2}")]
    InvalidEscapeSequence(char, usize, usize),

    #[error("Invalid number format at line {0}, column {1}")]
    InvalidNumber(usize, usize),

    #[error("Invalid Unicode escape at line {0}, column {1}")]
    InvalidUnicodeEscape(usize, usize),

    #[error("Unexpected end of file")]
    UnexpectedEof,
}
