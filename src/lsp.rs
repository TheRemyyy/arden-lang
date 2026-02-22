//! LSP (Language Server Protocol) implementation for Apex
//!
//! Provides IDE features like:
//! - Autocompletion
//! - Hover information
//! - Go to definition
//! - Diagnostics

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ast::{Decl, Program};
use crate::lexer;
use crate::parser::Parser;

/// Document state tracked by the LSP server
#[derive(Debug, Clone)]
struct Document {
    text: String,
    version: i32,
    parsed: Option<Program>,
}

/// LSP Server backend
pub struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, Document>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Parse a document and store the AST
    async fn parse_document(&self, uri: &Url) {
        let mut docs = self.documents.write().await;
        if let Some(doc) = docs.get_mut(uri) {
            match lexer::tokenize(&doc.text) {
                Ok(tokens) => {
                    let mut parser = Parser::new(tokens);
                    doc.parsed = parser.parse_program().ok();
                }
                Err(_) => {
                    doc.parsed = None;
                }
            }
        }
    }

    /// Get completion items for a position
    fn get_completions(&self, doc: &Document, _pos: Position) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Keywords
        let keywords = vec![
            "function",
            "class",
            "interface",
            "enum",
            "module",
            "if",
            "else",
            "while",
            "for",
            "in",
            "return",
            "break",
            "continue",
            "match",
            "mut",
            "let",
            "import",
            "package",
            "async",
            "await",
            "public",
            "private",
            "protected",
            "constructor",
            "destructor",
        ];

        for kw in keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("keyword".to_string()),
                ..Default::default()
            });
        }

        // Types
        let types = vec![
            "Integer", "Float", "Boolean", "String", "Char", "None", "Option", "Result", "List",
            "Map", "Set", "Box", "Rc", "Arc", "Task",
        ];

        for ty in types {
            items.push(CompletionItem {
                label: ty.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("type".to_string()),
                ..Default::default()
            });
        }

        // Functions from AST
        if let Some(program) = &doc.parsed {
            for decl in &program.declarations {
                if let Decl::Function(func) = &decl.node {
                    items.push(CompletionItem {
                        label: func.name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(format!("function: {}", func.name)),
                        ..Default::default()
                    });
                }
            }
        }

        items
    }

    /// Get hover information
    fn get_hover(&self, doc: &Document, pos: Position) -> Option<Hover> {
        // Simple word-based hover
        let line = doc.text.lines().nth(pos.line as usize)?;

        // Keywords documentation
        let keywords_docs: HashMap<&str, &str> = [
            ("function", "Define a function\n\n```apex\nfunction name(params): ReturnType {\n  // body\n}\n```"),
            ("class", "Define a class\n\n```apex\nclass Name {\n  field: Type;\n  function method(): Type { }\n}\n```"),
            ("if", "Conditional statement\n\n```apex\nif (condition) {\n  // then branch\n} else {\n  // else branch\n}\n```"),
            ("while", "While loop\n\n```apex\nwhile (condition) {\n  // body\n}\n```"),
            ("for", "For loop\n\n```apex\nfor (i in 0..10) {\n  // body\n}\n```"),
            ("match", "Pattern matching\n\n```apex\nmatch value {\n  Pattern => { },\n  _ => { },\n}\n```"),
            ("mut", "Mutable variable declaration\n\n```apex\nmut x: Integer = 10;\n```"),
            ("let", "Variable declaration\n\n```apex\nlet x: Integer = 10;\n```"),
            ("import", "Import from another module\n\n```apex\nimport utils.math.*;\n```"),
            ("package", "Declare package namespace\n\n```apex\npackage my.module;\n```"),
            ("async", "Async function or block\n\n```apex\nasync function foo(): Task<String> { }\n```"),
            ("await", "Await an async operation\n\n```apex\nlet result = await asyncFunction();\n```"),
            ("return", "Return from function\n\n```apex\nreturn value;\n```"),
        ].iter().cloned().collect();

        for (kw, doc) in keywords_docs {
            if line.contains(kw) {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc.to_string(),
                    }),
                    range: None,
                });
            }
        }

        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "apex-lsp".to_string(),
                version: Some("1.3.1".to_string()),
            }),
            offset_encoding: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), "(".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Apex LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;

        let mut docs = self.documents.write().await;
        docs.insert(
            uri.clone(),
            Document {
                text: text.clone(),
                version,
                parsed: None,
            },
        );
        drop(docs);

        self.parse_document(&uri).await;

        self.client
            .log_message(MessageType::INFO, format!("Opened document: {}", uri))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        if let Some(change) = params.content_changes.into_iter().next() {
            let mut docs = self.documents.write().await;
            if let Some(doc) = docs.get_mut(&uri) {
                doc.text = change.text;
                doc.version = params.text_document.version;
            }
            drop(docs);

            self.parse_document(&uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut docs = self.documents.write().await;
        docs.remove(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            let items = self.get_completions(doc, pos);
            return Ok(Some(CompletionResponse::Array(items)));
        }

        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            return Ok(self.get_hover(doc, pos));
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let _pos = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(_program) = &doc.parsed {
                // TODO: Implement goto definition
                // Need to track symbol positions in AST
                return Ok(None);
            }
        }

        Ok(None)
    }
}

/// Run the LSP server
pub async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
