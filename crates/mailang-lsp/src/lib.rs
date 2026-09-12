use dashmap::DashMap;
use mailang_analyzer::Analyzer;
use mailang_parser::Parser;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<DashMap<Url, String>>,
}

fn offset_to_position(text: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += ch.len_utf16() as u32;
        }
    }
    Position::new(line, col)
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
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => return vec![diag_at(&e.to_string(), DiagnosticSeverity::ERROR)],
    };

    let mut analyzer = Analyzer::new();
    match analyzer.analyze(&program) {
        Ok(()) => analyzer
            .diagnostics()
            .into_iter()
            .filter_map(|e| match e {
                mailang_analyzer::AnalyzerError::UnusedVariable(name) => Some(diag_at(
                    &format!("unused variable '{}'", name),
                    DiagnosticSeverity::INFORMATION,
                )),
                other => Some(diag_at(
                    &other.to_string(),
                    DiagnosticSeverity::WARNING,
                )),
            })
            .collect(),
        Err(errs) => errs
            .into_iter()
            .map(|e| diag_at(&e.to_string(), DiagnosticSeverity::ERROR))
            .collect(),
    }
}

fn builtin_completions() -> Vec<CompletionItem> {
    let names = [
        "println", "print", "input", "sqrt", "abs", "sin", "cos", "floor", "ceil", "round",
        "min", "max", "len", "to_string", "parse_int", "parse_float",
        "let", "var", "const", "fn", "if", "elif", "else", "while", "for", "in",
        "match", "return", "class", "trait", "extends", "implements", "super", "this",
        "Ok", "Err", "Some", "None",
        "gpio_write", "gpio_read", "delay_ms", "adc_read",
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
                detail: Some("MaìLang".into()),
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

fn find_definition(source: &str, position: Position, doc_uri: &Url) -> Option<Location> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(position.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let col = position.character as usize;
    if col > chars.len() {
        return None;
    }
    // Expand word under cursor
    let mut start = col.min(chars.len().saturating_sub(1));
    let mut end = start;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    if start >= end {
        return None;
    }
    let word: String = chars[start..end].iter().collect();

    // Search for `fn word`, `class word`, `let word`, `var word`, `trait word`
    for (i, l) in lines.iter().enumerate() {
        let patterns = [
            format!("fn {}", word),
            format!("class {}", word),
            format!("trait {}", word),
            format!("let {}", word),
            format!("var {}", word),
            format!("const {}", word),
        ];
        if let Some(pos) = patterns.iter().find_map(|p| l.find(p.as_str())) {
            let name_col = l[..pos].chars().count() as u32
                + if l[pos..].starts_with("fn ") {
                    3
                } else if l[pos..].starts_with("class ") {
                    6
                } else if l[pos..].starts_with("trait ") {
                    6
                } else if l[pos..].starts_with("const ") {
                    6
                } else {
                    4
                };
            return Some(Location {
                uri: doc_uri.clone(),
                range: Range::new(
                    Position::new(i as u32, name_col),
                    Position::new(i as u32, name_col + word.chars().count() as u32),
                ),
            });
        }
    }
    None
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "mailang-lsp".to_string(),
                version: Some("0.1.0".to_string()),
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
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "MaìLang LSP initialized")
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
            let lines: Vec<&str> = text.lines().collect();
            if let Some(line) = lines.get(pos.line as usize) {
                // Show the current line as a mini hover
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("```mailang\n{}\n```", line.trim()),
                    }),
                    range: None,
                }));
            }
        }
        Ok(None)
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
