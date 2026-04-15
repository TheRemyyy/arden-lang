//! Arden
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

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
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::ast::{Block, Decl, Expr, ImportDecl, Pattern, Program, Stmt};
use crate::cache::*;
#[cfg(test)]
pub(crate) use crate::cli::commands::check_file;
pub(crate) use crate::cli::commands::run_project;
use crate::cli::commands::{
    bench_target, bindgen_header, fix_target, format_targets, lint_target, profile_target,
    run_single_file, run_tests,
};
pub(crate) use crate::cli::commands::{check_command, new_project};
pub(crate) use crate::cli::commands::{lex_file, parse_file, show_project_info};
use crate::cli::output::*;
use crate::cli::paths::{current_dir_checked, validate_source_file_path};
use crate::codegen::Codegen;
#[cfg(test)]
use crate::dependency::*;
use crate::dependency::{
    run_dependency_graph_phase, DependencyGraphInputs, DependencyGraphOutputs,
};
use crate::diagnostics::*;
use crate::linker::*;
use crate::parser::Parser;
use crate::project::pipeline::{
    build_rewrite_fingerprint_context, compute_project_change_impact, evaluate_semantic_cache_gate,
    finalize_completed_build, run_compile_dispatch_phase, run_entry_validation_phase,
    run_parse_index_phase, run_postcheck_phase, run_rewrite_pipeline, run_rewrite_prep_phase,
    validate_symbol_collisions, CompileDispatchInputs, CompileDispatchOutcome, FinalizeBuildInputs,
    ParseIndexOutputs, PostcheckInputs, PostcheckOutcome, RewriteContextInputs,
    RewritePipelineInputs, RewritePrepInputs, RewritePreparation, SemanticGateInputs,
    SemanticPhaseInputs,
};
use crate::project::{find_project_root, resolve_project_output_path, OutputKind, ProjectConfig};
pub(crate) use crate::shared::frontend::{
    extract_imports, extract_top_level_imports, parse_program_from_source,
    run_single_file_semantic_checks, validate_entry_main_signature,
};
use crate::specialization::*;
use crate::symbol_lookup::GlobalSymbolMaps;

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
#[command(version = "1.3.8")]
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

#[derive(Debug)]
enum AppError {
    New(String),
    Build(String),
    Run(String),
    Compile(String),
    Check(String),
    Info(String),
    Lint(String),
    Fix(String),
    Fmt(String),
    Lex(String),
    Parse(String),
    Test(String),
    Bindgen(String),
    Bench(String),
    Profile(String),
    LspRuntimeInit(std::io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::New(message)
            | Self::Build(message)
            | Self::Run(message)
            | Self::Compile(message)
            | Self::Check(message)
            | Self::Info(message)
            | Self::Lint(message)
            | Self::Fix(message)
            | Self::Fmt(message)
            | Self::Lex(message)
            | Self::Parse(message)
            | Self::Test(message)
            | Self::Bindgen(message)
            | Self::Bench(message)
            | Self::Profile(message) => write!(f, "{message}"),
            Self::LspRuntimeInit(err) => write!(
                f,
                "{}: Failed to start runtime for LSP server: {}",
                "error".red().bold(),
                err
            ),
        }
    }
}

impl From<BuildProjectError> for AppError {
    fn from(value: BuildProjectError) -> Self {
        Self::Build(value.to_string())
    }
}

impl From<ParseProjectError> for AppError {
    fn from(value: ParseProjectError) -> Self {
        Self::Compile(value.to_string())
    }
}

impl From<CompilePipelineError> for AppError {
    fn from(value: CompilePipelineError) -> Self {
        Self::Compile(value.to_string())
    }
}

#[derive(Debug)]
enum ParseProjectError {
    MetadataRead(String),
    CacheLoad(String),
    SourceRead(String),
    Parse(String),
    CacheSave(String),
}

impl fmt::Display for ParseProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetadataRead(message)
            | Self::CacheLoad(message)
            | Self::SourceRead(message)
            | Self::Parse(message)
            | Self::CacheSave(message) => write!(f, "{message}"),
        }
    }
}

