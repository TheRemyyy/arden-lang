//! Import checker - verifies that all used functions are imported

use crate::ast::*;
use crate::stdlib::StdLib;
use std::collections::{HashMap, HashSet};

/// Error when using function without importing it
#[derive(Debug, Clone)]
pub struct ImportError {
    pub function_name: String,
    pub defined_in: String,
    pub used_in: String,
    #[allow(dead_code)]
    pub span: Span,
}

impl ImportError {
    pub fn format(&self) -> String {
        format!(
            "Function '{}' is defined in '{}' but not imported in '{}'\n  \
             Hint: Add 'import {}.{};' to the top of your file",
            self.function_name, self.defined_in, self.used_in, self.defined_in, self.function_name
        )
    }
}

/// Tracks which functions are defined in which files/namespaces
pub struct ImportChecker {
    /// function_name -> namespace (e.g., "factorial" -> "utils.math")
    function_namespaces: HashMap<String, String>,
    /// Current file namespace
    current_namespace: String,
    /// Imported functions in current file (just the name, e.g., "factorial")
    imported_functions: HashSet<String>,
    /// All imports (for wildcard resolution)
    #[allow(dead_code)]
    wildcard_imports: Vec<String>, // e.g., ["utils.math", "utils.strings"]
    /// Standard library registry
    stdlib: StdLib,
    errors: Vec<ImportError>,
}

impl ImportChecker {
    pub fn new(
        function_namespaces: HashMap<String, String>,
        current_namespace: String,
        imports: Vec<ImportDecl>,
    ) -> Self {
        let mut imported_functions = HashSet::new();
        let mut wildcard_imports = Vec::new();

        for import in imports {
            let path = import.path;

            if path.ends_with(".*") {
                // Wildcard import: utils.math.*
                let ns = path.trim_end_matches(".*");
                wildcard_imports.push(ns.to_string());

                // Add all functions from this namespace (user-defined)
                for (func, func_ns) in &function_namespaces {
                    if func_ns == ns {
                        imported_functions.insert(func.clone());
                    }
                }

                // Add all stdlib functions from this namespace
                let stdlib = StdLib::new();
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

        Self {
            function_namespaces,
            current_namespace,
            imported_functions,
            wildcard_imports,
            stdlib: StdLib::new(),
            errors: Vec::new(),
        }
    }

    /// Check if a function call is valid (imported or local)
    pub fn check_function_call(&mut self, name: &str, span: Span) {
        // Skip if it's a local function (defined in current file)
        if let Some(ns) = self.function_namespaces.get(name) {
            if ns == &self.current_namespace {
                // Local function - OK
                return;
            }

            // Check if imported
            if !self.imported_functions.contains(name) {
                self.errors.push(ImportError {
                    function_name: name.to_string(),
                    defined_in: ns.clone(),
                    used_in: self.current_namespace.clone(),
                    span,
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
                self.errors.push(ImportError {
                    function_name: name.to_string(),
                    defined_in: ns.clone(),
                    used_in: self.current_namespace.clone(),
                    span,
                });
            }
        }
        // If not in function_namespaces or stdlib, it might be a builtin (like println) - OK
    }

    /// Check an expression for function calls
    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { callee, args, .. } => {
                // Check if callee is a simple identifier
                if let Expr::Ident(name) = &callee.node {
                    self.check_function_call(name, callee.span.clone());
                } else {
                    // Check callee expression recursively
                    self.check_expr(&callee.node);
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
        for decl in &program.declarations {
            if let Decl::Function(func) = &decl.node {
                for stmt in &func.body {
                    self.check_stmt(&stmt.node);
                }
            }
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

    for decl in &program.declarations {
        if let Decl::Function(func) = &decl.node {
            result.insert(func.name.clone(), namespace.to_string());
        }
    }

    result
}
