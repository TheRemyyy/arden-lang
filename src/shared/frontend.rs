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
use std::sync::Arc;

pub(crate) fn parse_program_from_source(source: &str, filename: &str) -> Result<Program, String> {
    let tokens = lexer::tokenize(source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;
    let mut parser = Parser::new(tokens);
    parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, source, filename))
}

pub(crate) fn run_single_file_semantic_checks(
    source: &str,
    filename: &str,
    program: &Program,
) -> Result<(), String> {
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
        return Err(rendered.trim_end().to_string());
    }

    let mut type_checker = TypeChecker::new();
    if let Err(errors) = type_checker.check(program) {
        return Err(typeck::format_errors(&errors, source, filename));
    }

    let mut borrow_checker = BorrowChecker::new();
    if let Err(errors) = borrow_checker.check(program) {
        return Err(borrowck::format_borrow_errors(&errors, source, filename));
    }

    Ok(())
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
        Err(typeck::format_errors(&errors, source, filename))
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
