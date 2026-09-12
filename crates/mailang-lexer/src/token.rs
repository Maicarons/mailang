#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Null,

    // Identifiers and keywords
    Identifier(String),

    // Keywords
    Let,
    Var,
    Const,
    Fn,
    Class,
    Extends,
    Implements,
    Trait,
    If,
    Elif,
    Else,
    For,
    In,
    While,
    Return,
    Break,
    Continue,
    Match,
    Import,
    Module,
    Pub,
    This,
    Super,
    New,
    Override,
    Ok,
    Err,
    Some,
    None,

    // Operators
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Percent,        // %
    StarStar,       // **
    Ampersand,      // &
    Pipe,           // |
    Caret,          // ^
    Tilde,          // ~
    LessLess,       // <<
    GreaterGreater, // >>

    // Comparison
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=

    // Logical
    And, // &&
    Or,  // ||
    Not, // !

    // Assignment
    Assign,               // =
    PlusAssign,           // +=
    MinusAssign,          // -=
    StarAssign,           // *=
    SlashAssign,          // /=
    PercentAssign,        // %=
    AmpersandAssign,      // &=
    PipeAssign,           // |=
    CaretAssign,          // ^=
    LessLessAssign,       // <<=
    GreaterGreaterAssign, // >>=

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Comma,        // ,
    Dot,          // .
    Colon,        // :
    Semicolon,    // ;
    Arrow,        // ->
    FatArrow,     // =>
    DotDot,       // ..
    Question,     // ?

    // Special
    Eof,
    Newline,
}
