//! Arden

mod ast;
mod bindgen;
mod borrowck;
mod cache;
mod cli;
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
#[path = "project/rewrite/mod.rs"]
mod project_rewrite;
mod shared;
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
use std::collections::HashSet;
use std::env;
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
use crate::cli::output::*;
pub(crate) use crate::cli::paths::collect_arden_files;
use crate::cli::paths::{
    current_dir_checked, format_target_label, validate_source_file_path, with_process_current_dir,
};
use crate::cli::test_discovery::find_test_files;
use crate::codegen::Codegen;
#[cfg(test)]
use crate::dependency::*;
use crate::diagnostics::*;
use crate::import_check::ImportChecker;
use crate::linker::*;
use crate::parser::Parser;
use crate::project::pipeline::{
    build_rewrite_fingerprint_context, compute_project_change_impact, evaluate_semantic_cache_gate,
    run_dependency_graph_phase, run_entry_validation_phase, run_full_codegen_phase,
    run_import_check_phase, run_object_pipeline, run_parse_index_phase, run_rewrite_phase,
    run_rewrite_prep_phase, run_semantic_phase, validate_symbol_collisions, DependencyGraphInputs,
    DependencyGraphOutputs, FullCodegenInputs, FullCodegenRoute, ImportCheckInputs,
    ObjectPipelineInputs, ParseIndexOutputs, RewriteContextInputs, RewritePhaseInputs,
    RewritePrepInputs, RewritePreparation, SemanticGateInputs, SemanticPhaseInputs,
};
use crate::project::{find_project_root, OutputKind, ProjectConfig};
use crate::specialization::*;
use crate::stdlib::stdlib_registry;
use crate::symbol_lookup::GlobalSymbolMaps;
use crate::test_runner::{discover_tests, generate_test_runner_with_source, print_discovery};
use crate::typeck::TypeChecker;

#[derive(Clone)]
pub(crate) struct ObjectCodegenShard {
    pub(crate) member_indices: Vec<usize>,
    pub(crate) member_files: Vec<PathBuf>,
    pub(crate) member_fingerprints: Vec<ObjectShardMemberFingerprint>,
    pub(crate) cache_paths: Option<ObjectShardCachePaths>,
}

#[derive(ClapParser)]
#[command(name = "arden")]
#[command(bin_name = "arden")]
#[command(author = "TheRemyyy")]
#[command(version = "1.3.7")]
#[command(about = "Arden and project CLI")]
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
    /// Compile a single Arden file
    Compile {
        /// Input file
        file: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Optimization level: 0, 1, 2, 3, s, z, or fast
        #[arg(long)]
        opt_level: Option<String>,
        /// Target triple passed through to the native backend/linker
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
    /// Format Arden source
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
    /// Generate Arden extern bindings from a C header
    Bindgen {
        /// Input C header file
        header: PathBuf,
        /// Output Arden file (defaults to stdout)
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
    configure_cli_colors();
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
    let program = parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, source, filename))?;
    let namespace = program
        .package
        .clone()
        .unwrap_or_else(|| "global".to_string());
    let imports = extract_imports(&program);
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

