//! Standard library namespace definitions
//!
//! All standard library functions are organized under `std` namespace:
//! - std.io - Console I/O helpers
//! - std.fs - File operations
//! - std.system - Environment, shell, system calls
//! - std.time - Time-related functions
//! - std.math - Mathematical functions
//! - std.string - String utilities

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::OnceLock;

static STDLIB_REGISTRY: OnceLock<StdLib> = OnceLock::new();
const STDLIB_FUNCTIONS: &[(&str, &str)] = &[
    ("println", "std.io"),
    ("print", "std.io"),
    ("read_line", "std.io"),
    ("File__read", "std.fs"),
    ("File__write", "std.fs"),
    ("File__exists", "std.fs"),
    ("File__delete", "std.fs"),
    ("System__getenv", "std.system"),
    ("System__shell", "std.system"),
    ("System__exec", "std.system"),
    ("System__cwd", "std.system"),
    ("System__os", "std.system"),
    ("System__exit", "std.system"),
    ("Time__now", "std.time"),
    ("Time__unix", "std.time"),
    ("Time__sleep", "std.time"),
    ("Args__count", "std.args"),
    ("Args__get", "std.args"),
    ("Math__sqrt", "std.math"),
    ("Math__sin", "std.math"),
    ("Math__cos", "std.math"),
    ("Math__tan", "std.math"),
    ("Math__pow", "std.math"),
    ("Math__abs", "std.math"),
    ("Math__min", "std.math"),
    ("Math__max", "std.math"),
    ("Math__floor", "std.math"),
    ("Math__ceil", "std.math"),
    ("Math__round", "std.math"),
    ("Math__log", "std.math"),
    ("Math__log10", "std.math"),
    ("Math__exp", "std.math"),
    ("Math__pi", "std.math"),
    ("Math__e", "std.math"),
    ("Math__random", "std.math"),
    ("Str__len", "std.string"),
    ("Str__compare", "std.string"),
    ("Str__concat", "std.string"),
    ("Str__upper", "std.string"),
    ("Str__lower", "std.string"),
    ("Str__trim", "std.string"),
    ("Str__contains", "std.string"),
    ("Str__startsWith", "std.string"),
    ("Str__endsWith", "std.string"),
    ("assert", "builtin"),
    ("assert_eq", "builtin"),
    ("assert_ne", "builtin"),
    ("assert_true", "builtin"),
    ("assert_false", "builtin"),
    ("fail", "builtin"),
    ("to_string", "builtin"),
    ("exit", "builtin"),
    ("range", "builtin"),
];
const STDLIB_NAMESPACES: &[&str] = &[
    "builtin",
    "std.io",
    "std.fs",
    "std.system",
    "std.time",
    "std.args",
    "std.math",
    "std.string",
    "std.net",
];

fn alias_lookup_key(namespace_path: &str, member: &str) -> String {
    format!("{namespace_path}\u{0}{member}")
}

pub fn stdlib_registry() -> &'static StdLib {
    STDLIB_REGISTRY.get_or_init(StdLib::new)
}

/// Standard library function registry
pub struct StdLib {
    /// function_name -> namespace (e.g., "println" -> "std.io")
    functions: HashMap<String, String>,
    alias_calls: HashMap<String, String>,
    namespaces: HashSet<String>,
}

impl StdLib {
    pub fn new() -> Self {
        let mut functions = HashMap::with_capacity(STDLIB_FUNCTIONS.len());
        let mut alias_calls = HashMap::with_capacity(STDLIB_FUNCTIONS.len());
        let mut namespaces =
            HashSet::with_capacity(STDLIB_NAMESPACES.len() + STDLIB_FUNCTIONS.len());

        for namespace in STDLIB_NAMESPACES {
            namespaces.insert((*namespace).to_string());
        }

        for (function_name, namespace) in STDLIB_FUNCTIONS {
            functions.insert((*function_name).to_string(), (*namespace).to_string());
            namespaces.insert((*namespace).to_string());
            let member = function_name
                .split_once("__")
                .map(|(_, member)| member)
                .unwrap_or(function_name);
            alias_calls.insert(
                alias_lookup_key(namespace, member),
                (*function_name).to_string(),
            );
        }

        Self {
            functions,
            alias_calls,
            namespaces,
        }
    }

    /// Get the namespace for a stdlib function (returns None if not found)
    pub fn get_namespace(&self, name: &str) -> Option<&String> {
        self.functions.get(name)
    }

    /// Get all std functions as a map
    pub fn get_functions(&self) -> &HashMap<String, String> {
        &self.functions
    }

    pub fn known_namespaces(&self) -> &HashSet<String> {
        &self.namespaces
    }

    /// Resolve aliased std namespace member call to canonical callable symbol.
    ///
    /// Example:
    /// - ("std.io", "println") -> Some("println")
    /// - ("std.math", "abs") -> Some("Math__abs")
    /// - ("std.string", "len") -> Some("Str__len")
    pub fn resolve_alias_call(&self, namespace_path: &str, member: &str) -> Option<String> {
        self.alias_calls
            .get(&alias_lookup_key(namespace_path, member))
            .cloned()
    }
}

impl Default for StdLib {
    fn default() -> Self {
        Self::new()
    }
}
