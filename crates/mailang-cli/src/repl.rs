//! REPL helpers: simple multi-line continuation when a line ends with `{` or `\`.

/// True when the REPL should keep reading another line.
///
/// Continuation rules (intentionally small):
/// - line ends with `{` → block open
/// - line ends with `\` → soft join (backslash is stripped)
/// - unbalanced `{` / `}` still open → keep reading
pub fn needs_continuation(buf: &str) -> bool {
    let trimmed = buf.trim_end();
    if trimmed.ends_with('\\') {
        return true;
    }
    if brace_depth(trimmed) > 0 {
        return true;
    }
    // A trailing `{` is covered by brace_depth; also continue on bare open.
    trimmed.ends_with('{')
}

/// Strip a trailing soft-join backslash (and its trailing whitespace).
pub fn strip_soft_join(line: &str) -> String {
    let t = line.trim_end();
    if t.ends_with('\\') {
        let mut s = t[..t.len() - 1].to_string();
        while s.ends_with(' ') || s.ends_with('\t') {
            s.pop();
        }
        s
    } else {
        line.trim_end().to_string()
    }
}

/// Net open-brace count (naive; ignores braces in strings — fine for REPL).
pub fn brace_depth(s: &str) -> i32 {
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
    }
    depth
}

/// Append `line` to `buf`, handling soft-join (`\`).
/// Returns the updated buffer (with newline separators).
pub fn append_line(buf: &str, line: &str) -> String {
    let piece = strip_soft_join(line);
    if buf.is_empty() {
        piece
    } else {
        format!("{}\n{}", buf, piece)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continues_on_brace() {
        assert!(needs_continuation("fn f() {"));
        assert!(needs_continuation("while i < 3 {"));
        assert!(needs_continuation("fn f() {\n  return 1"));
        assert!(!needs_continuation("fn f() { return 1 }"));
        assert!(!needs_continuation("1 + 2"));
    }

    #[test]
    fn continues_on_backslash() {
        assert!(needs_continuation("let x = 1 + \\"));
        assert!(!needs_continuation("let x = 1"));
    }

    #[test]
    fn soft_join_strips_backslash() {
        assert_eq!(strip_soft_join("let x = 1 + \\"), "let x = 1 +");
        assert_eq!(append_line("", "let x = 1 + \\"), "let x = 1 +");
        assert_eq!(append_line("let x = 1 +", "  2"), "let x = 1 +\n  2");
    }

    #[test]
    fn brace_depth_tracks_blocks() {
        assert_eq!(brace_depth("fn f() {"), 1);
        assert_eq!(brace_depth("fn f() { }"), 0);
        assert_eq!(brace_depth("{{ }"), 1);
    }
}
