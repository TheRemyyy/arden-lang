//! Apex Programming Language Compiler

mod ast;
mod bindgen;
mod borrowck;
mod cache;
mod codegen;
mod dependency;
mod diagnostics;
mod formatter;
mod import_check;
mod lexer;
mod linker;
mod lint;
mod lsp;
mod parser;
mod project;
mod project_rewrite;
mod specialization;
mod stdlib;
mod symbol_lookup;
mod test_runner;
#[cfg(test)]
mod tests;
mod typeck;

use clap::{Parser as ClapParser, Subcommand};
use colored::*;
use inkwell::context::Context;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use std::time::UNIX_EPOCH;

use crate::ast::{Block, Decl, Expr, ImportDecl, Pattern, Program, Stmt, Type};
use crate::borrowck::BorrowChecker;
use crate::cache::*;
use crate::codegen::Codegen;
use crate::dependency::*;
use crate::diagnostics::*;
use crate::import_check::ImportChecker;
use crate::linker::*;
use crate::parser::{parse_type_source, Parser};
use crate::project::{find_project_root, OutputKind, ProjectConfig};
use crate::specialization::*;
use crate::stdlib::stdlib_registry;
use crate::symbol_lookup::*;
use crate::test_runner::{discover_tests, generate_test_runner_with_source, print_discovery};
use crate::typeck::{ClassMethodEffectsSummary, FunctionEffectsSummary, TypeChecker};

#[derive(Clone)]
struct ObjectCodegenShard {
    member_indices: Vec<usize>,
    member_files: Vec<PathBuf>,
    member_fingerprints: Vec<ObjectShardMemberFingerprint>,
    cache_paths: Option<ObjectShardCachePaths>,
}

