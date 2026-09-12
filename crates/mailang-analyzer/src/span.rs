//! Lightweight source positions for analyzer diagnostics.

use crate::error::AnalyzerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    /// 0-based line
    pub line: u32,
    /// 0-based column
    pub col: u32,
    pub severity: Severity,
}

impl Diagnostic {
    pub fn from_error(err: &AnalyzerError) -> Self {
        Self {
            message: err.to_string(),
            line: 0,
            col: 0,
            severity: Severity::Error,
        }
    }

    pub fn with_pos(mut self, line: u32, col: u32) -> Self {
        self.line = line;
        self.col = col;
        self
    }
}

/// Find the first identifier occurrence in `source` (word-boundary match).
pub fn locate_identifier(source: &str, name: &str) -> Option<(u32, u32)> {
    let bytes = source.as_bytes();
    let name_bytes = name.as_bytes();
    if name_bytes.is_empty() {
        return None;
    }
    let mut line = 0u32;
    let mut col = 0u32;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' {
            line += 1;
            col = 0;
            i += 1;
            continue;
        }
        let is_ident_start = b.is_ascii_alphabetic() || b == b'_' || b >= 0x80;
        if is_ident_start {
            let start = i;
            let start_col = col;
            while i < bytes.len() {
                let c = bytes[i];
                if c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80 {
                    i += 1;
                    col += 1;
                } else {
                    break;
                }
            }
            let word = &source[start..i];
            if word == name {
                return Some((line, start_col));
            }
        } else {
            i += 1;
            col += 1;
        }
    }
    None
}

/// Attach source positions to analyzer errors (best-effort identifier search).
pub fn diagnose(source: &str, errors: &[AnalyzerError]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|e| {
            let mut d = Diagnostic::from_error(e);
            if let AnalyzerError::UndefinedVariable(name) = e {
                if let Some((line, col)) = locate_identifier(source, name) {
                    d = d.with_pos(line, col);
                }
            }
            d
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locates_identifier() {
        let src = "let x = 1\nfoo_bar + 2\n";
        assert_eq!(locate_identifier(src, "foo_bar"), Some((1, 0)));
        assert_eq!(locate_identifier(src, "x"), Some((0, 4)));
        assert_eq!(locate_identifier(src, "missing"), None);
    }
}
