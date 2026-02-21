//! Apex Programming Language Compiler

mod ast;
mod borrowck;
mod codegen;
mod import_check;
mod lexer;
mod namespace;
mod parser;
mod project;
mod stdlib;
mod typeck;

use clap::{Parser as ClapParser, Subcommand};
use colored::*;
use inkwell::context::Context;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ast::{Decl, ImportDecl, Program};
use crate::borrowck::BorrowChecker;
use crate::codegen::Codegen;
use crate::import_check::ImportChecker;
use crate::parser::Parser;
use crate::project::{find_project_root, ProjectConfig};
use crate::typeck::TypeChecker;

#[derive(ClapParser)]
#[command(name = "apex")]
#[command(author = "Remyyy")]
#[command(version = "1.2.0")]
#[command(about = "Apex Programming Language Compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Apex project
    New {
        /// Project name
        name: String,
        /// Project path (default: current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Build the current project
    Build {
        /// Release build (optimized)
        #[arg(short, long)]
        release: bool,
        /// Emit LLVM IR
        #[arg(long)]
        emit_llvm: bool,
        /// Skip type checking
        #[arg(long)]
        no_check: bool,
    },
    /// Build and run the current project (or a single file)
    Run {
        /// Input file (optional, runs project if not specified)
        file: Option<PathBuf>,
        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Release build (optimized)
        #[arg(short, long)]
        release: bool,
        /// Skip type checking
        #[arg(long)]
        no_check: bool,
    },
    /// Compile a single file (legacy mode)
    Compile {
        /// Input file
        file: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Emit LLVM IR
        #[arg(long)]
        emit_llvm: bool,
        /// Skip type checking
        #[arg(long)]
        no_check: bool,
    },
    /// Check syntax and types of a file
    Check {
        /// Input file (default: project entry point)
        file: Option<PathBuf>,
    },
    /// Show project info
    Info,
    /// Show tokens (debug)
    Lex {
        /// Input file
        file: PathBuf,
    },
    /// Show AST (debug)
    Parse {
        /// Input file
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::New { name, path } => new_project(&name, path.as_deref()),
        Commands::Build {
            release,
            emit_llvm,
            no_check,
        } => build_project(release, emit_llvm, !no_check),
        Commands::Run {
            file,
            args,
            release,
            no_check,
        } => {
            if let Some(f) = file {
                run_single_file(&f, &args, release, !no_check)
            } else {
                run_project(&args, release, !no_check)
            }
        }
        Commands::Compile {
            file,
            output,
            emit_llvm,
            no_check,
        } => compile_file(&file, output.as_deref(), emit_llvm, !no_check),
        Commands::Check { file } => check_file(file.as_deref()),
        Commands::Info => show_project_info(),
        Commands::Lex { file } => lex_file(&file),
        Commands::Parse { file } => parse_file(&file),
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    std::process::exit(0);
}

/// Create a new project
fn new_project(name: &str, path: Option<&Path>) -> Result<(), String> {
    let project_path = path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(name));

    if project_path.exists() {
        return Err(format!(
            "{}: Directory '{}' already exists",
            "error".red().bold(),
            project_path.display()
        ));
    }

    // Create project directory structure
    fs::create_dir_all(&project_path).map_err(|e| {
        format!(
            "{}: Failed to create project directory: {}",
            "error".red().bold(),
            e
        )
    })?;

    fs::create_dir_all(project_path.join("src")).map_err(|e| {
        format!(
            "{}: Failed to create src directory: {}",
            "error".red().bold(),
            e
        )
    })?;

    // Create project config
    let config = ProjectConfig::new(name);
    let config_path = project_path.join("apex.toml");
    config.save(&config_path)?;

    // Create main.apex
    let main_content = format!(
        r#"// Welcome to {}!
// This is the entry point of your application.

function main(): None {{
    println("Hello from {}!");
    return None;
}}
"#,
        name, name
    );

    let main_path = project_path.join("src").join("main.apex");
    fs::write(&main_path, main_content).map_err(|e| {
        format!(
            "{}: Failed to create main.apex: {}",
            "error".red().bold(),
            e
        )
    })?;

    // Create README.md
    let readme_content = format!(
        r#"# {}

Apex project created with `apex new`.

## Project Structure

```
.
├── apex.toml       # Project configuration
├── src/
│   └── main.apex   # Entry point
└── README.md       # This file
```

## Commands

- `apex build` - Build the project
- `apex run` - Build and run the project
- `apex info` - Show project information

## Configuration

Edit `apex.toml` to customize your project:

```toml
name = "{}"
version = "0.1.0"
entry = "src/main.apex"
files = ["src/main.apex"]
output = "{}"
opt_level = "3"
```
"#,
        name, name, name
    );

    let readme_path = project_path.join("README.md");
    fs::write(&readme_path, readme_content).map_err(|e| {
        format!(
            "{}: Failed to create README.md: {}",
            "error".red().bold(),
            e
        )
    })?;

    println!(
        "{} {} project '{}'",
        "Created".green().bold(),
        "Apex".cyan(),
        name
    );
    println!(
        "  {} {}",
        "Location:".dimmed(),
        project_path
            .canonicalize()
            .unwrap_or(project_path)
            .display()
    );
    println!("\nTo get started:");
    println!("  cd {}", name);
    println!("  apex run");

    Ok(())
}

