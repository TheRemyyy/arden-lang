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

use crate::ast::{Block, Decl, Expr, FunctionDecl, Pattern, Program, Stmt};
use crate::lexer;
use crate::parser::{ParseError, Parser};

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

struct ScopedSymbolResolver<'a> {
    backend: &'a Backend,
    text: &'a str,
    symbol: &'a str,
    cursor_offset: usize,
    scopes: Vec<HashMap<String, usize>>,
    occurrences: HashMap<usize, Vec<std::ops::Range<usize>>>,
    selected_binding: Option<usize>,
    next_binding_id: usize,
}

impl<'a> ScopedSymbolResolver<'a> {
    fn new(backend: &'a Backend, text: &'a str, symbol: &'a str, cursor_offset: usize) -> Self {
        Self {
            backend,
            text,
            symbol,
            cursor_offset,
            scopes: vec![HashMap::new()],
            occurrences: HashMap::new(),
            selected_binding: None,
            next_binding_id: 0,
        }
    }

    fn resolve(mut self, program: &Program) -> Vec<std::ops::Range<usize>> {
        self.predeclare_globals(program);
        self.walk_program(program);
        let Some(binding_id) = self.selected_binding else {
            return Vec::new();
        };
        let mut spans = self.occurrences.remove(&binding_id).unwrap_or_default();
        spans.sort_by_key(|s| (s.start, s.end));
        spans.dedup_by(|a, b| a.start == b.start && a.end == b.end);
        spans
    }

    fn predeclare_globals(&mut self, program: &Program) {
        for decl in &program.declarations {
            self.predeclare_decl(decl);
        }
    }

    fn predeclare_decl(&mut self, decl: &crate::ast::Spanned<Decl>) {
        match &decl.node {
            Decl::Function(func) => self.predeclare_named(&func.name, &decl.span),
            Decl::Class(class) => self.predeclare_named(&class.name, &decl.span),
            Decl::Enum(en) => self.predeclare_named(&en.name, &decl.span),
            Decl::Interface(interface) => self.predeclare_named(&interface.name, &decl.span),
            Decl::Module(module) => {
                self.predeclare_named(&module.name, &decl.span);
                for inner in &module.declarations {
                    self.predeclare_decl(inner);
                }
            }
            Decl::Import(_) => {}
        }
    }

    fn predeclare_named(&mut self, name: &str, span: &std::ops::Range<usize>) {
        if name != self.symbol {
            return;
        }
        if let Some(name_span) = self
            .backend
            .find_name_occurrence_in_span(self.text, name, span)
        {
            let id = self.next_binding_id;
            self.next_binding_id += 1;
            if let Some(global) = self.scopes.first_mut() {
                global.insert(name.to_string(), id);
            }
            self.record_occurrence(id, name_span);
        }
    }

    fn walk_program(&mut self, program: &Program) {
        for decl in &program.declarations {
            self.walk_decl(decl);
        }
    }

