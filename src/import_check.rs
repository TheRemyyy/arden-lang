//! Import checker - verifies that all used functions are imported

use crate::ast::*;
use crate::stdlib::StdLib;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Error when using function without importing it
#[derive(Debug, Clone)]
pub struct ImportError {
    pub function_name: String,
    pub defined_in: String,
    pub used_in: String,
    #[allow(dead_code)]
    pub span: Span,
    pub suggestion: Option<String>,
}

impl ImportError {
    pub fn format(&self) -> String {
        if self.defined_in == "<unknown namespace alias>" {
            return format!(
                "Unknown namespace alias usage '{}' in '{}'\n  \
                 Hint: Import an existing namespace with 'import <namespace> as <alias>;'",
                self.function_name, self.used_in
            );
        }

        let import_hint = if self.function_name.contains("__") {
            format!("import {}.*;", self.defined_in)
        } else {
            format!("import {}.{};", self.defined_in, self.function_name)
        };

        let mut result = format!(
            "Function '{}' is defined in '{}' but not imported in '{}'\n  \
             Hint: Add '{}' to the top of your file",
            self.function_name, self.defined_in, self.used_in, import_hint
        );

        if let Some(suggestion) = &self.suggestion {
            result.push_str(&format!("\n  Or did you mean: '{}'?", suggestion));
        }

        result
    }
}

/// Calculate Levenshtein distance between two strings
#[allow(clippy::needless_range_loop)]
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut prev: Vec<usize> = (0..=len_b).collect();
    let mut curr: Vec<usize> = vec![0; len_b + 1];

    for (i, ca) in a_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b_chars.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[len_b]
}

/// Find the closest matching string from candidates
fn did_you_mean(name: &str, candidates: &[String]) -> Option<String> {
    let mut best_match: Option<(String, usize)> = None;

    for candidate in candidates {
        let distance = levenshtein_distance(name, candidate);
        // Only suggest if distance is reasonable (<= 3 and less than half the length)
        let threshold = (name.len() / 2).max(3);
        if distance <= threshold {
            if let Some((_, best_distance)) = &best_match {
                if distance < *best_distance {
                    best_match = Some((candidate.clone(), distance));
                }
            } else {
                best_match = Some((candidate.clone(), distance));
            }
        }
    }

    best_match.map(|(s, _)| s)
}

/// Tracks which functions are defined in which files/namespaces
pub struct ImportChecker<'a> {
    /// function_name -> namespace (e.g., "factorial" -> "utils.math")
    function_namespaces: Arc<HashMap<String, String>>,
    /// Current file namespace
    current_namespace: String,
    /// Imported functions in current file (just the name, e.g., "factorial")
    imported_functions: HashSet<String>,
    /// All imports (for wildcard resolution)
    #[allow(dead_code)]
    wildcard_imports: Vec<String>, // e.g., ["utils.math", "utils.strings"]
    /// Namespace aliases from imports (`import std.io as io`)
    namespace_aliases: HashMap<String, String>,
    /// Aliases that were declared but do not resolve to a known namespace.
    invalid_namespace_aliases: HashSet<String>,
    /// Standard library registry
    stdlib: &'a StdLib,
    /// Available function names for suggestions
    available_functions: Vec<String>,
    /// Functions declared in currently checked program/file.
    local_functions: HashSet<String>,
    /// Collected errors
    errors: Vec<ImportError>,
}