#[derive(ClapParser)]
#[command(name = "apex")]
#[command(bin_name = "apex")]
#[command(author = "TheRemyyy")]
#[command(version = "1.3.5")]
#[command(about = "Apex compiler and project CLI")]
#[command(long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a project skeleton
    New {
        /// Project name
        name: String,
        /// Project directory (defaults to ./<name>)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Build the current project
    Build {
        /// Enable optimized code generation
        #[arg(short, long)]
        release: bool,
        /// Write LLVM IR instead of a final artifact
        #[arg(long)]
        emit_llvm: bool,
        /// Skip type and borrow checking
        #[arg(long)]
        no_check: bool,
        /// Print internal project build phase timings
        #[arg(long)]
        timings: bool,
    },
    /// Build and run a project or single file
    Run {
        /// Input file (defaults to the current project)
        file: Option<PathBuf>,
        /// Arguments passed through to the compiled program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Enable optimized code generation
        #[arg(short, long)]
        release: bool,
        /// Skip type and borrow checking
        #[arg(long)]
        no_check: bool,
        /// Print internal project build phase timings
        #[arg(long)]
        timings: bool,
    },
    /// Compile a single Apex file
    Compile {
        /// Input file
        file: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Optimization level: 0, 1, 2, 3, s, z, or fast
        #[arg(long)]
        opt_level: Option<String>,
        /// Target triple passed through to clang
        #[arg(long)]
        target: Option<String>,
        /// Write LLVM IR instead of a final artifact
        #[arg(long)]
        emit_llvm: bool,
        /// Skip type and borrow checking
        #[arg(long)]
        no_check: bool,
    },
    /// Parse, type-check, and borrow-check source
    Check {
        /// Input file (defaults to the project entry point)
        file: Option<PathBuf>,
        /// Print internal project build phase timings in project mode
        #[arg(long)]
        timings: bool,
    },
    /// Print project configuration and build settings
    Info,
    /// Report static findings
    Lint {
        /// File to lint (defaults to the project entry point)
        path: Option<PathBuf>,
    },
    /// Apply safe fixes and reformat the result
    Fix {
        /// File to fix (defaults to the project entry point)
        path: Option<PathBuf>,
    },
    /// Format Apex source
    Fmt {
        /// File or directory to format
        path: Option<PathBuf>,
        /// Check formatting without writing changes
        #[arg(long)]
        check: bool,
    },
    /// Print lexer tokens
    Lex {
        /// Input file
        file: PathBuf,
    },
    /// Print the parsed AST
    Parse {
        /// Input file
        file: PathBuf,
    },
    /// Start the language server
    Lsp,
    /// Discover and run @Test suites
    Test {
        /// Input file or directory (defaults to project test files)
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// List tests without running them
        #[arg(short, long)]
        list: bool,
        /// Keep only tests whose names contain the pattern
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Generate Apex extern bindings from a C header
    Bindgen {
        /// Input C header file
        header: PathBuf,
        /// Output Apex file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Measure end-to-end execution time
    Bench {
        /// Input file (defaults to the current project)
        file: Option<PathBuf>,
        /// Number of measured runs
        #[arg(short, long, default_value_t = 5)]
        iterations: usize,
    },
    /// Run once and print a timing summary
    Profile {
        /// Input file (defaults to the current project)
        file: Option<PathBuf>,
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
            timings,
        } => build_project(release, emit_llvm, !no_check, false, timings),
        Commands::Run {
            file,
            args,
            release,
            no_check,
            timings,
        } => {
            if let Some(f) = file {
                run_single_file(&f, &args, release, !no_check)
            } else {
                run_project(&args, release, !no_check, timings)
            }
        }
        Commands::Compile {
            file,
            output,
            opt_level,
            target,
            emit_llvm,
            no_check,
        } => compile_file(
            &file,
            output.as_deref(),
            emit_llvm,
            !no_check,
            opt_level.as_deref(),
            target.as_deref(),
        ),
        Commands::Check { file, timings } => check_command(file.as_deref(), timings),
        Commands::Info => show_project_info(),
        Commands::Lint { path } => lint_target(path.as_deref()),
        Commands::Fix { path } => fix_target(path.as_deref()),
        Commands::Fmt { path, check } => format_targets(path.as_deref(), check),
        Commands::Lex { file } => lex_file(&file),
        Commands::Parse { file } => parse_file(&file),
        Commands::Lsp => {
            let runtime = tokio::runtime::Runtime::new()
                .map_err(|e| format!("{}: Failed to start runtime: {}", "error".red().bold(), e));
            match runtime {
                Ok(rt) => {
                    rt.block_on(lsp::run_lsp_server());
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        Commands::Test { path, list, filter } => {
            run_tests(path.as_deref(), list, filter.as_deref())
        }
        Commands::Bindgen { header, output } => bindgen_header(&header, output.as_deref()),
        Commands::Bench { file, iterations } => bench_target(file.as_deref(), iterations),
        Commands::Profile { file } => profile_target(file.as_deref()),
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    std::process::exit(0);
}

fn current_dir_checked() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| {
        format!(
            "{}: Failed to read current directory: {}",
            "error".red().bold(),
            e
        )
    })
}

/// Parsed data extracted from a source file, used internally by [`parse_project_unit`].
struct SourceParseResult {
    namespace: String,
    program: crate::ast::Program,
    imports: Vec<ImportDecl>,
    api_fingerprint: String,
    semantic_fingerprint: String,
    /// `Some(fp)` when the file was freshly parsed; `None` when loaded from cache.
    source_fingerprint_for_cache: Option<String>,
    from_parse_cache: bool,
}

/// Parse source text and extract the constituent parts of a [`ParsedProjectUnit`].
fn parse_source_text(
    source: &str,
    source_fp: String,
    filename: &str,
) -> Result<SourceParseResult, String> {
    let tokens = lexer::tokenize(source).map_err(|e| {
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
    let namespace = program
        .package
        .clone()
        .unwrap_or_else(|| "global".to_string());
    let imports: Vec<ImportDecl> = program
        .declarations
        .iter()
        .filter_map(|d| match &d.node {
            Decl::Import(import) => Some(import.clone()),
            _ => None,
        })
        .collect();
    let api_fingerprint = api_program_fingerprint(&program);
    let semantic_fingerprint = semantic_program_fingerprint(&program);
    Ok(SourceParseResult {
        namespace,
        program,
        imports,
        api_fingerprint,
        semantic_fingerprint,
        source_fingerprint_for_cache: Some(source_fp),
        from_parse_cache: false,
    })
}

fn parse_project_unit(project_root: &Path, file: &Path) -> Result<ParsedProjectUnit, String> {
    let filename = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown.apex");
    let file_metadata = current_file_metadata_stamp(file)?;
    let cached_entry = load_parsed_file_cache_entry(project_root, file)?;
    let read_source = |f: &Path| {
        fs::read_to_string(f).map_err(|e| {
            format!(
                "{}: Failed to read '{}': {}",
                "error".red().bold(),
                f.display(),
                e
            )
        })
    };
    let SourceParseResult {
        namespace,
        program,
        imports,
        api_fingerprint,
        semantic_fingerprint,
        source_fingerprint_for_cache,
        from_parse_cache,
    } = if let Some(cache) = cached_entry.as_ref() {
        let source = read_source(file)?;
        let source_fp = source_fingerprint(&source);
        if cache.source_fingerprint == source_fp {
            SourceParseResult {
                namespace: cache.namespace.clone(),
                program: cache.program.clone(),
                imports: cache.imports.clone(),
                api_fingerprint: cache.api_fingerprint.clone(),
                semantic_fingerprint: cache.semantic_fingerprint.clone(),
                source_fingerprint_for_cache: None,
                from_parse_cache: true,
            }
        } else {
            parse_source_text(&source, source_fp, filename)?
        }
    } else {
        let source = read_source(file)?;
        let source_fp = source_fingerprint(&source);
        parse_source_text(&source, source_fp, filename)?
    };

    let mut function_names = Vec::new();
    let mut class_names = Vec::new();
    let mut interface_names = Vec::new();
    let mut enum_names = Vec::new();
    let mut module_names = Vec::new();
    let mut referenced_symbols = HashSet::new();
    let mut qualified_symbol_refs: HashSet<Vec<String>> = HashSet::new();

    fn nested_decl_name(module_prefix: &Option<String>, name: &str) -> String {
        module_prefix
            .as_ref()
            .map(|module_name| format!("{module_name}__{name}"))
            .unwrap_or_else(|| name.to_string())
    }

    fn next_module_prefix(module_prefix: &Option<String>, module_name: &str) -> String {
        module_prefix
            .as_ref()
            .map(|prefix| format!("{prefix}__{module_name}"))
            .unwrap_or_else(|| module_name.to_string())
    }

    fn collect_decl_names(
        decl: &Decl,
        module_prefix: Option<String>,
        out: &mut Vec<String>,
        include_modules: bool,
        select_name: fn(&Decl) -> Option<&str>,
    ) {
        if let Some(name) = select_name(decl) {
            out.push(nested_decl_name(&module_prefix, name));
        }

        if let Decl::Module(module) = decl {
            let full_name = next_module_prefix(&module_prefix, &module.name);
            if include_modules {
                out.push(full_name.clone());
            }
            for inner in &module.declarations {
                collect_decl_names(
                    &inner.node,
                    Some(full_name.clone()),
                    out,
                    include_modules,
                    select_name,
                );
            }
        }
    }

    fn function_decl_name(decl: &Decl) -> Option<&str> {
        match decl {
            Decl::Function(func) => Some(&func.name),
            _ => None,
        }
    }

    fn class_decl_name(decl: &Decl) -> Option<&str> {
        match decl {
            Decl::Class(class) => Some(&class.name),
            _ => None,
        }
    }

    fn interface_decl_name(decl: &Decl) -> Option<&str> {
        match decl {
            Decl::Interface(interface) => Some(&interface.name),
            _ => None,
        }
    }

    fn enum_decl_name(decl: &Decl) -> Option<&str> {
        match decl {
            Decl::Enum(en) => Some(&en.name),
            _ => None,
        }
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

    fn collect_qualified_name_ref(
        name: &str,
        out: &mut HashSet<String>,
        qualified_out: &mut HashSet<Vec<String>>,
    ) {
        if let Some((root, _)) = name.split_once('.') {
            out.insert(root.to_string());
            qualified_out.insert(name.split('.').map(|part| part.to_string()).collect());
        } else {
            out.insert(name.to_string());
        }
    }

    fn collect_type_refs(
        ty: &ast::Type,
        out: &mut HashSet<String>,
        qualified_out: &mut HashSet<Vec<String>>,
    ) {
        match ty {
            ast::Type::Named(name) => {
                collect_qualified_name_ref(name, out, qualified_out);
            }
            ast::Type::Generic(name, args) => {
                collect_qualified_name_ref(name, out, qualified_out);
                for arg in args {
                    collect_type_refs(arg, out, qualified_out);
                }
            }
            ast::Type::Function(params, ret) => {
                for param in params {
                    collect_type_refs(param, out, qualified_out);
                }
                collect_type_refs(ret, out, qualified_out);
            }
            ast::Type::Option(inner) => {
                out.insert("Option".to_string());
                collect_type_refs(inner, out, qualified_out);
            }
            ast::Type::List(inner) => {
                out.insert("List".to_string());
                collect_type_refs(inner, out, qualified_out);
            }
            ast::Type::Set(inner) => {
                out.insert("Set".to_string());
                collect_type_refs(inner, out, qualified_out);
            }
            ast::Type::Box(inner) => {
                out.insert("Box".to_string());
                collect_type_refs(inner, out, qualified_out);
            }
            ast::Type::Rc(inner) => {
                out.insert("Rc".to_string());
                collect_type_refs(inner, out, qualified_out);
            }
            ast::Type::Arc(inner) => {
                out.insert("Arc".to_string());
                collect_type_refs(inner, out, qualified_out);
            }
            ast::Type::Ptr(inner) => {
                out.insert("Ptr".to_string());
                collect_type_refs(inner, out, qualified_out);
            }
            ast::Type::Task(inner) => {
                out.insert("Task".to_string());
                collect_type_refs(inner, out, qualified_out);
            }
            ast::Type::Range(inner) => {
                out.insert("Range".to_string());
                collect_type_refs(inner, out, qualified_out);
            }
            ast::Type::Ref(inner) | ast::Type::MutRef(inner) => {
                collect_type_refs(inner, out, qualified_out)
            }
            ast::Type::Result(ok, err) => {
                out.insert("Result".to_string());
                collect_type_refs(ok, out, qualified_out);
                collect_type_refs(err, out, qualified_out);
            }
            ast::Type::Map(ok, err) => {
                out.insert("Map".to_string());
                collect_type_refs(ok, out, qualified_out);
                collect_type_refs(err, out, qualified_out);
            }
            ast::Type::Integer
            | ast::Type::Float
            | ast::Type::Boolean
            | ast::Type::String
            | ast::Type::Char
            | ast::Type::None => {}
        }
    }

    fn collect_generic_param_bound_refs(
        generic_params: &[ast::GenericParam],
        out: &mut HashSet<String>,
        qualified_out: &mut HashSet<Vec<String>>,
    ) {
        for param in generic_params {
            for bound in &param.bounds {
                collect_qualified_name_ref(bound, out, qualified_out);
            }
        }
    }

    fn collect_expr_refs(
        expr: &Expr,
        out: &mut HashSet<String>,
        qualified_out: &mut HashSet<Vec<String>>,
    ) {
        match expr {
            Expr::Literal(_) | Expr::This => {}
            Expr::Ident(name) => {
                out.insert(name.clone());
            }
            Expr::Binary { left, right, .. } => {
                collect_expr_refs(&left.node, out, qualified_out);
                collect_expr_refs(&right.node, out, qualified_out);
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => collect_expr_refs(&expr.node, out, qualified_out),
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                if let Expr::Ident(name) = &callee.node {
                    out.insert(name.clone());
                } else if let Some(parts) = flatten_field_chain(&callee.node) {
                    if let Some(root) = parts.first() {
                        out.insert(root.clone());
                    }
                    qualified_out.insert(parts);
                } else if let Expr::Field { object, field } = &callee.node {
                    if let Expr::Construct { ty, .. } = &object.node {
                        out.insert(ty.clone());
                        out.insert(format!("{}__{}", ty, field));
                    }
                }
                collect_expr_refs(&callee.node, out, qualified_out);
                for arg in args {
                    collect_expr_refs(&arg.node, out, qualified_out);
                }
                for ty in type_args {
                    collect_type_refs(ty, out, qualified_out);
                }
            }
            Expr::Field { object, .. } => {
                if let Some(parts) = flatten_field_chain(expr) {
                    if let Some(root) = parts.first() {
                        out.insert(root.clone());
                    }
                    qualified_out.insert(parts);
                }
                collect_expr_refs(&object.node, out, qualified_out);
            }
            Expr::Index { object, index } => {
                collect_expr_refs(&object.node, out, qualified_out);
                collect_expr_refs(&index.node, out, qualified_out);
            }
            Expr::Construct { ty, args } => {
                collect_qualified_name_ref(ty, out, qualified_out);
                for arg in args {
                    collect_expr_refs(&arg.node, out, qualified_out);
                }
            }
            Expr::GenericFunctionValue { callee, type_args } => {
                collect_expr_refs(&callee.node, out, qualified_out);
                for type_arg in type_args {
                    collect_type_refs(type_arg, out, qualified_out);
                }
            }
            Expr::Lambda { params, body } => {
                for param in params {
                    collect_type_refs(&param.ty, out, qualified_out);
                }
                collect_expr_refs(&body.node, out, qualified_out);
            }
            Expr::Match { expr, arms } => {
                collect_expr_refs(&expr.node, out, qualified_out);
                for arm in arms {
                    collect_pattern_refs(&arm.pattern, out, qualified_out);
                    collect_block_refs(&arm.body, out, qualified_out);
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let ast::StringPart::Expr(expr) = part {
                        collect_expr_refs(&expr.node, out, qualified_out);
                    }
                }
            }
            Expr::AsyncBlock(body) | Expr::Block(body) => {
                collect_block_refs(body, out, qualified_out)
            }
            Expr::Require { condition, message } => {
                collect_expr_refs(&condition.node, out, qualified_out);
                if let Some(message) = message {
                    collect_expr_refs(&message.node, out, qualified_out);
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(start) = start {
                    collect_expr_refs(&start.node, out, qualified_out);
                }
                if let Some(end) = end {
                    collect_expr_refs(&end.node, out, qualified_out);
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_expr_refs(&condition.node, out, qualified_out);
                collect_block_refs(then_branch, out, qualified_out);
                if let Some(else_branch) = else_branch {
                    collect_block_refs(else_branch, out, qualified_out);
                }
            }
        }
    }

    fn collect_pattern_refs(
        pattern: &Pattern,
        out: &mut HashSet<String>,
        qualified_out: &mut HashSet<Vec<String>>,
    ) {
        if let Pattern::Variant(name, _) = pattern {
            collect_qualified_name_ref(name, out, qualified_out);
        }
    }

    fn collect_stmt_refs(
        stmt: &Stmt,
        out: &mut HashSet<String>,
        qualified_out: &mut HashSet<Vec<String>>,
    ) {
        match stmt {
            Stmt::Let { ty, value, .. } => {
                collect_type_refs(ty, out, qualified_out);
                collect_expr_refs(&value.node, out, qualified_out);
            }
            Stmt::Assign { target, value } => {
                collect_expr_refs(&target.node, out, qualified_out);
                collect_expr_refs(&value.node, out, qualified_out);
            }
            Stmt::Expr(expr) => collect_expr_refs(&expr.node, out, qualified_out),
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    collect_expr_refs(&expr.node, out, qualified_out);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_expr_refs(&condition.node, out, qualified_out);
                collect_block_refs(then_block, out, qualified_out);
                if let Some(else_block) = else_block {
                    collect_block_refs(else_block, out, qualified_out);
                }
            }
            Stmt::While { condition, body } => {
                collect_expr_refs(&condition.node, out, qualified_out);
                collect_block_refs(body, out, qualified_out);
            }
            Stmt::For {
                var_type,
                iterable,
                body,
                ..
            } => {
                if let Some(var_type) = var_type {
                    collect_type_refs(var_type, out, qualified_out);
                }
                collect_expr_refs(&iterable.node, out, qualified_out);
                collect_block_refs(body, out, qualified_out);
            }
            Stmt::Match { expr, arms } => {
                collect_expr_refs(&expr.node, out, qualified_out);
                for arm in arms {
                    collect_pattern_refs(&arm.pattern, out, qualified_out);
                    collect_block_refs(&arm.body, out, qualified_out);
                }
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn collect_block_refs(
        block: &Block,
        out: &mut HashSet<String>,
        qualified_out: &mut HashSet<Vec<String>>,
    ) {
        for stmt in block {
            collect_stmt_refs(&stmt.node, out, qualified_out);
        }
    }

    fn collect_decl_refs(
        decl: &Decl,
        out: &mut HashSet<String>,
        qualified_out: &mut HashSet<Vec<String>>,
    ) {
        match decl {
            Decl::Function(func) => {
                collect_generic_param_bound_refs(&func.generic_params, out, qualified_out);
                for param in &func.params {
                    collect_type_refs(&param.ty, out, qualified_out);
                }
                collect_type_refs(&func.return_type, out, qualified_out);
                collect_block_refs(&func.body, out, qualified_out);
            }
            Decl::Class(class) => {
                collect_generic_param_bound_refs(&class.generic_params, out, qualified_out);
                if let Some(parent) = &class.extends {
                    collect_qualified_name_ref(parent, out, qualified_out);
                }
                for implemented in &class.implements {
                    collect_qualified_name_ref(implemented, out, qualified_out);
                }
                for field in &class.fields {
                    collect_type_refs(&field.ty, out, qualified_out);
                }
                if let Some(ctor) = &class.constructor {
                    for param in &ctor.params {
                        collect_type_refs(&param.ty, out, qualified_out);
                    }
                    collect_block_refs(&ctor.body, out, qualified_out);
                }
                if let Some(dtor) = &class.destructor {
                    collect_block_refs(&dtor.body, out, qualified_out);
                }
                for method in &class.methods {
                    collect_generic_param_bound_refs(&method.generic_params, out, qualified_out);
                    for param in &method.params {
                        collect_type_refs(&param.ty, out, qualified_out);
                    }
                    collect_type_refs(&method.return_type, out, qualified_out);
                    collect_block_refs(&method.body, out, qualified_out);
                }
            }
            Decl::Enum(en) => {
                collect_generic_param_bound_refs(&en.generic_params, out, qualified_out);
                for variant in &en.variants {
                    for field in &variant.fields {
                        collect_type_refs(&field.ty, out, qualified_out);
                    }
                }
            }
            Decl::Interface(interface) => {
                collect_generic_param_bound_refs(&interface.generic_params, out, qualified_out);
                for extended in &interface.extends {
                    collect_qualified_name_ref(extended, out, qualified_out);
                }
                for method in &interface.methods {
                    for param in &method.params {
                        collect_type_refs(&param.ty, out, qualified_out);
                    }
                    collect_type_refs(&method.return_type, out, qualified_out);
                    if let Some(body) = &method.default_impl {
                        collect_block_refs(body, out, qualified_out);
                    }
                }
            }
            Decl::Module(module) => {
                out.insert(module.name.clone());
                for inner in &module.declarations {
                    collect_decl_refs(&inner.node, out, qualified_out);
                }
            }
            Decl::Import(_) => {}
        }
    }

    let (
        function_names,
        class_names,
        interface_names,
        enum_names,
        module_names,
        referenced_symbols,
        qualified_symbol_refs,
        api_referenced_symbols,
        import_check_fingerprint,
    ) = if from_parse_cache {
        let cache = cached_entry
            .as_ref()
            .expect("parse cache hit should have cache entry available");
        (
            cache.function_names.clone(),
            cache.class_names.clone(),
            cache.interface_names.clone(),
            cache.enum_names.clone(),
            cache.module_names.clone(),
            cache.referenced_symbols.clone(),
            cache.qualified_symbol_refs.clone(),
            cache.api_referenced_symbols.clone(),
            cache.import_check_fingerprint.clone(),
        )
    } else {
        for decl in &program.declarations {
            match &decl.node {
                Decl::Function(_) => collect_decl_names(
                    &decl.node,
                    None,
                    &mut function_names,
                    false,
                    function_decl_name,
                ),
                Decl::Module(_) => {
                    collect_decl_names(&decl.node, None, &mut module_names, true, |_| None);
                    collect_decl_names(
                        &decl.node,
                        None,
                        &mut function_names,
                        false,
                        function_decl_name,
                    );
                    collect_decl_names(&decl.node, None, &mut class_names, false, class_decl_name);
                    collect_decl_names(
                        &decl.node,
                        None,
                        &mut interface_names,
                        false,
                        interface_decl_name,
                    );
                    collect_decl_names(&decl.node, None, &mut enum_names, false, enum_decl_name);
                }
                Decl::Class(class) => class_names.push(class.name.clone()),
                Decl::Interface(interface) => interface_names.push(interface.name.clone()),
                Decl::Enum(en) => enum_names.push(en.name.clone()),
                _ => {}
            }
            collect_decl_refs(
                &decl.node,
                &mut referenced_symbols,
                &mut qualified_symbol_refs,
            );
        }
        let projected_program = api_projection_program(&program);
        let mut api_referenced_symbols = HashSet::new();
        let mut ignored_api_qualified_symbol_refs = HashSet::new();
        for decl in &projected_program.declarations {
            collect_decl_refs(
                &decl.node,
                &mut api_referenced_symbols,
                &mut ignored_api_qualified_symbol_refs,
            );
        }

        let mut referenced_symbols = referenced_symbols.into_iter().collect::<Vec<_>>();
        referenced_symbols.sort();
        let mut qualified_symbol_refs = qualified_symbol_refs.into_iter().collect::<Vec<_>>();
        qualified_symbol_refs.sort();
        let mut api_referenced_symbols = api_referenced_symbols.into_iter().collect::<Vec<_>>();
        api_referenced_symbols.sort();
        module_names.sort();
        interface_names.sort();
        enum_names.sort();
        let import_check_fingerprint = compute_import_check_fingerprint(
            &namespace,
            &imports,
            &referenced_symbols,
            &qualified_symbol_refs,
        );

        let cache_entry = ParsedFileCacheEntry {
            schema: PARSE_CACHE_SCHEMA.to_string(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            file_metadata,
            source_fingerprint: source_fingerprint_for_cache
                .expect("fresh parse should have source fingerprint"),
            api_fingerprint: api_fingerprint.clone(),
            semantic_fingerprint: semantic_fingerprint.clone(),
            import_check_fingerprint: import_check_fingerprint.clone(),
            namespace: namespace.clone(),
            program: program.clone(),
            imports: imports.clone(),
            function_names: function_names.clone(),
            class_names: class_names.clone(),
            interface_names: interface_names.clone(),
            enum_names: enum_names.clone(),
            module_names: module_names.clone(),
            referenced_symbols: referenced_symbols.clone(),
            qualified_symbol_refs: qualified_symbol_refs.clone(),
            api_referenced_symbols: api_referenced_symbols.clone(),
        };
        save_parsed_file_cache(project_root, file, &cache_entry)?;

        (
            function_names,
            class_names,
            interface_names,
            enum_names,
            module_names,
            referenced_symbols,
            qualified_symbol_refs,
            api_referenced_symbols,
            import_check_fingerprint,
        )
    };

    Ok(ParsedProjectUnit {
        file: file.to_path_buf(),
        namespace,
        program,
        imports,
        api_fingerprint,
        semantic_fingerprint,
        import_check_fingerprint,
        function_names,
        class_names,
        interface_names,
        enum_names,
        module_names,
        referenced_symbols,
        qualified_symbol_refs,
        api_referenced_symbols,
        from_parse_cache,
    })
}

/// Create a new project
fn new_project(name: &str, path: Option<&Path>) -> Result<(), String> {
    validate_new_project_name(name)?;

    let project_path = path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(name));

    if project_path.exists() {
        let kind = if project_path.is_dir() {
            "Directory"
        } else {
            "Path"
        };
        return Err(format!(
            "{}: {} '{}' already exists",
            "error".red().bold(),
            kind,
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
        r#"import std.io.*;

function main(): None {{
    println("hello from {}");
    return None;
}}
"#,
        name
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

## Common Commands

- `apex build` - Build the project
- `apex run` - Build and run the project
- `apex check` - Parse and type-check the project
- `apex test` - Run test files in the project

## Configuration

Edit `apex.toml` to customize your project:

```toml
name = "{}"
version = "0.1.0"
entry = "src/main.apex"
files = ["src/main.apex"]
output = "{}"
opt_level = "3"
output_kind = "bin"
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

    println!("{} {}", "Created project".green().bold(), name.cyan());
    println!(
        "  {} {}",
        "Root".dimmed(),
        project_path
            .canonicalize()
            .unwrap_or(project_path)
            .display()
    );
    println!("\n{}", "Next".dimmed());
    println!("  cd {}", path.unwrap_or(Path::new(name)).display());
    println!("  apex run");

    Ok(())
}

fn validate_new_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(format!(
            "{}: Project name cannot be empty",
            "error".red().bold()
        ));
    }

    let is_valid = name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');
    if is_valid {
        return Ok(());
    }

    Err(format!(
        "{}: Invalid project name '{}'. Use only ASCII letters, digits, '_' or '-'.",
        "error".red().bold(),
        name
    ))
}

/// Build the current project with proper namespace checking
fn build_project(
    _release: bool,
    emit_llvm: bool,
    do_check: bool,
    check_only: bool,
    show_timings: bool,
) -> Result<(), String> {
    let mut build_timings = BuildTimings::new(show_timings);
    reset_cache_io_timing_totals(&PARSE_CACHE_TIMING_TOTALS);
    reset_cache_io_timing_totals(&REWRITE_CACHE_TIMING_TOTALS);
    reset_cache_io_timing_totals(&OBJECT_CACHE_META_TIMING_TOTALS);
    let cwd = current_dir_checked()?;
    let project_root = find_project_root(&cwd)
        .ok_or_else(|| format!("{}: No apex.toml found. Are you in a project directory?\nRun `apex new <name>` to create a new project.",
            "error".red().bold()))?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;

    build_timings.measure("project config validation", || {
        config.validate(&project_root)
    })?;
    validate_opt_level(Some(&config.opt_level))?;
    let files = build_timings.measure_step("source file discovery", || {
        let mut files = config.get_source_files(&project_root);
        files.sort();
        files
    });
    build_timings.record_counts("source file discovery", &[("files", files.len())]);

    let output_path = project_root.join(&config.output);
    if !check_only {
        ensure_output_parent_dir(&output_path)?;
    }
    let fingerprint = build_timings.measure("project fingerprint", || {
        compute_project_fingerprint(&files, &config, emit_llvm, do_check)
    })?;
    if !check_only {
        if let Some(cached) = build_timings.measure("build cache lookup", || {
            load_cached_fingerprint(&project_root)
        })? {
            if cached == fingerprint && project_build_artifact_exists(&output_path, emit_llvm) {
                println!(
                    "{} {}",
                    "Build cache hit".green().bold(),
                    config.name.cyan(),
                );
                build_timings.print();
                return Ok(());
            }
        }
    }

    println!(
        "{} {} v{}",
        "Building".green().bold(),
        config.name.cyan(),
        config.version.dimmed()
    );

    // Phase 1: Parse all files (parallel) and extract namespace information
    let mut parsed_files: Vec<ParsedProjectUnit> = Vec::new();
    let mut global_function_map: HashMap<String, String> = HashMap::new(); // func_name -> namespace
    let mut global_function_file_map: HashMap<String, PathBuf> = HashMap::new(); // func_name -> owner file
    let mut global_class_map: HashMap<String, String> = HashMap::new(); // class_name -> namespace
    let mut global_class_file_map: HashMap<String, PathBuf> = HashMap::new(); // class_name -> owner file
    let mut global_interface_map: HashMap<String, String> = HashMap::new(); // interface_name -> namespace
    let mut global_interface_file_map: HashMap<String, PathBuf> = HashMap::new(); // interface_name -> owner file
    let mut global_enum_map: HashMap<String, String> = HashMap::new(); // enum_name -> namespace
    let mut global_enum_file_map: HashMap<String, PathBuf> = HashMap::new(); // enum_name -> owner file
    let mut global_module_map: HashMap<String, String> = HashMap::new(); // module_name -> namespace
    let mut global_module_file_map: HashMap<String, PathBuf> = HashMap::new(); // module_name -> owner file
    let mut project_symbol_lookup_exact: ExactSymbolLookup = HashMap::new();
    let mut project_symbol_lookup_wildcard_members: WildcardMemberLookup = HashMap::new();
    let mut namespace_class_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_interface_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_enum_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_module_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut function_collisions: Vec<(String, String, String)> = Vec::new();
    let mut class_collisions: Vec<(String, String, String)> = Vec::new();
    let mut interface_collisions: Vec<(String, String, String)> = Vec::new();
    let mut enum_collisions: Vec<(String, String, String)> = Vec::new();
    let mut module_collisions: Vec<(String, String, String)> = Vec::new();
    let mut parse_cache_hits: usize = 0;

    let mut parsed_units: Vec<ParsedProjectUnit> =
        build_timings.measure("parse + symbol scan", || {
            files
                .par_iter()
                .map(|file| parse_project_unit(&project_root, file))
                .collect::<Result<Vec<_>, String>>()
        })?;
    parsed_units.sort_by(|a, b| a.file.cmp(&b.file));
    let total_function_names: usize = parsed_units
        .iter()
        .map(|unit| unit.function_names.len())
        .sum();
    let total_class_names: usize = parsed_units.iter().map(|unit| unit.class_names.len()).sum();
    let total_interface_names: usize = parsed_units
        .iter()
        .map(|unit| unit.interface_names.len())
        .sum();
    let total_enum_names: usize = parsed_units.iter().map(|unit| unit.enum_names.len()).sum();
    let total_module_names: usize = parsed_units
        .iter()
        .map(|unit| unit.module_names.len())
        .sum();
    let needs_project_symbol_lookup = parsed_units.iter().any(|unit| {
        unit.imports
            .iter()
            .any(|import| !import.path.starts_with("std."))
    });
    global_function_map.reserve(total_function_names);
    global_function_file_map.reserve(total_function_names);
    global_class_map.reserve(total_class_names);
    global_class_file_map.reserve(total_class_names);
    global_interface_map.reserve(total_interface_names);
    global_interface_file_map.reserve(total_interface_names);
    global_enum_map.reserve(total_enum_names);
    global_enum_file_map.reserve(total_enum_names);
    global_module_map.reserve(total_module_names);
    global_module_file_map.reserve(total_module_names);
    if needs_project_symbol_lookup {
        project_symbol_lookup_exact.reserve(
            total_function_names
                + total_class_names
                + total_interface_names
                + total_enum_names
                + total_module_names,
        );
        project_symbol_lookup_wildcard_members.reserve(
            total_function_names
                + total_class_names
                + total_interface_names
                + total_enum_names
                + total_module_names,
        );
    }
    namespace_class_map.reserve(parsed_units.len());
    namespace_interface_map.reserve(parsed_units.len());
    namespace_enum_map.reserve(parsed_units.len());
    namespace_module_map.reserve(parsed_units.len());
    let mut parse_index_namespace_sets_ns = 0_u64;
    let mut parse_index_function_register_ns = 0_u64;
    let mut parse_index_class_register_ns = 0_u64;
    let mut parse_index_interface_register_ns = 0_u64;
    let mut parse_index_enum_register_ns = 0_u64;
    let mut parse_index_module_register_ns = 0_u64;
    let mut parse_index_parsed_file_push_ns = 0_u64;

    build_timings.measure_step("parse index assembly", || {
        for unit in parsed_units {
            if unit.from_parse_cache {
                parse_cache_hits += 1;
            }

            let namespace_sets_started_at = Instant::now();
            for func_name in &unit.function_names {
                let started_at = Instant::now();
                register_global_symbol(
                    func_name,
                    &unit.namespace,
                    &unit.file,
                    &mut GlobalSymbolRegistrationContext {
                        global_map: &mut global_function_map,
                        global_file_map: &mut global_function_file_map,
                        collisions: &mut function_collisions,
                        exact_lookup: &mut project_symbol_lookup_exact,
                        wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                        build_symbol_lookup: needs_project_symbol_lookup,
                    },
                );
                parse_index_function_register_ns += elapsed_nanos_u64(started_at);
            }
            if !unit.class_names.is_empty() {
                let class_entry = namespace_class_map
                    .entry(unit.namespace.clone())
                    .or_insert_with(|| HashSet::with_capacity(unit.class_names.len()));
                for class_name in &unit.class_names {
                    class_entry.insert(class_name.clone());
                }
                for class_name in &unit.class_names {
                    let started_at = Instant::now();
                    register_global_symbol(
                        class_name,
                        &unit.namespace,
                        &unit.file,
                        &mut GlobalSymbolRegistrationContext {
                            global_map: &mut global_class_map,
                            global_file_map: &mut global_class_file_map,
                            collisions: &mut class_collisions,
                            exact_lookup: &mut project_symbol_lookup_exact,
                            wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                            build_symbol_lookup: needs_project_symbol_lookup,
                        },
                    );
                    parse_index_class_register_ns += elapsed_nanos_u64(started_at);
                }
            }
            if !unit.interface_names.is_empty() {
                let interface_entry = namespace_interface_map
                    .entry(unit.namespace.clone())
                    .or_insert_with(|| HashSet::with_capacity(unit.interface_names.len()));
                for interface_name in &unit.interface_names {
                    interface_entry.insert(interface_name.clone());
                }
                for interface_name in &unit.interface_names {
                    let started_at = Instant::now();
                    register_global_symbol(
                        interface_name,
                        &unit.namespace,
                        &unit.file,
                        &mut GlobalSymbolRegistrationContext {
                            global_map: &mut global_interface_map,
                            global_file_map: &mut global_interface_file_map,
                            collisions: &mut interface_collisions,
                            exact_lookup: &mut project_symbol_lookup_exact,
                            wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                            build_symbol_lookup: needs_project_symbol_lookup,
                        },
                    );
                    parse_index_interface_register_ns += elapsed_nanos_u64(started_at);
                }
            }
            if !unit.enum_names.is_empty() {
                let enum_entry = namespace_enum_map
                    .entry(unit.namespace.clone())
                    .or_insert_with(|| HashSet::with_capacity(unit.enum_names.len()));
                for enum_name in &unit.enum_names {
                    enum_entry.insert(enum_name.clone());
                }
                for enum_name in &unit.enum_names {
                    let started_at = Instant::now();
                    register_global_symbol(
                        enum_name,
                        &unit.namespace,
                        &unit.file,
                        &mut GlobalSymbolRegistrationContext {
                            global_map: &mut global_enum_map,
                            global_file_map: &mut global_enum_file_map,
                            collisions: &mut enum_collisions,
                            exact_lookup: &mut project_symbol_lookup_exact,
                            wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                            build_symbol_lookup: needs_project_symbol_lookup,
                        },
                    );
                    parse_index_enum_register_ns += elapsed_nanos_u64(started_at);
                }
            }
            if !unit.module_names.is_empty() {
                let module_entry = namespace_module_map
                    .entry(unit.namespace.clone())
                    .or_insert_with(|| HashSet::with_capacity(unit.module_names.len()));
                for module_name in &unit.module_names {
                    module_entry.insert(module_name.clone());
                }
                for module_name in &unit.module_names {
                    let started_at = Instant::now();
                    register_global_symbol(
                        module_name,
                        &unit.namespace,
                        &unit.file,
                        &mut GlobalSymbolRegistrationContext {
                            global_map: &mut global_module_map,
                            global_file_map: &mut global_module_file_map,
                            collisions: &mut module_collisions,
                            exact_lookup: &mut project_symbol_lookup_exact,
                            wildcard_lookup: &mut project_symbol_lookup_wildcard_members,
                            build_symbol_lookup: needs_project_symbol_lookup,
                        },
                    );
                    parse_index_module_register_ns += elapsed_nanos_u64(started_at);
                }
            }
            parse_index_namespace_sets_ns += elapsed_nanos_u64(namespace_sets_started_at);

            let push_started_at = Instant::now();
            parsed_files.push(unit);
            parse_index_parsed_file_push_ns += elapsed_nanos_u64(push_started_at);
        }
    });

    if parse_cache_hits > 0 {
        println!(
            "{} Reused parse cache for {}/{} files",
            "→".cyan(),
            parse_cache_hits,
            files.len()
        );
    }
    build_timings.record_counts(
        "parse + symbol scan",
        &[
            ("considered", files.len()),
            ("reused", parse_cache_hits),
            ("parsed", files.len().saturating_sub(parse_cache_hits)),
        ],
    );
    build_timings.record_duration_ns(
        "parse cache/load",
        PARSE_CACHE_TIMING_TOTALS.load_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "parse cache/save",
        PARSE_CACHE_TIMING_TOTALS.save_ns.load(Ordering::Relaxed),
    );
    build_timings.record_counts(
        "parse cache/io",
        &[
            (
                "loads",
                PARSE_CACHE_TIMING_TOTALS.load_count.load(Ordering::Relaxed),
            ),
            (
                "saves",
                PARSE_CACHE_TIMING_TOTALS.save_count.load(Ordering::Relaxed),
            ),
            (
                "bytes_read",
                PARSE_CACHE_TIMING_TOTALS.bytes_read.load(Ordering::Relaxed) as usize,
            ),
            (
                "bytes_written",
                PARSE_CACHE_TIMING_TOTALS
                    .bytes_written
                    .load(Ordering::Relaxed) as usize,
            ),
        ],
    );
    build_timings.record_duration_ns("parse index/namespace sets", parse_index_namespace_sets_ns);
    build_timings.record_duration_ns(
        "parse index/register functions",
        parse_index_function_register_ns,
    );
    build_timings.record_duration_ns(
        "parse index/register classes",
        parse_index_class_register_ns,
    );
    build_timings.record_duration_ns(
        "parse index/register interfaces",
        parse_index_interface_register_ns,
    );
    build_timings.record_duration_ns("parse index/register enums", parse_index_enum_register_ns);
    build_timings.record_duration_ns(
        "parse index/register modules",
        parse_index_module_register_ns,
    );
    build_timings.record_duration_ns("parse index/push units", parse_index_parsed_file_push_ns);
    build_timings.record_counts(
        "parse index/details",
        &[
            ("functions", total_function_names),
            ("classes", total_class_names),
            ("interfaces", total_interface_names),
            ("enums", total_enum_names),
            ("modules", total_module_names),
            (
                "project_symbol_lookup",
                usize::from(needs_project_symbol_lookup),
            ),
        ],
    );
    let previous_dependency_graph = build_timings.measure("dependency cache load", || {
        load_dependency_graph_cache(&project_root)
    })?;
    let mut namespace_files_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    namespace_files_map.reserve(parsed_files.len() + total_module_names);
    let namespace_function_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let namespace_interface_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut dependency_lookup_base_namespace_ns = 0_u64;
    let mut dependency_lookup_module_namespace_ns = 0_u64;
    let mut dependency_lookup_sort_dedup_ns = 0_u64;
    build_timings.measure_step("dependency lookup prep", || {
        for unit in &parsed_files {
            let started_at = Instant::now();
            namespace_files_map
                .entry(unit.namespace.clone())
                .or_default()
                .push(unit.file.clone());
            dependency_lookup_base_namespace_ns += elapsed_nanos_u64(started_at);
            for module_name in &unit.module_names {
                let started_at = Instant::now();
                namespace_files_map
                    .entry(format!(
                        "{}.{}",
                        unit.namespace,
                        module_name.replace("__", ".")
                    ))
                    .or_default()
                    .push(unit.file.clone());
                dependency_lookup_module_namespace_ns += elapsed_nanos_u64(started_at);
            }
        }
        let sort_started_at = Instant::now();
        for files in namespace_files_map.values_mut() {
            files.sort();
            files.dedup();
        }
        dependency_lookup_sort_dedup_ns += elapsed_nanos_u64(sort_started_at);
    });
    build_timings.record_duration_ns(
        "dependency lookup/base namespace",
        dependency_lookup_base_namespace_ns,
    );
    build_timings.record_duration_ns(
        "dependency lookup/module namespace",
        dependency_lookup_module_namespace_ns,
    );
    build_timings.record_duration_ns("dependency lookup/function files", 0);
    build_timings.record_duration_ns("dependency lookup/class files", 0);
    build_timings.record_duration_ns("dependency lookup/interface files", 0);
    build_timings.record_duration_ns("dependency lookup/module files", 0);
    build_timings.record_duration_ns(
        "dependency lookup/sort dedup",
        dependency_lookup_sort_dedup_ns,
    );

    let project_symbol_lookup = ProjectSymbolLookup {
        exact: project_symbol_lookup_exact,
        wildcard_members: project_symbol_lookup_wildcard_members,
    };

    let dependency_resolution_ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        namespace_function_files: &namespace_function_files,
        namespace_class_files: &namespace_class_files,
        namespace_interface_files: &namespace_interface_files,
        namespace_module_files: &namespace_module_files,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_interface_map: &global_interface_map,
        global_interface_file_map: &global_interface_file_map,
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        symbol_lookup: Arc::new(project_symbol_lookup.clone()),
    };
    let dependency_graph_timing_totals = Arc::new(DependencyGraphTimingTotals::default());
    let (file_dependency_graph, dependency_graph_cache_hits) =
        build_timings.measure_value("dependency graph", || {
            build_file_dependency_graph_incremental(
                &parsed_files,
                &dependency_resolution_ctx,
                previous_dependency_graph.as_ref(),
                Some(dependency_graph_timing_totals.as_ref()),
            )
        });
    let reverse_file_dependency_graph = build_reverse_dependency_graph(&file_dependency_graph);
    let current_entry_namespace = parsed_files
        .iter()
        .find(|unit| unit.file == config.get_entry_path(&project_root))
        .map(|unit| unit.namespace.clone())
        .unwrap_or_else(|| "global".to_string());
    let current_dependency_graph_cache = dependency_graph_cache_from_state(
        &current_entry_namespace,
        &parsed_files,
        &file_dependency_graph,
    );
    if dependency_graph_cache_hits > 0 {
        println!(
            "{} Reused dependency graph entries for {}/{} files",
            "→".cyan(),
            dependency_graph_cache_hits,
            parsed_files.len()
        );
    }
    build_timings.record_counts(
        "dependency graph",
        &[
            ("considered", parsed_files.len()),
            ("reused", dependency_graph_cache_hits),
            (
                "rebuilt",
                parsed_files
                    .len()
                    .saturating_sub(dependency_graph_cache_hits),
            ),
        ],
    );
    build_timings.record_duration_ns(
        "dependency graph/cache validation",
        dependency_graph_timing_totals
            .cache_validation_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/direct refs",
        dependency_graph_timing_totals
            .direct_symbol_refs_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/import exact",
        dependency_graph_timing_totals
            .import_exact_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/import wildcard",
        dependency_graph_timing_totals
            .import_wildcard_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/import namespace alias",
        dependency_graph_timing_totals
            .import_namespace_alias_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/import parent namespace",
        dependency_graph_timing_totals
            .import_parent_namespace_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/namespace fallback",
        dependency_graph_timing_totals
            .namespace_fallback_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/owner lookup",
        dependency_graph_timing_totals
            .owner_lookup_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/namespace files",
        dependency_graph_timing_totals
            .namespace_files_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_counts(
        "dependency graph/details",
        &[
            (
                "reused_files",
                dependency_graph_timing_totals
                    .files_reused
                    .load(Ordering::Relaxed),
            ),
            (
                "rebuilt_files",
                dependency_graph_timing_totals
                    .files_rebuilt
                    .load(Ordering::Relaxed),
            ),
            (
                "direct_symbol_refs",
                dependency_graph_timing_totals
                    .direct_symbol_ref_count
                    .load(Ordering::Relaxed),
            ),
            (
                "exact_imports",
                dependency_graph_timing_totals
                    .import_exact_count
                    .load(Ordering::Relaxed),
            ),
            (
                "wildcard_imports",
                dependency_graph_timing_totals
                    .import_wildcard_count
                    .load(Ordering::Relaxed),
            ),
            (
                "namespace_alias_imports",
                dependency_graph_timing_totals
                    .import_namespace_alias_count
                    .load(Ordering::Relaxed),
            ),
            (
                "parent_namespace_imports",
                dependency_graph_timing_totals
                    .import_parent_namespace_count
                    .load(Ordering::Relaxed),
            ),
            (
                "namespace_fallbacks",
                dependency_graph_timing_totals
                    .namespace_fallback_count
                    .load(Ordering::Relaxed),
            ),
            (
                "qualified_refs",
                dependency_graph_timing_totals
                    .qualified_ref_count
                    .load(Ordering::Relaxed),
            ),
        ],
    );

    let previous_semantic_summary = load_semantic_summary_cache(&project_root)?;
    let previous_typecheck_summary = load_typecheck_summary_cache(&project_root)?;
    let mut body_only_changed = HashSet::new();
    let mut api_changed = HashSet::new();
    let mut dependent_api_impact = HashSet::new();

    if let Some(previous) = &previous_dependency_graph {
        let previous_files: HashMap<&PathBuf, &DependencyGraphFileEntry> = previous
            .files
            .iter()
            .map(|entry| (&entry.file, entry))
            .collect();

        for unit in &parsed_files {
            match previous_files.get(&unit.file) {
                Some(prev) if prev.semantic_fingerprint == unit.semantic_fingerprint => {}
                Some(prev) if prev.api_fingerprint == unit.api_fingerprint => {
                    body_only_changed.insert(unit.file.clone());
                }
                _ => {
                    api_changed.insert(unit.file.clone());
                }
            }
        }

        dependent_api_impact = if api_changed.is_empty() {
            HashSet::new()
        } else {
            let mut impacted = transitive_dependents(&reverse_file_dependency_graph, &api_changed);
            for changed in &api_changed {
                impacted.remove(changed);
            }
            impacted
        };

        if !body_only_changed.is_empty() || !api_changed.is_empty() {
            println!(
                "{} Impact graph: {} body-only, {} API, {} downstream dependents",
                "→".cyan(),
                body_only_changed.len(),
                api_changed.len(),
                dependent_api_impact.len()
            );
        }
    }

    let (semantic_fingerprint, semantic_cache_hit) =
        build_timings.measure("semantic cache gate", || {
            let semantic_fingerprint =
                compute_semantic_project_fingerprint(&config, &parsed_files, emit_llvm, do_check);
            let semantic_cache_hit = if !check_only {
                load_semantic_cached_fingerprint(&project_root)?.is_some_and(|cached| {
                    cached == semantic_fingerprint
                        && project_build_artifact_exists(&output_path, emit_llvm)
                })
            } else {
                false
            };
            Ok::<_, String>((semantic_fingerprint, semantic_cache_hit))
        })?;
    build_timings.record_counts(
        "semantic cache gate",
        &[
            ("files", parsed_files.len()),
            ("body_only", body_only_changed.len()),
            ("api", api_changed.len()),
            ("downstream", dependent_api_impact.len()),
            ("hit", usize::from(semantic_cache_hit)),
        ],
    );
    if semantic_cache_hit {
        println!(
            "{} {}",
            "Semantic cache hit".green().bold(),
            config.name.cyan(),
        );
        save_cached_fingerprint(&project_root, &fingerprint)?;
        build_timings.print();
        return Ok(());
    }

    if !function_collisions.is_empty() {
        eprintln!(
            "{} Function name collisions detected across namespaces:",
            "error".red().bold()
        );
        for (func, ns_a, ns_b) in function_collisions {
            eprintln!(
                "  → '{}' is defined in both '{}' and '{}'",
                func, ns_a, ns_b
            );
        }
        return Err(
            "Project contains colliding top-level function names. Use module-qualified names or rename functions."
                .to_string(),
        );
    }
    if !class_collisions.is_empty() {
        eprintln!(
            "{} Class name collisions detected across namespaces:",
            "error".red().bold()
        );
        for (name, ns_a, ns_b) in class_collisions {
            eprintln!(
                "  → '{}' is defined in both '{}' and '{}'",
                name, ns_a, ns_b
            );
        }
        return Err(
            "Project contains colliding top-level class names. Use unique class names per project."
                .to_string(),
        );
    }
    if !enum_collisions.is_empty() {
        eprintln!(
            "{} Enum name collisions detected across namespaces:",
            "error".red().bold()
        );
        for (name, ns_a, ns_b) in enum_collisions {
            eprintln!(
                "  → '{}' is defined in both '{}' and '{}'",
                name, ns_a, ns_b
            );
        }
        return Err(
            "Project contains colliding top-level enum names. Use unique enum names per project."
                .to_string(),
        );
    }
    if !interface_collisions.is_empty() {
        eprintln!(
            "{} Interface name collisions detected across namespaces:",
            "error".red().bold()
        );
        for (name, ns_a, ns_b) in interface_collisions {
            eprintln!(
                "  → '{}' is defined in both '{}' and '{}'",
                name, ns_a, ns_b
            );
        }
        return Err(
            "Project contains colliding top-level interface names. Use unique interface names per project."
                .to_string(),
        );
    }
    if !module_collisions.is_empty() {
        eprintln!(
            "{} Module name collisions detected across namespaces:",
            "error".red().bold()
        );
        for (name, ns_a, ns_b) in module_collisions {
            eprintln!(
                "  → '{}' is defined in both '{}' and '{}'",
                name, ns_a, ns_b
            );
        }
        return Err(
            "Project contains colliding top-level module names. Use unique module names per project."
                .to_string(),
        );
    }

    let entry_path = config.get_entry_path(&project_root);
    if !do_check {
        let entry_source = fs::read_to_string(&entry_path).map_err(|e| {
            format!(
                "{}: Failed to read entry file '{}': {}",
                "error".red().bold(),
                entry_path.display(),
                e
            )
        })?;
        let entry_program = parsed_files
            .iter()
            .find(|unit| unit.file == entry_path)
            .map(|unit| &unit.program)
            .ok_or_else(|| {
                format!(
                    "{}: Entry file '{}' was not parsed",
                    "error".red().bold(),
                    entry_path.display()
                )
            })?;
        let entry_filename = entry_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("main.apex");
        validate_entry_main_signature(entry_program, &entry_source, entry_filename)?;
    }

    let (namespace_functions, entry_namespace, namespace_api_fingerprints, file_api_fingerprints) =
        build_timings.measure_step("rewrite prep", || {
            let mut namespace_functions: HashMap<String, HashSet<String>> = HashMap::new();
            for unit in &parsed_files {
                if unit.function_names.is_empty() {
                    continue;
                }
                namespace_functions
                    .entry(unit.namespace.clone())
                    .or_insert_with(|| HashSet::with_capacity(unit.function_names.len()))
                    .extend(unit.function_names.iter().cloned());
            }

            let entry_namespace = parsed_files
                .iter()
                .find(|unit| unit.file == entry_path)
                .map(|unit| unit.namespace.clone())
                .unwrap_or_else(|| "global".to_string());
            let namespace_api_fingerprints = compute_namespace_api_fingerprints(&parsed_files);
            let file_api_fingerprints: HashMap<PathBuf, String> = parsed_files
                .iter()
                .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
                .collect();
            (
                namespace_functions,
                entry_namespace,
                namespace_api_fingerprints,
                file_api_fingerprints,
            )
        });
    let rewrite_fingerprint_ctx = RewriteFingerprintContext {
        namespace_functions: &namespace_functions,
        namespace_function_files: &namespace_function_files,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        namespace_classes: &namespace_class_map,
        namespace_class_files: &namespace_class_files,
        namespace_interface_files: &namespace_interface_files,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_interface_map: &global_interface_map,
        global_interface_file_map: &global_interface_file_map,
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        namespace_modules: &namespace_module_map,
        namespace_module_files: &namespace_module_files,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints,
        file_api_fingerprints: &file_api_fingerprints,
        symbol_lookup: Arc::new(project_symbol_lookup.clone()),
    };
    let safe_rewrite_cache_files: HashSet<PathBuf> =
        if can_reuse_safe_rewrite_cache(previous_dependency_graph.as_ref(), &entry_namespace) {
            parsed_files
                .iter()
                .filter(|unit| {
                    !body_only_changed.contains(&unit.file)
                        && !api_changed.contains(&unit.file)
                        && !dependent_api_impact.contains(&unit.file)
                })
                .map(|unit| unit.file.clone())
                .collect()
        } else {
            HashSet::new()
        };

    // Phase 2: Check imports for each file
    if do_check {
        println!("{} Checking imports...", "→".cyan());
        let shared_function_map = Arc::new(global_function_map.clone());
        let shared_known_namespace_paths =
            Arc::new(collect_known_namespace_paths_for_units(&parsed_files));
        let import_check_cache_hits = std::sync::atomic::AtomicUsize::new(0);
        let import_check_timing_totals = Arc::new(ImportCheckTimingTotals::default());

        let import_results: Vec<Result<(), String>> =
            build_timings.measure("import check", || {
                Ok::<_, String>(
                    parsed_files
                        .par_iter()
                        .map(|unit| {
                            let fingerprint_started_at = Instant::now();
                            let rewrite_context_fingerprint =
                                compute_rewrite_context_fingerprint_for_unit_impl(
                                    unit,
                                    &entry_namespace,
                                    &rewrite_fingerprint_ctx,
                                    None,
                                );
                            import_check_timing_totals
                                .rewrite_context_fingerprint_ns
                                .fetch_add(
                                    elapsed_nanos_u64(fingerprint_started_at),
                                    Ordering::Relaxed,
                                );
                            let cache_lookup_started_at = Instant::now();
                            if load_import_check_cache_hit(
                                &project_root,
                                &unit.file,
                                &unit.import_check_fingerprint,
                                &rewrite_context_fingerprint,
                            )? {
                                import_check_timing_totals.cache_lookup_ns.fetch_add(
                                    elapsed_nanos_u64(cache_lookup_started_at),
                                    Ordering::Relaxed,
                                );
                                import_check_cache_hits
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                return Ok(());
                            }
                            import_check_timing_totals.cache_lookup_ns.fetch_add(
                                elapsed_nanos_u64(cache_lookup_started_at),
                                Ordering::Relaxed,
                            );

                            let checker_init_started_at = Instant::now();
                            let mut checker = ImportChecker::new(
                                Arc::clone(&shared_function_map),
                                Arc::clone(&shared_known_namespace_paths),
                                unit.namespace.clone(),
                                unit.imports.clone(),
                                stdlib_registry(),
                            );
                            import_check_timing_totals.checker_init_ns.fetch_add(
                                elapsed_nanos_u64(checker_init_started_at),
                                Ordering::Relaxed,
                            );

                            let checker_run_started_at = Instant::now();
                            if let Err(errors) = checker.check_program(&unit.program) {
                                import_check_timing_totals.checker_run_ns.fetch_add(
                                    elapsed_nanos_u64(checker_run_started_at),
                                    Ordering::Relaxed,
                                );
                                let filename = unit
                                    .file
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown");
                                let mut rendered = format!(
                                    "{} Import errors in {}:\n",
                                    "error".red().bold(),
                                    filename
                                );
                                for err in errors {
                                    rendered.push_str(&format!("  → {}\n", err.format()));
                                }
                                return Err(rendered);
                            }
                            import_check_timing_totals.checker_run_ns.fetch_add(
                                elapsed_nanos_u64(checker_run_started_at),
                                Ordering::Relaxed,
                            );
                            let cache_save_started_at = Instant::now();
                            save_import_check_cache_hit(
                                &project_root,
                                &unit.file,
                                &unit.import_check_fingerprint,
                                &rewrite_context_fingerprint,
                            )?;
                            import_check_timing_totals.cache_save_ns.fetch_add(
                                elapsed_nanos_u64(cache_save_started_at),
                                Ordering::Relaxed,
                            );
                            Ok(())
                        })
                        .collect(),
                )
            })?;

        for result in import_results {
            if let Err(rendered) = result {
                return Err(format!("Import check failed\n{rendered}"));
            }
        }
        let import_check_cache_hits =
            import_check_cache_hits.load(std::sync::atomic::Ordering::Relaxed);
        if import_check_cache_hits > 0 {
            println!(
                "{} Reused import-check cache for {}/{} files",
                "→".cyan(),
                import_check_cache_hits,
                parsed_files.len()
            );
        }
        build_timings.record_counts(
            "import check",
            &[
                ("considered", parsed_files.len()),
                ("reused", import_check_cache_hits),
                (
                    "checked",
                    parsed_files.len().saturating_sub(import_check_cache_hits),
                ),
            ],
        );
        build_timings.record_duration_ns(
            "import check/context fingerprint",
            import_check_timing_totals
                .rewrite_context_fingerprint_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "import check/cache lookup",
            import_check_timing_totals
                .cache_lookup_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "import check/checker init",
            import_check_timing_totals
                .checker_init_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "import check/checker run",
            import_check_timing_totals
                .checker_run_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "import check/cache save",
            import_check_timing_totals
                .cache_save_ns
                .load(Ordering::Relaxed),
        );
    }

    // Phase 3: Build combined AST with deterministic namespace mangling.
    project_rewrite::reset_rewrite_timings();
    let rewrite_timing_totals = Arc::new(PipelineRewriteTimingTotals::default());
    let rewrite_fingerprint_timing_totals = Arc::new(RewriteFingerprintTimingTotals::default());
    let rewritten_results: Vec<Result<RewrittenProjectUnit, String>> =
        build_timings.measure("rewrite", || {
            Ok::<_, String>(
                parsed_files
                    .par_iter()
                    .map(|unit| {
                        if safe_rewrite_cache_files.contains(&unit.file) {
                            let cache_lookup_started_at = Instant::now();
                            if let Some(cached_entry) =
                                load_rewritten_file_cache_if_semantic_matches(
                                    &project_root,
                                    &unit.file,
                                    &unit.semantic_fingerprint,
                                )?
                            {
                                rewrite_timing_totals.cache_lookup_ns.fetch_add(
                                    elapsed_nanos_u64(cache_lookup_started_at),
                                    Ordering::Relaxed,
                                );
                                let cached = cached_entry.rewritten_program;
                                return Ok(RewrittenProjectUnit {
                                    file: unit.file.clone(),
                                    program: cached,
                                    api_program: cached_entry.api_program,
                                    specialization_projection: cached_entry
                                        .specialization_projection,
                                    semantic_fingerprint: unit.semantic_fingerprint.clone(),
                                    rewrite_context_fingerprint: cached_entry
                                        .rewrite_context_fingerprint,
                                    active_symbols: cached_entry
                                        .active_symbols
                                        .into_iter()
                                        .collect(),
                                    has_specialization_demand: cached_entry
                                        .has_specialization_demand,
                                    from_rewrite_cache: true,
                                });
                            }
                            rewrite_timing_totals.cache_lookup_ns.fetch_add(
                                elapsed_nanos_u64(cache_lookup_started_at),
                                Ordering::Relaxed,
                            );
                        }
                        let fingerprint_started_at = Instant::now();
                        let rewrite_context_fingerprint =
                            compute_rewrite_context_fingerprint_for_unit_impl(
                                unit,
                                &entry_namespace,
                                &rewrite_fingerprint_ctx,
                                Some(rewrite_fingerprint_timing_totals.as_ref()),
                            );
                        rewrite_timing_totals
                            .rewrite_context_fingerprint_ns
                            .fetch_add(
                                elapsed_nanos_u64(fingerprint_started_at),
                                Ordering::Relaxed,
                            );
                        let cache_lookup_started_at = Instant::now();
                        if let Some(cached) = load_rewritten_file_cache(
                            &project_root,
                            &unit.file,
                            &unit.semantic_fingerprint,
                            &rewrite_context_fingerprint,
                        )? {
                            rewrite_timing_totals.cache_lookup_ns.fetch_add(
                                elapsed_nanos_u64(cache_lookup_started_at),
                                Ordering::Relaxed,
                            );
                            let rewritten_program = cached.rewritten_program;
                            return Ok(RewrittenProjectUnit {
                                file: unit.file.clone(),
                                program: rewritten_program,
                                api_program: cached.api_program,
                                specialization_projection: cached.specialization_projection,
                                semantic_fingerprint: unit.semantic_fingerprint.clone(),
                                rewrite_context_fingerprint: rewrite_context_fingerprint.clone(),
                                active_symbols: cached.active_symbols.into_iter().collect(),
                                has_specialization_demand: cached.has_specialization_demand,
                                from_rewrite_cache: true,
                            });
                        }

                        rewrite_timing_totals.cache_lookup_ns.fetch_add(
                            elapsed_nanos_u64(cache_lookup_started_at),
                            Ordering::Relaxed,
                        );
                        let rewrite_program_started_at = Instant::now();
                        let rewritten = project_rewrite::rewrite_program_for_project(
                            &unit.program,
                            &project_rewrite::ProjectRewriteContext {
                                current_namespace: &unit.namespace,
                                entry_namespace: &entry_namespace,
                                namespace_functions: &namespace_functions,
                                global_function_map: &global_function_map,
                                namespace_classes: &namespace_class_map,
                                global_class_map: &global_class_map,
                                namespace_interfaces: &namespace_interface_map,
                                global_interface_map: &global_interface_map,
                                namespace_enums: &namespace_enum_map,
                                global_enum_map: &global_enum_map,
                                namespace_modules: &namespace_module_map,
                                global_module_map: &global_module_map,
                                imports: &unit.imports,
                            },
                        );
                        rewrite_timing_totals.rewrite_program_ns.fetch_add(
                            elapsed_nanos_u64(rewrite_program_started_at),
                            Ordering::Relaxed,
                        );
                        let cache_save_started_at = Instant::now();
                        let active_symbols_started_at = Instant::now();
                        let active_symbols = collect_active_symbols(&rewritten);
                        rewrite_timing_totals.active_symbols_ns.fetch_add(
                            elapsed_nanos_u64(active_symbols_started_at),
                            Ordering::Relaxed,
                        );
                        let api_projection_started_at = Instant::now();
                        let api_program = api_projection_program(&rewritten);
                        rewrite_timing_totals.api_projection_ns.fetch_add(
                            elapsed_nanos_u64(api_projection_started_at),
                            Ordering::Relaxed,
                        );
                        let specialization_projection_started_at = Instant::now();
                        let specialization_projection =
                            specialization_projection_program(&rewritten);
                        rewrite_timing_totals
                            .specialization_projection_ns
                            .fetch_add(
                                elapsed_nanos_u64(specialization_projection_started_at),
                                Ordering::Relaxed,
                            );
                        let specialization_demand_started_at = Instant::now();
                        let has_specialization_demand =
                            program_has_codegen_specialization_demand(&rewritten);
                        rewrite_timing_totals.specialization_demand_ns.fetch_add(
                            elapsed_nanos_u64(specialization_demand_started_at),
                            Ordering::Relaxed,
                        );
                        save_rewritten_file_cache(
                            &project_root,
                            &unit.file,
                            &unit.semantic_fingerprint,
                            &rewrite_context_fingerprint,
                            &rewritten,
                            &api_program,
                            &specialization_projection,
                            &active_symbols,
                            has_specialization_demand,
                        )?;
                        rewrite_timing_totals
                            .cache_save_ns
                            .fetch_add(elapsed_nanos_u64(cache_save_started_at), Ordering::Relaxed);
                        Ok(RewrittenProjectUnit {
                            file: unit.file.clone(),
                            active_symbols,
                            api_program,
                            specialization_projection,
                            program: rewritten,
                            semantic_fingerprint: unit.semantic_fingerprint.clone(),
                            rewrite_context_fingerprint,
                            has_specialization_demand,
                            from_rewrite_cache: false,
                        })
                    })
                    .collect(),
            )
        })?;

    let mut rewritten_files: Vec<RewrittenProjectUnit> = Vec::new();
    for result in rewritten_results {
        rewritten_files.push(result?);
    }
    rewritten_files.sort_by(|a, b| a.file.cmp(&b.file));

    let rewrite_cache_hits = rewritten_files
        .iter()
        .filter(|unit| unit.from_rewrite_cache)
        .count();
    if rewrite_cache_hits > 0 {
        println!(
            "{} Reused rewrite cache for {}/{} files",
            "→".cyan(),
            rewrite_cache_hits,
            rewritten_files.len()
        );
    }
    build_timings.record_counts(
        "rewrite",
        &[
            ("considered", rewritten_files.len()),
            ("reused", rewrite_cache_hits),
            (
                "rewritten",
                rewritten_files.len().saturating_sub(rewrite_cache_hits),
            ),
        ],
    );
    build_timings.record_duration_ns(
        "rewrite cache/load",
        REWRITE_CACHE_TIMING_TOTALS.load_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite cache/save",
        REWRITE_CACHE_TIMING_TOTALS.save_ns.load(Ordering::Relaxed),
    );
    build_timings.record_counts(
        "rewrite cache/io",
        &[
            (
                "loads",
                REWRITE_CACHE_TIMING_TOTALS
                    .load_count
                    .load(Ordering::Relaxed),
            ),
            (
                "saves",
                REWRITE_CACHE_TIMING_TOTALS
                    .save_count
                    .load(Ordering::Relaxed),
            ),
            (
                "bytes_read",
                REWRITE_CACHE_TIMING_TOTALS
                    .bytes_read
                    .load(Ordering::Relaxed) as usize,
            ),
            (
                "bytes_written",
                REWRITE_CACHE_TIMING_TOTALS
                    .bytes_written
                    .load(Ordering::Relaxed) as usize,
            ),
        ],
    );
    build_timings.record_duration_ns(
        "rewrite/context fingerprint",
        rewrite_timing_totals
            .rewrite_context_fingerprint_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite/cache lookup",
        rewrite_timing_totals
            .cache_lookup_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite/rewrite program",
        rewrite_timing_totals
            .rewrite_program_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite/cache save",
        rewrite_timing_totals.cache_save_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite/active symbols",
        rewrite_timing_totals
            .active_symbols_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite/api projection",
        rewrite_timing_totals
            .api_projection_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite/specialization projection",
        rewrite_timing_totals
            .specialization_projection_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite/specialization demand",
        rewrite_timing_totals
            .specialization_demand_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite fingerprint/local refs",
        rewrite_fingerprint_timing_totals
            .local_symbol_refs_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite fingerprint/wildcard imports",
        rewrite_fingerprint_timing_totals
            .wildcard_imports_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite fingerprint/namespace alias imports",
        rewrite_fingerprint_timing_totals
            .namespace_alias_imports_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite fingerprint/exact imports",
        rewrite_fingerprint_timing_totals
            .exact_imports_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite fingerprint/prefix expansion",
        rewrite_fingerprint_timing_totals
            .relevant_namespace_prefixes_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "rewrite fingerprint/namespace hashing",
        rewrite_fingerprint_timing_totals
            .namespace_hashing_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_counts(
        "rewrite fingerprint/details",
        &[
            (
                "local_refs",
                rewrite_fingerprint_timing_totals
                    .local_symbol_ref_count
                    .load(Ordering::Relaxed),
            ),
            (
                "wildcard_imports",
                rewrite_fingerprint_timing_totals
                    .wildcard_import_count
                    .load(Ordering::Relaxed),
            ),
            (
                "namespace_alias_imports",
                rewrite_fingerprint_timing_totals
                    .namespace_alias_import_count
                    .load(Ordering::Relaxed),
            ),
            (
                "exact_imports",
                rewrite_fingerprint_timing_totals
                    .exact_import_count
                    .load(Ordering::Relaxed),
            ),
            (
                "expanded_prefixes",
                rewrite_fingerprint_timing_totals
                    .prefix_expand_count
                    .load(Ordering::Relaxed),
            ),
        ],
    );
    let project_rewrite_timings = project_rewrite::snapshot_rewrite_timings();
    build_timings.record_duration_ns(
        "rewrite program/import map build",
        project_rewrite_timings.import_map_build_ns,
    );
    build_timings.record_duration_ns(
        "rewrite program/wildcard match",
        project_rewrite_timings.wildcard_match_ns,
    );
    build_timings.record_duration_ns(
        "rewrite program/exact import resolve",
        project_rewrite_timings.exact_import_resolve_ns,
    );
    build_timings.record_duration_ns(
        "rewrite program/block rewrite",
        project_rewrite_timings.block_rewrite_ns,
    );
    build_timings.record_duration_ns(
        "rewrite program/stmt rewrite",
        project_rewrite_timings.stmt_rewrite_ns,
    );
    build_timings.record_duration_ns(
        "rewrite program/expr rewrite",
        project_rewrite_timings.expr_rewrite_ns,
    );
    build_timings.record_duration_ns(
        "rewrite program/type rewrite",
        project_rewrite_timings.type_rewrite_ns,
    );
    build_timings.record_duration_ns(
        "rewrite program/pattern rewrite",
        project_rewrite_timings.pattern_rewrite_ns,
    );
    build_timings.record_counts(
        "rewrite program/details",
        &[
            (
                "wildcard_calls",
                project_rewrite_timings.wildcard_match_calls,
            ),
            (
                "exact_import_resolves",
                project_rewrite_timings.exact_import_resolve_calls,
            ),
            ("block_calls", project_rewrite_timings.block_rewrite_calls),
            ("stmt_calls", project_rewrite_timings.stmt_rewrite_calls),
            ("expr_calls", project_rewrite_timings.expr_rewrite_calls),
            ("type_calls", project_rewrite_timings.type_rewrite_calls),
            (
                "pattern_calls",
                project_rewrite_timings.pattern_rewrite_calls,
            ),
        ],
    );

    if do_check {
        let mut semantic_full_files: HashSet<PathBuf> =
            parsed_files.iter().map(|u| u.file.clone()).collect();
        if previous_dependency_graph.is_some() && previous_semantic_summary.is_some() {
            semantic_full_files = body_only_changed
                .union(&api_changed)
                .cloned()
                .collect::<HashSet<_>>();
            semantic_full_files.extend(dependent_api_impact.iter().cloned());
            if semantic_full_files.is_empty() {
                semantic_full_files.extend(parsed_files.iter().map(|u| u.file.clone()));
            }
        }
        let current_semantic_fingerprints: HashMap<PathBuf, String> = parsed_files
            .iter()
            .map(|unit| (unit.file.clone(), unit.semantic_fingerprint.clone()))
            .collect();
        let semantic_components = semantic_check_components(&parsed_files, &file_dependency_graph);
        let reusable_component_fps = previous_typecheck_summary
            .as_ref()
            .map(|cache| {
                reusable_component_fingerprints(
                    cache,
                    &current_semantic_fingerprints,
                    &semantic_components,
                )
            })
            .unwrap_or_default();
        let reusable_typecheck_cache = reusable_component_fps.len() == semantic_components.len()
            && typecheck_summary_cache_matches(
                previous_typecheck_summary
                    .as_ref()
                    .expect("component cache checked above"),
                &current_semantic_fingerprints,
                &semantic_components,
            );

        if reusable_typecheck_cache {
            println!(
                "{} Reused typecheck/borrowck cache for {}/{} files",
                "→".cyan(),
                current_semantic_fingerprints.len(),
                parsed_files.len()
            );
            build_timings.record_counts(
                "semantic",
                &[
                    ("components", semantic_components.len()),
                    ("reused_components", semantic_components.len()),
                    ("checked_components", 0),
                    ("reused_files", parsed_files.len()),
                    ("checked_files", 0),
                ],
            );
        } else {
            let reusable_component_files: HashSet<PathBuf> = semantic_components
                .iter()
                .filter(|component| {
                    reusable_component_fps.contains(&component_fingerprint(
                        component,
                        &current_semantic_fingerprints,
                    ))
                })
                .flat_map(|component| component.iter().cloned())
                .collect();
            let checked_components = semantic_components
                .iter()
                .filter(|component| {
                    !reusable_component_fps.contains(&component_fingerprint(
                        component,
                        &current_semantic_fingerprints,
                    ))
                })
                .cloned()
                .collect::<Vec<_>>();
            let seeded_function_effects;
            let seeded_class_method_effects;
            let seeded_class_mutating_methods;
            (
                seeded_function_effects,
                seeded_class_method_effects,
                seeded_class_mutating_methods,
            ) = previous_semantic_summary
                .as_ref()
                .map(|cache| {
                    semantic_seed_data_from_cache(
                        cache,
                        &current_semantic_fingerprints,
                        &semantic_full_files,
                    )
                })
                .unwrap_or_else(|| (HashMap::new(), HashMap::new(), HashMap::new()));

            if semantic_full_files.len() < parsed_files.len() {
                println!(
                    "{} Semantic delta: checking {}/{} files with full bodies",
                    "→".cyan(),
                    semantic_full_files.len(),
                    parsed_files.len()
                );
            }
            if semantic_components.len() > 1 {
                println!(
                    "{} Parallel semantic check across {} independent components",
                    "→".cyan(),
                    semantic_components.len()
                );
            }
            if !reusable_component_files.is_empty() {
                println!(
                    "{} Reused semantic component cache for {}/{} files",
                    "→".cyan(),
                    reusable_component_files.len(),
                    parsed_files.len()
                );
            }

            struct ComponentSemanticCheckResult {
                function_effects: FunctionEffectsSummary,
                class_method_effects: ClassMethodEffectsSummary,
                class_mutating_methods: HashMap<String, HashSet<String>>,
            }

            let semantic_results: Vec<Result<ComponentSemanticCheckResult, String>> = build_timings
                .measure("semantic", || {
                    Ok::<_, String>(
                        checked_components
                            .par_iter()
                            .map(|component| {
                                let component_files: HashSet<PathBuf> =
                                    component.iter().cloned().collect();
                                let semantic_program = semantic_program_for_component(
                                    &rewritten_files,
                                    &component_files,
                                    &semantic_full_files,
                                );

                                let mut type_checker = TypeChecker::new(String::new());
                                if let Err(errors) = type_checker.check_with_effect_seeds(
                                    &semantic_program,
                                    &seeded_function_effects,
                                    &seeded_class_method_effects,
                                ) {
                                    return Err(render_type_errors(errors));
                                }

                                let mut borrow_checker = BorrowChecker::new();
                                if let Err(errors) = borrow_checker
                                    .check_with_mutating_method_seeds(
                                        &semantic_program,
                                        &seeded_class_mutating_methods,
                                    )
                                {
                                    return Err(render_borrow_errors(errors));
                                }

                                let (function_effects, class_method_effects) =
                                    type_checker.export_effect_summary();
                                Ok(ComponentSemanticCheckResult {
                                    function_effects,
                                    class_method_effects,
                                    class_mutating_methods: borrow_checker
                                        .export_class_mutating_method_summary(),
                                })
                            })
                            .collect(),
                    )
                })?;
            build_timings.record_counts(
                "semantic",
                &[
                    ("components", semantic_components.len()),
                    (
                        "reused_components",
                        semantic_components
                            .len()
                            .saturating_sub(checked_components.len()),
                    ),
                    ("checked_components", checked_components.len()),
                    ("reused_files", reusable_component_files.len()),
                    (
                        "checked_files",
                        parsed_files
                            .len()
                            .saturating_sub(reusable_component_files.len()),
                    ),
                    ("full_body_files", semantic_full_files.len()),
                ],
            );

            let mut rendered_errors = String::new();
            let (mut function_effects, mut class_method_effects, mut class_mutating_methods) =
                previous_semantic_summary
                    .as_ref()
                    .map(|cache| {
                        merge_reusable_component_semantic_data(cache, &reusable_component_fps)
                    })
                    .unwrap_or_else(|| (HashMap::new(), HashMap::new(), HashMap::new()));

            for result in semantic_results {
                match result {
                    Ok(component) => {
                        function_effects.extend(component.function_effects);
                        class_method_effects.extend(component.class_method_effects);
                        class_mutating_methods.extend(component.class_mutating_methods);
                    }
                    Err(errors) => rendered_errors.push_str(&errors),
                }
            }

            if !rendered_errors.is_empty() {
                return Err(rendered_errors);
            }

            save_semantic_summary_cache(
                &project_root,
                &semantic_summary_cache_from_state(
                    &parsed_files,
                    &semantic_components,
                    function_effects,
                    class_method_effects,
                    class_mutating_methods,
                ),
            )?;
            save_typecheck_summary_cache(
                &project_root,
                &typecheck_summary_cache_from_state(
                    &current_semantic_fingerprints,
                    &semantic_components,
                ),
            )?;
        }
    }

    if check_only {
        println!("{} {}", "Check passed".green().bold(), config.name.cyan());
        build_timings.print();
        return Ok(());
    }

    // Compile combined program AST (import/type checks already done above).
    let link = LinkConfig {
        opt_level: Some(&config.opt_level),
        target: config.target.as_deref(),
        output_kind: config.output_kind.clone(),
        link_search: &config.link_search,
        link_libs: &config.link_libs,
        link_args: &config.link_args,
    };
    if emit_llvm {
        let combined_program = combined_program_for_files(&rewritten_files);
        build_timings.measure("full codegen", || {
            compile_program_ast(
                &combined_program,
                &entry_path,
                &output_path,
                emit_llvm,
                &link,
            )
        })?;
        build_timings.record_counts("full codegen", &[("files", rewritten_files.len())]);
    } else {
        if rewritten_files
            .iter()
            .any(|unit| unit.has_specialization_demand)
        {
            let combined_program = combined_program_for_files(&rewritten_files);
            build_timings.measure("full codegen", || {
                compile_program_ast(&combined_program, &entry_path, &output_path, false, &link)
            })?;
            build_timings.record_counts("full codegen", &[("files", rewritten_files.len())]);
            save_cached_fingerprint(&project_root, &fingerprint)?;
            println!("Built {} -> {}", config.name.cyan(), output_path.display());
            build_timings.print();
            return Ok(());
        }

        let object_build_fingerprint = compute_object_build_fingerprint(&link);
        let previous_link_manifest = build_timings.measure("link manifest load", || {
            load_link_manifest_cache(&project_root)
        })?;
        let (
            rewritten_file_indices,
            object_cache_paths_by_file,
            codegen_reference_metadata,
            precomputed_dependency_closures,
        ) = build_timings.measure_step("object prep", || {
            let rewritten_file_indices: HashMap<PathBuf, usize> = rewritten_files
                .iter()
                .enumerate()
                .map(|(index, unit)| (unit.file.clone(), index))
                .collect();
            let object_cache_paths_by_file: HashMap<PathBuf, ObjectCachePaths> = rewritten_files
                .iter()
                .map(|unit| {
                    (
                        unit.file.clone(),
                        object_cache_paths(&project_root, &unit.file),
                    )
                })
                .collect();
            let codegen_reference_metadata: HashMap<PathBuf, CodegenReferenceMetadata> =
                parsed_files
                    .iter()
                    .map(|unit| {
                        (
                            unit.file.clone(),
                            CodegenReferenceMetadata {
                                imports: unit.imports.clone(),
                                referenced_symbols: unit.referenced_symbols.clone(),
                                qualified_symbol_refs: unit.qualified_symbol_refs.clone(),
                                api_referenced_symbols: unit.api_referenced_symbols.clone(),
                            },
                        )
                    })
                    .collect();
            let precomputed_dependency_closures =
                precompute_all_transitive_dependencies(&file_dependency_graph);
            (
                rewritten_file_indices,
                object_cache_paths_by_file,
                codegen_reference_metadata,
                precomputed_dependency_closures,
            )
        });
        let mut object_paths: Vec<Option<PathBuf>> = vec![None; rewritten_files.len()];
        let object_candidate_count = rewritten_files
            .iter()
            .filter(|unit| !unit.active_symbols.is_empty())
            .count();
        let active_indices = rewritten_files
            .iter()
            .enumerate()
            .filter_map(|(index, unit)| (!unit.active_symbols.is_empty()).then_some(index))
            .collect::<Vec<_>>();
        let object_shard_size = object_codegen_shard_size();
        let object_shard_threshold = object_codegen_shard_threshold();
        let use_object_shards = active_indices.len() >= object_shard_threshold;
        let object_shards = if use_object_shards {
            active_indices
                .chunks(object_shard_size)
                .map(|chunk| {
                    let member_indices = chunk.to_vec();
                    let member_files = member_indices
                        .iter()
                        .map(|index| rewritten_files[*index].file.clone())
                        .collect::<Vec<_>>();
                    let member_fingerprints = member_indices
                        .iter()
                        .map(|index| {
                            let unit = &rewritten_files[*index];
                            ObjectShardMemberFingerprint {
                                file: unit.file.clone(),
                                semantic_fingerprint: unit.semantic_fingerprint.clone(),
                                rewrite_context_fingerprint: unit
                                    .rewrite_context_fingerprint
                                    .clone(),
                            }
                        })
                        .collect::<Vec<_>>();
                    let cache_paths = Some(object_shard_cache_paths(&project_root, &member_files));
                    ObjectCodegenShard {
                        member_indices,
                        member_files,
                        member_fingerprints,
                        cache_paths,
                    }
                })
                .collect::<Vec<_>>()
        } else {
            active_indices
                .iter()
                .map(|index| {
                    let unit = &rewritten_files[*index];
                    ObjectCodegenShard {
                        member_indices: vec![*index],
                        member_files: vec![unit.file.clone()],
                        member_fingerprints: vec![ObjectShardMemberFingerprint {
                            file: unit.file.clone(),
                            semantic_fingerprint: unit.semantic_fingerprint.clone(),
                            rewrite_context_fingerprint: unit.rewrite_context_fingerprint.clone(),
                        }],
                        cache_paths: None,
                    }
                })
                .collect::<Vec<_>>()
        };
        type ObjectCacheProbeResult = Result<(Vec<usize>, Option<PathBuf>), String>;
        let cache_probe_results: Vec<ObjectCacheProbeResult> =
            build_timings.measure("object cache probe", || {
                Ok::<_, String>(
                    object_shards
                        .par_iter()
                        .map(|shard| {
                            let cached_obj = if let Some(cache_paths) = &shard.cache_paths {
                                load_object_shard_cache_hit(
                                    cache_paths,
                                    &shard.member_fingerprints,
                                    &object_build_fingerprint,
                                )?
                            } else {
                                let index = shard.member_indices[0];
                                let unit = &rewritten_files[index];
                                let cache_paths = object_cache_paths_by_file
                                    .get(&unit.file)
                                    .expect("object cache paths should exist for rewritten unit");
                                load_object_cache_hit(
                                    cache_paths,
                                    &unit.semantic_fingerprint,
                                    &unit.rewrite_context_fingerprint,
                                    &object_build_fingerprint,
                                )?
                            };
                            Ok((shard.member_indices.clone(), cached_obj))
                        })
                        .collect(),
                )
            })?;

        let mut object_cache_hits: usize = 0;
        let mut cache_misses: Vec<ObjectCodegenShard> = Vec::new();
        for (shard, result) in object_shards.iter().zip(cache_probe_results) {
            let (member_indices, cached_obj) = result?;
            if let Some(cached_obj) = cached_obj {
                for index in member_indices {
                    object_paths[index] = Some(cached_obj.clone());
                    object_cache_hits += 1;
                }
            } else {
                cache_misses.push(shard.clone());
            }
        }
        build_timings.record_counts(
            "object cache probe",
            &[
                ("candidates", object_candidate_count),
                ("reused", object_cache_hits),
                (
                    "missed",
                    object_candidate_count.saturating_sub(object_cache_hits),
                ),
                ("shard_size", object_shard_size),
                ("shard_threshold", object_shard_threshold),
                ("shards", object_shards.len()),
                ("missed_shards", cache_misses.len()),
            ],
        );
        build_timings.record_duration_ns(
            "object cache meta/load",
            OBJECT_CACHE_META_TIMING_TOTALS
                .load_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_counts(
            "object cache meta/read",
            &[
                (
                    "loads",
                    OBJECT_CACHE_META_TIMING_TOTALS
                        .load_count
                        .load(Ordering::Relaxed),
                ),
                (
                    "bytes_read",
                    OBJECT_CACHE_META_TIMING_TOTALS
                        .bytes_read
                        .load(Ordering::Relaxed) as usize,
                ),
            ],
        );

        let object_codegen_timing_totals = Arc::new(ObjectCodegenTimingTotals::default());
        let declaration_closure_timing_totals = Arc::new(DeclarationClosureTimingTotals::default());
        let object_emit_timing_totals = Arc::new(ObjectEmitTimingTotals::default());
        crate::codegen::core::reset_codegen_phase_timings();
        crate::codegen::util::reset_object_write_timings();
        let compiled_results: Vec<(usize, PathBuf)> =
            build_timings.measure("object codegen", || {
                cache_misses
                    .par_iter()
                    .map(|shard| {
                        let obj_path = if let Some(cache_paths) = &shard.cache_paths {
                            cache_paths.object_path.clone()
                        } else {
                            let unit = &rewritten_files[shard.member_indices[0]];
                            object_cache_paths_by_file
                                .get(&unit.file)
                                .expect("object cache paths should exist for rewritten unit")
                                .object_path
                                .clone()
                        };
                        let declaration_closure_started_at = Instant::now();
                        let mut batch_active_symbols = HashSet::new();
                        let mut batch_declaration_symbols = HashSet::new();
                        let mut batch_closure_files = HashSet::new();
                        let global_maps = GlobalSymbolMaps {
                            function_map: &global_function_map,
                            function_file_map: &global_function_file_map,
                            class_map: &global_class_map,
                            class_file_map: &global_class_file_map,
                            interface_map: &global_interface_map,
                            interface_file_map: &global_interface_file_map,
                            enum_map: &global_enum_map,
                            enum_file_map: &global_enum_file_map,
                            module_map: &global_module_map,
                            module_file_map: &global_module_file_map,
                        };
                        for index in &shard.member_indices {
                            let unit = &rewritten_files[*index];
                            let declaration_closure = declaration_symbols_for_unit(
                                &unit.file,
                                &unit.active_symbols,
                                &precomputed_dependency_closures,
                                &codegen_reference_metadata,
                                &entry_namespace,
                                &project_symbol_lookup,
                                &global_maps,
                                Some(declaration_closure_timing_totals.as_ref()),
                            );
                            batch_active_symbols.extend(unit.active_symbols.iter().cloned());
                            batch_declaration_symbols.extend(declaration_closure.symbols);
                            batch_closure_files.extend(declaration_closure.files);
                        }
                        object_codegen_timing_totals
                            .declaration_closure_ns
                            .fetch_add(
                                elapsed_nanos_u64(declaration_closure_started_at),
                                Ordering::Relaxed,
                            );
                        let codegen_program_started_at = Instant::now();
                        let codegen_program = if shard
                            .member_indices
                            .iter()
                            .any(|index| rewritten_files[*index].has_specialization_demand)
                        {
                            combined_program_for_files(&rewritten_files)
                        } else {
                            codegen_program_for_units(
                                &rewritten_files,
                                &rewritten_file_indices,
                                &shard.member_files,
                                Some(&batch_closure_files),
                            )
                        };
                        object_codegen_timing_totals.codegen_program_ns.fetch_add(
                            elapsed_nanos_u64(codegen_program_started_at),
                            Ordering::Relaxed,
                        );
                        let closure_body_symbols_started_at = Instant::now();
                        let mut codegen_active_symbols = batch_active_symbols;
                        for index in &shard.member_indices {
                            let unit = &rewritten_files[*index];
                            codegen_active_symbols.extend(closure_body_symbols_for_unit(
                                &unit.file,
                                &batch_declaration_symbols,
                                &global_function_file_map,
                                &global_class_file_map,
                                &global_module_file_map,
                            ));
                        }
                        object_codegen_timing_totals
                            .closure_body_symbols_ns
                            .fetch_add(
                                elapsed_nanos_u64(closure_body_symbols_started_at),
                                Ordering::Relaxed,
                            );
                        let llvm_emit_started_at = Instant::now();
                        compile_program_ast_to_object_filtered(
                            &codegen_program,
                            &shard.member_files[0],
                            &obj_path,
                            &link,
                            &codegen_active_symbols,
                            &batch_declaration_symbols,
                            Some(object_emit_timing_totals.as_ref()),
                        )?;
                        object_codegen_timing_totals
                            .llvm_emit_ns
                            .fetch_add(elapsed_nanos_u64(llvm_emit_started_at), Ordering::Relaxed);
                        let cache_save_started_at = Instant::now();
                        if let Some(cache_paths) = &shard.cache_paths {
                            save_object_shard_cache_meta(
                                cache_paths,
                                &shard.member_fingerprints,
                                &object_build_fingerprint,
                            )?;
                        } else {
                            let unit = &rewritten_files[shard.member_indices[0]];
                            let cache_paths = object_cache_paths_by_file
                                .get(&unit.file)
                                .expect("object cache paths should exist for rewritten unit");
                            save_object_cache_meta(
                                cache_paths,
                                &unit.semantic_fingerprint,
                                &unit.rewrite_context_fingerprint,
                                &object_build_fingerprint,
                            )?;
                        }
                        object_codegen_timing_totals
                            .cache_save_ns
                            .fetch_add(elapsed_nanos_u64(cache_save_started_at), Ordering::Relaxed);
                        Ok::<Vec<(usize, PathBuf)>, String>(
                            shard
                                .member_indices
                                .iter()
                                .map(|index| (*index, obj_path.clone()))
                                .collect(),
                        )
                    })
                    .collect::<Result<Vec<_>, String>>()
                    .map(|results| results.into_iter().flatten().collect())
            })?;
        build_timings.record_counts(
            "object codegen",
            &[
                ("candidates", object_candidate_count),
                ("reused", object_cache_hits),
                (
                    "rebuilt",
                    object_candidate_count.saturating_sub(object_cache_hits),
                ),
                ("shard_size", object_shard_size),
                ("shard_threshold", object_shard_threshold),
                ("rebuilt_shards", cache_misses.len()),
            ],
        );
        build_timings.record_duration_ns(
            "object codegen/declaration closure",
            object_codegen_timing_totals
                .declaration_closure_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/program projection",
            object_codegen_timing_totals
                .codegen_program_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/closure body symbols",
            object_codegen_timing_totals
                .closure_body_symbols_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/llvm emit",
            object_codegen_timing_totals
                .llvm_emit_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/cache save",
            object_codegen_timing_totals
                .cache_save_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object cache meta/save",
            OBJECT_CACHE_META_TIMING_TOTALS
                .save_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_counts(
            "object cache meta/write",
            &[
                (
                    "saves",
                    OBJECT_CACHE_META_TIMING_TOTALS
                        .save_count
                        .load(Ordering::Relaxed),
                ),
                (
                    "bytes_written",
                    OBJECT_CACHE_META_TIMING_TOTALS
                        .bytes_written
                        .load(Ordering::Relaxed) as usize,
                ),
            ],
        );
        build_timings.record_duration_ns(
            "object codegen/emit context create",
            object_emit_timing_totals
                .context_create_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/emit codegen new",
            object_emit_timing_totals
                .codegen_new_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/emit compile filtered",
            object_emit_timing_totals
                .compile_filtered_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/emit object dir setup",
            object_emit_timing_totals
                .object_dir_setup_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/emit write object",
            object_emit_timing_totals
                .write_object_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_counts(
            "object codegen/emit details",
            &[
                (
                    "active_symbols",
                    object_emit_timing_totals
                        .active_symbol_count
                        .load(Ordering::Relaxed),
                ),
                (
                    "decl_symbols",
                    object_emit_timing_totals
                        .declaration_symbol_count
                        .load(Ordering::Relaxed),
                ),
                (
                    "program_decls",
                    object_emit_timing_totals
                        .program_decl_count
                        .load(Ordering::Relaxed),
                ),
            ],
        );
        let codegen_phase_timings = crate::codegen::core::snapshot_codegen_phase_timings();
        build_timings.record_duration_ns(
            "object codegen/core generic class check",
            codegen_phase_timings.program_has_generic_classes_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core specialize classes 1",
            codegen_phase_timings.specialize_generic_classes_initial_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core explicit generic check",
            codegen_phase_timings.program_has_explicit_generic_calls_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core specialize explicit",
            codegen_phase_timings.specialize_explicit_generic_calls_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core specialize classes 2",
            codegen_phase_timings.specialize_generic_classes_final_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core collect spec symbols",
            codegen_phase_timings.collect_generated_spec_symbols_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core specialize active",
            codegen_phase_timings.specialized_active_symbols_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core import aliases",
            codegen_phase_timings.import_alias_collection_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core enum pass",
            codegen_phase_timings.enum_declare_pass_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core enum filters",
            codegen_phase_timings.enum_declare_decl_filter_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core enum work",
            codegen_phase_timings.enum_declare_work_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core decl pass",
            codegen_phase_timings.decl_pass_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core decl filters",
            codegen_phase_timings.decl_pass_decl_filter_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core decl class work",
            codegen_phase_timings.decl_pass_class_work_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core decl fn work",
            codegen_phase_timings.decl_pass_function_work_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core decl module work",
            codegen_phase_timings.decl_pass_module_work_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core body pass",
            codegen_phase_timings.body_pass_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core body filters",
            codegen_phase_timings.body_pass_decl_filter_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core body fn work",
            codegen_phase_timings.body_pass_function_work_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core body class work",
            codegen_phase_timings.body_pass_class_work_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/core body module work",
            codegen_phase_timings.body_pass_module_work_ns,
        );
        build_timings.record_counts(
            "object codegen/core counts",
            &[
                ("decls", codegen_phase_timings.total_decls_count),
                ("import_aliases", codegen_phase_timings.import_alias_count),
                ("active_symbols", codegen_phase_timings.active_symbols_count),
                (
                    "decl_symbols",
                    codegen_phase_timings.declaration_symbols_count,
                ),
                (
                    "spec_owners",
                    codegen_phase_timings.generated_spec_owners_count,
                ),
                ("declared_enums", codegen_phase_timings.declared_enum_count),
                (
                    "declared_classes",
                    codegen_phase_timings.declared_class_count,
                ),
                (
                    "declared_functions",
                    codegen_phase_timings.declared_function_count,
                ),
                (
                    "declared_modules",
                    codegen_phase_timings.declared_module_count,
                ),
                (
                    "compiled_functions",
                    codegen_phase_timings.compiled_function_count,
                ),
                (
                    "compiled_classes",
                    codegen_phase_timings.compiled_class_count,
                ),
                (
                    "compiled_modules",
                    codegen_phase_timings.compiled_module_count,
                ),
            ],
        );
        build_timings.record_duration_ns(
            "object codegen/decl closure seed",
            declaration_closure_timing_totals
                .closure_seed_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/decl metadata lookup",
            declaration_closure_timing_totals
                .metadata_lookup_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/decl wildcard imports",
            declaration_closure_timing_totals
                .wildcard_imports_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/decl exact imports",
            declaration_closure_timing_totals
                .exact_imports_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/decl qualified refs",
            declaration_closure_timing_totals
                .qualified_refs_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_duration_ns(
            "object codegen/decl reference symbols",
            declaration_closure_timing_totals
                .reference_symbols_ns
                .load(Ordering::Relaxed),
        );
        build_timings.record_counts(
            "object codegen/decl details",
            &[
                (
                    "visited_files",
                    declaration_closure_timing_totals
                        .visited_file_count
                        .load(Ordering::Relaxed),
                ),
                (
                    "wildcard_imports",
                    declaration_closure_timing_totals
                        .wildcard_import_count
                        .load(Ordering::Relaxed),
                ),
                (
                    "exact_imports",
                    declaration_closure_timing_totals
                        .exact_import_count
                        .load(Ordering::Relaxed),
                ),
                (
                    "qualified_refs",
                    declaration_closure_timing_totals
                        .qualified_ref_count
                        .load(Ordering::Relaxed),
                ),
                (
                    "reference_symbols",
                    declaration_closure_timing_totals
                        .reference_symbol_count
                        .load(Ordering::Relaxed),
                ),
            ],
        );
        let object_write_timings = crate::codegen::util::snapshot_object_write_timings();
        build_timings.record_duration_ns(
            "object codegen/write object total",
            object_write_timings.emit_object_bytes_ns + object_write_timings.filesystem_write_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object with TM",
            object_write_timings.with_target_machine_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object TM config",
            object_write_timings.target_machine_config_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object TM init",
            object_write_timings.ensure_targets_initialized_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object triple",
            object_write_timings.target_triple_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object host cpu",
            object_write_timings.host_cpu_query_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object opt resolve",
            object_write_timings.opt_level_resolve_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object target from triple",
            object_write_timings.target_from_triple_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object TM create",
            object_write_timings.target_machine_create_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object target setup",
            object_write_timings.target_machine_setup_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object set triple",
            object_write_timings.module_set_triple_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object set layout",
            object_write_timings.module_set_data_layout_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object memory buffer",
            object_write_timings.write_to_memory_buffer_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object to vec",
            object_write_timings.memory_buffer_to_vec_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object direct file emit",
            object_write_timings.direct_write_to_file_ns,
        );
        build_timings.record_duration_ns(
            "object codegen/write object fs write",
            object_write_timings.filesystem_write_ns,
        );
        build_timings.record_counts(
            "object codegen/write object counts",
            &[
                (
                    "tm_cache_hits",
                    object_write_timings.target_machine_cache_hit_count,
                ),
                (
                    "tm_cache_misses",
                    object_write_timings.target_machine_cache_miss_count,
                ),
                ("emit_calls", object_write_timings.emit_object_call_count),
                ("write_calls", object_write_timings.write_object_call_count),
            ],
        );

        for (index, obj_path) in compiled_results {
            object_paths[index] = Some(obj_path);
        }

        if object_cache_hits > 0 {
            println!(
                "{} Reused object cache for {}/{} files",
                "→".cyan(),
                object_cache_hits,
                object_candidate_count
            );
        }

        let link_inputs = build_timings.measure_step("link input assembly", || {
            dedupe_link_inputs(object_paths.into_iter().flatten().collect())
        });
        let current_link_manifest =
            build_timings.measure_step("link manifest prep", || LinkManifestCache {
                schema: LINK_MANIFEST_CACHE_SCHEMA.to_string(),
                compiler_version: env!("CARGO_PKG_VERSION").to_string(),
                link_fingerprint: compute_link_fingerprint(&output_path, &link_inputs, &link),
                link_inputs: link_inputs.clone(),
            });

        if should_skip_final_link(
            previous_link_manifest.as_ref(),
            &current_link_manifest,
            &output_path,
            cache_misses.len(),
        ) {
            println!(
                "{} Reused final link output from manifest cache",
                "→".cyan()
            );
            build_timings.record_counts(
                "final link",
                &[("objects", link_inputs.len()), ("linked", 0), ("reused", 1)],
            );
        } else {
            build_timings.measure("final link", || {
                link_objects(&link_inputs, &output_path, &link)
            })?;
            build_timings.record_counts(
                "final link",
                &[("objects", link_inputs.len()), ("linked", 1), ("reused", 0)],
            );
            build_timings.measure("link manifest save", || {
                save_link_manifest_cache(&project_root, &current_link_manifest)
            })?;
        }
    }

    println!(
        "{} {} -> {}",
        "Built".green().bold(),
        config.name.cyan(),
        output_path.display()
    );

    if !check_only {
        build_timings.measure("build cache save", || {
            save_cached_fingerprint(&project_root, &fingerprint)?;
            save_semantic_cached_fingerprint(&project_root, &semantic_fingerprint)?;
            save_dependency_graph_cache(&project_root, &current_dependency_graph_cache)
        })?;
    }

    build_timings.print();

    Ok(())
}

fn compile_program_ast(
    program: &Program,
    source_path: &Path,
    output_path: &Path,
    emit_llvm: bool,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    ensure_output_parent_dir(output_path)?;

    let context = Context::create();
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let mut codegen = Codegen::new(&context, module_name);
    codegen
        .compile(program)
        .map_err(|e| format!("{}: Codegen error: {}", "error".red().bold(), e.message))?;

    if emit_llvm {
        let ll_path = output_path.with_extension("ll");
        codegen.write_ir(&ll_path)?;
        println!("{} {}", "LLVM IR".green().bold(), ll_path.display());
    } else {
        let ir_path = output_path.with_extension("ll");
        codegen.write_ir(&ir_path)?;
        compile_ir(&ir_path, output_path, link)?;
        let _ = fs::remove_file(&ir_path);
    }

    Ok(())
}

fn compile_program_ast_to_object_filtered(
    program: &Program,
    source_path: &Path,
    object_path: &Path,
    link: &LinkConfig<'_>,
    active_symbols: &HashSet<String>,
    declaration_symbols: &HashSet<String>,
    timings: Option<&ObjectEmitTimingTotals>,
) -> Result<(), String> {
    let context_started_at = Instant::now();
    let context = Context::create();
    if let Some(timings) = timings {
        timings
            .context_create_ns
            .fetch_add(elapsed_nanos_u64(context_started_at), Ordering::Relaxed);
        timings
            .active_symbol_count
            .fetch_add(active_symbols.len(), Ordering::Relaxed);
        timings
            .declaration_symbol_count
            .fetch_add(declaration_symbols.len(), Ordering::Relaxed);
        timings
            .program_decl_count
            .fetch_add(program.declarations.len(), Ordering::Relaxed);
    }
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let codegen_new_started_at = Instant::now();
    let mut codegen = Codegen::new(&context, module_name);
    if let Some(timings) = timings {
        timings
            .codegen_new_ns
            .fetch_add(elapsed_nanos_u64(codegen_new_started_at), Ordering::Relaxed);
    }
    let compile_started_at = Instant::now();
    codegen
        .compile_filtered_with_decl_symbols(program, active_symbols, declaration_symbols)
        .map_err(|e| format!("{}: Codegen error: {}", "error".red().bold(), e.message))?;
    if let Some(timings) = timings {
        timings
            .compile_filtered_ns
            .fetch_add(elapsed_nanos_u64(compile_started_at), Ordering::Relaxed);
    }

    if let Some(parent) = object_path.parent() {
        let object_dir_setup_started_at = Instant::now();
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create object cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
        if let Some(timings) = timings {
            timings.object_dir_setup_ns.fetch_add(
                elapsed_nanos_u64(object_dir_setup_started_at),
                Ordering::Relaxed,
            );
        }
    }
    let write_started_at = Instant::now();
    codegen
        .write_object_with_config(object_path, link.opt_level, link.target, &link.output_kind)
        .map_err(|e| {
            format!(
                "{}: Failed to emit object for '{}': {}",
                "error".red().bold(),
                source_path.display(),
                e
            )
        })?;
    if let Some(timings) = timings {
        timings
            .write_object_ns
            .fetch_add(elapsed_nanos_u64(write_started_at), Ordering::Relaxed);
    }
    Ok(())
}

/// Build and run the current project
fn run_project(
    args: &[String],
    release: bool,
    do_check: bool,
    show_timings: bool,
) -> Result<(), String> {
    let cwd = current_dir_checked()?;
    let project_root = find_project_root(&cwd)
        .ok_or_else(|| format!("{}: No apex.toml found", "error".red().bold()))?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;
    config.validate(&project_root)?;
    validate_opt_level(Some(&config.opt_level))?;
    ensure_project_is_runnable(&config.output_kind)?;

    build_project(release, false, do_check, false, show_timings)?;

    let output_path = project_root.join(&config.output);

    println!("{} {}", "Running".cyan().bold(), output_path.display());
    println!();

    let status = Command::new(&output_path)
        .args(args)
        .status()
        .map_err(|e| format!("{}: Failed to run: {}", "error".red().bold(), e))?;

    if !status.success() {
        return Err(format!(
            "{}: process exited with code {}",
            "error".red().bold(),
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

fn ensure_project_is_runnable(output_kind: &OutputKind) -> Result<(), String> {
    if *output_kind == OutputKind::Bin {
        return Ok(());
    }

    Err(format!(
        "{}: `apex run` requires `output_kind = \"bin\"`, found {:?}. Use `apex build` for library targets.",
        "error".red().bold(),
        output_kind
    ))
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

    compile_file(file, Some(&output), false, do_check, None, None)?;

    println!("{} {}", "Running".cyan().bold(), output.display());
    println!();

    let status = Command::new(&output)
        .args(args)
        .status()
        .map_err(|e| format!("{}: Failed to run: {}", "error".red().bold(), e))?;

    let _ = fs::remove_file(&output);

    if !status.success() {
        return Err(format!(
            "{}: process exited with code {}",
            "error".red().bold(),
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

fn check_command(file: Option<&Path>, show_timings: bool) -> Result<(), String> {
    if file.is_none() {
        if let Some(cwd_project_root) = find_project_root(&current_dir_checked()?) {
            let _ = cwd_project_root;
            return build_project(false, false, true, true, show_timings);
        }
    }
    check_file(file)
}

/// Compile a single file (legacy mode)
fn compile_file(
    file: &Path,
    output: Option<&Path>,
    emit_llvm: bool,
    do_check: bool,
    opt_level: Option<&str>,
    target: Option<&str>,
) -> Result<(), String> {
    validate_source_file_path(file)?;

    // Check if we're in a project
    if let Some(project_root) = find_project_root(&current_dir_checked()?) {
        if file.starts_with(&project_root) {
            println!(
                "{}",
                "note: file is inside a project; prefer `apex build` for project builds".yellow()
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

    ensure_output_parent_dir(&output_path)?;

    if !do_check {
        let filename = file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("input.apex");
        let tokens = lexer::tokenize(&source)
            .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse_program()
            .map_err(|e| format_parse_error(&e, &source, filename))?;
        validate_entry_main_signature(&program, &source, filename)?;
    }

    compile_source(
        &source,
        file,
        &output_path,
        emit_llvm,
        do_check,
        opt_level,
        target,
    )?;

    println!("{} {}", "Wrote".green().bold(), output_path.display());
    Ok(())
}

/// Compile source code
fn compile_source(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    emit_llvm: bool,
    do_check: bool,
    opt_level: Option<&str>,
    target: Option<&str>,
) -> Result<(), String> {
    validate_opt_level(opt_level)?;

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
        let function_namespaces = import_check::extract_function_namespaces(&program, &namespace);
        let known_namespace_paths =
            import_check::extract_known_namespace_paths(&program, &namespace);
        let mut import_checker = ImportChecker::new(
            Arc::new(function_namespaces),
            Arc::new(known_namespace_paths),
            namespace,
            imports,
            stdlib_registry(),
        );
        if let Err(errors) = import_checker.check_program(&program) {
            let mut rendered = format!("{} Import errors:\n", "error".red().bold());
            for err in errors {
                rendered.push_str(&format!("  → {}\n", err.format()));
            }
            return Err(format!("Import check failed\n{rendered}"));
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

        let link = LinkConfig {
            opt_level,
            target,
            output_kind: OutputKind::Bin,
            link_search: &[],
            link_libs: &[],
            link_args: &[],
        };
        compile_ir(&ir_path, output_path, &link)?;
        let _ = fs::remove_file(&ir_path);
    }

    Ok(())
}

fn check_file(file: Option<&Path>) -> Result<(), String> {
    let file_path = if let Some(f) = file {
        validate_source_file_path(f)?;
        f.to_path_buf()
    } else {
        // Use project entry point
        let project_root = find_project_root(&current_dir_checked()?).ok_or_else(|| {
            format!(
                "{}: No apex.toml found. Specify a file or run from a project directory.",
                "error".red().bold()
            )
        })?;

        let config_path = project_root.join("apex.toml");
        let config = ProjectConfig::load(&config_path)?;
        config.validate(&project_root)?;
        for source_file in config.get_source_files(&project_root) {
            validate_source_file_path(&source_file)?;
        }
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
    let function_namespaces = import_check::extract_function_namespaces(&program, &namespace);
    let known_namespace_paths = import_check::extract_known_namespace_paths(&program, &namespace);
    let mut import_checker = ImportChecker::new(
        Arc::new(function_namespaces),
        Arc::new(known_namespace_paths),
        namespace,
        imports,
        stdlib_registry(),
    );
    if let Err(errors) = import_checker.check_program(&program) {
        let mut rendered = format!("{} Import errors:\n", "error".red().bold());
        for err in errors {
            rendered.push_str(&format!("  → {}\n", err.format()));
        }
        return Err(format!("Import check failed\n{rendered}"));
    }

    let mut type_checker = TypeChecker::new(source.clone());
    if let Err(errors) = type_checker.check(&program) {
        return Err(typeck::format_errors(&errors, &source, filename));
    }

    let mut borrow_checker = BorrowChecker::new();
    if let Err(errors) = borrow_checker.check(&program) {
        return Err(borrowck::format_borrow_errors(&errors, &source, filename));
    }

    println!("{} {}", "Check passed".green().bold(), file_path.display());
    Ok(())
}

/// Extract namespace from a program
fn extract_namespace(program: &ast::Program) -> String {
    program
        .package
        .clone()
        .unwrap_or_else(|| "global".to_string())
}

fn validate_entry_main_signature(
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
    let project_root = find_project_root(&current_dir_checked()?).ok_or_else(|| {
        format!(
            "{}: No apex.toml found in current directory or parents.",
            "error".red().bold()
        )
    })?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;
    config.validate(&project_root)?;
    validate_opt_level(Some(&config.opt_level))?;
    for file in config.get_source_files(&project_root) {
        validate_source_file_path(&file)?;
    }

    println!("{}", "Project".cyan().bold());
    println!("  {}: {}", "name".dimmed(), config.name);
    println!("  {}: {}", "version".dimmed(), config.version);
    println!("  {}: {}", "entry".dimmed(), config.entry);
    println!("  {}: {}", "output".dimmed(), config.output);
    println!("  {}: {:?}", "output kind".dimmed(), config.output_kind);
    println!("  {}: {}", "opt level".dimmed(), config.opt_level);
    println!(
        "  {}: {}",
        "target".dimmed(),
        config.target.as_deref().unwrap_or("native/default")
    );
    println!("  {}: {}", "root".dimmed(), project_root.display());

    println!("\n{}", "source files".dimmed());
    for file in &config.files {
        println!("  - {}", file);
    }

    if !config.dependencies.is_empty() {
        println!("\n{}", "dependencies".dimmed());
        for (name, version) in &config.dependencies {
            println!("  - {} = {}", name, version);
        }
    }

    if !config.link_search.is_empty() {
        println!("\n{}", "link search".dimmed());
        for path in &config.link_search {
            println!("  - {}", path);
        }
    }

    if !config.link_libs.is_empty() {
        println!("\n{}", "link libraries".dimmed());
        for lib in &config.link_libs {
            println!("  - {}", lib);
        }
    }

    Ok(())
}

fn format_targets(path: Option<&Path>, check_only: bool) -> Result<(), String> {
    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    let targets = if let Some(path) = path {
        collect_apex_files(path)?
    } else if let Some(project_root) = find_project_root(&current_dir) {
        let config = ProjectConfig::load(&project_root.join("apex.toml"))?;
        config.validate(&project_root)?;
        config.get_source_files(&project_root)
    } else {
        collect_apex_files(&current_dir)?
    };

    if targets.is_empty() {
        return Err("No .apex files found to format".to_string());
    }

    let mut changed = Vec::new();
    for file in targets {
        let source = fs::read_to_string(&file)
            .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;
        let formatted = formatter::format_source(&source)
            .map_err(|e| format!("{} in '{}': {}", "error".red().bold(), file.display(), e))?;

        if source != formatted {
            if check_only {
                changed.push(file);
            } else {
                fs::write(&file, formatted).map_err(|e| {
                    format!(
                        "{}: Failed to write '{}': {}",
                        "error".red().bold(),
                        file.display(),
                        e
                    )
                })?;
                changed.push(file);
            }
        }
    }

    if check_only {
        if changed.is_empty() {
            println!("{}", "Format check passed".green());
            return Ok(());
        }

        eprintln!("{} format check failed for:", "error".red().bold());
        for file in changed {
            eprintln!("  - {}", file.display());
        }
        return Err("format check failed".to_string());
    }

    if changed.is_empty() {
        println!("{}", "No formatting changes".green());
    } else {
        println!("{} {} file(s)", "Formatted".green().bold(), changed.len());
        for file in changed {
            println!("  - {}", file.display());
        }
    }

    Ok(())
}

fn resolve_default_file(path: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = path {
        validate_source_file_path(path)?;
        return Ok(path.to_path_buf());
    }

    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    if let Some(project_root) = find_project_root(&current_dir) {
        let config = ProjectConfig::load(&project_root.join("apex.toml"))?;
        config.validate(&project_root)?;
        for source_file in config.get_source_files(&project_root) {
            validate_source_file_path(&source_file)?;
        }
        return Ok(config.get_entry_path(&project_root));
    }

    Err("No file specified and no apex.toml found in the current directory".to_string())
}

fn validate_source_file_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Path '{}' does not exist", path.display()));
    }
    if !path.is_file() {
        return Err(format!("Path '{}' is not a file", path.display()));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("apex") {
        return Err(format!("Path '{}' is not an .apex file", path.display()));
    }

    let metadata = fs::symlink_metadata(path).map_err(|e| {
        format!(
            "{}: Failed to inspect path '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    let parent_dir = path.parent().unwrap_or(Path::new("."));
    let normalized_parent = if parent_dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent_dir
    };
    let canonical_parent = normalized_parent.canonicalize().map_err(|e| {
        format!(
            "{}: Failed to resolve parent directory for '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    if metadata.file_type().is_symlink() {
        let canonical_path = path.canonicalize().map_err(|e| {
            format!(
                "{}: Failed to resolve path '{}': {}",
                "error".red().bold(),
                path.display(),
                e
            )
        })?;
        if !canonical_path.starts_with(&canonical_parent) {
            return Err(format!(
                "Path '{}' resolves outside the requested directory tree",
                path.display()
            ));
        }
    }

    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.as_os_str().is_empty() {
            break;
        }
        let ancestor_metadata = fs::symlink_metadata(dir).map_err(|e| {
            format!(
                "{}: Failed to inspect path ancestor '{}': {}",
                "error".red().bold(),
                dir.display(),
                e
            )
        })?;
        if ancestor_metadata.file_type().is_symlink() {
            return Err(format!(
                "Path '{}' must not traverse symlinked directories",
                path.display()
            ));
        }
        current = dir.parent();
    }

    Ok(())
}

fn lint_target(path: Option<&Path>) -> Result<(), String> {
    let file = resolve_default_file(path)?;
    let source = fs::read_to_string(&file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;
    let result = lint::lint_source(&source, false)
        .map_err(|e| format!("{} in '{}': {}", "error".red().bold(), file.display(), e))?;

    if result.findings.is_empty() {
        println!("{} {}", "Lint clean".green().bold(), file.display());
        return Ok(());
    }

    eprintln!(
        "{} lint findings in {}:",
        "warning".yellow().bold(),
        file.display()
    );
    for finding in result.findings {
        eprintln!("  {}", finding.format());
    }
    Err("lint failed".to_string())
}

fn fix_target(path: Option<&Path>) -> Result<(), String> {
    let file = resolve_default_file(path)?;
    let source = fs::read_to_string(&file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;
    let result = lint::lint_source(&source, true)
        .map_err(|e| format!("{} in '{}': {}", "error".red().bold(), file.display(), e))?;
    let fixed_source = result.fixed_source.unwrap_or(source.clone());

    let formatted_source = formatter::format_source(&fixed_source)
        .map_err(|e| format!("{} in '{}': {}", "error".red().bold(), file.display(), e))?;

    if source == formatted_source {
        println!("{} {}", "No safe fixes".green().bold(), file.display());
        return Ok(());
    }

    fs::write(&file, formatted_source)
        .map_err(|e| format!("{}: Failed to write file: {}", "error".red().bold(), e))?;
    println!("{} {}", "Updated".green().bold(), file.display());
    Ok(())
}

fn collect_apex_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        validate_source_file_path(path)?;
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.is_dir() {
        return Err(format!("Path '{}' does not exist", path.display()));
    }

    let mut files = Vec::new();
    collect_apex_files_recursive(path, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_apex_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read directory entry in '{}': {}",
                dir.display(),
                e
            )
        })?;
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect directory entry '{}': {}",
                entry.path().display(),
                e
            )
        })?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_apex_files_recursive(&path, files)?;
        } else if file_type.is_file()
            && path.extension().and_then(|ext| ext.to_str()) == Some("apex")
        {
            files.push(path);
        }
    }
    Ok(())
}

fn bench_target(file: Option<&Path>, iterations: usize) -> Result<(), String> {
    if iterations == 0 {
        return Err("Iterations must be greater than zero.".to_string());
    }

    let mut samples_ms = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        if let Some(file) = file {
            run_single_file(file, &[], false, true)?;
        } else {
            run_project(&[], false, true, false)?;
        }
        samples_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let min = samples_ms
        .iter()
        .copied()
        .fold(f64::INFINITY, |acc, value| acc.min(value));
    let max = samples_ms
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(value));
    let mean = samples_ms.iter().sum::<f64>() / samples_ms.len() as f64;

    println!("{}", "Benchmark".cyan().bold());
    println!("  runs: {}", samples_ms.len());
    println!("  min:  {:.3} ms", min);
    println!("  mean: {:.3} ms", mean);
    println!("  max:  {:.3} ms", max);
    Ok(())
}

fn profile_target(file: Option<&Path>) -> Result<(), String> {
    let start = Instant::now();
    if let Some(file) = file {
        run_single_file(file, &[], false, true)?;
    } else {
        run_project(&[], false, true, false)?;
    }
    let elapsed = start.elapsed();

    println!("{}", "Profile".cyan().bold());
    println!("  wall time: {:.3} ms", elapsed.as_secs_f64() * 1000.0);
    println!("  metrics: wall time only");
    Ok(())
}

/// Show tokens (debug)
fn lex_file(file: &Path) -> Result<(), String> {
    validate_source_file_path(file)?;

    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    println!("{}", "Tokens".cyan().bold());
    for (token, span) in tokens {
        println!("  {:?} @ {}..{}", token, span.start, span.end);
    }

    Ok(())
}

/// Show AST (debug)
fn parse_file(file: &Path) -> Result<(), String> {
    validate_source_file_path(file)?;

    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    let filename = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("input.apex");
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, &source, filename))?;

    println!("{}", "AST".cyan().bold());
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
    output.push_str(&format!("{}: {}\n", "error".red().bold(), error.message));
    output.push_str(&format!(
        "  {} {}:{}:{}\n",
        "-->".blue().bold(),
        filename,
        line_num,
        col
    ));
    output.push_str(&format!("   {}\n", "|".blue().bold()));

    if line_num <= lines.len() {
        output.push_str(&format!(
            "{} {}\n",
            format!("{:3} |", line_num).blue().bold(),
            lines[line_num - 1]
        ));

        let underline_start = col.saturating_sub(1);
        let underline_len = (error.span.end - error.span.start).max(1);
        let carets = "^".repeat(underline_len.min(50));
        output.push_str(&format!(
            "   {} {}{}\n",
            "|".blue().bold(),
            " ".repeat(underline_start),
            carets.red().bold()
        ));
    }

    output
}

/// Run tests for a file or project
fn run_tests(
    test_path: Option<&Path>,
    list_only: bool,
    filter: Option<&str>,
) -> Result<(), String> {
    // Determine which file(s) to test
    let test_files = if let Some(path) = test_path {
        if path.is_file() {
            validate_source_file_path(path)?;
            vec![path.to_path_buf()]
        } else {
            // Look for test files in directory
            find_test_files(path)?
        }
    } else {
        // Default: use project files when inside a project, otherwise scan cwd.
        let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
        default_test_files(&current_dir)?
    };

    if test_files.is_empty() {
        println!("{}", "No test files found".yellow());
        println!("Create files with functions marked `@Test`.");
        return Ok(());
    }

    // Process each test file
    let mut all_tests_found = false;

    for test_file in &test_files {
        let source = fs::read_to_string(test_file)
            .map_err(|e| format!("Failed to read test file: {}", e))?;

        // Parse the test file
        let tokens = lexer::tokenize(&source).map_err(|e| format!("Lexer error: {}", e))?;
        let filename = test_file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("input.apex");
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse_program()
            .map_err(|e| format_parse_error(&e, &source, filename))?;

        // Discover tests
        let discovery = discover_tests(&program);

        if discovery.total_tests == 0 {
            continue;
        }

        all_tests_found = true;

        // Apply filter if specified
        let filtered_suites: Vec<_> = if let Some(pattern) = filter {
            discovery
                .suites
                .into_iter()
                .map(|mut suite| {
                    suite.tests.retain(|t| t.name.contains(pattern));
                    suite
                })
                .filter(|s| !s.tests.is_empty())
                .collect()
        } else {
            discovery.suites
        };

        if filtered_suites.is_empty() {
            println!(
                "{}: no tests matched '{}'",
                test_file.display(),
                filter.unwrap_or("")
            );
            continue;
        }

        let filtered_total_tests: usize = filtered_suites.iter().map(|s| s.tests.len()).sum();
        let filtered_ignored_tests: usize = filtered_suites
            .iter()
            .map(|s| s.tests.iter().filter(|t| t.ignored).count())
            .sum();

        let filtered_discovery = test_runner::TestDiscovery {
            suites: filtered_suites,
            total_tests: filtered_total_tests,
            ignored_tests: filtered_ignored_tests,
        };

        // List or run tests
        if list_only {
            println!("\n{}", test_file.display().to_string().cyan().bold());
            print_discovery(&filtered_discovery);
        } else {
            // Generate and run test runner - include original source + test runner main
            let runner_code = generate_test_runner_with_source(&filtered_discovery, &source);
            if let Some(project_root) = test_file.parent().and_then(find_project_root) {
                let config_path = project_root.join("apex.toml");
                let config = ProjectConfig::load(&config_path)?;
                config.validate(&project_root)?;
                let (temp_dir, exe_path) = create_project_test_runner_workspace(
                    &project_root,
                    &config,
                    test_file,
                    &runner_code,
                )?;
                let previous_dir = current_dir_checked()?;
                std::env::set_current_dir(&temp_dir)
                    .map_err(|e| format!("Failed to enter test runner workspace: {}", e))?;
                let build_result = build_project(false, false, true, false, false);
                let _ = std::env::set_current_dir(&previous_dir);
                let result = build_result.and_then(|_| run_test_executable(&exe_path));
                let _ = fs::remove_dir_all(&temp_dir);
                result?;
            } else {
                // Create temporary file for test runner without clobbering user files next to the test.
                let (temp_dir, runner_path, exe_path) = create_test_runner_workspace(test_file)?;
                fs::write(&runner_path, &runner_code)
                    .map_err(|e| format!("Failed to write test runner: {}", e))?;

                // Compile and run the test runner
                let result = compile_and_run_test(&runner_path, &exe_path);

                // Clean up temporary files
                let _ = fs::remove_dir_all(&temp_dir);

                result?;
            }
        }
    }

    if !all_tests_found {
        println!("{}", "No tests discovered".yellow());
        println!("Mark functions with `@Test`:");
        println!("  {} function myTest(): None {{ ... }}", "@Test".cyan());
    }

    Ok(())
}

fn default_test_files(current_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if let Some(project_root) = find_project_root(current_dir) {
        let config_path = project_root.join("apex.toml");
        let config = ProjectConfig::load(&config_path)?;
        config.validate(&project_root)?;

        let mut files = config.get_source_files(&project_root);
        files.sort();
        return Ok(files);
    }

    find_test_files(current_dir)
}

fn create_test_runner_workspace(test_file: &Path) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let unique = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to create unique test runner path: {}", e))?
        .as_nanos();
    let stem = test_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("apex_test");
    let temp_dir = std::env::temp_dir().join(format!(
        "apex-test-runner-{}-{}-{}",
        stem,
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create test runner workspace: {}", e))?;

    let runner_path = temp_dir.join("runner.apex");
    let exe_path = temp_dir.join("runner.exe");
    Ok((temp_dir, runner_path, exe_path))
}

fn create_project_test_runner_workspace(
    project_root: &Path,
    config: &ProjectConfig,
    test_file: &Path,
    runner_code: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let unique = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to create unique test runner project path: {}", e))?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "apex-project-test-runner-{}-{}",
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create test runner project workspace: {}", e))?;

    let normalized_test_file = if test_file.is_absolute() {
        test_file.to_path_buf()
    } else {
        current_dir_checked()?.join(test_file)
    };

    let test_rel = normalized_test_file
        .strip_prefix(project_root)
        .map_err(|_| {
            format!(
                "Test file '{}' is outside project root '{}'",
                normalized_test_file.display(),
                project_root.display()
            )
        })?;
    let test_rel_string = test_rel.to_string_lossy().replace('\\', "/");

    for source_file in config.get_source_files(project_root) {
        let rel = source_file.strip_prefix(project_root).map_err(|_| {
            format!(
                "Project source '{}' is outside project root '{}'",
                source_file.display(),
                project_root.display()
            )
        })?;
        let dest = temp_dir.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create runner source directory: {}", e))?;
        }
        if source_file == normalized_test_file {
            fs::write(&dest, runner_code)
                .map_err(|e| format!("Failed to write generated project test runner: {}", e))?;
        } else {
            fs::copy(&source_file, &dest)
                .map_err(|e| format!("Failed to copy project source into test workspace: {}", e))?;
        }
    }

    let runner_dest = temp_dir.join(test_rel);
    if !runner_dest.exists() {
        if let Some(parent) = runner_dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create runner destination directory: {}", e))?;
        }
        fs::write(&runner_dest, runner_code)
            .map_err(|e| format!("Failed to write generated runner source: {}", e))?;
    }

    let mut temp_config = config.clone();
    temp_config.entry = test_rel_string.clone();
    if config.entry != test_rel_string {
        temp_config.files.retain(|file| file != &config.entry);
    }
    if !temp_config
        .files
        .iter()
        .any(|file| file == &test_rel_string)
    {
        temp_config.files.push(test_rel_string);
        temp_config.files.sort();
        temp_config.files.dedup();
    }
    temp_config.output = "runner".to_string();
    temp_config
        .save(&temp_dir.join("apex.toml"))
        .map_err(|e| format!("Failed to write test runner project config: {}", e))?;

    Ok((temp_dir.clone(), temp_dir.join("runner")))
}

/// Find test files in a directory
fn find_test_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Err(format!("Path '{}' does not exist", dir.display()));
    }
    if !dir.is_dir() {
        return Err(format!("Path '{}' is not a directory", dir.display()));
    }

    let mut test_files = Vec::new();
    find_test_files_recursive(dir, &mut test_files)?;
    test_files.sort();
    Ok(test_files)
}

fn is_test_like_file(path: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("apex") {
        return false;
    }

    let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let lowercase = file_name.to_ascii_lowercase();
    lowercase.contains("test") || lowercase.contains("spec")
}

fn find_test_files_recursive(dir: &Path, test_files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read directory entry in '{}': {}",
                dir.display(),
                e
            )
        })?;
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect directory entry '{}' : {}",
                entry.path().display(),
                e
            )
        })?;
        let path = entry.path();

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            find_test_files_recursive(&path, test_files)?;
            continue;
        }

        if file_type.is_file() && is_test_like_file(&path) {
            test_files.push(path);
        }
    }
    Ok(())
}

/// Compile and run a test file
fn compile_and_run_test(source_path: &Path, exe_path: &Path) -> Result<(), String> {
    // Compile the test runner
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read test runner: {}", e))?;

    compile_source(&source, source_path, exe_path, false, true, None, None)?;

    run_test_executable(exe_path)
}

fn run_test_executable(exe_path: &Path) -> Result<(), String> {
    use std::process::Command;

    println!("\n{}", "Running tests".cyan().bold());
    println!();

    let output = Command::new(exe_path)
        .output()
        .map_err(|e| format!("Failed to run test runner: {}", e))?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        return Err("test run failed".to_string());
    }

    Ok(())
}

fn bindgen_header(header: &Path, output: Option<&Path>) -> Result<(), String> {
    let count = bindgen::generate_bindings(header, output)?;
    if let Some(out) = output {
        println!(
            "{} {} binding(s) -> {}",
            "Generated".green().bold(),
            count,
            out.display()
        );
    } else {
        eprintln!("{} {} binding(s)", "Generated".green().bold(), count);
    }
    Ok(())
}
