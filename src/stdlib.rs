//! Standard library namespace definitions
//!
//! All standard library functions are organized under `std` namespace:
//! - std.io - File operations, print functions
//! - std.system - Environment, shell, system calls
//! - std.time - Time-related functions
//! - std.math - Mathematical functions
//! - std.string - String utilities

use std::collections::HashMap;

/// Standard library function registry
pub struct StdLib {
    /// function_name -> namespace (e.g., "println" -> "std.io")
    functions: HashMap<String, String>,
}

impl StdLib {
    pub fn new() -> Self {
        let mut functions = HashMap::new();

        // std.io - Input/Output
        functions.insert("println".to_string(), "std.io".to_string());
        functions.insert("print".to_string(), "std.io".to_string());
        functions.insert("read_line".to_string(), "std.io".to_string());

        // std.fs - File System
        functions.insert("File__read".to_string(), "std.fs".to_string());
        functions.insert("File__write".to_string(), "std.fs".to_string());
        functions.insert("File__exists".to_string(), "std.fs".to_string());
        functions.insert("File__delete".to_string(), "std.fs".to_string());

        // std.system - System operations
        functions.insert("System__getenv".to_string(), "std.system".to_string());
        functions.insert("System__shell".to_string(), "std.system".to_string());
        functions.insert("System__exec".to_string(), "std.system".to_string());
        functions.insert("System__cwd".to_string(), "std.system".to_string());
        functions.insert("System__os".to_string(), "std.system".to_string());
        functions.insert("System__exit".to_string(), "std.system".to_string());
        functions.insert("System__args".to_string(), "std.system".to_string());

        // std.time - Time functions
        functions.insert("Time__now".to_string(), "std.time".to_string());
        functions.insert("Time__unix".to_string(), "std.time".to_string());
        functions.insert("Time__sleep".to_string(), "std.time".to_string());

        // std.math - Math functions (already in Math module)
        functions.insert("Math__sqrt".to_string(), "std.math".to_string());
        functions.insert("Math__sin".to_string(), "std.math".to_string());
        functions.insert("Math__cos".to_string(), "std.math".to_string());
        functions.insert("Math__tan".to_string(), "std.math".to_string());
        functions.insert("Math__pow".to_string(), "std.math".to_string());
        functions.insert("Math__abs".to_string(), "std.math".to_string());
        functions.insert("Math__floor".to_string(), "std.math".to_string());
        functions.insert("Math__ceil".to_string(), "std.math".to_string());
        functions.insert("Math__round".to_string(), "std.math".to_string());
        functions.insert("Math__log".to_string(), "std.math".to_string());
        functions.insert("Math__log10".to_string(), "std.math".to_string());
        functions.insert("Math__exp".to_string(), "std.math".to_string());
        functions.insert("Math__pi".to_string(), "std.math".to_string());
        functions.insert("Math__e".to_string(), "std.math".to_string());
        functions.insert("Math__random".to_string(), "std.math".to_string());

        // std.string - String utilities
        functions.insert("Str__len".to_string(), "std.string".to_string());
        functions.insert("Str__compare".to_string(), "std.string".to_string());
        functions.insert("Str__concat".to_string(), "std.string".to_string());
        functions.insert("Str__upper".to_string(), "std.string".to_string());
        functions.insert("Str__lower".to_string(), "std.string".to_string());
        functions.insert("Str__trim".to_string(), "std.string".to_string());
        functions.insert("Str__contains".to_string(), "std.string".to_string());
        functions.insert("Str__startsWith".to_string(), "std.string".to_string());
        functions.insert("Str__endsWith".to_string(), "std.string".to_string());

        // std.assert - Assertion functions for testing (builtin - no import needed)
        functions.insert("assert".to_string(), "builtin".to_string());
        functions.insert("assert_eq".to_string(), "builtin".to_string());
        functions.insert("assert_ne".to_string(), "builtin".to_string());
        functions.insert("assert_true".to_string(), "builtin".to_string());
        functions.insert("assert_false".to_string(), "builtin".to_string());
        functions.insert("assert_null".to_string(), "builtin".to_string());
        functions.insert("assert_not_null".to_string(), "builtin".to_string());
        functions.insert("fail".to_string(), "builtin".to_string());

        // Builtin functions that DON'T require import
        // These are language primitives
        functions.insert("to_string".to_string(), "builtin".to_string());
        functions.insert("length".to_string(), "builtin".to_string());
        functions.insert("exit".to_string(), "builtin".to_string());
        functions.insert("range".to_string(), "builtin".to_string());

        Self { functions }
    }

    /// Get the namespace for a stdlib function (returns None if not found)
    pub fn get_namespace(&self, name: &str) -> Option<&String> {
        self.functions.get(name)
    }

    /// Get all std functions as a map
    #[allow(dead_code)]
    pub fn get_functions(&self) -> &HashMap<String, String> {
        &self.functions
    }
}

impl Default for StdLib {
    fn default() -> Self {
        Self::new()
    }
}