impl<'a> ImportChecker<'a> {
    pub fn new(
        function_namespaces: Arc<HashMap<String, String>>,
        known_namespace_paths: Arc<HashSet<String>>,
        current_namespace: String,
        imports: Vec<ImportDecl>,
        stdlib: &'a StdLib,
    ) -> Self {
        let mut imported_functions = HashSet::new();
        let mut wildcard_imports = Vec::new();
        let mut namespace_aliases = HashMap::new();
        let mut invalid_namespace_aliases = HashSet::new();
        let known_namespaces: HashSet<String> = function_namespaces
            .values()
            .cloned()
            .chain(stdlib.get_functions().values().cloned())
            .chain(known_namespace_paths.iter().cloned())
            .collect();

        for import in imports {
            let path = import.path;
            let alias = import.alias;

            if let Some(alias_name) = alias {
                // Alias only namespaces (e.g. import std.math as math).
                // Function aliasing is parser-accepted syntax but import checking currently
                // remains conservative and does not auto-import by alias identifier.
                if known_namespaces.contains(&path) {
                    namespace_aliases.insert(alias_name, path.clone());
                } else if path.contains('.') {
                    let mut parts = path.split('.').collect::<Vec<_>>();
                    let symbol = parts.pop().unwrap_or_default();
                    let symbol_ns = parts.join(".");
                    let is_known_symbol_alias = function_namespaces
                        .get(symbol)
                        .is_some_and(|ns| ns == &symbol_ns)
                        || known_namespaces.contains(&symbol_ns)
                        || Self::path_has_known_namespace_prefix(&known_namespace_paths, &path)
                        || stdlib
                            .get_namespace(symbol)
                            .is_some_and(|ns| ns == &symbol_ns)
                        || Self::path_resolves_to_user_module(
                            &function_namespaces,
                            &known_namespace_paths,
                            &path,
                        );
                    if is_known_symbol_alias {
                        namespace_aliases.insert(alias_name, path.clone());
                    } else {
                        invalid_namespace_aliases.insert(alias_name);
                    }
                } else {
                    invalid_namespace_aliases.insert(alias_name);
                }
                continue;
            }

            if path.ends_with(".*") {
                // Wildcard import: utils.math.*
                let ns = path.trim_end_matches(".*");
                wildcard_imports.push(ns.to_string());

                // Add all functions from this namespace (user-defined)
                for (func, func_ns) in function_namespaces.iter() {
                    if func_ns == ns {
                        imported_functions.insert(func.clone());
                    }
                }

                // Add all stdlib functions from this namespace
                for (func, func_ns) in stdlib.get_functions() {
                    if func_ns == ns {
                        imported_functions.insert(func.clone());
                    }
                }
            } else if path.contains('.') {
                // Specific import: utils.math.factorial
                let parts: Vec<&str> = path.split('.').collect();
                if let Some(func_name) = parts.last() {
                    imported_functions.insert(func_name.to_string());
                }
            }
        }

        // Collect available function names for suggestions
        let mut available_functions: Vec<String> = function_namespaces.keys().cloned().collect();
        available_functions.extend(stdlib.get_functions().keys().cloned());

        Self {
            function_namespaces,
            current_namespace,
            imported_functions,
            wildcard_imports,
            namespace_aliases,
            invalid_namespace_aliases,
            stdlib,
            available_functions,
            local_functions: HashSet::new(),
            errors: Vec::new(),
        }
    }

    fn collect_local_functions(&mut self, program: &Program) {
        fn walk_decl(out: &mut HashSet<String>, decl: &Decl, module_prefix: Option<&str>) {
            match decl {
                Decl::Function(func) => {
                    if let Some(module) = module_prefix {
                        out.insert(format!("{}__{}", module, func.name));
                    } else {
                        out.insert(func.name.clone());
                    }
                }
                Decl::Module(module) => {
                    let next_prefix = if let Some(prefix) = module_prefix {
                        format!("{}__{}", prefix, module.name)
                    } else {
                        module.name.clone()
                    };
                    for inner in &module.declarations {
                        walk_decl(out, &inner.node, Some(&next_prefix));
                    }
                }
                _ => {}
            }
        }

        self.local_functions.clear();
        for decl in &program.declarations {
            walk_decl(&mut self.local_functions, &decl.node, None);
        }
    }

    fn namespace_matches_module_hint(namespace: &str, module_hint: &str) -> bool {
        namespace
            .rsplit('.')
            .next()
            .map(|tail| tail.eq_ignore_ascii_case(module_hint))
            .unwrap_or(false)
    }