/// Build the current project with proper namespace checking
fn build_project(_release: bool, emit_llvm: bool, do_check: bool) -> Result<(), String> {
    let project_root = find_project_root(&std::env::current_dir().unwrap())
        .ok_or_else(|| format!("{}: No apex.toml found. Are you in a project directory?\nRun `apex new <name>` to create a new project.",
            "error".red().bold()))?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;

    // Validate project
    config.validate(&project_root)?;

    println!(
        "{} {} v{}",
        "Building".green().bold(),
        config.name.cyan(),
        config.version.dimmed()
    );

    // Phase 1: Parse all files and extract namespace information
    let files = config.get_source_files(&project_root);
    let mut parsed_files: Vec<(PathBuf, String, Program, Vec<ImportDecl>)> = Vec::new();
    let mut global_function_map: HashMap<String, String> = HashMap::new(); // func_name -> namespace

    for file in &files {
        let source = fs::read_to_string(file).map_err(|e| {
            format!(
                "{}: Failed to read '{}': {}",
                "error".red().bold(),
                file.display(),
                e
            )
        })?;

        let filename = file.file_name().unwrap().to_str().unwrap_or("unknown.apex");

        // Parse the file
        let tokens = lexer::tokenize(&source).map_err(|e| {
            format!(
                "{}: Lexer error in {}: {}",
                "error".red().bold(),
                filename,
                e
            )
        })?;

        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().map_err(|e| {
            format!(
                "{}: Parse error in {}: {}",
                "error".red().bold(),
                filename,
                e.message
            )
        })?;

        // Extract namespace from package declaration
        let namespace = program
            .package
            .clone()
            .unwrap_or_else(|| "global".to_string());

        // Extract imports
        let imports: Vec<ImportDecl> = program
            .declarations
            .iter()
            .filter_map(|d| match &d.node {
                Decl::Import(import) => Some(import.clone()),
                _ => None,
            })
            .collect();

        // Extract function definitions for global map
        for decl in &program.declarations {
            if let Decl::Function(func) = &decl.node {
                global_function_map.insert(func.name.clone(), namespace.clone());
            }
        }

        parsed_files.push((file.clone(), namespace, program, imports));
    }

    // Phase 2: Check imports for each file
    if do_check {
        println!("{} Checking imports...", "→".cyan());

        for (file, namespace, program, imports) in &parsed_files {
            let mut checker = ImportChecker::new(
                global_function_map.clone(),
                namespace.clone(),
                imports.clone(),
            );

            if let Err(errors) = checker.check_program(program) {
                let filename = file.file_name().unwrap().to_str().unwrap_or("unknown");
                eprintln!("{} Import errors in {}:", "error".red().bold(), filename);
                for err in errors {
                    eprintln!("  → {}", err.format());
                }
                return Err("Import check failed".to_string());
            }
        }
    }

    // Phase 3: Merge source files (keeping original names for now)
    // TODO: Implement name mangling in codegen for proper namespace separation
    let entry_path = config.get_entry_path(&project_root);
    let mut combined_source = String::new();
    let mut is_first_file = true;

    for file in &files {
        let source = fs::read_to_string(file).map_err(|e| {
            format!(
                "{}: Failed to read '{}': {}",
                "error".red().bold(),
                file.display(),
                e
            )
        })?;

        // Add file marker for error messages
        combined_source.push_str(&format!("// FILE: {}\n", file.display()));

        // Process source: remove package declarations from non-entry files
        for line in source.lines() {
            let trimmed = line.trim();

            // Skip package declarations from non-entry files
            if trimmed.starts_with("package ") && !is_first_file {
                continue;
            }

            // Skip import declarations (they're already checked)
            if trimmed.starts_with("import ") {
                continue;
            }

            combined_source.push_str(line);
            combined_source.push('\n');
        }

        is_first_file = false;
    }

    // Compile combined source
    let output_path = project_root.join(&config.output);
    compile_source(
        &combined_source,
        &entry_path,
        &output_path,
        emit_llvm,
        do_check,
    )?;

    println!(
        "{} {} -> {}",
        "Built".green().bold(),
        config.name.cyan(),
        output_path.display()
    );

    Ok(())
}