pub(crate) fn parse_project_unit(
    project_root: &Path,
    file: &Path,
) -> Result<ParsedProjectUnit, String> {
    let filename = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown.arden");
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
                } else if let Some(parts) = crate::ast::flatten_field_chain(&callee.node) {
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
                if let Some(parts) = crate::ast::flatten_field_chain(expr) {
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
            Expr::If {
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
        let cache = cached_entry.as_ref().ok_or_else(|| {
            format!(
                "{}: parse cache reported a hit for '{}' but no cache entry was available",
                cli_error("error"),
                file.display()
            )
        })?;
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
            source_fingerprint: source_fingerprint_for_cache.clone().ok_or_else(|| {
                format!(
                    "{}: parsed file '{}' is missing a source fingerprint",
                    cli_error("error"),
                    file.display()
                )
            })?,
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
    let config_path = project_path.join("arden.toml");
    config.save(&config_path)?;

    // Create main.arden
    let main_content = format!(
        r#"import std.io.*;

function main(): None {{
    println("hello from {}");
    return None;
}}
"#,
        name
    );

    let main_path = project_path.join("src").join("main.arden");
    fs::write(&main_path, main_content).map_err(|e| {
        format!(
            "{}: Failed to create main.arden: {}",
            "error".red().bold(),
            e
        )
    })?;

    // Create README.md
    let readme_content = format!(
        r#"# {}

Project scaffold created with `arden new`.

This directory is intentionally small, but it already contains the pieces Arden uses for project-mode development.

## Project Structure

```
.
├── arden.toml      # Project configuration
├── src/
│   └── main.arden  # Entry point
└── README.md       # Project notes and workflow guide
```

## What Each File Does

- `arden.toml` declares the project name, entry file, output path, optimization level, and explicit source list.
- `src/main.arden` is the default entrypoint used by `arden run` and `arden build`.
- `README.md` is where you should document the local workflow, architecture notes, and useful commands as the project grows.

## Common Workflow

- `arden build` - Build the project
- `arden run` - Build and run the project
- `arden check` - Parse and type-check the project
- `arden test` - Run test files in the project
- `arden fmt` - Format project sources
- `arden info` - Print the resolved project configuration

A good first pass after generating the project is:

```bash
arden info
arden run
arden check
```

## How Project Mode Works

When you run Arden inside this directory without passing a file path, the compiler reads `arden.toml` and uses it as the source of truth for:

- project name and version
- entrypoint
- native output name
- output kind
- which source files belong to the build

That makes project builds explicit and reproducible. As the project grows, add new files to `files = [...]` in `arden.toml`.

## Configuration

Edit `arden.toml` to customize your project:

```toml
name = "{}"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
output = "{}"
opt_level = "3"
output_kind = "bin"
```

## Next Steps

- add more `.arden` files under `src/`
- list them in `arden.toml`
- use `arden test` for test files and `arden fmt` before commits
- read the upstream documentation in the main Arden repository for language and stdlib details
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

    println!("{} {}", cli_success("Created project"), cli_accent(name));
    println!(
        "  {} {}",
        cli_tertiary("Root"),
        cli_path(&project_path.canonicalize().unwrap_or(project_path))
    );
    println!("\n{}", cli_tertiary("Next"));
    println!(
        "  {} {}",
        cli_tertiary("cd"),
        cli_soft(format_cli_path(path.unwrap_or(Path::new(name))))
    );
    println!("  {}", cli_soft(cli_new_run_hint()));

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
    release: bool,
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
        .ok_or_else(|| format!("{}: No arden.toml found. Are you in a project directory?\nRun `arden new <name>` to create a new project.",
            "error".red().bold()))?;

    let config_path = project_root.join("arden.toml");
    let mut config = ProjectConfig::load(&config_path)?;
    if release {
        config.opt_level = "3".to_string();
    }

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

    let output_path = resolve_project_output_path(&project_root, &config);
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
                print_cli_cache(format!(
                    "Reused whole-project build cache for {}",
                    config.name
                ));
                print_cli_artifact_result(
                    "Built",
                    &config.name,
                    &output_path,
                    build_timings.started_at.elapsed(),
                );
                build_timings.print();
                return Ok(());
            }
        }
    }

    println!(
        "{} {} {}",
        cli_accent("Building"),
        cli_accent(&config.name),
        cli_soft(format!("v{}", config.version))
    );
    print_cli_step(format!("Parsing {} source file(s)", files.len()));

    let ParseIndexOutputs {
        parsed_files,
        global_function_map,
        global_function_file_map,
        global_class_map,
        global_class_file_map,
        global_interface_map,
        global_interface_file_map,
        global_enum_map,
        global_enum_file_map,
        global_module_map,
        global_module_file_map,
        namespace_class_map,
        namespace_interface_map,
        namespace_enum_map,
        namespace_module_map,
        function_collisions,
        class_collisions,
        interface_collisions,
        enum_collisions,
        module_collisions,
        project_symbol_lookup,
        total_module_names,
    } = run_parse_index_phase(&mut build_timings, &project_root, &files)?;
    let entry_path = config.get_entry_path(&project_root);
    let DependencyGraphOutputs {
        previous_dependency_graph,
        file_dependency_graph,
        reverse_file_dependency_graph,
        current_dependency_graph_cache,
    } = run_dependency_graph_phase(
        &mut build_timings,
        DependencyGraphInputs {
            project_root: &project_root,
            entry_path: &entry_path,
            parsed_files: &parsed_files,
            total_module_names,
            global_maps: GlobalSymbolMaps {
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
            },
            project_symbol_lookup: &project_symbol_lookup,
        },
    )?;

    let previous_semantic_summary = load_semantic_summary_cache(&project_root)?;
    let previous_typecheck_summary = load_typecheck_summary_cache(&project_root)?;
    let impact = compute_project_change_impact(
        previous_dependency_graph.as_ref(),
        &parsed_files,
        &reverse_file_dependency_graph,
    );
    let (semantic_fingerprint, semantic_cache_hit) = evaluate_semantic_cache_gate(
        &mut build_timings,
        SemanticGateInputs {
            config: &config,
            parsed_files: &parsed_files,
            emit_llvm,
            do_check,
            check_only,
            project_root: &project_root,
            output_path: &output_path,
            impact: &impact,
        },
    )?;
    if semantic_cache_hit {
        print_cli_cache(format!("Reused semantic build cache for {}", config.name));
        save_cached_fingerprint(&project_root, &fingerprint)?;
        print_cli_artifact_result(
            "Built",
            &config.name,
            &output_path,
            build_timings.started_at.elapsed(),
        );
        build_timings.print();
        return Ok(());
    }

    validate_symbol_collisions(
        function_collisions,
        class_collisions,
        enum_collisions,
        interface_collisions,
        module_collisions,
    )?;

    run_entry_validation_phase(do_check, &entry_path, &parsed_files)?;

    let RewritePreparation {
        namespace_functions,
        entry_namespace,
        namespace_api_fingerprints,
        file_api_fingerprints,
        safe_rewrite_cache_files,
    } = run_rewrite_prep_phase(
        &mut build_timings,
        RewritePrepInputs {
            parsed_files: &parsed_files,
            entry_path: &entry_path,
            previous_dependency_graph: previous_dependency_graph.as_ref(),
            body_only_changed: &impact.body_only_changed,
            api_changed: &impact.api_changed,
            dependent_api_impact: &impact.dependent_api_impact,
        },
    );
    let rewrite_fingerprint_ctx = build_rewrite_fingerprint_context(RewriteContextInputs {
        namespace_functions: &namespace_functions,
        namespace_classes: &namespace_class_map,
        namespace_modules: &namespace_module_map,
        namespace_api_fingerprints: &namespace_api_fingerprints,
        file_api_fingerprints: &file_api_fingerprints,
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
        project_symbol_lookup: &project_symbol_lookup,
    });

    // Phase 2: Check imports for each file
    if do_check {
        print_cli_step("Checking imports");
        run_import_check_phase(
            &mut build_timings,
            ImportCheckInputs {
                project_root: &project_root,
                parsed_files: &parsed_files,
                global_function_map: &global_function_map,
                entry_namespace: &entry_namespace,
                rewrite_fingerprint_ctx: &rewrite_fingerprint_ctx,
            },
        )?;
    }

    // Phase 3: Build combined AST with deterministic namespace mangling.
    let rewritten_files = run_rewrite_phase(
        &mut build_timings,
        RewritePhaseInputs {
            project_root: &project_root,
            parsed_files: &parsed_files,
            safe_rewrite_cache_files: &safe_rewrite_cache_files,
            entry_namespace: &entry_namespace,
            rewrite_fingerprint_ctx: &rewrite_fingerprint_ctx,
            namespace_functions: &namespace_functions,
            global_function_map: &global_function_map,
            namespace_class_map: &namespace_class_map,
            global_class_map: &global_class_map,
            namespace_interface_map: &namespace_interface_map,
            global_interface_map: &global_interface_map,
            namespace_enum_map: &namespace_enum_map,
            global_enum_map: &global_enum_map,
            namespace_module_map: &namespace_module_map,
            global_module_map: &global_module_map,
        },
    )?;

    if do_check {
        run_semantic_phase(
            &mut build_timings,
            SemanticPhaseInputs {
                project_root: &project_root,
                parsed_files: &parsed_files,
                rewritten_files: &rewritten_files,
                file_dependency_graph: &file_dependency_graph,
                previous_dependency_graph_exists: previous_dependency_graph.is_some(),
                previous_semantic_summary: previous_semantic_summary.as_ref(),
                previous_typecheck_summary: previous_typecheck_summary.as_ref(),
                body_only_changed: &impact.body_only_changed,
                api_changed: &impact.api_changed,
                dependent_api_impact: &impact.dependent_api_impact,
            },
        )?;
    }

    if check_only {
        println!(
            "{} {} {}",
            cli_success("Check passed"),
            cli_accent(&config.name),
            cli_soft(format!(
                "({})",
                cli_elapsed(build_timings.started_at.elapsed())
            ))
        );
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
    match run_full_codegen_phase(
        &mut build_timings,
        FullCodegenInputs {
            rewritten_files: &rewritten_files,
            entry_path: &entry_path,
            output_path: &output_path,
            emit_llvm,
            link: &link,
        },
    )? {
        FullCodegenRoute::EmitLlvmCompleted => {}
        FullCodegenRoute::FullProgramCompleted => {
            save_cached_fingerprint(&project_root, &fingerprint)?;
            print_cli_artifact_result(
                "Built",
                &config.name,
                &output_path,
                build_timings.started_at.elapsed(),
            );
            build_timings.print();
            return Ok(());
        }
        FullCodegenRoute::ObjectsRequired => {
            run_object_pipeline(
                &mut build_timings,
                ObjectPipelineInputs {
                    project_root: &project_root,
                    output_path: &output_path,
                    parsed_files: &parsed_files,
                    rewritten_files: &rewritten_files,
                    file_dependency_graph: &file_dependency_graph,
                    link: &link,
                    entry_namespace: &entry_namespace,
                    project_symbol_lookup: &project_symbol_lookup,
                    global_maps: GlobalSymbolMaps {
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
                    },
                },
            )?;
        }
    }

    if !check_only {
        build_timings.measure("build cache save", || {
            save_cached_fingerprint(&project_root, &fingerprint)?;
            save_semantic_cached_fingerprint(&project_root, &semantic_fingerprint)?;
            save_dependency_graph_cache(&project_root, &current_dependency_graph_cache)
        })?;
    }

    print_cli_artifact_result(
        "Built",
        &config.name,
        &output_path,
        build_timings.started_at.elapsed(),
    );

    build_timings.print();

    Ok(())
}

pub(crate) fn compile_program_ast(
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
        println!("{} {}", cli_success("Wrote LLVM IR"), cli_path(&ll_path));
    } else {
        let object_path = output_path.with_extension(format!("arden-tmp.{}", object_ext()));
        codegen
            .write_object_with_config(&object_path, link.opt_level, link.target, &link.output_kind)
            .map_err(|e| {
                format!(
                    "{}: Failed to emit object for '{}': {}",
                    "error".red().bold(),
                    source_path.display(),
                    e
                )
            })?;
        let link_result = link_objects(std::slice::from_ref(&object_path), output_path, link);
        let _ = fs::remove_file(&object_path);
        link_result?;
    }

    Ok(())
}