    fn walk_decl(&mut self, decl: &crate::ast::Spanned<Decl>) {
        match &decl.node {
            Decl::Function(func) => {
                self.enter_scope();
                for param in &func.params {
                    if param.name == self.symbol {
                        if let Some(span) = self.backend.find_name_occurrence_in_span(
                            self.text,
                            &param.name,
                            &decl.span,
                        ) {
                            self.declare_binding(&param.name, span);
                        }
                    }
                }
                self.walk_block(&func.body);
                self.exit_scope();
            }
            Decl::Class(class) => {
                for field in &class.fields {
                    if field.name == self.symbol {
                        if let Some(span) = self.backend.find_name_occurrence_in_span(
                            self.text,
                            &field.name,
                            &decl.span,
                        ) {
                            let id = self.next_binding_id;
                            self.next_binding_id += 1;
                            self.record_occurrence(id, span);
                        }
                    }
                }
                if let Some(constructor) = &class.constructor {
                    self.enter_scope();
                    for param in &constructor.params {
                        if param.name == self.symbol {
                            if let Some(span) = self.backend.find_name_occurrence_in_span(
                                self.text,
                                &param.name,
                                &decl.span,
                            ) {
                                self.declare_binding(&param.name, span);
                            }
                        }
                    }
                    self.walk_block(&constructor.body);
                    self.exit_scope();
                }
                if let Some(destructor) = &class.destructor {
                    self.enter_scope();
                    self.walk_block(&destructor.body);
                    self.exit_scope();
                }
                for method in &class.methods {
                    self.enter_scope();
                    for param in &method.params {
                        if param.name == self.symbol {
                            if let Some(span) = self.backend.find_name_occurrence_in_span(
                                self.text,
                                &param.name,
                                &decl.span,
                            ) {
                                self.declare_binding(&param.name, span);
                            }
                        }
                    }
                    self.walk_block(&method.body);
                    self.exit_scope();
                }
            }
            Decl::Module(module) => {
                for inner in &module.declarations {
                    self.walk_decl(inner);
                }
            }
            Decl::Interface(interface) => {
                for method in &interface.methods {
                    if method.name == self.symbol {
                        if let Some(span) = self.backend.find_name_occurrence_in_span(
                            self.text,
                            &method.name,
                            &decl.span,
                        ) {
                            let id = self.next_binding_id;
                            self.next_binding_id += 1;
                            self.record_occurrence(id, span);
                        }
                    }
                    for param in &method.params {
                        if param.name == self.symbol {
                            if let Some(span) = self.backend.find_name_occurrence_in_span(
                                self.text,
                                &param.name,
                                &decl.span,
                            ) {
                                let id = self.next_binding_id;
                                self.next_binding_id += 1;
                                self.record_occurrence(id, span);
                            }
                        }
                    }
                }
            }
            Decl::Enum(_) | Decl::Import(_) => {}
        }
    }

    fn walk_block(&mut self, block: &Block) {
        for stmt in block {
            self.walk_stmt(stmt);
        }
    }