#[derive(Debug)]
enum BuildProjectError {
    CurrentDirRead(String),
    ProjectRootMissing(String),
    ProjectConfigLoad(String),
    ProjectConfigValidate(String),
    OptLevelValidate(String),
    OutputDirCreate(String),
    FingerprintCompute(String),
    BuildCacheLoad(String),
    BuildCacheSave(String),
    ParseIndex(String),
    DependencyGraph(String),
    SemanticGate(String),
    SymbolCollision(String),
    EntryValidation(String),
    RewritePipeline(String),
    Postcheck(String),
    CompileDispatch(String),
    FinalizeBuild(String),
    SemanticSummaryCacheLoad(String),
    TypecheckSummaryCacheLoad(String),
}

impl fmt::Display for BuildProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDirRead(message)
            | Self::ProjectRootMissing(message)
            | Self::ProjectConfigLoad(message)
            | Self::ProjectConfigValidate(message)
            | Self::OptLevelValidate(message)
            | Self::OutputDirCreate(message)
            | Self::FingerprintCompute(message)
            | Self::BuildCacheLoad(message)
            | Self::BuildCacheSave(message)
            | Self::ParseIndex(message)
            | Self::DependencyGraph(message)
            | Self::SemanticGate(message)
            | Self::SymbolCollision(message)
            | Self::EntryValidation(message)
            | Self::RewritePipeline(message)
            | Self::Postcheck(message)
            | Self::CompileDispatch(message)
            | Self::FinalizeBuild(message)
            | Self::SemanticSummaryCacheLoad(message)
            | Self::TypecheckSummaryCacheLoad(message) => write!(f, "{message}"),
        }
    }
}

impl From<ParseProjectError> for String {
    fn from(value: ParseProjectError) -> Self {
        value.to_string()
    }
}

impl From<BuildProjectError> for String {
    fn from(value: BuildProjectError) -> Self {
        value.to_string()
    }
}

#[derive(Debug)]
enum CompilePipelineError {
    OutputDirCreate(String),
    CodegenCompile(String),
    ObjectEmit(String),
    Link(String),
    SourcePathValidation(String),
    CurrentDirRead(String),
    SourceRead(String),
    Parse(String),
    EntryValidation(String),
    OptLevelValidate(String),
    SemanticCheck(String),
    IrWrite(String),
}

impl fmt::Display for CompilePipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputDirCreate(message)
            | Self::CodegenCompile(message)
            | Self::ObjectEmit(message)
            | Self::Link(message)
            | Self::SourcePathValidation(message)
            | Self::CurrentDirRead(message)
            | Self::SourceRead(message)
            | Self::Parse(message)
            | Self::EntryValidation(message)
            | Self::OptLevelValidate(message)
            | Self::SemanticCheck(message)
            | Self::IrWrite(message) => write!(f, "{message}"),
        }
    }
}

impl From<CompilePipelineError> for String {
    fn from(value: CompilePipelineError) -> Self {
        value.to_string()
    }
}