    fn resolve_stdlib_call_in_namespace(&self, namespace: &str, field: &str) -> Option<String> {
        // Direct form: println / print / read_line
        if self
            .stdlib
            .get_namespace(field)
            .is_some_and(|ns| ns == namespace)
        {
            return Some(field.to_string());
        }

        // Module-mangled form: Math__abs / Str__len / System__os ...
        let suffix = format!("__{}", field);
        let mut found: Option<String> = None;
        for (func, ns) in self.stdlib.get_functions() {
            if ns == namespace && func.ends_with(&suffix) {
                if found.is_some() {
                    // Ambiguous, keep conservative and do not resolve.
                    return None;
                }
                found = Some(func.clone());
            }
        }
        found
    }

    fn resolve_user_call_in_namespace(&self, namespace: &str, field: &str) -> Option<String> {
        let mut found: Option<String> = None;
        for (func, ns) in self.function_namespaces.iter() {
            if ns == namespace && (func == field || func.ends_with(&format!("__{}", field))) {
                if found.is_some() {
                    return None;
                }
                found = Some(func.clone());
            }
        }
        found
    }

    fn path_resolves_to_user_module(
        function_namespaces: &HashMap<String, String>,
        known_namespace_paths: &HashSet<String>,
        path: &str,
    ) -> bool {
        if known_namespace_paths.contains(path) {
            return true;
        }
        let namespaces: HashSet<&str> = function_namespaces.values().map(String::as_str).collect();
        for ns in namespaces {
            if path == ns {
                return true;
            }
            let Some(suffix) = path.strip_prefix(ns) else {
                continue;
            };
            let Some(module_path) = suffix.strip_prefix('.') else {
                continue;
            };
            if module_path.is_empty() {
                continue;
            }
            let module_prefix = module_path.replace('.', "__");
            if function_namespaces.iter().any(|(func, owner)| {
                owner == ns
                    && (func == &module_prefix || func.starts_with(&format!("{}__", module_prefix)))
            }) {
                return true;
            }
        }
        false
    }

    fn path_has_known_namespace_prefix(
        known_namespace_paths: &HashSet<String>,
        path: &str,
    ) -> bool {
        let mut current = path;
        while let Some((prefix, _)) = current.rsplit_once('.') {
            if known_namespace_paths.contains(prefix) {
                return true;
            }
            current = prefix;
        }
        false
    }

    fn resolve_user_call_in_namespace_path(
        &self,
        namespace_path: &str,
        field: &str,
    ) -> Option<String> {
        if let Some(found) = self.resolve_user_call_in_namespace(namespace_path, field) {
            return Some(found);
        }

        let namespaces: HashSet<&str> = self
            .function_namespaces
            .values()
            .map(String::as_str)
            .collect();
        for ns in namespaces {
            let Some(suffix) = namespace_path.strip_prefix(ns) else {
                continue;
            };
            let Some(module_path) = suffix.strip_prefix('.') else {
                continue;
            };
            if module_path.is_empty() {
                continue;
            }
            let module_prefix = module_path.replace('.', "__");
            let candidate = format!("{}__{}", module_prefix, field);
            if self
                .function_namespaces
                .get(&candidate)
                .is_some_and(|owner| owner == ns)
            {
                return Some(candidate);
            }
        }
        None
    }