pub(crate) fn compile_program_ast_to_object_filtered(
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
        .ok_or_else(|| format!("{}: No arden.toml found", "error".red().bold()))?;

    let config_path = project_root.join("arden.toml");
    let config = ProjectConfig::load(&config_path)?;
    config.validate(&project_root)?;
    validate_opt_level(Some(&config.opt_level))?;
    ensure_project_is_runnable(&config.output_kind)?;

    build_project(release, false, do_check, false, show_timings)?;

    let output_path = resolve_project_output_path(&project_root, &config);

    println!("{} {}", cli_accent("Running"), cli_path(&output_path));
    println!();

    run_binary(&output_path, args)
}

fn ensure_project_is_runnable(output_kind: &OutputKind) -> Result<(), String> {
    if *output_kind == OutputKind::Bin {
        return Ok(());
    }

    Err(format!(
        "{}: `arden run` requires `output_kind = \"bin\"`, found {:?}. Use `arden build` for library targets.",
        "error".red().bold(),
        output_kind
    ))
}

fn resolve_project_output_path(project_root: &Path, config: &ProjectConfig) -> PathBuf {
    let output_path = project_root.join(&config.output);
    #[cfg(windows)]
    {
        if config.output_kind == OutputKind::Bin && output_path.extension().is_none() {
            return output_path.with_extension("exe");
        }
    }
    output_path
}

