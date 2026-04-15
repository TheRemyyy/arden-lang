use crate::ast::{self, Decl, Program, Type};
use crate::borrowck;
use crate::borrowck::BorrowChecker;
use crate::diagnostics::format_parse_error;
use crate::import_check;
use crate::import_check::ImportChecker;
use crate::lexer;
use crate::parser::Parser;
use crate::stdlib::stdlib_registry;
use crate::typeck;
use crate::typeck::TypeChecker;
use colored::Colorize;
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
enum ParseFrontendError {
    Lexer(String),
    Parser(String),
}

impl fmt::Display for ParseFrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lexer(message) | Self::Parser(message) => write!(f, "{message}"),
        }
    }
}

impl From<ParseFrontendError> for String {
    fn from(value: ParseFrontendError) -> Self {
        value.to_string()
    }
}

#[derive(Debug)]
enum SemanticFrontendError {
    SymbolCollision(String),
    ImportCheck(String),
    Typecheck(String),
    BorrowCheck(String),
    MainSignature(String),
}

impl fmt::Display for SemanticFrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SymbolCollision(message)
            | Self::ImportCheck(message)
            | Self::Typecheck(message)
            | Self::BorrowCheck(message)
            | Self::MainSignature(message) => write!(f, "{message}"),
        }
    }
}

impl From<SemanticFrontendError> for String {
    fn from(value: SemanticFrontendError) -> Self {
        value.to_string()
    }
}

pub(crate) fn parse_program_from_source(source: &str, filename: &str) -> Result<Program, String> {
    parse_program_from_source_impl(source, filename).map_err(Into::into)
}

fn parse_program_from_source_impl(
    source: &str,
    filename: &str,
) -> Result<Program, ParseFrontendError> {
    let tokens = lexer::tokenize(source).map_err(|e| {
        ParseFrontendError::Lexer(format!("{}: Lexer error: {}", "error".red().bold(), e))
    })?;
    let mut parser = Parser::new(tokens);
    parser
        .parse_program()
        .map_err(|e| ParseFrontendError::Parser(format_parse_error(&e, source, filename)))
}

pub(crate) fn run_single_file_semantic_checks(
    source: &str,
    filename: &str,
    program: &Program,
) -> Result<(), String> {
    run_single_file_semantic_checks_impl(source, filename, program).map_err(Into::into)
}

fn run_single_file_semantic_checks_impl(
    source: &str,
    filename: &str,
    program: &Program,
) -> Result<(), SemanticFrontendError> {
    validate_single_file_top_level_symbol_collisions(program)?;

    let namespace = extract_namespace(program);
    let imports = extract_top_level_imports(program);
    let function_namespaces = import_check::extract_function_namespaces(program, &namespace);
    let known_namespace_paths = import_check::extract_known_namespace_paths(program, &namespace);
    let mut import_checker = ImportChecker::new(
        Arc::new(function_namespaces),
        Arc::new(known_namespace_paths),
        namespace,
        imports,
        stdlib_registry(),
    );
    if let Err(errors) = import_checker.check_program(program) {
        let mut rendered = String::new();
        for err in errors {
            rendered.push_str(&err.format_with_source(source, filename));
            rendered.push('\n');
        }
        return Err(SemanticFrontendError::ImportCheck(
            rendered.trim_end().to_string(),
        ));
    }

    let mut type_checker = TypeChecker::new();
    if let Err(errors) = type_checker.check(program) {
        return Err(SemanticFrontendError::Typecheck(typeck::format_errors(
            &errors, source, filename,
        )));
    }

    let mut borrow_checker = BorrowChecker::new();
    if let Err(errors) = borrow_checker.check(program) {
        return Err(SemanticFrontendError::BorrowCheck(
            borrowck::format_borrow_errors(&errors, source, filename),
        ));
    }

    Ok(())
}