    /// Check if a function call is valid (imported or local)
    pub fn check_function_call(&mut self, name: &str, span: Span) {
        if self.invalid_namespace_aliases.contains(name) {
            self.errors.push(ImportError {
                function_name: name.to_string(),
                defined_in: "<invalid import alias>".to_string(),
                used_in: self.current_namespace.clone(),
                span,
                suggestion: None,
            });
            return;
        }

        // Local function in the same checked program/file always wins over stdlib names.
        if self.local_functions.contains(name) {
            return;
        }

        // Skip if it's a local function (defined in current file)
        if let Some(ns) = self.function_namespaces.get(name) {
            if ns == &self.current_namespace {
                // Local function - OK
                return;
            }

            // Check if imported
            if !self.imported_functions.contains(name) {
                // Try to find a similar function name
                let suggestion = did_you_mean(name, &self.available_functions);

                self.errors.push(ImportError {
                    function_name: name.to_string(),
                    defined_in: ns.clone(),
                    used_in: self.current_namespace.clone(),
                    span,
                    suggestion,
                });
            }
            return;
        }

        // Check if it's a stdlib function that needs to be imported
        if let Some(ns) = self.stdlib.get_namespace(name) {
            // "builtin" namespace means no import needed
            if ns == "builtin" {
                return;
            }

            // Check if imported (either specific or wildcard)
            if !self.imported_functions.contains(name) && !self.wildcard_imports.contains(ns) {
                // Try to find a similar function name
                let suggestion = did_you_mean(name, &self.available_functions);

                self.errors.push(ImportError {
                    function_name: name.to_string(),
                    defined_in: ns.clone(),
                    used_in: self.current_namespace.clone(),
                    span,
                    suggestion,
                });
            }
        }
        // If not in function_namespaces or stdlib, it might be a builtin (like println) - OK
    }