fn main() {
    if let Err(e) = run_cli() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run_cli() -> Result<(), AppError> {
    configure_cli_colors();
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, path } => new_project(&name, path.as_deref()).map_err(AppError::New),
        Commands::Build {
            release,
            emit_llvm,
            no_check,
            timings,
        } => build_project_impl(release, emit_llvm, !no_check, false, timings)
            .map_err(|e| AppError::Build(e.to_string())),
        Commands::Run {
            file,
            args,
            release,
            no_check,
            timings,
        } => {
            if let Some(f) = file {
                run_single_file(&f, &args, release, !no_check).map_err(AppError::Run)
            } else {
                run_project(&args, release, !no_check, timings).map_err(AppError::Run)
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
        )
        .map_err(AppError::Compile),
        Commands::Check { file, timings } => {
            check_command(file.as_deref(), timings).map_err(AppError::Check)
        }
        Commands::Info => show_project_info().map_err(AppError::Info),
        Commands::Lint { path } => lint_target(path.as_deref()).map_err(AppError::Lint),
        Commands::Fix { path } => fix_target(path.as_deref()).map_err(AppError::Fix),
        Commands::Fmt { path, check } => {
            format_targets(path.as_deref(), check).map_err(AppError::Fmt)
        }
        Commands::Lex { file } => lex_file(&file).map_err(AppError::Lex),
        Commands::Parse { file } => parse_file(&file).map_err(AppError::Parse),
        Commands::Lsp => {
            let runtime = tokio::runtime::Runtime::new().map_err(AppError::LspRuntimeInit)?;
            runtime.block_on(lsp::run_lsp_server());
            Ok(())
        }
        Commands::Test { path, list, filter } => {
            run_tests(path.as_deref(), list, filter.as_deref()).map_err(AppError::Test)
        }
        Commands::Bindgen { header, output } => {
            bindgen_header(&header, output.as_deref()).map_err(AppError::Bindgen)
        }
        Commands::Bench { file, iterations } => {
            bench_target(file.as_deref(), iterations).map_err(AppError::Bench)
        }
        Commands::Profile { file } => profile_target(file.as_deref()).map_err(AppError::Profile),
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
) -> Result<SourceParseResult, ParseProjectError> {
    let tokens = lexer::tokenize(source).map_err(|e| {
        ParseProjectError::Parse(format!(
            "{}: Lexer error in {}: {}",
            "error".red().bold(),
            filename,
            e
        ))
    })?;
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| ParseProjectError::Parse(format_parse_error(&e, source, filename)))?;
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

fn format_project_file_label(project_root: &Path, file: &Path) -> String {
    if let Ok(relative) = file.strip_prefix(project_root) {
        return format_cli_path(relative);
    }
    if let (Ok(canonical_root), Ok(canonical_file)) =
        (project_root.canonicalize(), file.canonicalize())
    {
        if let Ok(relative) = canonical_file.strip_prefix(&canonical_root) {
            return format_cli_path(relative);
        }
    }
    format_cli_path(file)
}

pub(crate) fn parse_project_unit(
    project_root: &Path,
    file: &Path,
) -> Result<ParsedProjectUnit, String> {
    parse_project_unit_impl(project_root, file).map_err(Into::into)
}

fn parse_project_unit_impl(
    project_root: &Path,
    file: &Path,
) -> Result<ParsedProjectUnit, ParseProjectError> {
    let filename = format_project_file_label(project_root, file);
    let file_metadata =
        current_file_metadata_stamp(file).map_err(ParseProjectError::MetadataRead)?;
    let cached_entry =
        load_parsed_file_cache_entry(project_root, file).map_err(ParseProjectError::CacheLoad)?;
    let read_source = |f: &Path| -> Result<String, ParseProjectError> {
        fs::read_to_string(f).map_err(|e| {
            ParseProjectError::SourceRead(format!(
                "{}: Failed to read '{}': {}",
                "error".red().bold(),
                format_project_file_label(project_root, f),
                e
            ))
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
            parse_source_text(&source, source_fp, &filename)?
        }
    } else {
        let source = read_source(file)?;
        let source_fp = source_fingerprint(&source);
        parse_source_text(&source, source_fp, &filename)?
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
            ParseProjectError::CacheLoad(format!(
                "{}: parse cache reported a hit for '{}' but no cache entry was available",
                cli_error("error"),
                format_cli_path(file)
            ))
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
                ParseProjectError::CacheSave(format!(
                    "{}: parsed file '{}' is missing a source fingerprint",
                    cli_error("error"),
                    format_cli_path(file)
                ))
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
        save_parsed_file_cache(project_root, file, &cache_entry)
            .map_err(ParseProjectError::CacheSave)?;

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

/// Build the current project with proper namespace checking
pub(crate) fn build_project(
    release: bool,
    emit_llvm: bool,
    do_check: bool,
    check_only: bool,
    show_timings: bool,
) -> Result<(), String> {
    build_project_impl(release, emit_llvm, do_check, check_only, show_timings).map_err(Into::into)
}

fn build_project_impl(
    release: bool,
    emit_llvm: bool,
    do_check: bool,
    check_only: bool,
    show_timings: bool,
) -> Result<(), BuildProjectError> {
    let mut build_timings = BuildTimings::new(show_timings);
    reset_cache_io_timing_totals(&PARSE_CACHE_TIMING_TOTALS);
    reset_cache_io_timing_totals(&REWRITE_CACHE_TIMING_TOTALS);
    reset_cache_io_timing_totals(&OBJECT_CACHE_META_TIMING_TOTALS);
    let cwd = current_dir_checked().map_err(BuildProjectError::CurrentDirRead)?;
    let project_root = find_project_root(&cwd).ok_or_else(|| {
        BuildProjectError::ProjectRootMissing(format!(
            "{}: No arden.toml found from current directory '{}'. Are you in a project directory?\nRun `arden new <name>` to create a new project.",
            "error".red().bold(),
            format_cli_path(&cwd)
        ))
    })?;

    let config_path = project_root.join("arden.toml");
    let mut config =
        ProjectConfig::load(&config_path).map_err(BuildProjectError::ProjectConfigLoad)?;
    if release {
        config.opt_level = "3".to_string();
    }

    build_timings
        .measure("project config validation", || {
            config.validate(&project_root)
        })
        .map_err(BuildProjectError::ProjectConfigValidate)?;
    validate_opt_level(Some(&config.opt_level)).map_err(BuildProjectError::OptLevelValidate)?;
    let files = build_timings.measure_step("source file discovery", || {
        let mut files = config.get_source_files(&project_root);
        files.sort();
        files
    });
    build_timings.record_counts("source file discovery", &[("files", files.len())]);

    let output_path = resolve_project_output_path(&project_root, &config);
    if !check_only {
        ensure_output_parent_dir(&output_path).map_err(BuildProjectError::OutputDirCreate)?;
    }
    let fingerprint = build_timings
        .measure("project fingerprint", || {
            compute_project_fingerprint(&files, &config, emit_llvm, do_check)
        })
        .map_err(BuildProjectError::FingerprintCompute)?;
    if !check_only {
        if let Some(cached) = build_timings
            .measure("build cache lookup", || {
                load_cached_fingerprint(&project_root)
            })
            .map_err(BuildProjectError::BuildCacheLoad)?
        {
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
    } = run_parse_index_phase(&mut build_timings, &project_root, &files)
        .map_err(BuildProjectError::ParseIndex)?;
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
    )
    .map_err(BuildProjectError::DependencyGraph)?;

    let previous_semantic_summary = load_semantic_summary_cache(&project_root)
        .map_err(BuildProjectError::SemanticSummaryCacheLoad)?;
    let previous_typecheck_summary = load_typecheck_summary_cache(&project_root)
        .map_err(BuildProjectError::TypecheckSummaryCacheLoad)?;
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
    )
    .map_err(BuildProjectError::SemanticGate)?;
    if semantic_cache_hit {
        print_cli_cache(format!("Reused semantic build cache for {}", config.name));
        save_cached_fingerprint(&project_root, &fingerprint)
            .map_err(BuildProjectError::BuildCacheSave)?;
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
    )
    .map_err(BuildProjectError::SymbolCollision)?;

    run_entry_validation_phase(do_check, &entry_path, &parsed_files)
        .map_err(BuildProjectError::EntryValidation)?;

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

    let rewritten_files = run_rewrite_pipeline(
        &mut build_timings,
        RewritePipelineInputs {
            do_check,
            project_root: &project_root,
            parsed_files: &parsed_files,
            global_function_map: &global_function_map,
            entry_namespace: &entry_namespace,
            rewrite_fingerprint_ctx: &rewrite_fingerprint_ctx,
            safe_rewrite_cache_files: &safe_rewrite_cache_files,
            namespace_functions: &namespace_functions,
            namespace_class_map: &namespace_class_map,
            global_class_map: &global_class_map,
            namespace_interface_map: &namespace_interface_map,
            global_interface_map: &global_interface_map,
            namespace_enum_map: &namespace_enum_map,
            global_enum_map: &global_enum_map,
            namespace_module_map: &namespace_module_map,
            global_module_map: &global_module_map,
        },
    )
    .map_err(BuildProjectError::RewritePipeline)?;

    if let PostcheckOutcome::Completed = run_postcheck_phase(
        &mut build_timings,
        PostcheckInputs {
            do_check,
            check_only,
            config_name: &config.name,
            semantic_inputs: SemanticPhaseInputs {
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
        },
    )
    .map_err(BuildProjectError::Postcheck)?
    {
        return Ok(());
    }

    let link = LinkConfig {
        opt_level: Some(&config.opt_level),
        target: config.target.as_deref(),
        output_kind: config.output_kind.clone(),
        link_search: &config.link_search,
        link_libs: &config.link_libs,
        link_args: &config.link_args,
    };
    if let CompileDispatchOutcome::Completed = run_compile_dispatch_phase(
        &mut build_timings,
        CompileDispatchInputs {
            rewritten_files: &rewritten_files,
            entry_path: &entry_path,
            output_path: &output_path,
            emit_llvm,
            link: &link,
            project_root: &project_root,
            config_name: &config.name,
            fingerprint: &fingerprint,
            parsed_files: &parsed_files,
            file_dependency_graph: &file_dependency_graph,
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
    )
    .map_err(BuildProjectError::CompileDispatch)?
    {
        return Ok(());
    }

    finalize_completed_build(
        &mut build_timings,
        FinalizeBuildInputs {
            project_root: &project_root,
            config_name: &config.name,
            output_path: &output_path,
            fingerprint: &fingerprint,
            semantic_fingerprint: &semantic_fingerprint,
            current_dependency_graph_cache: &current_dependency_graph_cache,
        },
    )
    .map_err(BuildProjectError::FinalizeBuild)?;

    Ok(())
}

pub(crate) fn compile_program_ast(
    program: &Program,
    source_path: &Path,
    output_path: &Path,
    emit_llvm: bool,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    compile_program_ast_impl(program, source_path, output_path, emit_llvm, link).map_err(Into::into)
}

fn compile_program_ast_impl(
    program: &Program,
    source_path: &Path,
    output_path: &Path,
    emit_llvm: bool,
    link: &LinkConfig<'_>,
) -> Result<(), CompilePipelineError> {
    ensure_output_parent_dir(output_path).map_err(CompilePipelineError::OutputDirCreate)?;

    let context = Context::create();
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let mut codegen = Codegen::new(&context, module_name);
    codegen.compile(program).map_err(|e| {
        CompilePipelineError::CodegenCompile(format!(
            "{}: Codegen error in '{}': {}",
            "error".red().bold(),
            format_cli_path(source_path),
            e.message
        ))
    })?;

    if emit_llvm {
        let ll_path = output_path.with_extension("ll");
        codegen
            .write_ir(&ll_path)
            .map_err(CompilePipelineError::IrWrite)?;
        println!("{} {}", cli_success("Wrote LLVM IR"), cli_path(&ll_path));
    } else {
        let object_path = output_path.with_extension(format!("arden-tmp.{}", object_ext()));
        codegen
            .write_object_with_config(&object_path, link.opt_level, link.target, &link.output_kind)
            .map_err(|e| {
                CompilePipelineError::ObjectEmit(format!(
                    "{}: Failed to emit object for '{}': {}",
                    "error".red().bold(),
                    format_cli_path(source_path),
                    e
                ))
            })?;
        let link_result = link_objects(std::slice::from_ref(&object_path), output_path, link);
        if let Err(err) = fs::remove_file(&object_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "{}: failed to remove temporary object '{}': {}",
                    cli_warning("warning"),
                    format_cli_path(&object_path),
                    err
                );
            }
        }
        link_result.map_err(CompilePipelineError::Link)?;
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
    compile_program_ast_to_object_filtered_impl(
        program,
        source_path,
        object_path,
        link,
        active_symbols,
        declaration_symbols,
        timings,
    )
    .map_err(Into::into)
}

fn compile_program_ast_to_object_filtered_impl(
    program: &Program,
    source_path: &Path,
    object_path: &Path,
    link: &LinkConfig<'_>,
    active_symbols: &HashSet<String>,
    declaration_symbols: &HashSet<String>,
    timings: Option<&ObjectEmitTimingTotals>,
) -> Result<(), CompilePipelineError> {
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
        .map_err(|e| {
            CompilePipelineError::CodegenCompile(format!(
                "{}: Codegen error in '{}': {}",
                "error".red().bold(),
                format_cli_path(source_path),
                e.message
            ))
        })?;
    if let Some(timings) = timings {
        timings
            .compile_filtered_ns
            .fetch_add(elapsed_nanos_u64(compile_started_at), Ordering::Relaxed);
    }

    if let Some(parent) = object_path.parent() {
        let object_dir_setup_started_at = Instant::now();
        fs::create_dir_all(parent).map_err(|e| {
            CompilePipelineError::OutputDirCreate(format!(
                "{}: Failed to create object cache directory '{}': {}",
                "error".red().bold(),
                format_cli_path(parent),
                e
            ))
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
            CompilePipelineError::ObjectEmit(format!(
                "{}: Failed to emit object for '{}': {}",
                "error".red().bold(),
                format_cli_path(source_path),
                e
            ))
        })?;
    if let Some(timings) = timings {
        timings
            .write_object_ns
            .fetch_add(elapsed_nanos_u64(write_started_at), Ordering::Relaxed);
    }
    Ok(())
}

/// Compile a single file (legacy mode)
pub(crate) fn compile_file(
    file: &Path,
    output: Option<&Path>,
    emit_llvm: bool,
    do_check: bool,
    opt_level: Option<&str>,
    target: Option<&str>,
) -> Result<(), String> {
    compile_file_impl(file, output, emit_llvm, do_check, opt_level, target).map_err(Into::into)
}

fn compile_file_impl(
    file: &Path,
    output: Option<&Path>,
    emit_llvm: bool,
    do_check: bool,
    opt_level: Option<&str>,
    target: Option<&str>,
) -> Result<(), CompilePipelineError> {
    let compile_started = Instant::now();
    validate_source_file_path(file).map_err(CompilePipelineError::SourcePathValidation)?;

    // Check if we're in a project
    if let Some(project_root) =
        find_project_root(&current_dir_checked().map_err(CompilePipelineError::CurrentDirRead)?)
    {
        if file.starts_with(&project_root) {
            println!(
                "{}",
                "note: file is inside a project; prefer `arden build` for project builds".yellow()
            );
        }
    }

    let source = fs::read_to_string(file).map_err(|e| {
        CompilePipelineError::SourceRead(format!(
            "{}: Failed to read file '{}': {}",
            "error".red().bold(),
            format_cli_path(file),
            e
        ))
    })?;

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

    ensure_output_parent_dir(&output_path).map_err(CompilePipelineError::OutputDirCreate)?;

    if !do_check {
        let filename = format_cli_path(file);
        let program =
            parse_program_from_source(&source, &filename).map_err(CompilePipelineError::Parse)?;
        validate_entry_main_signature(&program, &source, &filename)
            .map_err(CompilePipelineError::EntryValidation)?;
    }

    compile_source(
        &source,
        file,
        &output_path,
        emit_llvm,
        do_check,
        opt_level,
        target,
    )
    .map_err(CompilePipelineError::CodegenCompile)?;

    println!(
        "{} {} {}",
        cli_success("Wrote"),
        cli_path(&output_path),
        cli_soft(format!("({})", cli_elapsed(compile_started.elapsed())))
    );
    Ok(())
}

/// Compile source code
pub(crate) fn compile_source(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    emit_llvm: bool,
    do_check: bool,
    opt_level: Option<&str>,
    target: Option<&str>,
) -> Result<(), String> {
    compile_source_impl(
        source,
        source_path,
        output_path,
        emit_llvm,
        do_check,
        opt_level,
        target,
    )
    .map_err(Into::into)
}

fn compile_source_impl(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    emit_llvm: bool,
    do_check: bool,
    opt_level: Option<&str>,
    target: Option<&str>,
) -> Result<(), CompilePipelineError> {
    validate_opt_level(opt_level).map_err(CompilePipelineError::OptLevelValidate)?;

    let filename = format_cli_path(source_path);

    // Tokenize
    let program =
        parse_program_from_source(source, &filename).map_err(CompilePipelineError::Parse)?;

    // Type check
    if do_check {
        run_single_file_semantic_checks(source, &filename, &program)
            .map_err(CompilePipelineError::SemanticCheck)?;
    }

    // Codegen
    let context = Context::create();
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let mut codegen = Codegen::new(&context, module_name);
    codegen.compile(&program).map_err(|e| {
        CompilePipelineError::CodegenCompile(format!(
            "{}: Codegen error in '{}': {}",
            "error".red().bold(),
            format_cli_path(source_path),
            e.message
        ))
    })?;

    if emit_llvm {
        let ll_path = output_path.with_extension("ll");
        codegen
            .write_ir(&ll_path)
            .map_err(CompilePipelineError::IrWrite)?;
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
                CompilePipelineError::ObjectEmit(format!(
                    "{}: Failed to emit object for '{}': {}",
                    "error".red().bold(),
                    format_cli_path(source_path),
                    e
                ))
            })?;
        let link_result = link_objects(std::slice::from_ref(&object_path), output_path, &link);
        if let Err(err) = fs::remove_file(&object_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "{}: failed to remove temporary object '{}': {}",
                    cli_warning("warning"),
                    format_cli_path(&object_path),
                    err
                );
            }
        }
        link_result.map_err(CompilePipelineError::Link)?;
    }

    Ok(())
}