/// Build and run the current project
fn run_project(args: &[String], release: bool, do_check: bool) -> Result<(), String> {
    build_project(release, false, do_check)?;

    let project_root = find_project_root(&std::env::current_dir().unwrap())
        .ok_or_else(|| format!("{}: No apex.toml found", "error".red().bold()))?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;
    let output_path = project_root.join(&config.output);

    println!("{} {}", "Running".cyan().bold(), config.name);
    println!();

    let status = Command::new(&output_path)
        .args(args)
        .status()
        .map_err(|e| format!("{}: Failed to run: {}", "error".red().bold(), e))?;

    if !status.success() {
        return Err(format!(
            "\n{}: Program exited with code: {}",
            "error".red().bold(),
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Run a single file (legacy mode)
fn run_single_file(
    file: &Path,
    args: &[String],
    _release: bool,
    do_check: bool,
) -> Result<(), String> {
    #[cfg(windows)]
    let output = file.with_extension("run.exe");
    #[cfg(not(windows))]
    let output = file.with_extension("run");

    compile_file(file, Some(&output), false, do_check)?;

    println!("{}", "Running...".cyan().bold());
    println!();

    let status = Command::new(&output)
        .args(args)
        .status()
        .map_err(|e| format!("{}: Failed to run: {}", "error".red().bold(), e))?;

    let _ = fs::remove_file(&output);

    if !status.success() {
        return Err(format!(
            "{}: Program exited with code: {}",
            "error".red().bold(),
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Compile a single file (legacy mode)
fn compile_file(
    file: &Path,
    output: Option<&Path>,
    emit_llvm: bool,
    do_check: bool,
) -> Result<(), String> {
    // Check if we're in a project
    if let Some(project_root) = find_project_root(&std::env::current_dir().unwrap()) {
        if file.starts_with(&project_root) {
            println!(
                "{}",
                "Note: Running in project context. Consider using `apex build` instead.".yellow()
            );
        }
    }

    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let output_path = output.map(PathBuf::from).unwrap_or_else(|| {
        #[cfg(windows)]
        {
            file.with_extension("exe")
        }
        #[cfg(not(windows))]
        {
            file.with_extension("")
        }
    });

    compile_source(&source, file, &output_path, emit_llvm, do_check)?;

    println!("{} {}", "Output".green().bold(), output_path.display());
    Ok(())
}

/// Compile source code
fn compile_source(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    emit_llvm: bool,
    do_check: bool,
) -> Result<(), String> {
    let filename = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("input.apex");

    // Tokenize
    let tokens = lexer::tokenize(source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    // Parse
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, source, filename))?;

    // Type check
    if do_check {
        // Import check
        let namespace = extract_namespace(&program);
        let imports = extract_imports(&program);
        let mut import_checker = ImportChecker::new(HashMap::new(), namespace, imports);
        if let Err(errors) = import_checker.check_program(&program) {
            eprintln!("{} Import errors:", "error".red().bold());
            for err in errors {
                eprintln!("  → {}", err.format());
            }
            return Err("Import check failed".to_string());
        }

        let mut type_checker = TypeChecker::new(source.to_string());
        if let Err(errors) = type_checker.check(&program) {
            return Err(typeck::format_errors(&errors, source, filename));
        }

        let mut borrow_checker = BorrowChecker::new();
        if let Err(errors) = borrow_checker.check(&program) {
            return Err(borrowck::format_borrow_errors(&errors, source, filename));
        }
    }

    // Codegen
    let context = Context::create();
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let mut codegen = Codegen::new(&context, module_name);
    codegen
        .compile(&program)
        .map_err(|e| format!("{}: Codegen error: {}", "error".red().bold(), e.message))?;

    if emit_llvm {
        let ll_path = output_path.with_extension("ll");
        codegen.write_ir(&ll_path)?;
        println!("{} {}", "LLVM IR".green().bold(), ll_path.display());
    } else {
        let ir_path = output_path.with_extension("ll");
        codegen.write_ir(&ir_path)?;

        compile_ir(&ir_path, output_path)?;
        let _ = fs::remove_file(&ir_path);
    }

    Ok(())
}

/// Compile LLVM IR using clang
fn compile_ir(ir_path: &Path, output_path: &Path) -> Result<(), String> {
    let mut cmd = Command::new("clang");
    cmd.arg(ir_path)
        .arg("-o")
        .arg(output_path)
        .arg("-Wno-override-module")
        .arg("-O3");

    #[cfg(windows)]
    cmd.arg("-llegacy_stdio_definitions");

    #[cfg(not(windows))]
    cmd.arg("-lm");

    let result = cmd.output();

    match result {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!(
                    "{}: Clang failed: {}",
                    "error".red().bold(),
                    stderr
                ))
            }
        }
        Err(_) => Err(format!(
            "{}: Clang not found. Install clang to compile.",
            "error".red().bold()
        )),
    }
}

/// Check a single file
fn check_file(file: Option<&Path>) -> Result<(), String> {
    let file_path = if let Some(f) = file {
        f.to_path_buf()
    } else {
        // Use project entry point
        let project_root =
            find_project_root(&std::env::current_dir().unwrap()).ok_or_else(|| {
                format!(
                    "{}: No apex.toml found. Specify a file or run from a project directory.",
                    "error".red().bold()
                )
            })?;

        let config_path = project_root.join("apex.toml");
        let config = ProjectConfig::load(&config_path)?;
        config.get_entry_path(&project_root)
    };

    println!("{} {}", "Checking".cyan().bold(), file_path.display());

    let source = fs::read_to_string(&file_path)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let filename = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("input.apex");

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, &source, filename))?;

    // Run import checker
    let namespace = extract_namespace(&program);
    let imports = extract_imports(&program);
    let mut import_checker = ImportChecker::new(HashMap::new(), namespace, imports);
    if let Err(errors) = import_checker.check_program(&program) {
        eprintln!("{} Import errors:", "error".red().bold());
        for err in errors {
            eprintln!("  → {}", err.format());
        }
        return Err("Import check failed".to_string());
    }

    let mut type_checker = TypeChecker::new(source.clone());
    if let Err(errors) = type_checker.check(&program) {
        return Err(typeck::format_errors(&errors, &source, filename));
    }

    let mut borrow_checker = BorrowChecker::new();
    if let Err(errors) = borrow_checker.check(&program) {
        return Err(borrowck::format_borrow_errors(&errors, &source, filename));
    }

    println!("{}", "No errors found.".green());
    Ok(())
}

/// Extract namespace from a program
fn extract_namespace(program: &ast::Program) -> String {
    program
        .package
        .clone()
        .unwrap_or_else(|| "global".to_string())
}

/// Extract imports from a program
fn extract_imports(program: &ast::Program) -> Vec<ast::ImportDecl> {
    program
        .declarations
        .iter()
        .filter_map(|d| match &d.node {
            ast::Decl::Import(import) => Some(import.clone()),
            _ => None,
        })
        .collect()
}

/// Show project information
fn show_project_info() -> Result<(), String> {
    let project_root = find_project_root(&std::env::current_dir().unwrap()).ok_or_else(|| {
        format!(
            "{}: No apex.toml found in current directory or parents.",
            "error".red().bold()
        )
    })?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;

    println!("{}", "Project Information".cyan().bold());
    println!("  {}: {}", "Name".dimmed(), config.name);
    println!("  {}: {}", "Version".dimmed(), config.version);
    println!("  {}: {}", "Entry".dimmed(), config.entry);
    println!("  {}: {}", "Output".dimmed(), config.output);
    println!("  {}: {}", "Opt Level".dimmed(), config.opt_level);
    println!("  {}: {}", "Root".dimmed(), project_root.display());

    println!("\n{}", "Source Files:".dimmed());
    for file in &config.files {
        println!("  - {}", file);
    }

    if !config.dependencies.is_empty() {
        println!("\n{}", "Dependencies:".dimmed());
        for (name, version) in &config.dependencies {
            println!("  - {} = {}", name, version);
        }
    }

    Ok(())
}

/// Show tokens (debug)
fn lex_file(file: &Path) -> Result<(), String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    println!("{} tokens:", "Found".cyan().bold());
    for (token, span) in tokens {
        println!("  {:?} @ {}..{}", token, span.start, span.end);
    }

    Ok(())
}

/// Show AST (debug)
fn parse_file(file: &Path) -> Result<(), String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format!("{}: Parse error: {}", "error".red().bold(), e.message))?;

    println!("{}", "AST:".cyan().bold());
    println!("{:#?}", program);

    Ok(())
}

/// Format parse error with source context
fn format_parse_error(error: &parser::ParseError, source: &str, filename: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();

    let mut line_num: usize = 1;
    let mut col: usize = 1;
    for (i, ch) in source.char_indices() {
        if i >= error.span.start {
            break;
        }
        if ch == '\n' {
            line_num += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    let mut output = String::new();
    output.push_str(&format!("\x1b[1;31merror\x1b[0m: {}\n", error.message));
    output.push_str(&format!(
        "  \x1b[1;34m-->\x1b[0m {}:{}:{}\n",
        filename, line_num, col
    ));
    output.push_str("   \x1b[1;34m|\x1b[0m\n");

    if line_num <= lines.len() {
        output.push_str(&format!(
            "\x1b[1;34m{:3} |\x1b[0m {}\n",
            line_num,
            lines[line_num - 1]
        ));

        let underline_start = col.saturating_sub(1);
        let underline_len = (error.span.end - error.span.start).max(1);
        output.push_str(&format!(
            "   \x1b[1;34m|\x1b[0m {}\x1b[1;31m{}\x1b[0m\n",
            " ".repeat(underline_start),
            "^".repeat(underline_len.min(50))
        ));
    }

    output
}
