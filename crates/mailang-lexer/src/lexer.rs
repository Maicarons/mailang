use crate::error::LexerError;
use crate::token::Token;
use unicode_xid::UnicodeXID;

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.source.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.source.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // skip *
        self.advance(); // skip *

        let mut depth = 1;
        while depth > 0 {
            match self.peek() {
                Some('*') if self.peek_at(1) == Some('*') => {
                    self.advance();
                    self.advance();
                    depth -= 1;
                }
                Some('*') if self.peek_at(1) == Some('/') => {
                    return Err(LexerError::UnexpectedCharacter('*', self.line, self.column));
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    self.advance();
                    self.advance();
                    depth += 1;
                }
                Some(_) => {
                    self.advance();
                }
                None => {
                    return Err(LexerError::UnexpectedCharacter('*', start_line, start_col));
                }
            }
        }
        Ok(())
    }

    fn read_string(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // skip opening quote

        let mut result = String::new();

        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    return Ok(Token::String(result));
                }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n') => result.push('\n'),
                        Some('r') => result.push('\r'),
                        Some('t') => result.push('\t'),
                        Some('\\') => result.push('\\'),
                        Some('"') => result.push('"'),
                        Some('0') => result.push('\0'),
                        Some('u') => {
                            self.advance(); // skip {
                            let mut hex = String::new();
                            loop {
                                match self.peek() {
                                    Some('}') => {
                                        self.advance();
                                        break;
                                    }
                                    Some(ch) if ch.is_ascii_hexdigit() => {
                                        hex.push(ch);
                                        self.advance();
                                    }
                                    _ => {
                                        return Err(LexerError::InvalidUnicodeEscape(
                                            self.line,
                                            self.column,
                                        ));
                                    }
                                }
                            }
                            let codepoint = u32::from_str_radix(&hex, 16)
                                .map_err(|_| LexerError::InvalidUnicodeEscape(self.line, self.column))?;
                            let ch = char::from_u32(codepoint)
                                .ok_or(LexerError::InvalidUnicodeEscape(self.line, self.column))?;
                            result.push(ch);
                        }
                        Some(ch) => {
                            return Err(LexerError::InvalidEscapeSequence(ch, self.line, self.column));
                        }
                        None => {
                            return Err(LexerError::UnterminatedString(start_line, start_col));
                        }
                    }
                }
                Some('{') if self.peek_at(1) == Some('{') => {
                    self.advance();
                    self.advance();
                    result.push('{');
                }
                Some('{') => {
                    self.advance();
                    // TODO: Handle string interpolation
                    result.push('{');
                }
                Some(ch) => {
                    result.push(ch);
                    self.advance();
                }
                None => {
                    return Err(LexerError::UnterminatedString(start_line, start_col));
                }
            }
        }
    }

    fn read_char(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // skip opening quote

        let ch = match self.advance() {
            Some('\\') => match self.advance() {
                Some('n') => '\n',
                Some('r') => '\r',
                Some('t') => '\t',
                Some('\\') => '\\',
                Some('\'') => '\'',
                Some('0') => '\0',
                Some(ch) => {
                    return Err(LexerError::InvalidEscapeSequence(ch, self.line, self.column));
                }
                None => {
                    return Err(LexerError::UnterminatedChar(start_line, start_col));
                }
            },
            Some(ch) => ch,
            None => {
                return Err(LexerError::UnterminatedChar(start_line, start_col));
            }
        };

        match self.peek() {
            Some('\'') => {
                self.advance();
                Ok(Token::Char(ch))
            }
            _ => Err(LexerError::UnterminatedChar(start_line, start_col)),
        }
    }

    /// Check if we should parse a negative number (i.e., `-` is a unary minus, not a binary operator)
    fn should_parse_negative(&self) -> bool {
        // If at the start, parse as negative
        if self.pos <= 1 {
            return true;
        }
        // Look at the previous non-whitespace character
        let mut i = self.pos - 1;
        while i > 0 && self.source[i] == ' ' {
            i -= 1;
        }
        let prev = self.source[i];
        // After an operator, opening paren/bracket, comma, or start - parse as negative
        // After an identifier, digit, or closing paren/bracket - parse as minus operator
        match prev {
            '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '~' | '!' | '=' | '<' | '>'
            | '(' | '[' | '{' | ',' | ':' | ';' | '\n' | '\r' | '\t' | ' ' => true,
            _ => false,
        }
    }

    fn read_number(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        let mut num_str = String::new();
        let mut is_float = false;

        // Handle negative numbers
        if self.peek() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        // Read digits
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !is_float {
                if let Some(next) = self.peek_at(1) {
                    if next.is_ascii_digit() {
                        is_float = true;
                        num_str.push('.');
                        self.advance();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else if ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        if is_float {
            num_str
                .parse::<f64>()
                .map(Token::Float)
                .map_err(|_| LexerError::InvalidNumber(start_line, start_col))
        } else {
            num_str
                .parse::<i64>()
                .map(Token::Integer)
                .map_err(|_| LexerError::InvalidNumber(start_line, start_col))
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();

        while let Some(ch) = self.peek() {
            if ch.is_xid_continue() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        ident
    }

    fn keyword_or_ident(ident: &str) -> Token {
        match ident {
            "let" => Token::Let,
            "var" => Token::Var,
            "const" => Token::Const,
            "fn" => Token::Fn,
            "class" => Token::Class,
            "extends" => Token::Extends,
            "implements" => Token::Implements,
            "trait" => Token::Trait,
            "if" => Token::If,
            "elif" => Token::Elif,
            "else" => Token::Else,
            "for" => Token::For,
            "in" => Token::In,
            "while" => Token::While,
            "return" => Token::Return,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "match" => Token::Match,
            "import" => Token::Import,
            "module" => Token::Module,
            "pub" => Token::Pub,
            "this" => Token::This,
            "super" => Token::Super,
            "new" => Token::New,
            "override" => Token::Override,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            "null" => Token::Null,
            "Ok" => Token::Ok,
            "Err" => Token::Err,
            "Some" => Token::Some,
            "None" => Token::None,
            _ => Token::Identifier(ident.to_string()),
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            match self.peek() {
                None => {
                    tokens.push(Token::Eof);
                    break;
                }
                Some('\n') => {
                    self.advance();
                    tokens.push(Token::Newline);
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    self.skip_line_comment();
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    self.advance();
                    self.advance();
                    self.skip_block_comment()?;
                }
                Some('"') => {
                    let token = self.read_string()?;
                    tokens.push(token);
                }
                Some('\'') => {
                    let token = self.read_char()?;
                    tokens.push(token);
                }
                Some(ch) if ch.is_ascii_digit() || (ch == '-' && self.peek_at(1).map_or(false, |c| c.is_ascii_digit()) && self.should_parse_negative()) => {
                    let token = self.read_number()?;
                    tokens.push(token);
                }
                Some(ch) if ch.is_xid_start() || ch == '_' => {
                    let ident = self.read_identifier();
                    let token = Self::keyword_or_ident(&ident);
                    tokens.push(token);
                }
                Some('+') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::PlusAssign);
                    } else {
                        tokens.push(Token::Plus);
                    }
                }
                Some('-') => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        tokens.push(Token::Arrow);
                    } else if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::MinusAssign);
                    } else {
                        tokens.push(Token::Minus);
                    }
                }
                Some('*') => {
                    self.advance();
                    if self.peek() == Some('*') {
                        self.advance();
                        tokens.push(Token::StarStar);
                    } else if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::StarAssign);
                    } else {
                        tokens.push(Token::Star);
                    }
                }
                Some('/') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::SlashAssign);
                    } else {
                        tokens.push(Token::Slash);
                    }
                }
                Some('%') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::PercentAssign);
                    } else {
                        tokens.push(Token::Percent);
                    }
                }
                Some('&') => {
                    self.advance();
                    if self.peek() == Some('&') {
                        self.advance();
                        tokens.push(Token::And);
                    } else if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::AmpersandAssign);
                    } else {
                        tokens.push(Token::Ampersand);
                    }
                }
                Some('|') => {
                    self.advance();
                    if self.peek() == Some('|') {
                        self.advance();
                        tokens.push(Token::Or);
                    } else if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::PipeAssign);
                    } else {
                        tokens.push(Token::Pipe);
                    }
                }
                Some('^') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::CaretAssign);
                    } else {
                        tokens.push(Token::Caret);
                    }
                }
                Some('~') => {
                    self.advance();
                    tokens.push(Token::Tilde);
                }
                Some('<') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::LessEqual);
                    } else if self.peek() == Some('<') {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push(Token::LessLessAssign);
                        } else {
                            tokens.push(Token::LessLess);
                        }
                    } else {
                        tokens.push(Token::Less);
                    }
                }
                Some('>') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::GreaterEqual);
                    } else if self.peek() == Some('>') {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push(Token::GreaterGreaterAssign);
                        } else {
                            tokens.push(Token::GreaterGreater);
                        }
                    } else {
                        tokens.push(Token::Greater);
                    }
                }
                Some('=') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::Equal);
                    } else if self.peek() == Some('>') {
                        self.advance();
                        tokens.push(Token::FatArrow);
                    } else {
                        tokens.push(Token::Assign);
                    }
                }
                Some('!') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token::NotEqual);
                    } else {
                        tokens.push(Token::Not);
                    }
                }
                Some('(') => {
                    self.advance();
                    tokens.push(Token::LeftParen);
                }
                Some(')') => {
                    self.advance();
                    tokens.push(Token::RightParen);
                }
                Some('{') => {
                    self.advance();
                    tokens.push(Token::LeftBrace);
                }
                Some('}') => {
                    self.advance();
                    tokens.push(Token::RightBrace);
                }
                Some('[') => {
                    self.advance();
                    tokens.push(Token::LeftBracket);
                }
                Some(']') => {
                    self.advance();
                    tokens.push(Token::RightBracket);
                }
                Some(',') => {
                    self.advance();
                    tokens.push(Token::Comma);
                }
                Some('.') => {
                    self.advance();
                    if self.peek() == Some('.') {
                        self.advance();
                        tokens.push(Token::DotDot);
                    } else {
                        tokens.push(Token::Dot);
                    }
                }
                Some(':') => {
                    self.advance();
                    tokens.push(Token::Colon);
                }
                Some(';') => {
                    self.advance();
                    tokens.push(Token::Semicolon);
                }
                Some('?') => {
                    self.advance();
                    tokens.push(Token::Question);
                }
                Some(ch) => {
                    return Err(LexerError::UnexpectedCharacter(ch, self.line, self.column));
                }
            }
        }

        Ok(tokens)
    }
}