    /// Check an expression for function calls
    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { callee, args, .. } => {
                match &callee.node {
                    // Direct function call
                    Expr::Ident(name) => self.check_function_call(name, callee.span.clone()),
                    // Module-style call: Module.func(...)
                    Expr::Field { object, field } => {
                        if let Expr::Ident(module_or_type) = &object.node {
                            let mut handled_alias_call = false;
                            if let Some(ns) = self.namespace_aliases.get(module_or_type) {
                                if let Some(canonical_name) =
                                    self.resolve_stdlib_call_in_namespace(ns, field)
                                {
                                    let _ = canonical_name;
                                    handled_alias_call = true;
                                } else if let Some(canonical_name) =
                                    self.resolve_user_call_in_namespace_path(ns, field)
                                {
                                    let _ = canonical_name;
                                    handled_alias_call = true;
                                } else if ns.contains('.') {
                                    // Exact imported symbol aliases like `import util.E as Enum`
                                    // may be used as constructor-like call roots (`Enum.A(...)`).
                                    handled_alias_call = true;
                                }
                            } else if self.invalid_namespace_aliases.contains(module_or_type) {
                                self.errors.push(ImportError {
                                    function_name: format!("{}.{}", module_or_type, field),
                                    defined_in: "<unknown namespace alias>".to_string(),
                                    used_in: self.current_namespace.clone(),
                                    span: callee.span.clone(),
                                    suggestion: None,
                                });
                                handled_alias_call = true;
                            }

                            if !handled_alias_call {
                                let mangled = format!("{}__{}", module_or_type, field);
                                // Only treat as import-checkable function when known.
                                if self.local_functions.contains(&mangled)
                                    || self.function_namespaces.contains_key(&mangled)
                                    || self.stdlib.get_namespace(&mangled).is_some()
                                {
                                    self.check_function_call(&mangled, callee.span.clone());
                                } else if let Some(ns) = self.stdlib.get_namespace(field) {
                                    if Self::namespace_matches_module_hint(ns, module_or_type) {
                                        self.check_function_call(field, callee.span.clone());
                                    } else {
                                        self.check_expr(&callee.node);
                                    }
                                } else {
                                    self.check_expr(&callee.node);
                                }
                            }
                        } else {
                            self.check_expr(&callee.node);
                        }
                    }
                    // Check callee expression recursively
                    _ => self.check_expr(&callee.node),
                }

                // Check arguments
                for arg in args {
                    self.check_expr(&arg.node);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.check_expr(&left.node);
                self.check_expr(&right.node);
            }
            Expr::Unary { expr, .. } => {
                self.check_expr(&expr.node);
            }
            Expr::Field { object, .. } => {
                self.check_expr(&object.node);
            }
            Expr::Index { object, index } => {
                self.check_expr(&object.node);
                self.check_expr(&index.node);
            }

            Expr::Block(block) => {
                for stmt in block {
                    self.check_stmt(&stmt.node);
                }
            }

            Expr::Match { expr, arms } => {
                self.check_expr(&expr.node);
                for arm in arms {
                    for stmt in &arm.body {
                        self.check_stmt(&stmt.node);
                    }
                }
            }
            Expr::Lambda { body, .. } => {
                self.check_expr(&body.node);
            }
            Expr::Construct { args, .. } => {
                for arg in args {
                    self.check_expr(&arg.node);
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.check_expr(&condition.node);
                for stmt in then_branch {
                    self.check_stmt(&stmt.node);
                }
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.check_stmt(&stmt.node);
                    }
                }
            }
            Expr::Require { condition, message } => {
                self.check_expr(&condition.node);
                if let Some(msg) = message {
                    self.check_expr(&msg.node);
                }
            }
            Expr::AsyncBlock(body) => {
                for stmt in body {
                    self.check_stmt(&stmt.node);
                }
            }
            Expr::Try(inner)
            | Expr::Borrow(inner)
            | Expr::MutBorrow(inner)
            | Expr::Deref(inner) => {
                self.check_expr(&inner.node);
            }
            Expr::Await(inner) => {
                self.check_expr(&inner.node);
            }
            Expr::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.check_expr(&s.node);
                }
                if let Some(e) = end {
                    self.check_expr(&e.node);
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let crate::ast::StringPart::Expr(expr) = part {
                        self.check_expr(&expr.node);
                    }
                }
            }
            _ => {} // Literals, identifiers (non-call), etc.
        }
    }

    /// Check a statement for function calls
    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(expr) => {
                self.check_expr(&expr.node);
            }
            Stmt::Let { value, .. } => {
                self.check_expr(&value.node);
            }
            Stmt::Return(Some(expr)) => {
                self.check_expr(&expr.node);
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.check_expr(&condition.node);
                for stmt in then_block {
                    self.check_stmt(&stmt.node);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.check_stmt(&stmt.node);
                    }
                }
            }
            Stmt::While { condition, body } => {
                self.check_expr(&condition.node);
                for stmt in body {
                    self.check_stmt(&stmt.node);
                }
            }
            Stmt::For { iterable, body, .. } => {
                self.check_expr(&iterable.node);
                for stmt in body {
                    self.check_stmt(&stmt.node);
                }
            }
            Stmt::Match { expr, arms } => {
                self.check_expr(&expr.node);
                for arm in arms {
                    for stmt in &arm.body {
                        self.check_stmt(&stmt.node);
                    }
                }
            }
            _ => {} // Break, Continue, Return(None), etc.
        }
    }

    /// Check entire program for import violations
    pub fn check_program(&mut self, program: &Program) -> Result<(), Vec<ImportError>> {
        fn check_decl(checker: &mut ImportChecker<'_>, decl: &Decl) {
            match decl {
                Decl::Function(func) => {
                    for stmt in &func.body {
                        checker.check_stmt(&stmt.node);
                    }
                }
                Decl::Class(class) => {
                    if let Some(ctor) = &class.constructor {
                        for stmt in &ctor.body {
                            checker.check_stmt(&stmt.node);
                        }
                    }
                    if let Some(dtor) = &class.destructor {
                        for stmt in &dtor.body {
                            checker.check_stmt(&stmt.node);
                        }
                    }
                    for method in &class.methods {
                        for stmt in &method.body {
                            checker.check_stmt(&stmt.node);
                        }
                    }
                }
                Decl::Module(module) => {
                    for inner in &module.declarations {
                        check_decl(checker, &inner.node);
                    }
                }
                Decl::Interface(interface) => {
                    for method in &interface.methods {
                        if let Some(default_impl) = &method.default_impl {
                            for stmt in default_impl {
                                checker.check_stmt(&stmt.node);
                            }
                        }
                    }
                }
                Decl::Enum(_) | Decl::Import(_) => {}
            }
        }

        self.collect_local_functions(program);

        for decl in &program.declarations {
            check_decl(self, &decl.node);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }
}