/// Run a single file (legacy mode)
fn run_single_file(
    file: &Path,
    args: &[String],
    release: bool,
    do_check: bool,
) -> Result<(), String> {
    #[cfg(windows)]
    let output = file.with_extension("run.exe");
    #[cfg(not(windows))]
    let output = file.with_extension("run");

    compile_file(
        file,
        Some(&output),
        false,
        do_check,
        release.then_some("3"),
        None,
    )?;

    println!("{} {}", cli_accent("Running"), cli_path(&output));
    println!();

    let result = run_binary(&output, args);
    let _ = fs::remove_file(&output);
    result
}

fn check_command(file: Option<&Path>, show_timings: bool) -> Result<(), String> {
    if file.is_none() && find_project_root(&current_dir_checked()?).is_some() {
        return build_project(false, false, true, true, show_timings);
    }
    check_file(file)
}

fn parse_program_from_source(source: &str, filename: &str) -> Result<Program, String> {
    let tokens = lexer::tokenize(source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;
    let mut parser = Parser::new(tokens);
    parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, source, filename))
}

fn run_single_file_semantic_checks(
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

/// Compile a single file (legacy mode)
fn compile_file(
    file: &Path,
    output: Option<&Path>,
    emit_llvm: bool,
    do_check: bool,
    opt_level: Option<&str>,
    target: Option<&str>,
) -> Result<(), String> {
    let compile_started = Instant::now();
    validate_source_file_path(file)?;

    // Check if we're in a project
    if let Some(project_root) = find_project_root(&current_dir_checked()?) {
        if file.starts_with(&project_root) {
            println!(
                "{}",
                "note: file is inside a project; prefer `arden build` for project builds".yellow()
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
            .unwrap_or("input.arden");
        let program = parse_program_from_source(&source, filename)?;
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

    println!(
        "{} {} {}",
        cli_success("Wrote"),
        cli_path(&output_path),
        cli_soft(format!("({})", cli_elapsed(compile_started.elapsed())))
    );
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
        .unwrap_or("input.arden");

    // Tokenize
    let program = parse_program_from_source(source, filename)?;

    // Type check
    if do_check {
        run_single_file_semantic_checks(source, filename, &program)?;
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
        println!("{} {}", cli_success("Wrote LLVM IR"), cli_path(&ll_path));
    } else {
        let link = LinkConfig {
            opt_level,
            target,
            output_kind: OutputKind::Bin,
            link_search: &[],
            link_libs: &[],
            link_args: &[],
        };
        let object_path = output_path.with_extension(format!("arden-tmp.{}", object_ext()));
        codegen
            .write_object_with_config(&object_path, opt_level, target, &OutputKind::Bin)
            .map_err(|e| {
                format!(
                    "{}: Failed to emit object for '{}': {}",
                    "error".red().bold(),
                    source_path.display(),
                    e
                )
            })?;
        let link_result = link_objects(std::slice::from_ref(&object_path), output_path, &link);
        let _ = fs::remove_file(&object_path);
        link_result?;
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
                "{}: No arden.toml found. Specify a file or run from a project directory.",
                "error".red().bold()
            )
        })?;

        let config_path = project_root.join("arden.toml");
        let config = ProjectConfig::load(&config_path)?;
        config.validate(&project_root)?;
        for source_file in config.get_source_files(&project_root) {
            validate_source_file_path(&source_file)?;
        }
        config.get_entry_path(&project_root)
    };

    println!("{} {}", cli_accent("Checking"), cli_path(&file_path));

    let source = fs::read_to_string(&file_path)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let filename = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("input.arden");

    let program = parse_program_from_source(&source, filename)?;
    run_single_file_semantic_checks(&source, filename, &program)?;

    println!("{} {}", cli_success("Check passed"), cli_path(&file_path));
    Ok(())
}

/// Extract namespace from a program
fn extract_namespace(program: &ast::Program) -> String {
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

/// Extract imports from a program
fn extract_imports(program: &ast::Program) -> Vec<ast::ImportDecl> {
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

fn extract_top_level_imports(program: &ast::Program) -> Vec<ast::ImportDecl> {
    program
        .declarations
        .iter()
        .filter_map(|decl| match &decl.node {
            ast::Decl::Import(import) => Some(import.clone()),
            _ => None,
        })
        .collect()
}

/// Show project information
fn show_project_info() -> Result<(), String> {
    let project_root = find_project_root(&current_dir_checked()?).ok_or_else(|| {
        format!(
            "{}: No arden.toml found in current directory or parents.",
            "error".red().bold()
        )
    })?;

    let config_path = project_root.join("arden.toml");
    let config = ProjectConfig::load(&config_path)?;
    config.validate(&project_root)?;
    validate_opt_level(Some(&config.opt_level))?;
    for file in config.get_source_files(&project_root) {
        validate_source_file_path(&file)?;
    }

    println!("{}", cli_accent("Project"));
    println!("  {}: {}", cli_tertiary("name"), cli_soft(&config.name));
    println!(
        "  {}: {}",
        cli_tertiary("version"),
        cli_soft(&config.version)
    );
    println!("  {}: {}", cli_tertiary("entry"), cli_soft(&config.entry));
    println!("  {}: {}", cli_tertiary("output"), cli_soft(&config.output));
    println!(
        "  {}: {}",
        cli_tertiary("output kind"),
        cli_soft(format!("{:?}", config.output_kind))
    );
    println!(
        "  {}: {}",
        cli_tertiary("opt level"),
        cli_soft(&config.opt_level)
    );
    println!(
        "  {}: {}",
        cli_tertiary("target"),
        cli_soft(config.target.as_deref().unwrap_or("native/default"))
    );
    println!("  {}: {}", cli_tertiary("root"), cli_path(&project_root));

    println!("\n{}", cli_tertiary("source files"));
    for file in &config.files {
        println!("  - {}", cli_soft(file));
    }

    if !config.dependencies.is_empty() {
        println!("\n{}", cli_tertiary("dependencies"));
        for (name, version) in &config.dependencies {
            println!("  - {} = {}", cli_soft(name), cli_soft(version));
        }
    }

    if !config.link_search.is_empty() {
        println!("\n{}", cli_tertiary("link search"));
        for path in &config.link_search {
            println!("  - {}", cli_soft(path));
        }
    }

    if !config.link_libs.is_empty() {
        println!("\n{}", cli_tertiary("link libraries"));
        for lib in &config.link_libs {
            println!("  - {}", cli_soft(lib));
        }
    }

    Ok(())
}

fn format_targets(path: Option<&Path>, check_only: bool) -> Result<(), String> {
    let current_dir = current_dir_checked()?;
    let target_label = format_target_label(path, &current_dir);
    let targets = if let Some(path) = path {
        collect_arden_files(path)?
    } else if let Some(project_root) = find_project_root(&current_dir) {
        let config = ProjectConfig::load(&project_root.join("arden.toml"))?;
        config.validate(&project_root)?;
        config.get_source_files(&project_root)
    } else {
        collect_arden_files(&current_dir)?
    };

    if targets.is_empty() {
        return Err("No .arden files found to format".to_string());
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
            println!(
                "{} {}",
                cli_success("Fmt check passed"),
                cli_soft(&target_label)
            );
            return Ok(());
        }

        eprintln!("{} format check failed for:", cli_error("error"));
        for file in changed {
            eprintln!("  - {}", cli_path(&file));
        }
        return Err("format check failed".to_string());
    }

    if changed.is_empty() {
        println!("{} {}", cli_success("Fmt clean"), cli_soft(&target_label));
    } else {
        println!("{} {} file(s)", cli_success("Formatted"), changed.len());
        for file in changed {
            println!("  - {}", cli_path(&file));
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
        let config = ProjectConfig::load(&project_root.join("arden.toml"))?;
        config.validate(&project_root)?;
        for source_file in config.get_source_files(&project_root) {
            validate_source_file_path(&source_file)?;
        }
        return Ok(config.get_entry_path(&project_root));
    }

    Err("No file specified and no arden.toml found in the current directory".to_string())
}

fn lint_target(path: Option<&Path>) -> Result<(), String> {
    let file = resolve_default_file(path)?;
    let source = fs::read_to_string(&file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;
    let result = lint::lint_source(&source, false)
        .map_err(|e| format!("{} in '{}': {}", "error".red().bold(), file.display(), e))?;

    if result.findings.is_empty() {
        println!("{} {}", cli_success("Lint clean"), cli_path(&file));
        return Ok(());
    }

    eprintln!(
        "{} lint findings in {}:",
        cli_warning("warning"),
        cli_path(&file)
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
        println!("{} {}", cli_success("Fix clean"), cli_path(&file));
        return Ok(());
    }

    fs::write(&file, formatted_source)
        .map_err(|e| format!("{}: Failed to write file: {}", "error".red().bold(), e))?;
    println!("{} {}", cli_success("Updated"), cli_path(&file));
    Ok(())
}

fn run_binary(exe_path: &Path, args: &[String]) -> Result<(), String> {
    let status = Command::new(exe_path)
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

fn prepare_bench_binary(
    file: Option<&Path>,
    release: bool,
) -> Result<(PathBuf, Option<PathBuf>, Vec<String>), String> {
    if let Some(file) = file {
        #[cfg(windows)]
        let output = file.with_extension("bench.exe");
        #[cfg(not(windows))]
        let output = file.with_extension("bench");
        compile_file(
            file,
            Some(&output),
            false,
            true,
            release.then_some("3"),
            None,
        )?;
        return Ok((output.clone(), Some(output), Vec::new()));
    }

    let cwd = current_dir_checked()?;
    let project_root = find_project_root(&cwd)
        .ok_or_else(|| format!("{}: No arden.toml found", "error".red().bold()))?;
    let config_path = project_root.join("arden.toml");
    let config = ProjectConfig::load(&config_path)?;
    config.validate(&project_root)?;
    ensure_project_is_runnable(&config.output_kind)?;
    build_project(release, false, true, false, false)?;
    Ok((project_root.join(&config.output), None, Vec::new()))
}

fn bench_target(file: Option<&Path>, iterations: usize) -> Result<(), String> {
    if iterations == 0 {
        return Err("Iterations must be greater than zero.".to_string());
    }

    let (exe_path, cleanup_path, args) = prepare_bench_binary(file, false)?;
    let mut samples_ms = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        run_binary(&exe_path, &args)?;
        samples_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    if let Some(cleanup_path) = cleanup_path {
        let _ = fs::remove_file(cleanup_path);
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

    println!("{}", cli_accent("Benchmark"));
    println!(
        "  {} {}",
        cli_tertiary("runs"),
        cli_soft(samples_ms.len().to_string())
    );
    println!(
        "  {} {}",
        cli_tertiary("min"),
        cli_soft(format!("{:.6} s", min / 1000.0))
    );
    println!(
        "  {} {}",
        cli_tertiary("mean"),
        cli_soft(format!("{:.6} s", mean / 1000.0))
    );
    println!(
        "  {} {}",
        cli_tertiary("max"),
        cli_soft(format!("{:.6} s", max / 1000.0))
    );
    Ok(())
}

fn profile_target(file: Option<&Path>) -> Result<(), String> {
    let build_started = Instant::now();
    let (exe_path, cleanup_path, args) = prepare_bench_binary(file, false)?;
    let build_elapsed = build_started.elapsed();
    let run_started = Instant::now();
    run_binary(&exe_path, &args)?;
    let run_elapsed = run_started.elapsed();
    if let Some(cleanup_path) = cleanup_path {
        let _ = fs::remove_file(cleanup_path);
    }

    println!("{}", cli_accent("Timing profile"));
    println!(
        "  {} {}",
        cli_tertiary("build"),
        cli_soft(cli_elapsed(build_elapsed))
    );
    println!(
        "  {} {}",
        cli_tertiary("run"),
        cli_soft(cli_elapsed(run_elapsed))
    );
    println!(
        "  {} {}",
        cli_tertiary("total"),
        cli_soft(cli_elapsed(build_elapsed + run_elapsed))
    );
    Ok(())
}

/// Show tokens (debug)
fn lex_file(file: &Path) -> Result<(), String> {
    validate_source_file_path(file)?;

    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    println!("{}", cli_accent("Tokens"));
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
        .unwrap_or("input.arden");
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, &source, filename))?;

    println!("{}", cli_accent("AST"));
    println!("{:#?}", program);

    Ok(())
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
        let current_dir = current_dir_checked()?;
        default_test_files(&current_dir)?
    };

    if test_files.is_empty() {
        println!("{}", cli_warning("No test files found"));
        println!(
            "{}",
            cli_soft("Create files with functions marked `@Test`.")
        );
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
            .unwrap_or("input.arden");
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
        let filtered_out_tests = discovery
            .total_tests
            .saturating_sub(filtered_discovery.total_tests);

        // List or run tests
        if list_only {
            println!("\n{}", cli_accent(test_file.display().to_string()));
            print_discovery(&filtered_discovery);
        } else {
            // Generate and run test runner - include original source + test runner main
            let runner_code = generate_test_runner_with_source(&filtered_discovery, &source);
            if let Some(project_root) = test_file.parent().and_then(find_project_root) {
                let config_path = project_root.join("arden.toml");
                let config = ProjectConfig::load(&config_path)?;
                config.validate(&project_root)?;
                let (temp_dir, exe_path) = create_project_test_runner_workspace(
                    &project_root,
                    &config,
                    test_file,
                    &runner_code,
                )?;
                let build_result = with_process_current_dir(&temp_dir, || {
                    build_project(false, false, true, false, false)
                });
                let result =
                    build_result.and_then(|_| run_test_executable(&exe_path, filtered_out_tests));
                let _ = fs::remove_dir_all(&temp_dir);
                result?;
            } else {
                // Create temporary file for test runner without clobbering user files next to the test.
                let (temp_dir, runner_path, exe_path) = create_test_runner_workspace(test_file)?;
                fs::write(&runner_path, &runner_code)
                    .map_err(|e| format!("Failed to write test runner: {}", e))?;

                // Compile and run the test runner
                let result = compile_and_run_test(&runner_path, &exe_path, filtered_out_tests);

                // Clean up temporary files
                let _ = fs::remove_dir_all(&temp_dir);

                result?;
            }
        }
    }

    if !all_tests_found {
        println!("{}", cli_warning("No tests discovered"));
        println!("{}", cli_soft("Mark functions with `@Test`:"));
        println!(
            "  {} function myTest(): None {{ ... }}",
            cli_tertiary("@Test")
        );
    }

    Ok(())
}

fn default_test_files(current_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if let Some(project_root) = find_project_root(current_dir) {
        let config_path = project_root.join("arden.toml");
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
        .unwrap_or("arden_test");
    let temp_dir = std::env::temp_dir().join(format!(
        "arden-test-runner-{}-{}-{}",
        stem,
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create test runner workspace: {}", e))?;

    let runner_path = temp_dir.join("runner.arden");
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
        "arden-project-test-runner-{}-{}",
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
        .save(&temp_dir.join("arden.toml"))
        .map_err(|e| format!("Failed to write test runner project config: {}", e))?;

    Ok((temp_dir.clone(), temp_dir.join("runner")))
}

/// Compile and run a test file
fn compile_and_run_test(
    source_path: &Path,
    exe_path: &Path,
    filtered_out: usize,
) -> Result<(), String> {
    // Compile the test runner
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read test runner: {}", e))?;

    compile_source(&source, source_path, exe_path, false, true, None, None)?;

    run_test_executable(exe_path, filtered_out)
}

fn run_test_executable(exe_path: &Path, filtered_out: usize) -> Result<(), String> {
    use std::process::Command;

    let started_at = Instant::now();
    println!();

    let working_dir = exe_path
        .parent()
        .filter(|dir| dir.is_dir())
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir);
    let output = Command::new(exe_path)
        .current_dir(working_dir)
        .output()
        .map_err(|e| format!("Failed to run test runner: {}", e))?;

    let report = print_test_runner_output(
        &String::from_utf8_lossy(&output.stdout),
        output.status.success(),
    );
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    let elapsed = started_at.elapsed();

    println!();
    println!("{}", cli_accent("test result:"));
    println!(" {}", cli_soft(format!("{} passed;", report.passed)));
    println!(" {}", cli_soft(format!("{} failed;", report.failed)));
    println!(" {}", cli_soft(format!("{} ignored;", report.ignored)));
    println!(" {}", cli_soft("0 measured;"));
    println!(" {}", cli_soft(format!("{} filtered out;", filtered_out)));
    println!(
        " {}",
        cli_soft(format!("finished in {}", cli_elapsed(elapsed)))
    );

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
            cli_success("Generated"),
            count,
            cli_path(out)
        );
    } else {
        eprintln!(
            "{} {}",
            cli_success("Generated"),
            cli_soft(format!("{count} binding(s)"))
        );
    }
    Ok(())
}
