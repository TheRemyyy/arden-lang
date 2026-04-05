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

        if self.defined_in == "<unresolved import alias>" {
            return format!(
                "Imported alias '{}' no longer resolves in '{}'\n  \
                 Hint: Update or remove the stale import for '{}'",
                self.function_name, self.used_in, self.function_name
            );
        }

        if self.defined_in == "<unresolved namespace alias member>" {
            let (alias, member) = self
                .function_name
                .split_once('.')
                .unwrap_or((self.function_name.as_str(), ""));
            return format!(
                "Imported namespace alias '{}' has no member '{}' in '{}'\n  \
                 Hint: Update the import target or the member access",
                alias, member, self.used_in
            );
        }

        if self.defined_in == "<unresolved wildcard import>" {
            return format!(
                "Wildcard import '{}.*' no longer provides '{}' in '{}'\n  \
                 Hint: Update the wildcard import target or the referenced symbol",
                self.suggestion.as_deref().unwrap_or(""),
                self.function_name,
                self.used_in
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

fn flatten_field_chain(expr: &Expr) -> Option<Vec<String>> {
    match expr {
        Expr::Ident(name) => Some(vec![name.clone()]),
        Expr::Field { object, field } => {
            let mut parts = flatten_field_chain(&object.node)?;
            parts.push(field.clone());
            Some(parts)
        }
        _ => None,
    }
}

fn looks_like_function_symbol(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
}

fn builtin_exact_import_alias_canonical(path: &str) -> Option<&'static str> {
    match path {
        "Option.Some" => Some("Option__some"),
        "Option.None" => Some("Option__none"),
        "Result.Ok" => Some("Result__ok"),
        "Result.Error" => Some("Result__error"),
        _ => None,
    }
}

fn direct_wildcard_member_name(
    import_path: &str,
    owner_ns: &str,
    symbol_name: &str,
) -> Option<String> {
    if owner_ns == import_path {
        return (!symbol_name.contains("__")).then(|| symbol_name.to_string());
    }

    let module_path = import_path.strip_prefix(owner_ns)?.strip_prefix('.')?;
    if module_path.is_empty() {
        return None;
    }
    let module_prefix = module_path.replace('.', "__");
    let remainder = symbol_name.strip_prefix(&format!("{}__", module_prefix))?;
    (!remainder.is_empty() && !remainder.contains("__")).then(|| remainder.to_string())
}

/// Tracks which functions are defined in which files/namespaces
pub struct ImportChecker<'a> {
    /// function_name -> namespace (e.g., "factorial" -> "utils.math")
    function_namespaces: Arc<HashMap<String, String>>,
    /// Known user namespace/module paths for module-aware import reasoning.
    known_namespace_paths: Arc<HashSet<String>>,
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
    /// Lexical scopes for local value bindings that can shadow import aliases.
    local_scopes: Vec<HashSet<String>>,
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
                    let current_qualified_symbol_ns = if current_namespace.is_empty() {
                        symbol_ns.clone()
                    } else {
                        format!("{}.{}", current_namespace, symbol_ns)
                    };
                    let is_known_symbol_alias = function_namespaces
                        .get(symbol)
                        .is_some_and(|ns| ns == &symbol_ns)
                        || builtin_exact_import_alias_canonical(&path).is_some()
                        || known_namespaces.contains(&symbol_ns)
                        || known_namespaces.contains(&current_qualified_symbol_ns)
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
                    let current_qualified_path = if current_namespace.is_empty() {
                        path.clone()
                    } else {
                        format!("{}.{}", current_namespace, path)
                    };
                    let is_current_namespace_symbol_alias = function_namespaces
                        .get(&path)
                        .is_some_and(|ns| ns == &current_namespace)
                        || stdlib
                            .get_namespace(&path)
                            .is_some_and(|ns| ns == &current_namespace);
                    if known_namespaces.contains(&current_qualified_path)
                        || is_current_namespace_symbol_alias
                    {
                        namespace_aliases.insert(alias_name, current_qualified_path);
                    } else {
                        invalid_namespace_aliases.insert(alias_name);
                    }
                }
                continue;
            }

            if path.ends_with(".*") {
                // Wildcard import: utils.math.*
                let ns = path.trim_end_matches(".*");
                wildcard_imports.push(ns.to_string());

                // Add all functions from this namespace (user-defined)
                for (func, func_ns) in function_namespaces.iter() {
                    if let Some(imported_name) = direct_wildcard_member_name(ns, func_ns, func) {
                        imported_functions.insert(imported_name);
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
            known_namespace_paths,
            current_namespace,
            imported_functions,
            wildcard_imports,
            namespace_aliases,
            invalid_namespace_aliases,
            stdlib,
            available_functions,
            local_functions: HashSet::new(),
            local_scopes: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.local_scopes.push(HashSet::new());
    }

    fn exit_scope(&mut self) {
        let _ = self.local_scopes.pop();
    }

    fn bind_local(&mut self, name: &str) {
        if let Some(scope) = self.local_scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn bind_parameter_locals(&mut self, params: &[Parameter]) {
        for param in params {
            self.bind_local(&param.name);
        }
    }

    fn bind_pattern_locals(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Ident(name) => self.bind_local(name),
            Pattern::Variant(_, bindings) => {
                for binding in bindings {
                    self.bind_local(binding);
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
    }

    fn is_local_value(&self, name: &str) -> bool {
        self.local_scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    fn check_block_in_scope(&mut self, block: &[Spanned<Stmt>]) {
        self.enter_scope();
        for stmt in block {
            self.check_stmt(&stmt.node);
        }
        self.exit_scope();
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

    fn check_qualified_name_alias_usage(&mut self, name: &str, span: Span) {
        if self.invalid_namespace_aliases.contains(name) {
            self.errors.push(ImportError {
                function_name: name.to_string(),
                defined_in: "<unknown namespace alias>".to_string(),
                used_in: self.current_namespace.clone(),
                span,
                suggestion: None,
            });
            return;
        }
        let Some((alias, _)) = name.split_once('.') else {
            return;
        };
        if self.invalid_namespace_aliases.contains(alias) {
            self.errors.push(ImportError {
                function_name: name.to_string(),
                defined_in: "<unknown namespace alias>".to_string(),
                used_in: self.current_namespace.clone(),
                span,
                suggestion: None,
            });
        }
    }

    fn check_type(&mut self, ty: &Type, span: Span) {
        match ty {
            Type::Named(name) => {
                if self.check_simple_wildcard_imported_type(name, span.clone()) {
                    return;
                }
                if let Some(path) = self.namespace_aliases.get(name) {
                    if path.contains('.') && !self.exact_import_alias_resolves(path) {
                        self.errors.push(ImportError {
                            function_name: name.clone(),
                            defined_in: "<unresolved import alias>".to_string(),
                            used_in: self.current_namespace.clone(),
                            span,
                            suggestion: None,
                        });
                        return;
                    }
                }
                self.check_qualified_name_alias_usage(name, span)
            }
            Type::Generic(name, args) => {
                if self.check_simple_wildcard_imported_type(name, span.clone()) {
                    return;
                }
                if let Some(path) = self.namespace_aliases.get(name) {
                    if path.contains('.') && !self.exact_import_alias_resolves(path) {
                        self.errors.push(ImportError {
                            function_name: name.clone(),
                            defined_in: "<unresolved import alias>".to_string(),
                            used_in: self.current_namespace.clone(),
                            span: span.clone(),
                            suggestion: None,
                        });
                        return;
                    }
                }
                self.check_qualified_name_alias_usage(name, span.clone());
                for arg in args {
                    self.check_type(arg, span.clone());
                }
            }
            Type::Function(params, ret) => {
                for param in params {
                    self.check_type(param, span.clone());
                }
                self.check_type(ret, span);
            }
            Type::Option(inner)
            | Type::List(inner)
            | Type::Set(inner)
            | Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Box(inner)
            | Type::Rc(inner)
            | Type::Arc(inner)
            | Type::Ptr(inner)
            | Type::Task(inner)
            | Type::Range(inner) => self.check_type(inner, span),
            Type::Result(ok, err) | Type::Map(ok, err) => {
                self.check_type(ok, span.clone());
                self.check_type(err, span);
            }
            Type::Integer
            | Type::Float
            | Type::Boolean
            | Type::String
            | Type::Char
            | Type::None => {}
        }
    }

    fn check_decl_type(&mut self, ty: &Type, span: Span) {
        match ty {
            Type::Named(name) => {
                if let Some(path_parts) = parse_alias_member_path(name) {
                    if self
                        .check_alias_member_call(&path_parts, span.clone())
                        .is_some()
                    {
                        return;
                    }
                }
                self.check_type(ty, span);
            }
            Type::Generic(name, args) => {
                if let Some(path_parts) = parse_alias_member_path(name) {
                    if self
                        .check_alias_member_call(&path_parts, span.clone())
                        .is_some()
                    {
                        for arg in args {
                            self.check_decl_type(arg, span.clone());
                        }
                        return;
                    }
                }
                self.check_type(ty, span.clone());
                for arg in args {
                    self.check_decl_type(arg, span.clone());
                }
            }
            Type::Function(params, ret) => {
                for param in params {
                    self.check_decl_type(param, span.clone());
                }
                self.check_decl_type(ret, span);
            }
            Type::Option(inner)
            | Type::List(inner)
            | Type::Set(inner)
            | Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Box(inner)
            | Type::Rc(inner)
            | Type::Arc(inner)
            | Type::Ptr(inner)
            | Type::Task(inner)
            | Type::Range(inner) => self.check_decl_type(inner, span),
            Type::Result(ok, err) | Type::Map(ok, err) => {
                self.check_decl_type(ok, span.clone());
                self.check_decl_type(err, span);
            }
            Type::Integer
            | Type::Float
            | Type::Boolean
            | Type::String
            | Type::Char
            | Type::None => self.check_type(ty, span),
        }
    }

    fn check_pattern(&mut self, pattern: &Pattern, span: Span) {
        if let Pattern::Variant(name, _) = pattern {
            if let Some(path) = self.namespace_aliases.get(name) {
                if path.contains('.') && !self.exact_import_alias_resolves(path) {
                    self.errors.push(ImportError {
                        function_name: name.clone(),
                        defined_in: "<unresolved import alias>".to_string(),
                        used_in: self.current_namespace.clone(),
                        span: span.clone(),
                        suggestion: None,
                    });
                    return;
                }
            }
            self.check_qualified_name_alias_usage(name, span);
        }
    }

    fn check_generic_param_bounds(&mut self, generic_params: &[GenericParam]) {
        for param in generic_params {
            for bound in &param.bounds {
                if let Ok(parsed_ty) = crate::parser::parse_type_source(bound) {
                    self.check_decl_type(&parsed_ty, 0..0);
                } else {
                    self.check_qualified_name_alias_usage(bound, 0..0);
                }
            }
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
        let mut namespace_candidates = vec![namespace_path.to_string()];
        if !self.current_namespace.is_empty()
            && namespace_path != self.current_namespace
            && !namespace_path.starts_with(&format!("{}.", self.current_namespace))
        {
            namespace_candidates.push(format!("{}.{}", self.current_namespace, namespace_path));
        }

        for namespace_candidate in namespace_candidates {
            if let Some(found) = self.resolve_user_call_in_namespace(&namespace_candidate, field) {
                return Some(found);
            }

            let namespaces: HashSet<&str> = self
                .function_namespaces
                .values()
                .map(String::as_str)
                .collect();
            for ns in namespaces {
                let Some(suffix) = namespace_candidate.strip_prefix(ns) else {
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
        }
        None
    }

    fn check_alias_member_call(&mut self, path_parts: &[String], span: Span) -> Option<()> {
        if path_parts.len() < 2 {
            return None;
        }
        let alias = path_parts.first()?;
        if self.is_local_value(alias) {
            return None;
        }
        let member = path_parts.last()?;
        let alias_path = self.namespace_aliases.get(alias)?;
        let namespace_path = if path_parts.len() == 2 {
            alias_path.clone()
        } else {
            format!(
                "{}.{}",
                alias_path,
                path_parts[1..path_parts.len() - 1].join(".")
            )
        };
        let full_path = format!("{}.{}", namespace_path, member);

        if self.known_namespace_paths.contains(&full_path) {
            return Some(());
        }

        if self
            .resolve_stdlib_call_in_namespace(&namespace_path, member)
            .is_some()
            || self
                .resolve_user_call_in_namespace_path(&namespace_path, member)
                .is_some()
        {
            return Some(());
        }

        if alias_path.contains('.') && path_parts.len() == 2 {
            if self.exact_import_alias_resolves(alias_path) {
                return Some(());
            }
            self.errors.push(ImportError {
                function_name: alias.clone(),
                defined_in: "<unresolved import alias>".to_string(),
                used_in: self.current_namespace.clone(),
                span,
                suggestion: None,
            });
            return Some(());
        }

        self.errors.push(ImportError {
            function_name: format!("{}.{}", alias, path_parts[1..].join(".")),
            defined_in: "<unresolved namespace alias member>".to_string(),
            used_in: self.current_namespace.clone(),
            span,
            suggestion: None,
        });
        Some(())
    }

    fn check_simple_wildcard_imported_type(&mut self, name: &str, span: Span) -> bool {
        if name.contains('.')
            || self.namespace_aliases.contains_key(name)
            || self.invalid_namespace_aliases.contains(name)
        {
            return false;
        }
        if !name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return false;
        }
        let local_path = format!("{}.{}", self.current_namespace, name);
        if self.known_namespace_paths.contains(&local_path) {
            return false;
        }
        if self.wildcard_imports.len() != 1 {
            return false;
        }
        let wildcard = self.wildcard_imports[0].clone();
        if !Self::path_resolves_to_user_module(
            &self.function_namespaces,
            &self.known_namespace_paths,
            &wildcard,
        ) {
            return false;
        }
        let imported_path = format!("{}.{}", wildcard, name);
        if self.known_namespace_paths.contains(&imported_path) {
            return false;
        }

        self.errors.push(ImportError {
            function_name: name.to_string(),
            defined_in: "<unresolved wildcard import>".to_string(),
            used_in: self.current_namespace.clone(),
            span,
            suggestion: Some(wildcard),
        });
        true
    }

    fn exact_import_alias_resolves(&self, path: &str) -> bool {
        if builtin_exact_import_alias_canonical(path).is_some() {
            return true;
        }
        let mut path_candidates = vec![path.to_string()];
        if !self.current_namespace.is_empty()
            && path != self.current_namespace
            && !path.starts_with(&format!("{}.", self.current_namespace))
        {
            path_candidates.push(format!("{}.{}", self.current_namespace, path));
        }
        if path_candidates
            .iter()
            .any(|candidate| self.known_namespace_paths.contains(candidate))
        {
            return true;
        }
        let Some((namespace, symbol)) = path.rsplit_once('.') else {
            return false;
        };
        self.resolve_stdlib_call_in_namespace(namespace, symbol)
            .is_some()
            || self
                .resolve_user_call_in_namespace_path(namespace, symbol)
                .is_some()
    }

    /// Check if a function call is valid (imported or local)
    pub fn check_function_call(&mut self, name: &str, span: Span) {
        if self.is_local_value(name) {
            return;
        }

        if self.invalid_namespace_aliases.contains(name) {
            self.errors.push(ImportError {
                function_name: name.to_string(),
                defined_in: "<unknown namespace alias>".to_string(),
                used_in: self.current_namespace.clone(),
                span,
                suggestion: None,
            });
            return;
        }

        if self.imported_functions.contains(name) {
            return;
        }

        if let Some(path) = self.namespace_aliases.get(name) {
            if path.contains('.') {
                if self.exact_import_alias_resolves(path) {
                    return;
                }
                self.errors.push(ImportError {
                    function_name: name.to_string(),
                    defined_in: "<unresolved import alias>".to_string(),
                    used_in: self.current_namespace.clone(),
                    span,
                    suggestion: None,
                });
                return;
            }
            if !path.ends_with(".*") {
                let mut parts = path.split('.').collect::<Vec<_>>();
                if let Some(symbol) = parts.pop() {
                    let namespace = parts.join(".");
                    if self
                        .resolve_stdlib_call_in_namespace(&namespace, symbol)
                        .is_some()
                    {
                        return;
                    }
                    if self
                        .resolve_user_call_in_namespace_path(&namespace, symbol)
                        .is_some()
                    {
                        return;
                    }

                    self.errors.push(ImportError {
                        function_name: name.to_string(),
                        defined_in: "<unresolved import alias>".to_string(),
                        used_in: self.current_namespace.clone(),
                        span,
                        suggestion: None,
                    });
                    return;
                }
            }
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
                    span: span.clone(),
                    suggestion,
                });
            }
            return;
        }
        if self.wildcard_imports.len() == 1
            && self.wildcard_imports.first().is_some_and(|wildcard| {
                Self::path_resolves_to_user_module(
                    &self.function_namespaces,
                    &self.known_namespace_paths,
                    wildcard,
                )
            })
        {
            self.errors.push(ImportError {
                function_name: name.to_string(),
                defined_in: "<unresolved wildcard import>".to_string(),
                used_in: self.current_namespace.clone(),
                span,
                suggestion: self.wildcard_imports.first().cloned(),
            });
        }
        // If not in function_namespaces or stdlib, it might be a builtin (like println) - OK
    }

    /// Check an expression for function calls
    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { callee, args, .. } => {
                if let Some(path_parts) = flatten_field_chain(&callee.node) {
                    if self
                        .check_alias_member_call(&path_parts, callee.span.clone())
                        .is_some()
                    {
                        for arg in args {
                            self.check_expr(&arg.node);
                        }
                        if let Expr::Call { type_args, .. } = expr {
                            for ty in type_args {
                                self.check_decl_type(ty, 0..0);
                            }
                        }
                        return;
                    }
                }
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
                if let Expr::Call { type_args, .. } = expr {
                    for ty in type_args {
                        self.check_decl_type(ty, 0..0);
                    }
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
                if let Some(path_parts) = flatten_field_chain(expr) {
                    if self.check_alias_member_call(&path_parts, 0..0).is_some() {
                        return;
                    }
                }
                if !matches!(
                    &object.node,
                    Expr::Ident(name)
                        if self.namespace_aliases.contains_key(name)
                            || self.invalid_namespace_aliases.contains(name)
                ) {
                    self.check_expr(&object.node);
                }
            }
            Expr::Index { object, index } => {
                self.check_expr(&object.node);
                self.check_expr(&index.node);
            }

            Expr::Block(block) => {
                self.check_block_in_scope(block);
            }

            Expr::Match { expr, arms } => {
                self.check_expr(&expr.node);
                for arm in arms {
                    self.enter_scope();
                    self.check_pattern(&arm.pattern, 0..0);
                    self.bind_pattern_locals(&arm.pattern);
                    for stmt in &arm.body {
                        self.check_stmt(&stmt.node);
                    }
                    self.exit_scope();
                }
            }
            Expr::Lambda { body, params } => {
                if let Expr::Lambda { params, .. } = expr {
                    for param in params {
                        self.check_decl_type(&param.ty, 0..0);
                    }
                }
                self.enter_scope();
                self.bind_parameter_locals(params);
                self.check_expr(&body.node);
                self.exit_scope();
            }
            Expr::Construct { args, .. } => {
                if let Expr::Construct { ty, .. } = expr {
                    if let Ok(parsed_ty) = crate::parser::parse_type_source(ty) {
                        self.check_decl_type(&parsed_ty, 0..0);
                    } else {
                        self.check_qualified_name_alias_usage(ty, 0..0);
                    }
                }
                for arg in args {
                    self.check_expr(&arg.node);
                }
            }
            Expr::GenericFunctionValue { callee, type_args } => {
                if let Some(path_parts) = flatten_field_chain(&callee.node) {
                    if self
                        .check_alias_member_call(&path_parts, callee.span.clone())
                        .is_some()
                    {
                        for ty in type_args {
                            self.check_decl_type(ty, 0..0);
                        }
                        return;
                    }
                }

                match &callee.node {
                    Expr::Ident(name) => {
                        if let Some(path) = self.namespace_aliases.get(name) {
                            let imported_symbol = path.rsplit('.').next().unwrap_or(path.as_str());
                            if looks_like_function_symbol(imported_symbol) {
                                self.check_function_call(name, callee.span.clone());
                            }
                        } else {
                            self.check_function_call(name, callee.span.clone());
                        }
                    }
                    Expr::Field { object, field } => {
                        if let Expr::Ident(module_or_type) = &object.node {
                            let mut handled_alias_value = false;
                            if let Some(ns) = self.namespace_aliases.get(module_or_type) {
                                if self.resolve_stdlib_call_in_namespace(ns, field).is_some()
                                    || self
                                        .resolve_user_call_in_namespace_path(ns, field)
                                        .is_some()
                                {
                                    handled_alias_value = true;
                                }
                            } else if self.invalid_namespace_aliases.contains(module_or_type) {
                                self.errors.push(ImportError {
                                    function_name: format!("{}.{}", module_or_type, field),
                                    defined_in: "<unknown namespace alias>".to_string(),
                                    used_in: self.current_namespace.clone(),
                                    span: callee.span.clone(),
                                    suggestion: None,
                                });
                                handled_alias_value = true;
                            }

                            if !handled_alias_value {
                                let mangled = format!("{}__{}", module_or_type, field);
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
                    _ => self.check_expr(&callee.node),
                }

                for ty in type_args {
                    self.check_decl_type(ty, 0..0);
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                self.check_expr(&condition.node);
                self.check_block_in_scope(then_branch);
                if let Some(else_stmts) = else_branch {
                    self.check_block_in_scope(else_stmts);
                }
            }
            Expr::Require { condition, message } => {
                self.check_expr(&condition.node);
                if let Some(msg) = message {
                    self.check_expr(&msg.node);
                }
            }
            Expr::AsyncBlock(body) => {
                self.check_block_in_scope(body);
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
            Expr::Ident(name) => {
                if self.is_local_value(name) {
                    return;
                }
                if let Some(path) = self.namespace_aliases.get(name) {
                    if path.contains('.') {
                        let imported_symbol = path.rsplit('.').next().unwrap_or(path.as_str());
                        if looks_like_function_symbol(imported_symbol) {
                            self.check_function_call(name, 0..0);
                        }
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
            Stmt::Assign { target, value } => {
                self.check_expr(&target.node);
                self.check_expr(&value.node);
            }
            Stmt::Let { value, .. } => {
                if let Stmt::Let { ty, .. } = stmt {
                    self.check_decl_type(ty, 0..0);
                }
                self.check_expr(&value.node);
                if let Stmt::Let { name, .. } = stmt {
                    self.bind_local(name);
                }
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
                self.check_block_in_scope(then_block);
                if let Some(else_stmts) = else_block {
                    self.check_block_in_scope(else_stmts);
                }
            }
            Stmt::While { condition, body } => {
                self.check_expr(&condition.node);
                self.check_block_in_scope(body);
            }
            Stmt::For { iterable, body, .. } => {
                if let Stmt::For {
                    var_type: Some(var_type),
                    ..
                } = stmt
                {
                    self.check_decl_type(var_type, 0..0);
                }
                self.check_expr(&iterable.node);
                self.enter_scope();
                if let Stmt::For { var, .. } = stmt {
                    self.bind_local(var);
                }
                for stmt in body {
                    self.check_stmt(&stmt.node);
                }
                self.exit_scope();
            }
            Stmt::Match { expr, arms } => {
                self.check_expr(&expr.node);
                for arm in arms {
                    self.enter_scope();
                    self.check_pattern(&arm.pattern, 0..0);
                    self.bind_pattern_locals(&arm.pattern);
                    for stmt in &arm.body {
                        self.check_stmt(&stmt.node);
                    }
                    self.exit_scope();
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
                    checker.check_generic_param_bounds(&func.generic_params);
                    let check_signature_type = |checker: &mut ImportChecker<'_>, ty: &Type| {
                        if func.is_extern || func.extern_abi.is_some() {
                            checker.check_decl_type(ty, 0..0);
                        } else {
                            checker.check_type(ty, 0..0);
                        }
                    };
                    for param in &func.params {
                        check_signature_type(checker, &param.ty);
                    }
                    check_signature_type(checker, &func.return_type);
                    checker.enter_scope();
                    checker.bind_parameter_locals(&func.params);
                    for stmt in &func.body {
                        checker.check_stmt(&stmt.node);
                    }
                    checker.exit_scope();
                }
                Decl::Class(class) => {
                    checker.check_generic_param_bounds(&class.generic_params);
                    if let Some(parent) = &class.extends {
                        if let Ok(parsed_ty) = crate::parser::parse_type_source(parent) {
                            checker.check_decl_type(&parsed_ty, 0..0);
                        } else {
                            checker.check_qualified_name_alias_usage(parent, 0..0);
                        }
                    }
                    for implemented in &class.implements {
                        if let Ok(parsed_ty) = crate::parser::parse_type_source(implemented) {
                            checker.check_decl_type(&parsed_ty, 0..0);
                        } else {
                            checker.check_qualified_name_alias_usage(implemented, 0..0);
                        }
                    }
                    for field in &class.fields {
                        checker.check_decl_type(&field.ty, 0..0);
                    }
                    if let Some(ctor) = &class.constructor {
                        for param in &ctor.params {
                            checker.check_decl_type(&param.ty, 0..0);
                        }
                        checker.enter_scope();
                        checker.bind_parameter_locals(&ctor.params);
                        for stmt in &ctor.body {
                            checker.check_stmt(&stmt.node);
                        }
                        checker.exit_scope();
                    }
                    if let Some(dtor) = &class.destructor {
                        checker.check_block_in_scope(&dtor.body);
                    }
                    for method in &class.methods {
                        checker.check_generic_param_bounds(&method.generic_params);
                        for param in &method.params {
                            checker.check_decl_type(&param.ty, 0..0);
                        }
                        checker.check_decl_type(&method.return_type, 0..0);
                        checker.enter_scope();
                        checker.bind_parameter_locals(&method.params);
                        for stmt in &method.body {
                            checker.check_stmt(&stmt.node);
                        }
                        checker.exit_scope();
                    }
                }
                Decl::Module(module) => {
                    for inner in &module.declarations {
                        check_decl(checker, &inner.node);
                    }
                }
                Decl::Interface(interface) => {
                    checker.check_generic_param_bounds(&interface.generic_params);
                    for extended in &interface.extends {
                        if let Ok(parsed_ty) = crate::parser::parse_type_source(extended) {
                            checker.check_decl_type(&parsed_ty, 0..0);
                        } else {
                            checker.check_qualified_name_alias_usage(extended, 0..0);
                        }
                    }
                    for method in &interface.methods {
                        for param in &method.params {
                            checker.check_decl_type(&param.ty, 0..0);
                        }
                        checker.check_decl_type(&method.return_type, 0..0);
                        if let Some(default_impl) = &method.default_impl {
                            checker.enter_scope();
                            checker.bind_parameter_locals(&method.params);
                            for stmt in default_impl {
                                checker.check_stmt(&stmt.node);
                            }
                            checker.exit_scope();
                        }
                    }
                }
                Decl::Enum(en) => {
                    checker.check_generic_param_bounds(&en.generic_params);
                    for variant in &en.variants {
                        for field in &variant.fields {
                            checker.check_decl_type(&field.ty, 0..0);
                        }
                    }
                }
                Decl::Import(_) => {}
            }
        }

        self.collect_local_functions(program);

        for decl in &program.declarations {
            check_decl(self, &decl.node);
        }

        let mut seen = HashSet::new();
        self.errors.retain(|error| {
            seen.insert((
                error.function_name.clone(),
                error.defined_in.clone(),
                error.used_in.clone(),
                error.suggestion.clone(),
            ))
        });

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }
}

fn parse_alias_member_path(name: &str) -> Option<Vec<String>> {
    let mut parts = name
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    if let Some(last) = parts.last_mut() {
        if let Some((base, _)) = last.split_once('<') {
            *last = base.to_string();
        }
    }
    Some(parts)
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
            Decl::Class(class) => {
                let path = if let Some(prefix) = module_prefix {
                    format!("{}.{}", prefix, class.name)
                } else {
                    format!("{}.{}", namespace, class.name)
                };
                out.insert(path);
            }
            Decl::Enum(en) => {
                let path = if let Some(prefix) = module_prefix.as_ref() {
                    format!("{}.{}", prefix, en.name)
                } else {
                    format!("{}.{}", namespace, en.name)
                };
                out.insert(path);
                for variant in &en.variants {
                    let variant_path = if let Some(prefix) = module_prefix.as_ref() {
                        format!("{}.{}.{}", prefix, en.name, variant.name)
                    } else {
                        format!("{}.{}.{}", namespace, en.name, variant.name)
                    };
                    out.insert(variant_path);
                }
            }
            Decl::Interface(interface) => {
                let path = if let Some(prefix) = module_prefix {
                    format!("{}.{}", prefix, interface.name)
                } else {
                    format!("{}.{}", namespace, interface.name)
                };
                out.insert(path);
            }
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
            Decl::Function(_) | Decl::Import(_) => {}
        }
    }

    for decl in &program.declarations {
        walk_decl(&mut result, &decl.node, namespace, None);
    }

    result
}

#[cfg(test)]
mod tests;