    fn walk_stmt(&mut self, stmt: &crate::ast::Spanned<Stmt>) {
        match &stmt.node {
            Stmt::Let { name, value, .. } => {
                self.walk_expr(value);
                if name == self.symbol {
                    if let Some(span) = self
                        .backend
                        .find_name_occurrence_in_span(self.text, name, &stmt.span)
                    {
                        self.declare_binding(name, span);
                    }
                }
            }
            Stmt::Assign { target, value } => {
                self.walk_expr(target);
                self.walk_expr(value);
            }
            Stmt::Expr(expr) => self.walk_expr(expr),
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.walk_expr(expr);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.walk_expr(condition);
                self.enter_scope();
                self.walk_block(then_block);
                self.exit_scope();
                if let Some(else_block) = else_block {
                    self.enter_scope();
                    self.walk_block(else_block);
                    self.exit_scope();
                }
            }
            Stmt::While { condition, body } => {
                self.walk_expr(condition);
                self.enter_scope();
                self.walk_block(body);
                self.exit_scope();
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                self.walk_expr(iterable);
                self.enter_scope();
                if var == self.symbol {
                    if let Some(span) = self
                        .backend
                        .find_name_occurrence_in_span(self.text, var, &stmt.span)
                    {
                        self.declare_binding(var, span);
                    }
                }
                self.walk_block(body);
                self.exit_scope();
            }
            Stmt::Match { expr, arms } => {
                self.walk_expr(expr);
                for arm in arms {
                    self.enter_scope();
                    self.walk_pattern_bindings(&arm.pattern, &stmt.span);
                    self.walk_block(&arm.body);
                    self.exit_scope();
                }
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn walk_expr(&mut self, expr: &crate::ast::Spanned<Expr>) {
        match &expr.node {
            Expr::Ident(name) => {
                if name == self.symbol {
                    if let Some(id) = self.resolve_binding(name) {
                        self.record_occurrence(id, expr.span.clone());
                    }
                }
            }
            Expr::Call { callee, args, .. } => {
                self.walk_expr(callee);
                for arg in args {
                    self.walk_expr(arg);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.walk_expr(left);
                self.walk_expr(right);
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => self.walk_expr(expr),
            Expr::Field { object, .. } => self.walk_expr(object),
            Expr::Index { object, index } => {
                self.walk_expr(object);
                self.walk_expr(index);
            }
            Expr::Construct { args, .. } => {
                for arg in args {
                    self.walk_expr(arg);
                }
            }
            Expr::Lambda { params, body } => {
                self.enter_scope();
                for param in params {
                    if param.name == self.symbol {
                        if let Some(span) = self.backend.find_name_occurrence_in_span(
                            self.text,
                            &param.name,
                            &expr.span,
                        ) {
                            self.declare_binding(&param.name, span);
                        }
                    }
                }
                self.walk_expr(body);
                self.exit_scope();
            }
            Expr::Match { expr, arms } => {
                self.walk_expr(expr);
                for arm in arms {
                    self.enter_scope();
                    self.walk_pattern_bindings(&arm.pattern, &expr.span);
                    self.walk_block(&arm.body);
                    self.exit_scope();
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let crate::ast::StringPart::Expr(inner) = part {
                        self.walk_expr(inner);
                    }
                }
            }
            Expr::AsyncBlock(block) | Expr::Block(block) => {
                self.enter_scope();
                self.walk_block(block);
                self.exit_scope();
            }
            Expr::Require { condition, message } => {
                self.walk_expr(condition);
                if let Some(msg) = message {
                    self.walk_expr(msg);
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(start) = start {
                    self.walk_expr(start);
                }
                if let Some(end) = end {
                    self.walk_expr(end);
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.walk_expr(condition);
                self.enter_scope();
                self.walk_block(then_branch);
                self.exit_scope();
                if let Some(else_branch) = else_branch {
                    self.enter_scope();
                    self.walk_block(else_branch);
                    self.exit_scope();
                }
            }
            Expr::Literal(_) | Expr::This => {}
        }
    }

    fn walk_pattern_bindings(&mut self, pattern: &Pattern, search_span: &std::ops::Range<usize>) {
        match pattern {
            Pattern::Ident(name) => {
                if name == self.symbol {
                    if let Some(span) =
                        self.backend
                            .find_name_occurrence_in_span(self.text, name, search_span)
                    {
                        self.declare_binding(name, span);
                    }
                }
            }
            Pattern::Variant(_, bindings) => {
                for binding in bindings {
                    if binding == self.symbol {
                        if let Some(span) = self.backend.find_name_occurrence_in_span(
                            self.text,
                            binding,
                            search_span,
                        ) {
                            self.declare_binding(binding, span);
                        }
                    }
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
    }

    fn declare_binding(&mut self, name: &str, span: std::ops::Range<usize>) {
        let id = self.next_binding_id;
        self.next_binding_id += 1;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), id);
        }
        self.record_occurrence(id, span);
    }

    fn resolve_binding(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.get(name) {
                return Some(*id);
            }
        }
        None
    }

    fn record_occurrence(&mut self, id: usize, span: std::ops::Range<usize>) {
        if self.cursor_offset >= span.start && self.cursor_offset <= span.end {
            self.selected_binding = Some(id);
        }
        self.occurrences.entry(id).or_default().push(span);
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }
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
        let (text, version) = {
            let docs = self.documents.read().await;
            if let Some(doc) = docs.get(uri) {
                (doc.text.clone(), doc.version)
            } else {
                return;
            }
        };

        let mut diagnostics = Vec::new();
        let parsed = match lexer::tokenize(&text) {
            Ok(tokens) => {
                let mut parser = Parser::new(tokens);
                match parser.parse_program() {
                    Ok(program) => Some(program),
                    Err(err) => {
                        diagnostics.push(self.parse_error_to_diagnostic(&text, &err));
                        None
                    }
                }
            }
            Err(msg) => {
                diagnostics.push(self.lexer_error_to_diagnostic(&text, &msg));
                None
            }
        };

        {
            let mut docs = self.documents.write().await;
            if let Some(doc) = docs.get_mut(uri) {
                doc.parsed = parsed;
            }
        }

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(version))
            .await;
    }

    fn parse_error_to_diagnostic(&self, text: &str, err: &ParseError) -> Diagnostic {
        let mut message = err.message.clone();
        if message.contains("Reserved keyword") {
            message.push_str(
                "\nHint: use currently supported constructs or remove this keyword for now.",
            );
        }
        Diagnostic {
            range: self.span_to_range(text, err.span.clone()),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("apex-parser".to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn lexer_error_to_diagnostic(&self, text: &str, msg: &str) -> Diagnostic {
        // Expected shape: "Unknown token at <offset>: '<snippet>'"
        let offset = msg
            .split("Unknown token at ")
            .nth(1)
            .and_then(|s| s.split(':').next())
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(0);
        Diagnostic {
            range: self.span_to_range(text, offset..(offset + 1)),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("apex-lexer".to_string()),
            message: msg.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn span_to_range(&self, text: &str, span: std::ops::Range<usize>) -> Range {
        Range {
            start: self.offset_to_position(text, span.start),
            end: self.offset_to_position(text, span.end),
        }
    }

    fn offset_to_position(&self, text: &str, target: usize) -> Position {
        let mut line = 0u32;
        let mut col = 0u32;
        let mut last_idx = 0usize;

        for (idx, ch) in text.char_indices() {
            if idx >= target {
                break;
            }
            last_idx = idx;
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        if target > last_idx && target <= text.len() {
            // No-op: col is already best-effort for UTF-8 char boundaries.
        }

        Position::new(line, col)
    }

    fn position_to_offset(&self, text: &str, pos: Position) -> usize {
        let mut line = 0u32;
        let mut col = 0u32;
        for (idx, ch) in text.char_indices() {
            if line == pos.line && col == pos.character {
                return idx;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        text.len()
    }

    fn word_at_position(&self, text: &str, pos: Position) -> Option<String> {
        let line = text.lines().nth(pos.line as usize)?;
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let mut idx = (pos.character as usize).min(chars.len().saturating_sub(1));
        if !chars[idx].is_alphanumeric() && chars[idx] != '_' && idx > 0 {
            idx -= 1;
        }
        if !chars[idx].is_alphanumeric() && chars[idx] != '_' {
            return None;
        }

        let mut start = idx;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        let mut end = idx;
        while end + 1 < chars.len() && (chars[end + 1].is_alphanumeric() || chars[end + 1] == '_') {
            end += 1;
        }
        Some(chars[start..=end].iter().collect())
    }

    fn definition_locations(
        &self,
        uri: &Url,
        text: &str,
        program: &Program,
        symbol: &str,
        cursor_offset: usize,
    ) -> Vec<Location> {
        let mut out = Vec::new();
        for decl in &program.declarations {
            self.collect_definition_locations(text, uri, decl, symbol, &mut out);
        }

        if out.is_empty() {
            let mut ranges = self.find_symbol_ranges(text, program, symbol, cursor_offset);
            ranges.sort_by_key(|range| {
                (
                    range.start.line,
                    range.start.character,
                    range.end.line,
                    range.end.character,
                )
            });
            if let Some(range) = ranges.into_iter().next() {
                out.push(Location::new(uri.clone(), range));
            }
        }
        out
    }

    fn collect_definition_locations(
        &self,
        text: &str,
        uri: &Url,
        decl: &crate::ast::Spanned<Decl>,
        symbol: &str,
        out: &mut Vec<Location>,
    ) {
        match &decl.node {
            Decl::Function(func) if func.name == symbol => {
                out.push(Location::new(
                    uri.clone(),
                    self.span_to_range(text, decl.span.clone()),
                ));
            }
            Decl::Class(class) if class.name == symbol => {
                out.push(Location::new(
                    uri.clone(),
                    self.span_to_range(text, decl.span.clone()),
                ));
            }
            Decl::Enum(en) if en.name == symbol => {
                out.push(Location::new(
                    uri.clone(),
                    self.span_to_range(text, decl.span.clone()),
                ));
            }
            Decl::Interface(inter) if inter.name == symbol => {
                out.push(Location::new(
                    uri.clone(),
                    self.span_to_range(text, decl.span.clone()),
                ));
            }
            Decl::Module(module) => {
                if module.name == symbol {
                    out.push(Location::new(
                        uri.clone(),
                        self.span_to_range(text, decl.span.clone()),
                    ));
                }
                for inner in &module.declarations {
                    self.collect_definition_locations(text, uri, inner, symbol, out);
                }
            }
            Decl::Function(_)
            | Decl::Class(_)
            | Decl::Enum(_)
            | Decl::Interface(_)
            | Decl::Import(_) => {}
        }
    }

    fn find_symbol_ranges(
        &self,
        text: &str,
        program: &Program,
        symbol: &str,
        cursor_offset: usize,
    ) -> Vec<Range> {
        if symbol.is_empty() {
            return Vec::new();
        }

        let mut spans =
            ScopedSymbolResolver::new(self, text, symbol, cursor_offset).resolve(program);
        if spans.is_empty() {
            let mut legacy_spans: Vec<std::ops::Range<usize>> = Vec::new();
            self.collect_symbol_spans_program(text, program, symbol, &mut legacy_spans);
            spans = legacy_spans;
        }
        spans.sort_by_key(|s| (s.start, s.end));
        spans.dedup_by(|a, b| a.start == b.start && a.end == b.end);
        spans
            .into_iter()
            .map(|span| self.span_to_range(text, span))
            .collect()
    }

    fn collect_symbol_spans_program(
        &self,
        text: &str,
        program: &Program,
        symbol: &str,
        out: &mut Vec<std::ops::Range<usize>>,
    ) {
        for decl in &program.declarations {
            self.collect_symbol_spans_decl(text, symbol, decl, out);
        }
    }

    fn collect_symbol_spans_decl(
        &self,
        text: &str,
        symbol: &str,
        decl: &crate::ast::Spanned<Decl>,
        out: &mut Vec<std::ops::Range<usize>>,
    ) {
        match &decl.node {
            Decl::Function(func) => {
                if func.name == symbol {
                    if let Some(span) =
                        self.find_name_occurrence_in_span(text, &func.name, &decl.span)
                    {
                        out.push(span);
                    }
                }
                self.collect_symbol_spans_function(text, symbol, func, &decl.span, out);
            }
            Decl::Class(class) => {
                if class.name == symbol {
                    if let Some(span) =
                        self.find_name_occurrence_in_span(text, &class.name, &decl.span)
                    {
                        out.push(span);
                    }
                }
                for field in &class.fields {
                    if field.name == symbol {
                        if let Some(span) =
                            self.find_name_occurrence_in_span(text, &field.name, &decl.span)
                        {
                            out.push(span);
                        }
                    }
                }
                if let Some(constructor) = &class.constructor {
                    for param in &constructor.params {
                        if param.name == symbol {
                            if let Some(span) =
                                self.find_name_occurrence_in_span(text, &param.name, &decl.span)
                            {
                                out.push(span);
                            }
                        }
                    }
                    self.collect_symbol_spans_block(text, symbol, &constructor.body, out);
                }
                if let Some(destructor) = &class.destructor {
                    self.collect_symbol_spans_block(text, symbol, &destructor.body, out);
                }
                for method in &class.methods {
                    self.collect_symbol_spans_function(text, symbol, method, &decl.span, out);
                }
            }
            Decl::Module(module) => {
                if module.name == symbol {
                    if let Some(span) =
                        self.find_name_occurrence_in_span(text, &module.name, &decl.span)
                    {
                        out.push(span);
                    }
                }
                for inner in &module.declarations {
                    self.collect_symbol_spans_decl(text, symbol, inner, out);
                }
            }
            Decl::Enum(en) => {
                if en.name == symbol {
                    if let Some(span) =
                        self.find_name_occurrence_in_span(text, &en.name, &decl.span)
                    {
                        out.push(span);
                    }
                }
            }
            Decl::Interface(interface) => {
                if interface.name == symbol {
                    if let Some(span) =
                        self.find_name_occurrence_in_span(text, &interface.name, &decl.span)
                    {
                        out.push(span);
                    }
                }
                for method in &interface.methods {
                    if method.name == symbol {
                        if let Some(span) =
                            self.find_name_occurrence_in_span(text, &method.name, &decl.span)
                        {
                            out.push(span);
                        }
                    }
                    for param in &method.params {
                        if param.name == symbol {
                            if let Some(span) =
                                self.find_name_occurrence_in_span(text, &param.name, &decl.span)
                            {
                                out.push(span);
                            }
                        }
                    }
                }
            }
            Decl::Import(_) => {}
        }
    }

    fn collect_symbol_spans_function(
        &self,
        text: &str,
        symbol: &str,
        func: &FunctionDecl,
        fallback_span: &std::ops::Range<usize>,
        out: &mut Vec<std::ops::Range<usize>>,
    ) {
        if func.name == symbol {
            if let Some(span) = self.find_name_occurrence_in_span(text, &func.name, fallback_span) {
                out.push(span);
            }
        }
        for param in &func.params {
            if param.name == symbol {
                if let Some(span) =
                    self.find_name_occurrence_in_span(text, &param.name, fallback_span)
                {
                    out.push(span);
                }
            }
        }
        self.collect_symbol_spans_block(text, symbol, &func.body, out);
    }

    fn collect_symbol_spans_block(
        &self,
        text: &str,
        symbol: &str,
        block: &Block,
        out: &mut Vec<std::ops::Range<usize>>,
    ) {
        for stmt in block {
            self.collect_symbol_spans_stmt(text, symbol, stmt, out);
        }
    }

    fn collect_symbol_spans_stmt(
        &self,
        text: &str,
        symbol: &str,
        stmt: &crate::ast::Spanned<Stmt>,
        out: &mut Vec<std::ops::Range<usize>>,
    ) {
        match &stmt.node {
            Stmt::Let { name, value, .. } => {
                if name == symbol {
                    if let Some(span) = self.find_name_occurrence_in_span(text, name, &stmt.span) {
                        out.push(span);
                    }
                }
                self.collect_symbol_spans_expr(text, symbol, value, out);
            }
            Stmt::Assign { target, value } => {
                self.collect_symbol_spans_expr(text, symbol, target, out);
                self.collect_symbol_spans_expr(text, symbol, value, out);
            }
            Stmt::Expr(expr) => self.collect_symbol_spans_expr(text, symbol, expr, out),
            Stmt::Return(value) => {
                if let Some(expr) = value {
                    self.collect_symbol_spans_expr(text, symbol, expr, out);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.collect_symbol_spans_expr(text, symbol, condition, out);
                self.collect_symbol_spans_block(text, symbol, then_block, out);
                if let Some(block) = else_block {
                    self.collect_symbol_spans_block(text, symbol, block, out);
                }
            }
            Stmt::While { condition, body } => {
                self.collect_symbol_spans_expr(text, symbol, condition, out);
                self.collect_symbol_spans_block(text, symbol, body, out);
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                if var == symbol {
                    if let Some(span) = self.find_name_occurrence_in_span(text, var, &stmt.span) {
                        out.push(span);
                    }
                }
                self.collect_symbol_spans_expr(text, symbol, iterable, out);
                self.collect_symbol_spans_block(text, symbol, body, out);
            }
            Stmt::Match { expr, arms } => {
                self.collect_symbol_spans_expr(text, symbol, expr, out);
                for arm in arms {
                    self.collect_symbol_spans_block(text, symbol, &arm.body, out);
                }
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn collect_symbol_spans_expr(
        &self,
        text: &str,
        symbol: &str,
        expr: &crate::ast::Spanned<Expr>,
        out: &mut Vec<std::ops::Range<usize>>,
    ) {
        match &expr.node {
            Expr::Ident(name) => {
                if name == symbol {
                    out.push(expr.span.clone());
                }
            }
            Expr::Call { callee, args, .. } => {
                self.collect_symbol_spans_expr(text, symbol, callee, out);
                for arg in args {
                    self.collect_symbol_spans_expr(text, symbol, arg, out);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.collect_symbol_spans_expr(text, symbol, left, out);
                self.collect_symbol_spans_expr(text, symbol, right, out);
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => self.collect_symbol_spans_expr(text, symbol, expr, out),
            Expr::Field { object, .. } => self.collect_symbol_spans_expr(text, symbol, object, out),
            Expr::Index { object, index } => {
                self.collect_symbol_spans_expr(text, symbol, object, out);
                self.collect_symbol_spans_expr(text, symbol, index, out);
            }
            Expr::Construct { args, .. } => {
                for arg in args {
                    self.collect_symbol_spans_expr(text, symbol, arg, out);
                }
            }
            Expr::Lambda { params, body } => {
                for param in params {
                    if param.name == symbol {
                        if let Some(span) =
                            self.find_name_occurrence_in_span(text, &param.name, &expr.span)
                        {
                            out.push(span);
                        }
                    }
                }
                self.collect_symbol_spans_expr(text, symbol, body, out);
            }
            Expr::Match { expr, arms } => {
                self.collect_symbol_spans_expr(text, symbol, expr, out);
                for arm in arms {
                    self.collect_symbol_spans_block(text, symbol, &arm.body, out);
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let crate::ast::StringPart::Expr(inner) = part {
                        self.collect_symbol_spans_expr(text, symbol, inner, out);
                    }
                }
            }
            Expr::AsyncBlock(block) | Expr::Block(block) => {
                self.collect_symbol_spans_block(text, symbol, block, out);
            }
            Expr::Require { condition, message } => {
                self.collect_symbol_spans_expr(text, symbol, condition, out);
                if let Some(msg) = message {
                    self.collect_symbol_spans_expr(text, symbol, msg, out);
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(start) = start {
                    self.collect_symbol_spans_expr(text, symbol, start, out);
                }
                if let Some(end) = end {
                    self.collect_symbol_spans_expr(text, symbol, end, out);
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_symbol_spans_expr(text, symbol, condition, out);
                self.collect_symbol_spans_block(text, symbol, then_branch, out);
                if let Some(else_branch) = else_branch {
                    self.collect_symbol_spans_block(text, symbol, else_branch, out);
                }
            }
            Expr::Literal(_) | Expr::This => {}
        }
    }

    fn find_name_occurrence_in_span(
        &self,
        text: &str,
        name: &str,
        span: &std::ops::Range<usize>,
    ) -> Option<std::ops::Range<usize>> {
        if name.is_empty() || span.start >= span.end || span.end > text.len() {
            return None;
        }
        let bytes = text.as_bytes();
        let sym = name.as_bytes();
        let mut i = span.start;
        while i + sym.len() <= span.end {
            if &bytes[i..i + sym.len()] == sym {
                let left_ok =
                    i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
                let right_idx = i + sym.len();
                let right_ok = right_idx == bytes.len()
                    || !(bytes[right_idx].is_ascii_alphanumeric() || bytes[right_idx] == b'_');
                if left_ok && right_ok {
                    return Some(i..right_idx);
                }
            }
            i += 1;
        }
        None
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
            "extern",
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
            ("extern", "Declare an external C ABI function\n\n```apex\nextern function puts(msg: String): Integer;\nextern(c) function puts2(msg: String): Integer;\nextern(system, \"printf\") function sys_printf(fmt: String, ...): Integer;\n```"),
            ("async", "Async function or block\n\n```apex\nasync function foo(): Task<String> { }\n```"),
            ("await", "Await an async operation\n\n```apex\nlet result = await asyncFunction();\n```"),
            ("return", "Return from function\n\n```apex\nreturn value;\n```"),
        ].iter().cloned().collect();

        if let Some(word) = self.word_at_position(&doc.text, pos) {
            if let Some(doc) = keywords_docs.get(word.as_str()) {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: (*doc).to_string(),
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
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
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
        let pos = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(program) = &doc.parsed {
                if let Some(symbol) = self.word_at_position(&doc.text, pos) {
                    let locations = self.definition_locations(
                        &uri,
                        &doc.text,
                        program,
                        &symbol,
                        self.position_to_offset(&doc.text, pos),
                    );
                    if !locations.is_empty() {
                        return Ok(Some(GotoDefinitionResponse::Array(locations)));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(program) = &doc.parsed {
                if let Some(symbol) = self.word_at_position(&doc.text, pos) {
                    let cursor_offset = self.position_to_offset(&doc.text, pos);
                    let ranges =
                        self.find_symbol_ranges(&doc.text, program, &symbol, cursor_offset);
                    let locations: Vec<Location> = ranges
                        .into_iter()
                        .map(|range| Location::new(uri.clone(), range))
                        .collect();
                    return Ok(Some(locations));
                }
            }
        }
        Ok(Some(Vec::new()))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = params.new_name;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(program) = &doc.parsed {
                if let Some(symbol) = self.word_at_position(&doc.text, pos) {
                    let cursor_offset = self.position_to_offset(&doc.text, pos);
                    let ranges =
                        self.find_symbol_ranges(&doc.text, program, &symbol, cursor_offset);
                    let edits: Vec<TextEdit> = ranges
                        .into_iter()
                        .map(|range| TextEdit {
                            range,
                            new_text: new_name.clone(),
                        })
                        .collect();

                    let mut changes = HashMap::new();
                    changes.insert(uri.clone(), edits);
                    return Ok(Some(WorkspaceEdit {
                        changes: Some(changes),
                        document_changes: None,
                        change_annotations: None,
                    }));
                }
            }
        }

        Ok(None)
    }
}

/// Run the LSP server
pub async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