fn validate_single_file_top_level_symbol_collisions(
    program: &Program,
) -> Result<(), SemanticFrontendError> {
    let mut seen_functions = HashSet::new();
    let mut seen_classes = HashSet::new();
    let mut seen_enums = HashSet::new();
    let mut seen_interfaces = HashSet::new();
    let mut seen_modules = HashSet::new();
    let mut collisions = Vec::new();

    for decl in &program.declarations {
        match &decl.node {
            Decl::Function(function) => {
                if !seen_functions.insert(function.name.clone()) {
                    collisions.push(format!(
                        "Function '{}' is declared multiple times",
                        function.name
                    ));
                }
            }
            Decl::Class(class_decl) => {
                if !seen_classes.insert(class_decl.name.clone()) {
                    collisions.push(format!(
                        "Class '{}' is declared multiple times",
                        class_decl.name
                    ));
                }
            }
            Decl::Enum(enum_decl) => {
                if !seen_enums.insert(enum_decl.name.clone()) {
                    collisions.push(format!(
                        "Enum '{}' is declared multiple times",
                        enum_decl.name
                    ));
                }
            }
            Decl::Interface(interface_decl) => {
                if !seen_interfaces.insert(interface_decl.name.clone()) {
                    collisions.push(format!(
                        "Interface '{}' is declared multiple times",
                        interface_decl.name
                    ));
                }
            }
            Decl::Module(module_decl) => {
                if !seen_modules.insert(module_decl.name.clone()) {
                    collisions.push(format!(
                        "Module '{}' is declared multiple times",
                        module_decl.name
                    ));
                }
            }
            Decl::Import(_) => {}
        }
    }

    if collisions.is_empty() {
        return Ok(());
    }

    let details = collisions
        .into_iter()
        .map(|item| format!("  - {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    Err(SemanticFrontendError::SymbolCollision(format!(
        "{}: Single-file checks found colliding top-level symbols.\n{}",
        "error".red().bold(),
        details
    )))
}

pub(crate) fn extract_namespace(program: &ast::Program) -> String {
    program
        .package
        .clone()
        .unwrap_or_else(|| "global".to_string())
}

pub(crate) fn validate_entry_main_signature(
    program: &Program,
    source: &str,
    filename: &str,
) -> Result<(), String> {
    validate_entry_main_signature_impl(program, source, filename).map_err(Into::into)
}

fn validate_entry_main_signature_impl(
    program: &Program,
    source: &str,
    filename: &str,
) -> Result<(), SemanticFrontendError> {
    let mut errors = Vec::new();
    for decl in &program.declarations {
        let Decl::Function(func) = &decl.node else {
            continue;
        };
        if func.name != "main" {
            continue;
        }

        if !func.generic_params.is_empty() {
            errors.push(typeck::TypeError::new(
                "main() cannot declare generic parameters",
                decl.span.clone(),
            ));
        }
        if !func.params.is_empty() {
            errors.push(typeck::TypeError::new(
                "main() cannot declare parameters",
                decl.span.clone(),
            ));
        }
        if func.is_async {
            errors.push(typeck::TypeError::new(
                "main() cannot be async; use a synchronous main() entrypoint",
                decl.span.clone(),
            ));
        }
        if func.is_extern || func.extern_abi.is_some() {
            errors.push(typeck::TypeError::new(
                "main() cannot be declared extern",
                decl.span.clone(),
            ));
        }
        if func.is_variadic {
            errors.push(typeck::TypeError::new(
                "main() cannot be variadic",
                decl.span.clone(),
            ));
        }
        if !matches!(func.return_type, Type::None | Type::Integer) {
            errors.push(typeck::TypeError::new(
                "main() must return None or Integer",
                decl.span.clone(),
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SemanticFrontendError::MainSignature(typeck::format_errors(
            &errors, source, filename,
        )))
    }
}

pub(crate) fn extract_imports(program: &ast::Program) -> Vec<ast::ImportDecl> {
    fn collect_imports(
        declarations: &[ast::Spanned<ast::Decl>],
        imports: &mut Vec<ast::ImportDecl>,
    ) {
        for decl in declarations {
            match &decl.node {
                ast::Decl::Import(import) => imports.push(import.clone()),
                ast::Decl::Module(module) => collect_imports(&module.declarations, imports),
                _ => {}
            }
        }
    }

    let mut imports = Vec::new();
    collect_imports(&program.declarations, &mut imports);
    imports
}

pub(crate) fn extract_top_level_imports(program: &ast::Program) -> Vec<ast::ImportDecl> {
    program
        .declarations
        .iter()
        .filter_map(|decl| match &decl.node {
            ast::Decl::Import(import) => Some(import.clone()),
            _ => None,
        })
        .collect()
}
