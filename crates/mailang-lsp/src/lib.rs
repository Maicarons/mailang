use dashmap::DashMap;
use mailang_analyzer::Analyzer;
use mailang_parser::Parser;
use std::collections::HashMap;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<DashMap<Url, String>>,
}

fn parse_line_col(message: &str) -> Option<(u32, u32)> {
    // Patterns: "at line 3, column 5" or "line 3, col 5"
    let lower = message.to_lowercase();
    let line_idx = lower.find("line ")?;
    let rest = &lower[line_idx + 5..];
    let line: u32 = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()?;
    let col: u32 = if let Some(ci) = rest.find("column ") {
        rest[ci + 7..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .ok()
    } else if let Some(ci) = rest.find("col ") {
        rest[ci + 4..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .ok()
    } else {
        Some(0)
    }?;
    // AST/parser lines are 1-based; LSP is 0-based
    Some((line.saturating_sub(1u32), col.saturating_sub(1u32)))
}

fn diag_at(message: &str, severity: DiagnosticSeverity) -> Diagnostic {
    let (line, col) = parse_line_col(message).unwrap_or((0, 0));
    let end_col = col + 1;
    Diagnostic {
        range: Range::new(Position::new(line, col), Position::new(line, end_col)),
        severity: Some(severity),
        message: message.to_string(),
        source: Some("mailang".into()),
        ..Default::default()
    }
}

fn collect_diagnostics(source: &str) -> Vec<Diagnostic> {
    let mut parser = match Parser::new(source) {
        Ok(p) => p,
        Err(e) => return vec![diag_at(&e.to_string(), DiagnosticSeverity::ERROR)],
    };
    // Token-sync recovery: report every syntax error, then analyze the partial AST.
    let (program, parse_errors) = parser.parse_program_recovering();
    let mut diags: Vec<Diagnostic> = parse_errors
        .iter()
        .map(|e| diag_at(&e.to_string(), DiagnosticSeverity::ERROR))
        .collect();

    let mut analyzer = Analyzer::new();
    match analyzer.analyze(&program) {
        Ok(()) => {
            let unused: Vec<_> = analyzer
                .diagnostics()
                .into_iter()
                .filter(|e| matches!(e, mailang_analyzer::AnalyzerError::UnusedVariable(_)))
                .collect();
            diags.extend(
                mailang_analyzer::diagnose(source, &unused)
                    .into_iter()
                    .map(|d| {
                        let sev = match d.severity {
                            mailang_analyzer::Severity::Error => DiagnosticSeverity::ERROR,
                            mailang_analyzer::Severity::Warning => DiagnosticSeverity::WARNING,
                            mailang_analyzer::Severity::Information => {
                                DiagnosticSeverity::INFORMATION
                            }
                        };
                        Diagnostic {
                            range: Range::new(
                                Position::new(d.line, d.col),
                                Position::new(d.line, d.col + 1),
                            ),
                            severity: Some(sev),
                            message: d.message,
                            source: Some("mailang".into()),
                            ..Default::default()
                        }
                    }),
            );
        }
        Err(errs) => {
            diags.extend(
                mailang_analyzer::diagnose(source, &errs)
                    .into_iter()
                    .map(|d| Diagnostic {
                        range: Range::new(
                            Position::new(d.line, d.col),
                            Position::new(d.line, d.col + 1),
                        ),
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: d.message,
                        source: Some("mailang".into()),
                        ..Default::default()
                    }),
            );
        }
    }
    diags
}

fn builtin_completions() -> Vec<CompletionItem> {
    let names = [
        "println",
        "print",
        "input",
        "sqrt",
        "abs",
        "sin",
        "cos",
        "floor",
        "ceil",
        "round",
        "min",
        "max",
        "len",
        "to_string",
        "parse_int",
        "parse_float",
        "let",
        "var",
        "const",
        "fn",
        "if",
        "elif",
        "else",
        "while",
        "for",
        "in",
        "match",
        "return",
        "class",
        "trait",
        "extends",
        "implements",
        "super",
        "this",
        "Ok",
        "Err",
        "Some",
        "None",
        "gpio_write",
        "gpio_read",
        "delay_ms",
        "adc_read",
    ];
    names
        .iter()
        .map(|n| {
            let kind = match *n {
                "let" | "var" | "const" | "fn" | "if" | "elif" | "else" | "while" | "for"
                | "in" | "match" | "return" | "class" | "trait" | "extends" | "implements" => {
                    Some(CompletionItemKind::KEYWORD)
                }
                _ => Some(CompletionItemKind::FUNCTION),
            };
            CompletionItem {
                label: n.to_string(),
                kind,
                detail: Some("Ma矛Lang".into()),
                ..Default::default()
            }
        })
        .collect()
}

fn word_completions(source: &str, _position: Position) -> Vec<CompletionItem> {
    let mut items = builtin_completions();
    // Collect simple identifiers defined in the document
    let mut seen = std::collections::HashSet::new();
    for word in source
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty() && w.chars().next().unwrap().is_ascii_alphabetic())
    {
        if seen.insert(word.to_string()) && word.len() > 1 {
            items.push(CompletionItem {
                label: word.to_string(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("local".into()),
                ..Default::default()
            });
        }
    }
    items
}

// ---------------------------------------------------------------------------
// Identifier scan (word boundaries; skips comments / non-interpolated strings)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct IdentOcc {
    name: String,
    /// 0-based line
    line: u32,
    /// 0-based char column
    col: u32,
}

impl IdentOcc {
    fn range(&self) -> Range {
        let end = self.col + self.name.chars().count() as u32;
        Range::new(
            Position::new(self.line, self.col),
            Position::new(self.line, end),
        )
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// Collect identifier tokens with positions. Skips `//` and `/* */` comments and
/// string/char literal text, but keeps `{expr}` interpolation holes in `"..."`.
fn scan_identifiers(source: &str) -> Vec<IdentOcc> {
    #[derive(Clone, Copy, PartialEq)]
    enum St {
        Normal,
        LineComment,
        BlockComment,
        Str,
        CharLit,
        Interp,
    }

    let mut out = Vec::new();
    let mut st = St::Normal;
    let mut line = 0u32;
    let mut col = 0u32;
    let mut chars = source.chars().peekable();

    while let Some(c) = chars.next() {
        match st {
            St::LineComment => {
                if c == '\n' {
                    st = St::Normal;
                    line += 1;
                    col = 0;
                } else {
                    col += 1;
                }
            }
            St::BlockComment => {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    col += 2;
                    st = St::Normal;
                } else if c == '\n' {
                    line += 1;
                    col = 0;
                } else {
                    col += 1;
                }
            }
            St::Str => {
                if c == '\\' {
                    // skip escaped char
                    if chars.next().is_some() {
                        col += 2;
                    } else {
                        col += 1;
                    }
                } else if c == '"' {
                    st = St::Normal;
                    col += 1;
                } else if c == '{' {
                    st = St::Interp;
                    col += 1;
                } else if c == '\n' {
                    line += 1;
                    col = 0;
                } else {
                    col += 1;
                }
            }
            St::CharLit => {
                if c == '\\' {
                    if chars.next().is_some() {
                        col += 2;
                    } else {
                        col += 1;
                    }
                } else if c == '\'' {
                    st = St::Normal;
                    col += 1;
                } else if c == '\n' {
                    line += 1;
                    col = 0;
                } else {
                    col += 1;
                }
            }
            St::Interp => {
                // Nested braces inside interpolation expressions
                if c == '}' {
                    st = St::Str;
                    col += 1;
                } else if c == '\n' {
                    line += 1;
                    col = 0;
                } else if is_ident_start(c) {
                    let start_col = col;
                    let mut name = String::from(c);
                    col += 1;
                    while let Some(&n) = chars.peek() {
                        if is_ident_continue(n) {
                            name.push(n);
                            chars.next();
                            col += 1;
                        } else {
                            break;
                        }
                    }
                    out.push(IdentOcc {
                        name,
                        line,
                        col: start_col,
                    });
                } else {
                    col += 1;
                }
            }
            St::Normal => {
                if c == '/' && chars.peek() == Some(&'/') {
                    chars.next();
                    col += 2;
                    st = St::LineComment;
                } else if c == '/' && chars.peek() == Some(&'*') {
                    chars.next();
                    col += 2;
                    st = St::BlockComment;
                } else if c == '"' {
                    st = St::Str;
                    col += 1;
                } else if c == '\'' {
                    st = St::CharLit;
                    col += 1;
                } else if c == '\n' {
                    line += 1;
                    col = 0;
                } else if is_ident_start(c) {
                    let start_col = col;
                    let mut name = String::from(c);
                    col += 1;
                    while let Some(&n) = chars.peek() {
                        if is_ident_continue(n) {
                            name.push(n);
                            chars.next();
                            col += 1;
                        } else {
                            break;
                        }
                    }
                    out.push(IdentOcc {
                        name,
                        line,
                        col: start_col,
                    });
                } else {
                    col += 1;
                }
            }
        }
    }
    out
}

/// Word under `position` (char columns), or None if the cursor is not on one.
fn word_at(source: &str, position: Position) -> Option<IdentOcc> {
    let line_text = source.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = position.character as usize;
    if col > chars.len() {
        return None;
    }
    let mut start = col.min(chars.len().saturating_sub(1));
    if chars.is_empty() {
        return None;
    }
    if !is_ident_continue(chars[start]) {
        // Cursor may sit just after the word
        if start > 0 && is_ident_continue(chars[start - 1]) {
            start -= 1;
        } else {
            return None;
        }
    }
    while start > 0 && is_ident_continue(chars[start - 1]) {
        start -= 1;
    }
    let mut end = start;
    while end < chars.len() && is_ident_continue(chars[end]) {
        end += 1;
    }
    if start >= end {
        return None;
    }
    let name: String = chars[start..end].iter().collect();
    Some(IdentOcc {
        name,
        line: position.line,
        col: start as u32,
    })
}

fn find_occurrences(source: &str, name: &str) -> Vec<IdentOcc> {
    scan_identifiers(source)
        .into_iter()
        .filter(|o| o.name == name)
        .collect()
}

fn name_boundaries_ok(line: &str, idx: usize, name: &str) -> bool {
    let before_ok = line[..idx]
        .chars()
        .next_back()
        .map(|c| !is_ident_continue(c))
        .unwrap_or(true);
    let after = idx + name.len();
    let after_ok = line[after..]
        .chars()
        .next()
        .map(|c| !is_ident_continue(c))
        .unwrap_or(true);
    before_ok && after_ok
}

/// Declaration keyword + name patterns used by definition / hover.
fn is_declaration_name(line: &str, name: &str) -> bool {
    for kw in [
        "fn ", "class ", "trait ", "module ", "let ", "var ", "const ",
    ] {
        let pat = format!("{}{}", kw, name);
        if let Some(idx) = line.find(pat.as_str()) {
            // pattern is `kw + name`; name starts after the keyword (including its space)
            let name_idx = idx + kw.len();
            if name_boundaries_ok(line, name_idx, name) {
                return true;
            }
        }
    }
    false
}

/// Kind of declaration for `name` on `line` (fn / class / trait / let / var / const / module).
fn decl_kind(line: &str, name: &str) -> Option<&'static str> {
    for (kw, kind) in [
        ("fn ", "function"),
        ("class ", "class"),
        ("trait ", "trait"),
        ("module ", "module"),
        ("let ", "let"),
        ("var ", "var"),
        ("const ", "const"),
    ] {
        let pat = format!("{}{}", kw, name);
        if let Some(idx) = line.find(pat.as_str()) {
            let name_idx = idx + kw.len();
            if name_boundaries_ok(line, name_idx, name) {
                return Some(kind);
            }
        }
    }
    None
}

fn find_declaration(source: &str, name: &str) -> Option<IdentOcc> {
    for occ in find_occurrences(source, name) {
        let line_text = source.lines().nth(occ.line as usize)?;
        if is_declaration_name(line_text, name) {
            return Some(occ);
        }
    }
    None
}

/// Expand a declaration into a multi-line snippet (signature + body head).
fn declaration_snippet(source: &str, name: &str) -> Option<(String, String, IdentOcc)> {
    let decl = find_declaration(source, name)?;
    let kind = {
        let line_text = source.lines().nth(decl.line as usize)?;
        decl_kind(line_text, name).unwrap_or("binding")
    };
    let lines: Vec<&str> = source.lines().collect();
    let mut snippet_lines: Vec<&str> = Vec::new();
    let mut brace_depth: i32 = 0;
    let mut seen_open = false;
    for l in lines.iter().skip(decl.line as usize).take(16) {
        snippet_lines.push(*l);
        for ch in l.chars() {
            match ch {
                '{' => {
                    brace_depth += 1;
                    seen_open = true;
                }
                '}' => brace_depth -= 1,
                _ => {}
            }
        }
        // Stop after a closed block, or after the signature line if there is no block.
        if seen_open && brace_depth <= 0 {
            break;
        }
        if !seen_open && !snippet_lines.is_empty() {
            // single-line declaration (let/var/const or fn without body on same line)
            let trimmed = l.trim_end();
            if !trimmed.ends_with('{')
                && !trimmed.ends_with(',')
                && !trimmed.ends_with("implements")
                && !trimmed.ends_with("extends")
            {
                // keep scanning a bit if line looks incomplete
                if !trimmed.ends_with('\\') {
                    break;
                }
            }
        }
    }
    let snippet = snippet_lines.join("\n");
    Some((kind.to_string(), snippet, decl))
}

fn hover_markdown(source: &str, word: &IdentOcc) -> String {
    if let Some((kind, snippet, decl)) = declaration_snippet(source, &word.name) {
        format!(
            "**{}** `{}`\n\n```mailang\n{}\n```\n\n_Defined at line {}_",
            kind,
            word.name,
            snippet,
            decl.line + 1
        )
    } else {
        // Fall back to the full line under the cursor (not a trimmed preview).
        let line = source
            .lines()
            .nth(word.line as usize)
            .unwrap_or("")
            .to_string();
        format!("```mailang\n{}\n```", line)
    }
}

fn find_definition(source: &str, position: Position, doc_uri: &Url) -> Option<Location> {
    let word = word_at(source, position)?;
    let decl = find_declaration(source, &word.name)?;
    Some(Location {
        uri: doc_uri.clone(),
        range: decl.range(),
    })
}

fn end_position(source: &str) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    for c in source.chars() {
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position::new(line, col)
}

fn full_doc_range(source: &str) -> Range {
    Range::new(Position::new(0, 0), end_position(source))
}

fn workspace_edit(uri: &Url, edits: Vec<TextEdit>) -> WorkspaceEdit {
    let mut map = HashMap::new();
    map.insert(uri.clone(), edits);
    WorkspaceEdit {
        changes: Some(map),
        document_changes: None,
        change_annotations: None,
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "mailang-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                document_formatting_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Ma矛Lang LSP initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.insert(uri.clone(), text.clone());
        let diags = collect_diagnostics(&text);
        self.client.publish_diagnostics(uri, diags, None).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri;
            self.documents.insert(uri.clone(), change.text.clone());
            let diags = collect_diagnostics(&change.text);
            self.client.publish_diagnostics(uri, diags, None).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let items = if let Some(text) = self.documents.get(uri) {
            word_completions(&text, params.text_document_position.position)
        } else {
            builtin_completions()
        };
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        if let Some(text) = self.documents.get(uri) {
            if let Some(loc) = find_definition(&text, pos, uri) {
                return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
            }
        }
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        if let Some(text) = self.documents.get(uri) {
            if let Some(word) = word_at(&text, pos) {
                let value = hover_markdown(&text, &word);
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value,
                    }),
                    range: Some(word.range()),
                }));
            }
            // Not on a word: still show the full current line.
            if let Some(line) = text.lines().nth(pos.line as usize) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("```mailang\n{}\n```", line),
                    }),
                    range: None,
                }));
            }
        }
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let include_decl = params.context.include_declaration;
        let Some(text) = self.documents.get(uri) else {
            return Ok(None);
        };
        let Some(word) = word_at(&text, pos) else {
            return Ok(None);
        };
        let decl = find_declaration(&text, &word.name);
        let mut locs: Vec<Location> = Vec::new();
        for occ in find_occurrences(&text, &word.name) {
            if !include_decl {
                if let Some(d) = &decl {
                    if d.line == occ.line && d.col == occ.col {
                        continue;
                    }
                }
            }
            locs.push(Location {
                uri: uri.clone(),
                range: occ.range(),
            });
        }
        Ok(Some(locs))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let pos = params.position;
        let Some(text) = self.documents.get(uri) else {
            return Ok(None);
        };
        let Some(word) = word_at(&text, pos) else {
            return Ok(None);
        };
        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: word.range(),
            placeholder: word.name,
        }))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        let Some(text) = self.documents.get(uri) else {
            return Ok(None);
        };
        let Some(word) = word_at(&text, pos) else {
            return Ok(None);
        };
        if new_name.is_empty() || !is_ident_start(new_name.chars().next().unwrap_or('_')) {
            return Ok(None);
        }
        let edits: Vec<TextEdit> = find_occurrences(&text, &word.name)
            .into_iter()
            .map(|occ| TextEdit {
                range: occ.range(),
                new_text: new_name.clone(),
            })
            .collect();
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(workspace_edit(uri, edits)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;
        let Some(text) = self.documents.get(uri) else {
            return Ok(None);
        };
        let source = text.clone();
        match mailang_core::format_source(&source) {
            Ok(formatted) => {
                if formatted == source {
                    Ok(Some(Vec::new()))
                } else {
                    Ok(Some(vec![TextEdit {
                        range: full_doc_range(&source),
                        new_text: formatted,
                    }]))
                }
            }
            // Leave the buffer untouched when the file does not parse.
            Err(_) => Ok(None),
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn run_lsp() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Arc::new(DashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_at_finds_identifier() {
        let src = "let x = foo_bar + 2\n";
        let w = word_at(src, Position::new(0, 4)).unwrap();
        assert_eq!(w.name, "x");
        assert_eq!(
            w.range(),
            Range::new(Position::new(0, 4), Position::new(0, 5))
        );
        let w = word_at(src, Position::new(0, 8)).unwrap();
        assert_eq!(w.name, "foo_bar");
    }

    #[test]
    fn scan_skips_comments_and_string_text() {
        let src = "// x = 1\nlet y = \"x\"\n/* z */ let x = 1\n";
        let names: Vec<_> = scan_identifiers(src).into_iter().map(|o| o.name).collect();
        assert!(names.contains(&"y".to_string()));
        assert!(names.contains(&"x".to_string()));
        // `x` inside `"x"` is skipped; `x` in the comment is skipped; decl remains
        let x_count = names.iter().filter(|n| *n == "x").count();
        assert_eq!(x_count, 1);
    }

    #[test]
    fn scan_keeps_interpolation_idents() {
        let src = "let n = 1\nlet s = \"hi {n}!\"\n";
        let names: Vec<_> = scan_identifiers(src).into_iter().map(|o| o.name).collect();
        assert_eq!(names.iter().filter(|n| *n == "n").count(), 2);
    }

    #[test]
    fn finds_all_occurrences_and_declaration() {
        let src = "fn add(a, b) {\n  return a + b\n}\nlet z = add(1, 2)\n";
        let occs = find_occurrences(src, "add");
        assert_eq!(occs.len(), 2);
        let decl = find_declaration(src, "add").unwrap();
        assert_eq!(decl.line, 0);
        assert_eq!(decl.col, 3);
    }

    #[test]
    fn hover_shows_declaration_snippet() {
        let src = "fn add(a, b) {\n  return a + b\n}\nlet z = add(1, 2)\n";
        let word = word_at(src, Position::new(3, 8)).unwrap();
        let md = hover_markdown(src, &word);
        assert!(md.contains("**function** `add`"));
        assert!(md.contains("fn add(a, b)"));
        assert!(md.contains("return a + b"));
        assert!(md.contains("_Defined at line 1_"));
    }

    #[test]
    fn rename_edits_cover_all_occurrences() {
        let src = "let x = 1\nlet y = x + x\n";
        let occs = find_occurrences(src, "x");
        assert_eq!(occs.len(), 3);
        assert_eq!(
            occs[0].range(),
            Range::new(Position::new(0, 4), Position::new(0, 5))
        );
        assert_eq!(occs[1].col, 8);
        assert_eq!(occs[2].col, 12);
    }

    #[test]
    fn full_range_covers_document() {
        let src = "ab\ncd\n";
        let r = full_doc_range(src);
        assert_eq!(r.start, Position::new(0, 0));
        assert_eq!(r.end, Position::new(2, 0));
        let src = "ab\ncd";
        let r = full_doc_range(src);
        assert_eq!(r.end, Position::new(1, 2));
    }
}
