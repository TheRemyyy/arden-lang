use crate::cli::output::{cli_accent, format_cli_path};
use crate::cli::paths::validate_source_file_path;
use crate::diagnostics::format_parse_error;
use crate::lexer;
use crate::parser::Parser;
use colored::Colorize;
use std::fs;
use std::path::Path;

pub(crate) fn lex_file(file: &Path) -> Result<(), String> {
    validate_source_file_path(file)?;

    let source = fs::read_to_string(file).map_err(|e| {
        format!(
            "{}: Failed to read file '{}': {}",
            "error".red().bold(),
            format_cli_path(file),
            e
        )
    })?;

    let tokens = lexer::tokenize(&source).map_err(|e| {
        format!(
            "{}: Lexer error in '{}': {}",
            "error".red().bold(),
            format_cli_path(file),
            e
        )
    })?;

    println!("{}", cli_accent("Tokens"));
    for (token, span) in tokens {
        println!("  {:?} @ {}..{}", token, span.start, span.end);
    }

    Ok(())
}

pub(crate) fn parse_file(file: &Path) -> Result<(), String> {
    validate_source_file_path(file)?;

    let source = fs::read_to_string(file).map_err(|e| {
        format!(
            "{}: Failed to read file '{}': {}",
            "error".red().bold(),
            format_cli_path(file),
            e
        )
    })?;

    let tokens = lexer::tokenize(&source).map_err(|e| {
        format!(
            "{}: Lexer error in '{}': {}",
            "error".red().bold(),
            format_cli_path(file),
            e
        )
    })?;

    let filename = format_cli_path(file);
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, &source, &filename))?;

    println!("{}", cli_accent("AST"));
    println!("{:#?}", program);

    Ok(())
}