/// Extract all function definitions from a program with their namespace
#[allow(dead_code)]
pub fn extract_function_namespaces(program: &Program, namespace: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    fn walk_decl(
        out: &mut HashMap<String, String>,
        decl: &Decl,
        namespace: &str,
        module_prefix: Option<String>,
    ) {
        match decl {
            Decl::Function(func) => {
                if let Some(module) = module_prefix {
                    out.insert(format!("{}__{}", module, func.name), namespace.to_string());
                } else {
                    out.insert(func.name.clone(), namespace.to_string());
                }
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner in &module.declarations {
                    walk_decl(out, &inner.node, namespace, Some(next_prefix.clone()));
                }
            }
            Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }
    for decl in &program.declarations {
        walk_decl(&mut result, &decl.node, namespace, None);
    }

    result
}

pub fn extract_known_namespace_paths(program: &Program, namespace: &str) -> HashSet<String> {
    let mut result = HashSet::from([namespace.to_string()]);

    fn walk_decl(
        out: &mut HashSet<String>,
        decl: &Decl,
        namespace: &str,
        module_prefix: Option<String>,
    ) {
        match decl {
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}.{}", prefix, module.name)
                } else {
                    format!("{}.{}", namespace, module.name)
                };
                out.insert(next_prefix.clone());
                for inner in &module.declarations {
                    walk_decl(out, &inner.node, namespace, Some(next_prefix.clone()));
                }
            }
            Decl::Function(_)
            | Decl::Class(_)
            | Decl::Enum(_)
            | Decl::Interface(_)
            | Decl::Import(_) => {}
        }
    }

    for decl in &program.declarations {
        walk_decl(&mut result, &decl.node, namespace, None);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::Parser;
    use crate::stdlib::stdlib_registry;

    fn check_import_errors(source: &str) -> Vec<ImportError> {
        let tokens = tokenize(source).expect("tokenize");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("parse");
        let namespace = program
            .package
            .clone()
            .unwrap_or_else(|| "global".to_string());
        let imports = program
            .declarations
            .iter()
            .filter_map(|d| match &d.node {
                Decl::Import(i) => Some(i.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let function_namespaces = extract_function_namespaces(&program, &namespace);
        let known_namespace_paths = extract_known_namespace_paths(&program, &namespace);
        let mut checker = ImportChecker::new(
            Arc::new(function_namespaces),
            Arc::new(known_namespace_paths),
            namespace,
            imports,
            stdlib_registry(),
        );

        checker.check_program(&program).err().unwrap_or_default()
    }

    #[test]
    fn local_function_can_shadow_stdlib_name() {
        let source = r#"
function print(owned s: String): None { return None; }
function main(): None {
    s: String = "x";
    print(s);
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert!(errors.is_empty());
    }

    #[test]
    fn module_dot_stdlib_call_requires_import() {
        let source = r#"
function main(): None {
    x: Float = Math.abs(-1.0);
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn alias_import_allows_namespaced_stdlib_calls() {
        let source = r#"
import std.io as io;
import std.math as math;
import std.string as str;

function main(): None {
    io.println("x");
    y: Integer = math.abs(-2);
    z: Integer = str.len("ok");
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert!(errors.is_empty());
    }

    #[test]
    fn dotted_module_alias_allows_module_style_calls() {
        let source = r#"
package lib;
import lib.A.X as ax;

module A {
    module X {
        function f(): Integer { return 1; }
    }
}

function main(): None {
    x: Integer = ax.f();
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn alias_call_still_checks_nested_argument_calls() {
        let source = r#"
import std.io as io;
function main(): None {
    io.println(to_string(Math.abs(-3)));
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn invalid_namespace_alias_reports_import_error_on_use() {
        let source = r#"
import does_not_exist as dne;
function main(): None {
    dne.print("x");
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "dne.print");
        assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
    }

    #[test]
    fn invalid_dotted_namespace_alias_reports_import_error_on_use() {
        let source = r#"
import nope.ns as n;
function main(): None {
    n.call();
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "n.call");
        assert_eq!(errors[0].defined_in, "<unknown namespace alias>");
    }

    #[test]
    fn exact_imported_enum_alias_allows_variant_calls() {
        let source = r#"
package app;
import util.E as Enum;

function main(): None {
    Enum.A(1);
    return None;
}
"#;
        let tokens = tokenize(source).expect("tokenize");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("parse");
        let imports = program
            .declarations
            .iter()
            .filter_map(|d| match &d.node {
                Decl::Import(i) => Some(i.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut checker = ImportChecker::new(
            Arc::new(HashMap::new()),
            Arc::new(HashSet::from(["util".to_string()])),
            "app".to_string(),
            imports,
            stdlib_registry(),
        );
        let errors = checker.check_program(&program).err().unwrap_or_default();
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn invalid_namespace_alias_format_message_is_actionable() {
        let err = ImportError {
            function_name: "dne.print".to_string(),
            defined_in: "<unknown namespace alias>".to_string(),
            used_in: "app".to_string(),
            span: 0..0,
            suggestion: None,
        };

        let rendered = err.format();
        assert!(rendered.contains("Unknown namespace alias usage 'dne.print'"));
        assert!(rendered.contains("import <namespace> as <alias>;"));
        assert!(!rendered.contains("<unknown namespace alias>.dne.print"));
    }

    #[test]
    fn if_expression_condition_checks_missing_imports() {
        let source = r#"
function main(): None {
    x: Integer = if (Math.abs(-1.0) > 0.0) { 1; } else { 2; };
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn if_expression_branch_checks_missing_imports() {
        let source = r#"
function main(): None {
    x: Float = if (true) { Math.abs(-1.0); } else { 0.0; };
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn require_expression_checks_missing_imports() {
        let source = r#"
function main(): None {
    require(Math.abs(-1.0) > 0.0, "x");
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn async_block_checks_missing_imports() {
        let source = r#"
function main(): None {
    t: Task<Integer> = async { return Math.abs(-1); };
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn class_method_checks_missing_imports() {
        let source = r#"
class C {
    function compute(): Float {
        return Math.abs(-1.0);
    }
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn constructor_checks_missing_imports() {
        let source = r#"
class C {
    constructor() {
        x: Float = Math.abs(-2.0);
    }
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn module_function_checks_missing_imports() {
        let source = r#"
module Utils {
    function f(): Float {
        return Math.abs(-3.0);
    }
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn interface_default_impl_checks_missing_imports() {
        let source = r#"
interface I {
    function f(): Float {
        return Math.abs(-4.0);
    }
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn extracts_module_functions_as_mangled_namespaces() {
        let source = r#"
module MathEx {
    function addOne(x: Integer): Integer { return x + 1; }
}
"#;
        let tokens = tokenize(source).expect("tokenize");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("parse");
        let map = extract_function_namespaces(&program, "demo");
        assert!(map.contains_key("MathEx__addOne"));
        assert!(!map.contains_key("addOne"));
    }

    #[test]
    fn extracts_nested_module_functions_as_deep_mangled_namespaces() {
        let source = r#"
module Outer {
    module Inner {
        function ping(): Integer { return 1; }
    }
}
"#;
        let tokens = tokenize(source).expect("tokenize");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("parse");
        let map = extract_function_namespaces(&program, "demo");
        assert!(map.contains_key("Outer__Inner__ping"));
        assert!(!map.contains_key("Inner__ping"));
    }

    #[test]
    fn alias_namespace_does_not_import_direct_mangled_stdlib_calls() {
        let source = r#"
import std.math as math;
function main(): None {
    x: Float = Math__abs(-1.0);
    y: Float = math.abs(-2.0);
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn alias_namespace_does_not_import_module_style_symbol_without_alias() {
        let source = r#"
import std.math as math;
function main(): None {
    x: Float = Math.abs(-1.0);
    y: Float = math.abs(-2.0);
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "Math__abs");
    }

    #[test]
    fn invalid_namespace_alias_direct_call_is_reported_at_import_check_time() {
        let source = r#"
import nope.missing as alias;
function main(): None {
    alias();
    return None;
}
"#;
        let errors = check_import_errors(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].function_name, "alias");
    }
}
