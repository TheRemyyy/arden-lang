//! Apex Programming Language Compiler

mod ast;
mod bindgen;
mod borrowck;
mod codegen;
mod formatter;
mod import_check;
mod lexer;
mod lint;
mod lsp;
mod namespace;
mod parser;
mod project;
mod project_rewrite;
mod stdlib;
mod test_runner;
mod typeck;

use clap::{Parser as ClapParser, Subcommand};
use colored::*;
use inkwell::context::Context;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use std::time::UNIX_EPOCH;
use twox_hash::XxHash64;

use crate::ast::{Block, Decl, Expr, ImportDecl, Pattern, Program, Spanned, Stmt};
use crate::borrowck::BorrowChecker;
use crate::codegen::Codegen;
use crate::import_check::ImportChecker;
use crate::parser::Parser;
use crate::project::{find_project_root, OutputKind, ProjectConfig};
use crate::stdlib::stdlib_registry;
use crate::test_runner::{discover_tests, generate_test_runner_with_source, print_discovery};
use crate::typeck::{ClassMethodEffectsSummary, FunctionEffectsSummary, TypeChecker};

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

fn project_cache_file(project_root: &Path) -> PathBuf {
    project_root.join(".apexcache").join("build_fingerprint")
}

fn semantic_project_cache_file(project_root: &Path) -> PathBuf {
    project_root
        .join(".apexcache")
        .join("semantic_build_fingerprint")
}

fn stable_hasher() -> XxHash64 {
    XxHash64::with_seed(0)
}

fn read_cache_blob<T: DeserializeOwned>(path: &Path, label: &str) -> Result<Option<T>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read(path).map_err(|e| {
        format!(
            "{}: Failed to read {} '{}': {}",
            "error".red().bold(),
            label,
            path.display(),
            e
        )
    })?;
    Ok(bincode::deserialize(&raw).ok())
}

fn write_cache_blob<T: Serialize>(path: &Path, label: &str, value: &T) -> Result<(), String> {
    let bytes = bincode::serialize(value).map_err(|e| {
        format!(
            "{}: Failed to serialize {} '{}': {}",
            "error".red().bold(),
            label,
            path.display(),
            e
        )
    })?;
    fs::write(path, bytes).map_err(|e| {
        format!(
            "{}: Failed to write {} '{}': {}",
            "error".red().bold(),
            label,
            path.display(),
            e
        )
    })
}

fn project_build_artifact_exists(output_path: &Path, emit_llvm: bool) -> bool {
    if emit_llvm {
        output_path.with_extension("ll").exists()
    } else {
        output_path.exists()
    }
}

fn compute_project_fingerprint(
    project_root: &Path,
    config: &ProjectConfig,
    emit_llvm: bool,
    do_check: bool,
) -> Result<String, String> {
    let mut hasher = stable_hasher();

    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    config.name.hash(&mut hasher);
    config.version.hash(&mut hasher);
    config.entry.hash(&mut hasher);
    config.output.hash(&mut hasher);
    config.opt_level.hash(&mut hasher);
    config.target.hash(&mut hasher);
    format!("{:?}", config.output_kind).hash(&mut hasher);
    config.link_search.hash(&mut hasher);
    config.link_libs.hash(&mut hasher);
    config.link_args.hash(&mut hasher);
    emit_llvm.hash(&mut hasher);
    do_check.hash(&mut hasher);

    let mut files = config.get_source_files(project_root);
    files.sort();
    for file in files {
        file.hash(&mut hasher);
        let meta = fs::metadata(&file).map_err(|e| {
            format!(
                "{}: Failed to read metadata for '{}': {}",
                "error".red().bold(),
                file.display(),
                e
            )
        })?;
        meta.len().hash(&mut hasher);

        let modified = meta
            .modified()
            .map_err(|e| {
                format!(
                    "{}: Failed to read modified time for '{}': {}",
                    "error".red().bold(),
                    file.display(),
                    e
                )
            })?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                format!(
                    "{}: Invalid modified time for '{}': {}",
                    "error".red().bold(),
                    file.display(),
                    e
                )
            })?;
        modified.as_secs().hash(&mut hasher);
        modified.subsec_nanos().hash(&mut hasher);
    }

    Ok(format!("{:016x}", hasher.finish()))
}

fn load_cached_fingerprint(project_root: &Path) -> Option<String> {
    fs::read_to_string(project_cache_file(project_root))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn load_semantic_cached_fingerprint(project_root: &Path) -> Option<String> {
    fs::read_to_string(semantic_project_cache_file(project_root))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn save_cached_fingerprint(project_root: &Path, fingerprint: &str) -> Result<(), String> {
    let cache_file = project_cache_file(project_root);
    if let Some(parent) = cache_file.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }
    fs::write(&cache_file, fingerprint).map_err(|e| {
        format!(
            "{}: Failed to write build cache '{}': {}",
            "error".red().bold(),
            cache_file.display(),
            e
        )
    })
}

fn save_semantic_cached_fingerprint(project_root: &Path, fingerprint: &str) -> Result<(), String> {
    let cache_file = semantic_project_cache_file(project_root);
    if let Some(parent) = cache_file.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }
    fs::write(&cache_file, fingerprint).map_err(|e| {
        format!(
            "{}: Failed to write semantic build cache '{}': {}",
            "error".red().bold(),
            cache_file.display(),
            e
        )
    })
}

const PARSE_CACHE_SCHEMA: &str = "v8";
const DEPENDENCY_GRAPH_CACHE_SCHEMA: &str = "v2";
const SEMANTIC_SUMMARY_CACHE_SCHEMA: &str = "v2";
const TYPECHECK_SUMMARY_CACHE_SCHEMA: &str = "v4";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileMetadataStamp {
    len: u64,
    modified_secs: u64,
    modified_nanos: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedFileCacheEntry {
    schema: String,
    compiler_version: String,
    file_metadata: FileMetadataStamp,
    source_fingerprint: String,
    api_fingerprint: String,
    semantic_fingerprint: String,
    import_check_fingerprint: String,
    namespace: String,
    program: Program,
    imports: Vec<ImportDecl>,
    function_names: Vec<String>,
    class_names: Vec<String>,
    enum_names: Vec<String>,
    module_names: Vec<String>,
    referenced_symbols: Vec<String>,
    qualified_symbol_refs: Vec<Vec<String>>,
    api_referenced_symbols: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedProjectUnit {
    file: PathBuf,
    namespace: String,
    program: Program,
    imports: Vec<ImportDecl>,
    api_fingerprint: String,
    semantic_fingerprint: String,
    import_check_fingerprint: String,
    function_names: Vec<String>,
    class_names: Vec<String>,
    enum_names: Vec<String>,
    module_names: Vec<String>,
    referenced_symbols: Vec<String>,
    qualified_symbol_refs: Vec<Vec<String>>,
    api_referenced_symbols: Vec<String>,
    from_parse_cache: bool,
}

#[derive(Debug, Clone)]
struct RewrittenProjectUnit {
    file: PathBuf,
    program: Program,
    api_program: Program,
    semantic_fingerprint: String,
    rewrite_context_fingerprint: String,
    active_symbols: HashSet<String>,
    from_rewrite_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DependencyGraphCache {
    schema: String,
    compiler_version: String,
    files: Vec<DependencyGraphFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DependencyGraphFileEntry {
    file: PathBuf,
    semantic_fingerprint: String,
    api_fingerprint: String,
    direct_dependencies: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemanticSummaryCache {
    schema: String,
    compiler_version: String,
    files: Vec<SemanticSummaryFileEntry>,
    components: Vec<SemanticSummaryComponentEntry>,
    function_effects: HashMap<String, Vec<String>>,
    class_method_effects: HashMap<String, HashMap<String, Vec<String>>>,
    class_mutating_methods: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemanticSummaryFileEntry {
    file: PathBuf,
    semantic_fingerprint: String,
    function_names: Vec<String>,
    class_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemanticSummaryComponentEntry {
    component_fingerprint: String,
    files: Vec<PathBuf>,
    function_names: Vec<String>,
    class_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TypecheckSummaryCache {
    schema: String,
    compiler_version: String,
    files: Vec<TypecheckSummaryFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TypecheckSummaryFileEntry {
    file: PathBuf,
    semantic_fingerprint: String,
    component_fingerprint: String,
}

struct BuildTimingPhase {
    label: String,
    ms: f64,
    counters: Vec<(String, usize)>,
}

struct BuildTimings {
    enabled: bool,
    started_at: Instant,
    phases: Vec<BuildTimingPhase>,
}

impl BuildTimings {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            started_at: Instant::now(),
            phases: Vec::new(),
        }
    }

    fn measure<T, E, F>(&mut self, label: &str, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        let start = Instant::now();
        let result = f();
        if self.enabled {
            self.phases.push(BuildTimingPhase {
                label: label.to_string(),
                ms: start.elapsed().as_secs_f64() * 1000.0,
                counters: Vec::new(),
            });
        }
        result
    }

    fn measure_value<T, F>(&mut self, label: &str, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        if self.enabled {
            self.phases.push(BuildTimingPhase {
                label: label.to_string(),
                ms: start.elapsed().as_secs_f64() * 1000.0,
                counters: Vec::new(),
            });
        }
        result
    }

    fn record_counts(&mut self, label: &str, counters: &[(&str, usize)]) {
        if !self.enabled {
            return;
        }

        if let Some(phase) = self.phases.iter_mut().rfind(|phase| phase.label == label) {
            phase.counters = counters
                .iter()
                .map(|(name, value)| ((*name).to_string(), *value))
                .collect();
            return;
        }

        self.phases.push(BuildTimingPhase {
            label: label.to_string(),
            ms: 0.0,
            counters: counters
                .iter()
                .map(|(name, value)| ((*name).to_string(), *value))
                .collect(),
        });
    }

    fn print(&self) {
        if !self.enabled {
            return;
        }

        println!("{}", "Build timings".cyan().bold());
        for phase in &self.phases {
            let counters = if phase.counters.is_empty() {
                String::new()
            } else {
                format!(
                    "  {}",
                    phase
                        .counters
                        .iter()
                        .map(|(label, value)| format!("{label}={value}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            println!("  {:<28} {:>10.3} ms{}", phase.label, phase.ms, counters);
        }
        println!(
            "  {:<28} {:>10.3} ms",
            "total",
            self.started_at.elapsed().as_secs_f64() * 1000.0
        );
    }
}

fn parsed_file_cache_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = stable_hasher();
    file.hash(&mut hasher);
    project_root
        .join(".apexcache")
        .join("parsed")
        .join(format!("{:016x}.json", hasher.finish()))
}

fn source_fingerprint(source: &str) -> String {
    let mut hasher = stable_hasher();
    source.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn semantic_program_fingerprint(program: &Program) -> String {
    let canonical = formatter::format_program_canonical(program);
    source_fingerprint(&canonical)
}

fn empty_block() -> Block {
    Vec::new()
}

fn api_projection_decl(decl: &Spanned<Decl>) -> Spanned<Decl> {
    let projected = match &decl.node {
        Decl::Function(func) => {
            let mut func = func.clone();
            if !func.is_extern {
                func.body = empty_block();
            }
            Decl::Function(func)
        }
        Decl::Class(class) => {
            let mut class = class.clone();
            if let Some(constructor) = &mut class.constructor {
                constructor.body = empty_block();
            }
            if let Some(destructor) = &mut class.destructor {
                destructor.body = empty_block();
            }
            class.methods = class
                .methods
                .into_iter()
                .map(|mut method| {
                    method.body = empty_block();
                    method
                })
                .collect();
            Decl::Class(class)
        }
        Decl::Interface(interface) => {
            let mut interface = interface.clone();
            interface.methods = interface
                .methods
                .into_iter()
                .map(|mut method| {
                    method.default_impl = method.default_impl.map(|_| empty_block());
                    method
                })
                .collect();
            Decl::Interface(interface)
        }
        Decl::Module(module) => {
            let mut module = module.clone();
            module.declarations = module
                .declarations
                .iter()
                .map(api_projection_decl)
                .collect();
            Decl::Module(module)
        }
        Decl::Enum(en) => Decl::Enum(en.clone()),
        Decl::Import(import) => Decl::Import(import.clone()),
    };
    Spanned::new(projected, decl.span.clone())
}

fn api_projection_program(program: &Program) -> Program {
    Program {
        package: program.package.clone(),
        declarations: program
            .declarations
            .iter()
            .map(api_projection_decl)
            .collect(),
    }
}

fn filter_codegen_decl_by_symbols(
    decl: &Spanned<Decl>,
    declaration_symbols: &HashSet<String>,
) -> Option<Spanned<Decl>> {
    fn class_has_requested_method(class_name: &str, declaration_symbols: &HashSet<String>) -> bool {
        let method_prefix = format!("{}__", class_name);
        declaration_symbols
            .iter()
            .any(|symbol| symbol.starts_with(&method_prefix))
    }

    match &decl.node {
        Decl::Function(func) => (declaration_symbols.contains(&func.name)
            || func.name.contains("__spec__"))
        .then(|| decl.clone()),
        Decl::Class(class) => (declaration_symbols.contains(&class.name)
            || class.name.contains("__spec__")
            || class_has_requested_method(&class.name, declaration_symbols))
        .then(|| decl.clone()),
        Decl::Enum(en) => declaration_symbols.contains(&en.name).then(|| decl.clone()),
        Decl::Module(module) => {
            if declaration_symbols.contains(&module.name) {
                return Some(decl.clone());
            }

            let filtered_declarations = module
                .declarations
                .iter()
                .filter_map(|inner| filter_codegen_decl_by_symbols(inner, declaration_symbols))
                .collect::<Vec<_>>();

            if filtered_declarations.is_empty() {
                None
            } else {
                let mut filtered_module = module.clone();
                filtered_module.declarations = filtered_declarations;
                Some(Spanned::new(
                    Decl::Module(filtered_module),
                    decl.span.clone(),
                ))
            }
        }
        Decl::Interface(_) | Decl::Import(_) => None,
    }
}

fn filter_codegen_program_by_symbols(
    program: &Program,
    declaration_symbols: &HashSet<String>,
) -> Program {
    Program {
        package: program.package.clone(),
        declarations: program
            .declarations
            .iter()
            .filter_map(|decl| filter_codegen_decl_by_symbols(decl, declaration_symbols))
            .collect(),
    }
}

fn api_program_fingerprint(program: &Program) -> String {
    let projected = api_projection_program(program);
    let canonical = formatter::format_program_canonical(&projected);
    source_fingerprint(&canonical)
}

fn codegen_program_for_unit(
    rewritten_files: &[RewrittenProjectUnit],
    rewritten_file_indices: &HashMap<PathBuf, usize>,
    active_file: &Path,
    dependency_closure: Option<&HashSet<PathBuf>>,
    declaration_symbols: Option<&HashSet<String>>,
) -> Program {
    let mut program = Program {
        package: None,
        declarations: Vec::new(),
    };

    let mut relevant_files = dependency_closure
        .map(|closure| closure.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_else(|| {
            rewritten_files
                .iter()
                .map(|unit| unit.file.clone())
                .collect()
        });
    if !relevant_files.iter().any(|file| file == active_file) {
        relevant_files.push(active_file.to_path_buf());
    }
    relevant_files.sort();
    relevant_files.dedup();

    for file in relevant_files {
        let Some(index) = rewritten_file_indices.get(&file).copied() else {
            continue;
        };
        let unit = &rewritten_files[index];
        let source_program = if unit.file == active_file {
            unit.program.clone()
        } else {
            declaration_symbols
                .map(|symbols| filter_codegen_program_by_symbols(&unit.program, symbols))
                .unwrap_or_else(|| unit.api_program.clone())
        };
        program.declarations.extend(source_program.declarations);
    }

    program
}

fn combined_program_for_files(rewritten_files: &[RewrittenProjectUnit]) -> Program {
    let mut program = Program {
        package: None,
        declarations: Vec::new(),
    };

    for unit in rewritten_files {
        program
            .declarations
            .extend(unit.program.declarations.clone());
    }

    program
}

fn mangle_project_symbol_for_codegen(namespace: &str, entry_namespace: &str, name: &str) -> String {
    if name == "main" && namespace == entry_namespace {
        "main".to_string()
    } else {
        format!("{}__{}", namespace.replace('.', "__"), name)
    }
}

#[derive(Debug, Clone)]
struct CodegenReferenceMetadata {
    imports: Vec<ImportDecl>,
    referenced_symbols: Vec<String>,
    qualified_symbol_refs: Vec<Vec<String>>,
    api_referenced_symbols: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
fn extend_declaration_symbols_for_reference(
    symbol: &str,
    entry_namespace: &str,
    declaration_symbols: &mut HashSet<String>,
    stack: &mut Vec<PathBuf>,
    closure_files: &HashSet<PathBuf>,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
) {
    let mut push_owner = |owner_ns: &str, owner_file: &Path| {
        if closure_files.contains(owner_file) {
            declaration_symbols.insert(mangle_project_symbol_for_codegen(
                owner_ns,
                entry_namespace,
                symbol,
            ));
            stack.push(owner_file.to_path_buf());
        }
    };

    if let Some((owner_symbol, _member)) = symbol.rsplit_once("__") {
        if let (Some(owner_ns), Some(owner_file)) = (
            global_class_map.get(owner_symbol),
            global_class_file_map.get(owner_symbol),
        ) {
            push_owner(owner_ns, owner_file);
        }
    }

    if let (Some(owner_ns), Some(owner_file)) = (
        global_function_map.get(symbol),
        global_function_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        global_class_map.get(symbol),
        global_class_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        global_enum_map.get(symbol),
        global_enum_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        global_module_map.get(symbol),
        global_module_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
}

fn resolve_exact_imported_symbol_file<'a>(
    namespace_path: &str,
    symbol_name: &str,
    global_symbol_map: &HashMap<String, String>,
    global_symbol_file_map: &'a HashMap<String, PathBuf>,
) -> Option<(String, String, &'a PathBuf)> {
    if global_symbol_map
        .get(symbol_name)
        .is_some_and(|owner_ns| owner_ns == namespace_path)
    {
        return global_symbol_file_map
            .get(symbol_name)
            .map(|file| (namespace_path.to_string(), symbol_name.to_string(), file));
    }

    let full_path = format!("{}.{}", namespace_path, symbol_name);
    let mut matches = global_symbol_map
        .iter()
        .filter_map(|(candidate, owner_ns)| {
            let candidate_path = format!("{}.{}", owner_ns, candidate.replace("__", "."));
            (candidate_path == full_path).then(|| {
                global_symbol_file_map
                    .get(candidate)
                    .map(|file| (owner_ns.clone(), candidate.clone(), file))
            })?
        })
        .collect::<Vec<_>>();
    matches.sort_unstable_by(|a, b| a.1.cmp(&b.1));
    matches.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    (matches.len() == 1).then(|| matches.swap_remove(0))
}

#[allow(clippy::too_many_arguments)]
fn resolve_exact_imported_symbol_owner<'a>(
    namespace_path: &str,
    symbol_name: &str,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &'a HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &'a HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &'a HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &'a HashMap<String, PathBuf>,
) -> Option<(String, String, &'a PathBuf)> {
    resolve_exact_imported_symbol_file(
        namespace_path,
        symbol_name,
        global_function_map,
        global_function_file_map,
    )
    .or_else(|| {
        resolve_exact_imported_symbol_file(
            namespace_path,
            symbol_name,
            global_class_map,
            global_class_file_map,
        )
    })
    .or_else(|| {
        resolve_exact_imported_symbol_file(
            namespace_path,
            symbol_name,
            global_enum_map,
            global_enum_file_map,
        )
    })
    .or_else(|| {
        resolve_exact_imported_symbol_file(
            namespace_path,
            symbol_name,
            global_module_map,
            global_module_file_map,
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn extend_declaration_symbols_for_exact_import(
    import: &ImportDecl,
    entry_namespace: &str,
    declaration_symbols: &mut HashSet<String>,
    stack: &mut Vec<PathBuf>,
    closure_files: &HashSet<PathBuf>,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
) {
    let Some((namespace, symbol)) = import.path.rsplit_once('.') else {
        return;
    };

    if let Some((owner_ns, symbol_name, owner_file)) = resolve_exact_imported_symbol_owner(
        namespace,
        symbol,
        global_function_map,
        global_function_file_map,
        global_class_map,
        global_class_file_map,
        global_enum_map,
        global_enum_file_map,
        global_module_map,
        global_module_file_map,
    ) {
        if closure_files.contains(owner_file) {
            declaration_symbols.insert(mangle_project_symbol_for_codegen(
                &owner_ns,
                entry_namespace,
                &symbol_name,
            ));
            stack.push(owner_file.clone());
        }
        return;
    }

    if let Some((enum_namespace, enum_name)) = namespace.rsplit_once('.') {
        if let Some((owner_ns, resolved_enum_name, owner_file)) = resolve_exact_imported_symbol_file(
            enum_namespace,
            enum_name,
            global_enum_map,
            global_enum_file_map,
        ) {
            if closure_files.contains(owner_file) {
                declaration_symbols.insert(mangle_project_symbol_for_codegen(
                    &owner_ns,
                    entry_namespace,
                    &resolved_enum_name,
                ));
                stack.push(owner_file.clone());
            }
            return;
        }
    }

    let mut push_owner = |owner_ns: &str, owner_file: &Path| {
        if owner_ns == namespace && closure_files.contains(owner_file) {
            declaration_symbols.insert(mangle_project_symbol_for_codegen(
                owner_ns,
                entry_namespace,
                symbol,
            ));
            stack.push(owner_file.to_path_buf());
        }
    };

    if let (Some(owner_ns), Some(owner_file)) = (
        global_function_map.get(symbol),
        global_function_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        global_class_map.get(symbol),
        global_class_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        global_enum_map.get(symbol),
        global_enum_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
    if let (Some(owner_ns), Some(owner_file)) = (
        global_module_map.get(symbol),
        global_module_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
}

#[derive(Debug, Clone)]
struct DeclarationClosure {
    symbols: HashSet<String>,
    files: HashSet<PathBuf>,
}

#[allow(clippy::too_many_arguments)]
fn declaration_symbols_for_unit(
    root_file: &Path,
    root_active_symbols: &HashSet<String>,
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
    reference_metadata: &HashMap<PathBuf, CodegenReferenceMetadata>,
    entry_namespace: &str,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
) -> DeclarationClosure {
    let mut closure_files = transitive_dependencies(forward_graph, root_file);
    closure_files.insert(root_file.to_path_buf());

    let mut declaration_symbols = root_active_symbols.clone();
    let mut visited_files = HashSet::new();
    let mut stack = vec![root_file.to_path_buf()];

    while let Some(file) = stack.pop() {
        if !visited_files.insert(file.clone()) {
            continue;
        }

        let Some(metadata) = reference_metadata.get(&file) else {
            continue;
        };

        for import in &metadata.imports {
            extend_declaration_symbols_for_exact_import(
                import,
                entry_namespace,
                &mut declaration_symbols,
                &mut stack,
                &closure_files,
                global_function_map,
                global_function_file_map,
                global_class_map,
                global_class_file_map,
                global_enum_map,
                global_enum_file_map,
                global_module_map,
                global_module_file_map,
            );

            let import_key = import_lookup_key(import);
            for path in &metadata.qualified_symbol_refs {
                if path.first().is_some_and(|part| part == &import_key) {
                    let rest = &path[1..];
                    if let Some((owner_ns, candidate)) = resolve_symbol_in_namespace_path(
                        &import.path,
                        rest,
                        global_function_map,
                        global_class_map,
                        global_enum_map,
                        global_module_map,
                    ) {
                        let owner_file = global_function_file_map
                            .get(&candidate)
                            .or_else(|| global_class_file_map.get(&candidate))
                            .or_else(|| global_enum_file_map.get(&candidate))
                            .or_else(|| global_module_file_map.get(&candidate));
                        if let Some(owner_file) = owner_file {
                            if closure_files.contains(owner_file) {
                                declaration_symbols.insert(mangle_project_symbol_for_codegen(
                                    &owner_ns,
                                    entry_namespace,
                                    &candidate,
                                ));
                                stack.push(owner_file.to_path_buf());
                            }
                        }
                    }
                }
            }
        }

        let symbols = if file == root_file {
            &metadata.referenced_symbols
        } else {
            &metadata.api_referenced_symbols
        };
        for symbol in symbols {
            extend_declaration_symbols_for_reference(
                symbol,
                entry_namespace,
                &mut declaration_symbols,
                &mut stack,
                &closure_files,
                global_function_map,
                global_function_file_map,
                global_class_map,
                global_class_file_map,
                global_enum_map,
                global_enum_file_map,
                global_module_map,
                global_module_file_map,
            );
        }
    }

    DeclarationClosure {
        symbols: declaration_symbols,
        files: visited_files,
    }
}

fn closure_body_symbols_for_unit(
    root_file: &Path,
    root_namespace: &str,
    declaration_symbols: &HashSet<String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_file_map: &HashMap<String, PathBuf>,
) -> HashSet<String> {
    let namespace_prefix = format!("{}__", root_namespace.replace('.', "__"));
    declaration_symbols
        .iter()
        .filter(|symbol| {
            let raw_symbol = symbol
                .strip_prefix(&namespace_prefix)
                .unwrap_or(symbol.as_str());

            if global_function_file_map
                .get(raw_symbol)
                .is_some_and(|owner_file| owner_file == root_file)
            {
                return true;
            }

            if global_class_file_map
                .get(raw_symbol)
                .is_some_and(|owner_file| owner_file == root_file)
            {
                return true;
            }

            if let Some(owner) = raw_symbol.strip_suffix("__new") {
                return global_class_file_map
                    .get(owner)
                    .is_some_and(|owner_file| owner_file == root_file);
            }

            if let Some((owner, _)) = raw_symbol.rsplit_once("__") {
                return global_class_file_map
                    .get(owner)
                    .is_some_and(|owner_file| owner_file == root_file);
            }

            false
        })
        .cloned()
        .collect()
}

fn current_file_metadata_stamp(file: &Path) -> Result<FileMetadataStamp, String> {
    let metadata = fs::metadata(file).map_err(|e| {
        format!(
            "{}: Failed to stat '{}': {}",
            "error".red().bold(),
            file.display(),
            e
        )
    })?;
    let modified = metadata.modified().map_err(|e| {
        format!(
            "{}: Failed to read modified time for '{}': {}",
            "error".red().bold(),
            file.display(),
            e
        )
    })?;
    let duration = modified.duration_since(UNIX_EPOCH).map_err(|e| {
        format!(
            "{}: Invalid modified time for '{}': {}",
            "error".red().bold(),
            file.display(),
            e
        )
    })?;

    Ok(FileMetadataStamp {
        len: metadata.len(),
        modified_secs: duration.as_secs(),
        modified_nanos: duration.subsec_nanos(),
    })
}

fn load_parsed_file_cache_entry(
    project_root: &Path,
    file: &Path,
) -> Result<Option<ParsedFileCacheEntry>, String> {
    let path = parsed_file_cache_path(project_root, file);
    let entry: ParsedFileCacheEntry = match read_cache_blob(&path, "parse cache")? {
        Some(entry) => entry,
        None => return Ok(None),
    };

    if entry.schema != PARSE_CACHE_SCHEMA || entry.compiler_version != env!("CARGO_PKG_VERSION") {
        return Ok(None);
    }

    Ok(Some(entry))
}

fn save_parsed_file_cache(
    project_root: &Path,
    file: &Path,
    entry: &ParsedFileCacheEntry,
) -> Result<(), String> {
    let path = parsed_file_cache_path(project_root, file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create parse cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }
    write_cache_blob(&path, "parse cache", entry)
}

const IMPORT_CHECK_CACHE_SCHEMA: &str = "v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImportCheckCacheEntry {
    schema: String,
    compiler_version: String,
    import_check_fingerprint: String,
    rewrite_context_fingerprint: String,
}

fn compute_import_check_fingerprint(
    namespace: &str,
    imports: &[ImportDecl],
    referenced_symbols: &[String],
    qualified_symbol_refs: &[Vec<String>],
) -> String {
    let mut hasher = stable_hasher();
    namespace.hash(&mut hasher);
    hash_imports(imports, &mut hasher);
    for symbol in referenced_symbols {
        symbol.hash(&mut hasher);
    }
    for path in qualified_symbol_refs {
        path.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn import_check_cache_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = stable_hasher();
    file.hash(&mut hasher);
    project_root
        .join(".apexcache")
        .join("import_check")
        .join(format!("{:016x}.json", hasher.finish()))
}

fn load_import_check_cache_hit(
    project_root: &Path,
    file: &Path,
    import_check_fingerprint: &str,
    rewrite_context_fingerprint: &str,
) -> Result<bool, String> {
    let path = import_check_cache_path(project_root, file);
    let entry: ImportCheckCacheEntry = match read_cache_blob(&path, "import-check cache")? {
        Some(entry) => entry,
        None => return Ok(false),
    };

    Ok(entry.schema == IMPORT_CHECK_CACHE_SCHEMA
        && entry.compiler_version == env!("CARGO_PKG_VERSION")
        && entry.import_check_fingerprint == import_check_fingerprint
        && entry.rewrite_context_fingerprint == rewrite_context_fingerprint)
}

fn save_import_check_cache_hit(
    project_root: &Path,
    file: &Path,
    import_check_fingerprint: &str,
    rewrite_context_fingerprint: &str,
) -> Result<(), String> {
    let path = import_check_cache_path(project_root, file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create import-check cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }

    let entry = ImportCheckCacheEntry {
        schema: IMPORT_CHECK_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        import_check_fingerprint: import_check_fingerprint.to_string(),
        rewrite_context_fingerprint: rewrite_context_fingerprint.to_string(),
    };
    write_cache_blob(&path, "import-check cache", &entry)
}

fn dependency_graph_cache_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".apexcache")
        .join("dependency_graph")
        .join("latest.json")
}

fn load_dependency_graph_cache(
    project_root: &Path,
) -> Result<Option<DependencyGraphCache>, String> {
    let path = dependency_graph_cache_path(project_root);
    let cache: DependencyGraphCache = match read_cache_blob(&path, "dependency graph cache")? {
        Some(cache) => cache,
        None => return Ok(None),
    };
    if cache.schema != DEPENDENCY_GRAPH_CACHE_SCHEMA
        || cache.compiler_version != env!("CARGO_PKG_VERSION")
    {
        return Ok(None);
    }
    Ok(Some(cache))
}

fn save_dependency_graph_cache(
    project_root: &Path,
    cache: &DependencyGraphCache,
) -> Result<(), String> {
    let path = dependency_graph_cache_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create dependency graph cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }

    write_cache_blob(&path, "dependency graph cache", cache)
}

fn semantic_summary_cache_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".apexcache")
        .join("semantic_summary")
        .join("latest.json")
}

fn load_semantic_summary_cache(
    project_root: &Path,
) -> Result<Option<SemanticSummaryCache>, String> {
    let path = semantic_summary_cache_path(project_root);
    let cache: SemanticSummaryCache = match read_cache_blob(&path, "semantic summary cache")? {
        Some(cache) => cache,
        None => return Ok(None),
    };
    if cache.schema != SEMANTIC_SUMMARY_CACHE_SCHEMA
        || cache.compiler_version != env!("CARGO_PKG_VERSION")
    {
        return Ok(None);
    }
    Ok(Some(cache))
}

fn save_semantic_summary_cache(
    project_root: &Path,
    cache: &SemanticSummaryCache,
) -> Result<(), String> {
    let path = semantic_summary_cache_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create semantic summary cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }

    write_cache_blob(&path, "semantic summary cache", cache)
}

fn typecheck_summary_cache_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".apexcache")
        .join("typecheck_summary")
        .join("latest.json")
}

fn load_typecheck_summary_cache(
    project_root: &Path,
) -> Result<Option<TypecheckSummaryCache>, String> {
    let path = typecheck_summary_cache_path(project_root);
    let cache: TypecheckSummaryCache = match read_cache_blob(&path, "typecheck summary cache")? {
        Some(cache) => cache,
        None => return Ok(None),
    };
    if cache.schema != TYPECHECK_SUMMARY_CACHE_SCHEMA
        || cache.compiler_version != env!("CARGO_PKG_VERSION")
    {
        return Ok(None);
    }
    Ok(Some(cache))
}

fn save_typecheck_summary_cache(
    project_root: &Path,
    cache: &TypecheckSummaryCache,
) -> Result<(), String> {
    let path = typecheck_summary_cache_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create typecheck summary cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }

    write_cache_blob(&path, "typecheck summary cache", cache)
}

const REWRITE_CACHE_SCHEMA: &str = "v8";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RewrittenFileCacheEntry {
    schema: String,
    compiler_version: String,
    semantic_fingerprint: String,
    rewrite_context_fingerprint: String,
    rewritten_program: Program,
}

const OBJECT_CACHE_SCHEMA: &str = "v3";
const LINK_MANIFEST_CACHE_SCHEMA: &str = "v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObjectCacheEntry {
    schema: String,
    compiler_version: String,
    semantic_fingerprint: String,
    rewrite_context_fingerprint: String,
    object_build_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct LinkManifestCache {
    schema: String,
    compiler_version: String,
    link_fingerprint: String,
    link_inputs: Vec<PathBuf>,
}

fn rewritten_file_cache_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    file.hash(&mut hasher);
    project_root
        .join(".apexcache")
        .join("rewritten")
        .join(format!("{:016x}.json", hasher.finish()))
}

fn namespace_prefixes(namespace: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let mut current = namespace.trim();
    while !current.is_empty() {
        prefixes.push(current.to_string());
        if let Some((prefix, _)) = current.rsplit_once('.') {
            current = prefix;
        } else {
            break;
        }
    }
    prefixes
}

fn hash_imports(imports: &[ImportDecl], hasher: &mut impl Hasher) {
    let mut normalized = imports
        .iter()
        .map(|import| {
            (
                import.path.clone(),
                import.alias.clone().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    normalized.sort();
    for (path, alias) in normalized {
        path.hash(hasher);
        alias.hash(hasher);
    }
}

fn hash_filtered_namespace_map(
    map: &HashMap<String, HashSet<String>>,
    relevant_namespaces: &HashSet<String>,
    hasher: &mut impl Hasher,
) {
    let mut entries = map
        .iter()
        .filter(|(namespace, _)| relevant_namespaces.contains(*namespace))
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (namespace, symbols) in entries {
        namespace.hash(hasher);
        let mut values = symbols.iter().collect::<Vec<_>>();
        values.sort();
        for value in values {
            value.hash(hasher);
        }
    }
}

fn hash_filtered_global_map(
    map: &HashMap<String, String>,
    relevant_namespaces: &HashSet<String>,
    hasher: &mut impl Hasher,
) {
    let mut entries = map
        .iter()
        .filter(|(_, namespace)| relevant_namespaces.contains(*namespace))
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.cmp(b.1)));
    for (symbol, namespace) in entries {
        symbol.hash(hasher);
        namespace.hash(hasher);
    }
}

fn compute_namespace_api_fingerprints(
    parsed_files: &[ParsedProjectUnit],
) -> HashMap<String, String> {
    let mut grouped: HashMap<String, Vec<(&PathBuf, &str)>> = HashMap::new();
    for unit in parsed_files {
        grouped
            .entry(unit.namespace.clone())
            .or_default()
            .push((&unit.file, unit.api_fingerprint.as_str()));
    }

    let mut result = HashMap::new();
    for (namespace, mut entries) in grouped {
        entries.sort_by(|a, b| a.0.cmp(b.0));
        let mut hasher = stable_hasher();
        namespace.hash(&mut hasher);
        for (file, api_fingerprint) in entries {
            file.hash(&mut hasher);
            api_fingerprint.hash(&mut hasher);
        }
        result.insert(namespace, format!("{:016x}", hasher.finish()));
    }
    result
}

fn hash_namespace_api_fingerprints(
    map: &HashMap<String, String>,
    relevant_namespaces: &HashSet<String>,
    hasher: &mut impl Hasher,
) {
    let mut entries = map
        .iter()
        .filter(|(namespace, _)| relevant_namespaces.contains(*namespace))
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (namespace, fingerprint) in entries {
        namespace.hash(hasher);
        fingerprint.hash(hasher);
    }
}

fn hash_file_api_fingerprint(
    file_api_fingerprints: &HashMap<PathBuf, String>,
    file: &Path,
    hasher: &mut impl Hasher,
) {
    if let Some(fingerprint) = file_api_fingerprints.get(file) {
        file.hash(hasher);
        fingerprint.hash(hasher);
    }
}

#[allow(clippy::too_many_arguments)]
fn import_path_owner_file<'a>(
    path: &str,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &'a HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &'a HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &'a HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &'a HashMap<String, PathBuf>,
) -> Option<&'a PathBuf> {
    let (namespace, symbol) = path.rsplit_once('.')?;

    if let Some((_, _, owner_file)) = resolve_exact_imported_symbol_owner(
        namespace,
        symbol,
        global_function_map,
        global_function_file_map,
        global_class_map,
        global_class_file_map,
        global_enum_map,
        global_enum_file_map,
        global_module_map,
        global_module_file_map,
    ) {
        return Some(owner_file);
    }

    if let Some((enum_namespace, enum_name)) = namespace.rsplit_once('.') {
        if let Some((_, _, owner_file)) = resolve_exact_imported_symbol_file(
            enum_namespace,
            enum_name,
            global_enum_map,
            global_enum_file_map,
        ) {
            return Some(owner_file);
        }
    }

    if global_function_map
        .get(symbol)
        .is_some_and(|owner_ns| owner_ns == namespace)
    {
        return global_function_file_map.get(symbol);
    }
    if global_class_map
        .get(symbol)
        .is_some_and(|owner_ns| owner_ns == namespace)
    {
        return global_class_file_map.get(symbol);
    }
    if global_enum_map
        .get(symbol)
        .is_some_and(|owner_ns| owner_ns == namespace)
    {
        return global_enum_file_map.get(symbol);
    }
    if global_module_map
        .get(symbol)
        .is_some_and(|owner_ns| owner_ns == namespace)
    {
        return global_module_file_map.get(symbol);
    }

    None
}

struct RewriteFingerprintContext<'a> {
    namespace_functions: &'a HashMap<String, HashSet<String>>,
    namespace_function_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    global_function_map: &'a HashMap<String, String>,
    global_function_file_map: &'a HashMap<String, PathBuf>,
    namespace_classes: &'a HashMap<String, HashSet<String>>,
    namespace_class_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    global_class_map: &'a HashMap<String, String>,
    global_class_file_map: &'a HashMap<String, PathBuf>,
    global_enum_map: &'a HashMap<String, String>,
    global_enum_file_map: &'a HashMap<String, PathBuf>,
    namespace_modules: &'a HashMap<String, HashSet<String>>,
    namespace_module_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    global_module_map: &'a HashMap<String, String>,
    global_module_file_map: &'a HashMap<String, PathBuf>,
    namespace_api_fingerprints: &'a HashMap<String, String>,
    file_api_fingerprints: &'a HashMap<PathBuf, String>,
}

struct DependencyResolutionContext<'a> {
    namespace_files_map: &'a HashMap<String, Vec<PathBuf>>,
    namespace_function_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    namespace_class_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    namespace_module_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    global_function_map: &'a HashMap<String, String>,
    global_function_file_map: &'a HashMap<String, PathBuf>,
    global_class_map: &'a HashMap<String, String>,
    global_class_file_map: &'a HashMap<String, PathBuf>,
    global_enum_map: &'a HashMap<String, String>,
    global_enum_file_map: &'a HashMap<String, PathBuf>,
    global_module_map: &'a HashMap<String, String>,
    global_module_file_map: &'a HashMap<String, PathBuf>,
}

fn import_lookup_key(import: &ImportDecl) -> String {
    import
        .alias
        .as_ref()
        .cloned()
        .unwrap_or_else(|| import.path.rsplit('.').next().unwrap_or("").to_string())
}

fn resolve_function_file_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
) -> Option<PathBuf> {
    if member_parts.is_empty() {
        return None;
    }

    let function_name = member_parts.last().expect("checked non-empty");
    let module_tail = if member_parts.len() > 1 {
        Some(member_parts[..member_parts.len() - 1].join("__"))
    } else {
        None
    };

    let mut owner_namespaces: HashSet<&str> = HashSet::new();
    owner_namespaces.extend(global_function_map.values().map(String::as_str));

    for owner_ns in owner_namespaces {
        let imported_module_prefix = if namespace_path == owner_ns {
            String::new()
        } else if let Some(suffix) = namespace_path.strip_prefix(owner_ns) {
            if let Some(rest) = suffix.strip_prefix('.') {
                rest.replace('.', "__")
            } else {
                continue;
            }
        } else {
            continue;
        };

        let candidate = match (&*imported_module_prefix, module_tail.as_deref()) {
            ("", None) => function_name.clone(),
            ("", Some(tail)) => format!("{}__{}", tail, function_name),
            (prefix, None) => format!("{}__{}", prefix, function_name),
            (prefix, Some(tail)) => format!("{}__{}__{}", prefix, tail, function_name),
        };

        if global_function_map
            .get(&candidate)
            .is_some_and(|owner| owner == owner_ns)
        {
            if let Some(file) = global_function_file_map.get(&candidate) {
                return Some(file.clone());
            }
        }
    }

    None
}

fn resolve_function_symbol_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    global_function_map: &HashMap<String, String>,
) -> Option<(String, String)> {
    if member_parts.is_empty() {
        return None;
    }

    let function_name = member_parts.last().expect("checked non-empty");
    let module_tail = if member_parts.len() > 1 {
        Some(member_parts[..member_parts.len() - 1].join("__"))
    } else {
        None
    };

    let mut owner_namespaces: HashSet<&str> = HashSet::new();
    owner_namespaces.extend(global_function_map.values().map(String::as_str));

    for owner_ns in owner_namespaces {
        let imported_module_prefix = if namespace_path == owner_ns {
            String::new()
        } else if let Some(suffix) = namespace_path.strip_prefix(owner_ns) {
            if let Some(rest) = suffix.strip_prefix('.') {
                rest.replace('.', "__")
            } else {
                continue;
            }
        } else {
            continue;
        };

        let candidate = match (&*imported_module_prefix, module_tail.as_deref()) {
            ("", None) => function_name.clone(),
            ("", Some(tail)) => format!("{}__{}", tail, function_name),
            (prefix, None) => format!("{}__{}", prefix, function_name),
            (prefix, Some(tail)) => format!("{}__{}__{}", prefix, tail, function_name),
        };

        if global_function_map
            .get(&candidate)
            .is_some_and(|owner| owner == owner_ns)
        {
            return Some((owner_ns.to_string(), candidate));
        }
    }

    None
}

fn resolve_symbol_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    global_function_map: &HashMap<String, String>,
    global_class_map: &HashMap<String, String>,
    global_enum_map: &HashMap<String, String>,
    global_module_map: &HashMap<String, String>,
) -> Option<(String, String)> {
    if let Some(found) =
        resolve_function_symbol_in_namespace_path(namespace_path, member_parts, global_function_map)
    {
        return Some(found);
    }

    if member_parts.is_empty() {
        return None;
    }

    let candidate = member_parts.join("__");
    if let Some(owner_ns) = global_class_map.get(&candidate) {
        if owner_ns == namespace_path {
            return Some((owner_ns.clone(), candidate));
        }
    }
    if let Some(owner_ns) = global_enum_map.get(&candidate) {
        if owner_ns == namespace_path {
            return Some((owner_ns.clone(), candidate));
        }
    }
    if let Some(owner_ns) = global_module_map.get(&candidate) {
        if owner_ns == namespace_path {
            return Some((owner_ns.clone(), candidate));
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn resolve_owner_file_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
) -> Option<PathBuf> {
    if let Some(file) = resolve_function_file_in_namespace_path(
        namespace_path,
        member_parts,
        global_function_map,
        global_function_file_map,
    ) {
        return Some(file);
    }

    if member_parts.is_empty() {
        return None;
    }

    let candidate = member_parts.join("__");
    if global_class_map
        .get(&candidate)
        .is_some_and(|owner_ns| owner_ns == namespace_path)
    {
        return global_class_file_map.get(&candidate).cloned();
    }
    if global_enum_map
        .get(&candidate)
        .is_some_and(|owner_ns| owner_ns == namespace_path)
    {
        return global_enum_file_map.get(&candidate).cloned();
    }
    if global_module_map
        .get(&candidate)
        .is_some_and(|owner_ns| owner_ns == namespace_path)
    {
        return global_module_file_map.get(&candidate).cloned();
    }

    None
}

fn resolve_symbol_owner_files_in_namespace(
    namespace: &str,
    referenced_symbols: &HashSet<String>,
    qualified_symbol_refs: &[Vec<String>],
    ctx: &DependencyResolutionContext<'_>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();

    for symbol in referenced_symbols {
        if ctx
            .global_function_map
            .get(symbol)
            .is_some_and(|owner_ns| owner_ns == namespace)
        {
            if let Some(file) = ctx.global_function_file_map.get(symbol) {
                deps.insert(file.clone());
            }
        }
        if ctx
            .global_class_map
            .get(symbol)
            .is_some_and(|owner_ns| owner_ns == namespace)
        {
            if let Some(file) = ctx.global_class_file_map.get(symbol) {
                deps.insert(file.clone());
            }
        }
        if ctx
            .global_enum_map
            .get(symbol)
            .is_some_and(|owner_ns| owner_ns == namespace)
        {
            if let Some(file) = ctx.global_enum_file_map.get(symbol) {
                deps.insert(file.clone());
            }
        }
        if ctx
            .global_module_map
            .get(symbol)
            .is_some_and(|owner_ns| owner_ns == namespace)
        {
            if let Some(file) = ctx.global_module_file_map.get(symbol) {
                deps.insert(file.clone());
            }
        }
    }

    for path in qualified_symbol_refs {
        if let Some(file) = resolve_owner_file_in_namespace_path(
            namespace,
            path,
            ctx.global_function_map,
            ctx.global_function_file_map,
            ctx.global_class_map,
            ctx.global_class_file_map,
            ctx.global_enum_map,
            ctx.global_enum_file_map,
            ctx.global_module_map,
            ctx.global_module_file_map,
        ) {
            deps.insert(file);
        }
    }

    deps
}

fn resolve_import_dependency_files(
    unit: &ParsedProjectUnit,
    import: &ImportDecl,
    referenced_symbols: &HashSet<String>,
    qualified_symbol_refs: &[Vec<String>],
    ctx: &DependencyResolutionContext<'_>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();

    if import.path.ends_with(".*") {
        let namespace = import.path.trim_end_matches(".*");
        return resolve_symbol_owner_files_in_namespace(
            namespace,
            referenced_symbols,
            qualified_symbol_refs,
            ctx,
        );
    }

    if let Some(owner_file) = import_path_owner_file(
        &import.path,
        ctx.global_function_map,
        ctx.global_function_file_map,
        ctx.global_class_map,
        ctx.global_class_file_map,
        ctx.global_enum_map,
        ctx.global_enum_file_map,
        ctx.global_module_map,
        ctx.global_module_file_map,
    ) {
        deps.insert(owner_file.clone());
        return deps;
    }

    let import_key = import_lookup_key(import);
    let namespace_like_import = ctx.namespace_files_map.contains_key(&import.path)
        || unit
            .imports
            .iter()
            .any(|candidate| candidate.path == import.path && candidate.alias.is_some());
    if namespace_like_import {
        for path in qualified_symbol_refs {
            if path.first().is_some_and(|part| part == &import_key) {
                let rest = &path[1..];
                if let Some(file) = resolve_owner_file_in_namespace_path(
                    &import.path,
                    rest,
                    ctx.global_function_map,
                    ctx.global_function_file_map,
                    ctx.global_class_map,
                    ctx.global_class_file_map,
                    ctx.global_enum_map,
                    ctx.global_enum_file_map,
                    ctx.global_module_map,
                    ctx.global_module_file_map,
                ) {
                    deps.insert(file);
                }
            }
        }
        return deps;
    }

    if let Some((namespace, _)) = import.path.rsplit_once('.') {
        deps.extend(resolve_symbol_owner_files_in_namespace(
            namespace,
            referenced_symbols,
            qualified_symbol_refs,
            ctx,
        ));
    }

    deps
}

fn resolve_direct_dependencies_for_unit(
    unit: &ParsedProjectUnit,
    ctx: &DependencyResolutionContext<'_>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();
    let referenced_symbols: HashSet<String> = unit.referenced_symbols.iter().cloned().collect();

    for symbol in &unit.referenced_symbols {
        if let Some(owner_file) = ctx
            .namespace_function_files
            .get(&unit.namespace)
            .and_then(|map| map.get(symbol))
        {
            if owner_file != &unit.file {
                deps.insert(owner_file.clone());
            }
        }
        if let Some(owner_file) = ctx
            .namespace_class_files
            .get(&unit.namespace)
            .and_then(|map| map.get(symbol))
        {
            if owner_file != &unit.file {
                deps.insert(owner_file.clone());
            }
        }
        if let Some(owner_file) = ctx
            .namespace_module_files
            .get(&unit.namespace)
            .and_then(|map| map.get(symbol))
        {
            if owner_file != &unit.file {
                deps.insert(owner_file.clone());
            }
        }
    }

    for import in &unit.imports {
        deps.extend(resolve_import_dependency_files(
            unit,
            import,
            &referenced_symbols,
            &unit.qualified_symbol_refs,
            ctx,
        ));
    }

    deps.remove(&unit.file);
    deps
}

fn build_file_dependency_graph_incremental(
    parsed_files: &[ParsedProjectUnit],
    ctx: &DependencyResolutionContext<'_>,
    previous: Option<&DependencyGraphCache>,
) -> (HashMap<PathBuf, HashSet<PathBuf>>, usize) {
    let current_api_fingerprints: HashMap<&PathBuf, &str> = parsed_files
        .iter()
        .map(|unit| (&unit.file, unit.api_fingerprint.as_str()))
        .collect();
    let previous_entries = previous
        .map(|cache| {
            cache
                .files
                .iter()
                .map(|entry| (&entry.file, entry))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let previous_reverse_graph = previous
        .map(|cache| {
            let mut reverse: HashMap<&PathBuf, HashSet<&PathBuf>> = HashMap::new();
            for entry in &cache.files {
                reverse.entry(&entry.file).or_default();
                for dep in &entry.direct_dependencies {
                    reverse.entry(dep).or_default().insert(&entry.file);
                }
            }
            reverse
        })
        .unwrap_or_default();

    let mut graph = HashMap::new();
    let mut reused = 0usize;

    for unit in parsed_files {
        let deps = if let Some(previous_entry) = previous_entries.get(&unit.file) {
            let direct_dependency_api_changed =
                previous_entry.direct_dependencies.iter().any(|dep| {
                    previous_entries
                        .get(dep)
                        .and_then(|previous_dep| {
                            current_api_fingerprints
                                .get(dep)
                                .map(|current| previous_dep.api_fingerprint != *current)
                        })
                        .unwrap_or(true)
                });
            let direct_dependent_api_changed = previous_reverse_graph
                .get(&unit.file)
                .into_iter()
                .flatten()
                .any(|dependent| {
                    previous_entries
                        .get(dependent)
                        .and_then(|previous_dependent| {
                            current_api_fingerprints
                                .get(dependent)
                                .map(|current| previous_dependent.api_fingerprint != *current)
                        })
                        .unwrap_or(true)
                });

            if previous_entry.semantic_fingerprint == unit.semantic_fingerprint
                && previous_entry.api_fingerprint == unit.api_fingerprint
                && !direct_dependency_api_changed
                && !direct_dependent_api_changed
            {
                reused += 1;
                previous_entry
                    .direct_dependencies
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>()
            } else {
                resolve_direct_dependencies_for_unit(unit, ctx)
            }
        } else {
            resolve_direct_dependencies_for_unit(unit, ctx)
        };
        graph.insert(unit.file.clone(), deps);
    }

    (graph, reused)
}

fn semantic_check_components(
    parsed_files: &[ParsedProjectUnit],
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> Vec<Vec<PathBuf>> {
    let reverse_graph = build_reverse_dependency_graph(forward_graph);
    let mut remaining: HashSet<PathBuf> =
        parsed_files.iter().map(|unit| unit.file.clone()).collect();
    let mut components = Vec::new();

    while let Some(start) = remaining.iter().next().cloned() {
        let mut component = Vec::new();
        let mut stack = vec![start.clone()];
        remaining.remove(&start);

        while let Some(file) = stack.pop() {
            component.push(file.clone());

            if let Some(next) = forward_graph.get(&file) {
                for dep in next {
                    if remaining.remove(dep) {
                        stack.push(dep.clone());
                    }
                }
            }
            if let Some(next) = reverse_graph.get(&file) {
                for dep in next {
                    if remaining.remove(dep) {
                        stack.push(dep.clone());
                    }
                }
            }
        }

        component.sort();
        components.push(component);
    }

    components.sort_by(|a, b| a.first().cmp(&b.first()));
    components
}

fn build_reverse_dependency_graph(
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> HashMap<PathBuf, HashSet<PathBuf>> {
    let mut reverse = HashMap::new();
    for (file, deps) in forward_graph {
        reverse.entry(file.clone()).or_insert_with(HashSet::new);
        for dep in deps {
            reverse
                .entry(dep.clone())
                .or_insert_with(HashSet::new)
                .insert(file.clone());
        }
    }
    reverse
}

fn transitive_dependents(
    reverse_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
    roots: &HashSet<PathBuf>,
) -> HashSet<PathBuf> {
    let mut out = HashSet::new();
    let mut stack: Vec<PathBuf> = roots.iter().cloned().collect();
    while let Some(file) = stack.pop() {
        if !out.insert(file.clone()) {
            continue;
        }
        if let Some(next) = reverse_graph.get(&file) {
            stack.extend(next.iter().cloned());
        }
    }
    out
}

fn transitive_dependencies(
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
    root: &Path,
) -> HashSet<PathBuf> {
    let mut out = HashSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(file) = stack.pop() {
        if let Some(next) = forward_graph.get(&file) {
            for dep in next {
                if out.insert(dep.clone()) {
                    stack.push(dep.clone());
                }
            }
        }
    }
    out
}

#[allow(dead_code)]
fn precompute_all_transitive_dependencies(
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> HashMap<PathBuf, HashSet<PathBuf>> {
    fn visit(
        file: &PathBuf,
        forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
        memo: &mut HashMap<PathBuf, HashSet<PathBuf>>,
        visiting: &mut HashSet<PathBuf>,
    ) -> HashSet<PathBuf> {
        if let Some(cached) = memo.get(file) {
            return cached.clone();
        }
        if !visiting.insert(file.clone()) {
            return HashSet::new();
        }

        let mut closure = HashSet::new();
        if let Some(deps) = forward_graph.get(file) {
            for dep in deps {
                closure.insert(dep.clone());
                closure.extend(visit(dep, forward_graph, memo, visiting));
            }
        }

        visiting.remove(file);
        memo.insert(file.clone(), closure.clone());
        closure
    }

    let mut memo = HashMap::new();
    let mut visiting = HashSet::new();
    let mut files: Vec<PathBuf> = forward_graph.keys().cloned().collect();
    files.sort();

    for file in files {
        visit(&file, forward_graph, &mut memo, &mut visiting);
    }

    memo
}

fn dependency_graph_cache_from_state(
    parsed_files: &[ParsedProjectUnit],
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> DependencyGraphCache {
    let mut files: Vec<DependencyGraphFileEntry> = parsed_files
        .iter()
        .map(|unit| {
            let mut direct_dependencies = forward_graph
                .get(&unit.file)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect::<Vec<_>>();
            direct_dependencies.sort();
            DependencyGraphFileEntry {
                file: unit.file.clone(),
                semantic_fingerprint: unit.semantic_fingerprint.clone(),
                api_fingerprint: unit.api_fingerprint.clone(),
                direct_dependencies,
            }
        })
        .collect();
    files.sort_by(|a, b| a.file.cmp(&b.file));

    DependencyGraphCache {
        schema: DEPENDENCY_GRAPH_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        files,
    }
}

fn semantic_summary_cache_from_state(
    parsed_files: &[ParsedProjectUnit],
    components: &[Vec<PathBuf>],
    function_effects: HashMap<String, Vec<String>>,
    class_method_effects: HashMap<String, HashMap<String, Vec<String>>>,
    class_mutating_methods: HashMap<String, HashSet<String>>,
) -> SemanticSummaryCache {
    let mut files: Vec<SemanticSummaryFileEntry> = parsed_files
        .iter()
        .map(|unit| SemanticSummaryFileEntry {
            file: unit.file.clone(),
            semantic_fingerprint: unit.semantic_fingerprint.clone(),
            function_names: unit.function_names.clone(),
            class_names: unit.class_names.clone(),
        })
        .collect();
    files.sort_by(|a, b| a.file.cmp(&b.file));

    let class_mutating_methods = class_mutating_methods
        .into_iter()
        .map(|(class_name, methods)| {
            let mut methods = methods.into_iter().collect::<Vec<_>>();
            methods.sort();
            (class_name, methods)
        })
        .collect();

    let file_entries: HashMap<&PathBuf, &SemanticSummaryFileEntry> =
        files.iter().map(|entry| (&entry.file, entry)).collect();
    let current_fingerprints: HashMap<PathBuf, String> = parsed_files
        .iter()
        .map(|unit| (unit.file.clone(), unit.semantic_fingerprint.clone()))
        .collect();
    let mut components = components
        .iter()
        .map(|component| {
            let mut function_names = Vec::new();
            let mut class_names = Vec::new();
            for file in component {
                if let Some(entry) = file_entries.get(file) {
                    function_names.extend(entry.function_names.iter().cloned());
                    class_names.extend(entry.class_names.iter().cloned());
                }
            }
            function_names.sort();
            function_names.dedup();
            class_names.sort();
            class_names.dedup();
            SemanticSummaryComponentEntry {
                component_fingerprint: component_fingerprint(component, &current_fingerprints),
                files: component.clone(),
                function_names,
                class_names,
            }
        })
        .collect::<Vec<_>>();
    components.sort_by(|a, b| a.component_fingerprint.cmp(&b.component_fingerprint));

    SemanticSummaryCache {
        schema: SEMANTIC_SUMMARY_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        files,
        components,
        function_effects,
        class_method_effects,
        class_mutating_methods,
    }
}

fn component_fingerprint(
    component_files: &[PathBuf],
    current_fingerprints: &HashMap<PathBuf, String>,
) -> String {
    let mut hasher = stable_hasher();
    for file in component_files {
        file.hash(&mut hasher);
        if let Some(fingerprint) = current_fingerprints.get(file) {
            fingerprint.hash(&mut hasher);
        }
    }
    format!("{:016x}", hasher.finish())
}

fn typecheck_summary_cache_from_state(
    current_fingerprints: &HashMap<PathBuf, String>,
    components: &[Vec<PathBuf>],
) -> TypecheckSummaryCache {
    let mut files = Vec::new();
    for component in components {
        let component_fingerprint = component_fingerprint(component, current_fingerprints);
        for file in component {
            if let Some(semantic_fingerprint) = current_fingerprints.get(file) {
                files.push(TypecheckSummaryFileEntry {
                    file: file.clone(),
                    semantic_fingerprint: semantic_fingerprint.clone(),
                    component_fingerprint: component_fingerprint.clone(),
                });
            }
        }
    }
    files.sort_by(|a, b| a.file.cmp(&b.file));

    TypecheckSummaryCache {
        schema: TYPECHECK_SUMMARY_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        files,
    }
}

fn typecheck_summary_cache_matches(
    cache: &TypecheckSummaryCache,
    current_fingerprints: &HashMap<PathBuf, String>,
    components: &[Vec<PathBuf>],
) -> bool {
    let cached_entries: HashMap<&PathBuf, &TypecheckSummaryFileEntry> = cache
        .files
        .iter()
        .map(|entry| (&entry.file, entry))
        .collect();

    if cached_entries.len() != current_fingerprints.len() {
        return false;
    }

    for component in components {
        let current_component_fingerprint = component_fingerprint(component, current_fingerprints);
        for file in component {
            let Some(entry) = cached_entries.get(file) else {
                return false;
            };
            let Some(current_semantic_fingerprint) = current_fingerprints.get(file) else {
                return false;
            };
            if entry.semantic_fingerprint != *current_semantic_fingerprint
                || entry.component_fingerprint != current_component_fingerprint
            {
                return false;
            }
        }
    }

    true
}

fn reusable_component_fingerprints(
    cache: &TypecheckSummaryCache,
    current_fingerprints: &HashMap<PathBuf, String>,
    components: &[Vec<PathBuf>],
) -> HashSet<String> {
    let cached_entries: HashMap<&PathBuf, &TypecheckSummaryFileEntry> = cache
        .files
        .iter()
        .map(|entry| (&entry.file, entry))
        .collect();

    let mut reusable = HashSet::new();
    for component in components {
        let current_component_fingerprint = component_fingerprint(component, current_fingerprints);
        let matches = component.iter().all(|file| {
            cached_entries.get(file).is_some_and(|entry| {
                current_fingerprints.get(file).is_some_and(|current_fp| {
                    entry.semantic_fingerprint == *current_fp
                        && entry.component_fingerprint == current_component_fingerprint
                })
            })
        });
        if matches {
            reusable.insert(current_component_fingerprint);
        }
    }
    reusable
}

fn merge_reusable_component_semantic_data(
    cache: &SemanticSummaryCache,
    reusable_component_fingerprints: &HashSet<String>,
) -> (
    FunctionEffectsSummary,
    ClassMethodEffectsSummary,
    HashMap<String, HashSet<String>>,
) {
    let reusable_components = cache
        .components
        .iter()
        .filter(|component| {
            reusable_component_fingerprints.contains(&component.component_fingerprint)
        })
        .collect::<Vec<_>>();

    let mut function_effects = HashMap::new();
    let mut class_method_effects = HashMap::new();
    let mut class_mutating_methods = HashMap::new();

    for component in reusable_components {
        for function_name in &component.function_names {
            if let Some(effects) = cache.function_effects.get(function_name) {
                function_effects.insert(function_name.clone(), effects.clone());
            }
        }
        for class_name in &component.class_names {
            if let Some(methods) = cache.class_method_effects.get(class_name) {
                class_method_effects.insert(class_name.clone(), methods.clone());
            }
            if let Some(methods) = cache.class_mutating_methods.get(class_name) {
                class_mutating_methods
                    .insert(class_name.clone(), methods.iter().cloned().collect());
            }
        }
    }

    (
        function_effects,
        class_method_effects,
        class_mutating_methods,
    )
}

fn semantic_program_for_component(
    rewritten_files: &[RewrittenProjectUnit],
    component_files: &HashSet<PathBuf>,
    full_files: &HashSet<PathBuf>,
) -> Program {
    let mut program = Program {
        package: None,
        declarations: Vec::new(),
    };

    for unit in rewritten_files {
        if !component_files.contains(&unit.file) {
            continue;
        }
        let source_program = if full_files.contains(&unit.file) {
            unit.program.clone()
        } else {
            unit.api_program.clone()
        };
        program.declarations.extend(source_program.declarations);
    }

    program
}

fn render_type_errors(errors: Vec<typeck::TypeError>) -> String {
    let mut rendered = String::new();
    for error in errors {
        rendered.push_str(&format!("\x1b[1;31merror\x1b[0m: {}\n", error.message));
    }
    rendered
}

fn render_borrow_errors(errors: Vec<borrowck::BorrowError>) -> String {
    let mut rendered = String::new();
    for error in errors {
        rendered.push_str(&format!(
            "\x1b[1;31merror[E0505]\x1b[0m: {}\n",
            error.message
        ));
    }
    rendered
}

fn semantic_seed_data_from_cache(
    cache: &SemanticSummaryCache,
    current_fingerprints: &HashMap<PathBuf, String>,
    full_files: &HashSet<PathBuf>,
) -> (
    FunctionEffectsSummary,
    ClassMethodEffectsSummary,
    HashMap<String, HashSet<String>>,
) {
    let file_entries: HashMap<&PathBuf, &SemanticSummaryFileEntry> = cache
        .files
        .iter()
        .map(|entry| (&entry.file, entry))
        .collect();

    let valid_seed_entries: Vec<&SemanticSummaryFileEntry> = current_fingerprints
        .iter()
        .filter(|(file, current_fp)| {
            !full_files.contains(*file)
                && file_entries
                    .get(*file)
                    .is_some_and(|entry| entry.semantic_fingerprint == **current_fp)
        })
        .filter_map(|(file, _)| file_entries.get(file).copied())
        .collect();

    let mut function_effects = HashMap::new();
    let mut class_method_effects = HashMap::new();
    let mut class_mutating_methods = HashMap::new();

    for entry in valid_seed_entries {
        for function_name in &entry.function_names {
            if let Some(effects) = cache.function_effects.get(function_name) {
                function_effects.insert(function_name.clone(), effects.clone());
            }
        }
        for class_name in &entry.class_names {
            if let Some(methods) = cache.class_method_effects.get(class_name) {
                class_method_effects.insert(class_name.clone(), methods.clone());
            }
            if let Some(methods) = cache.class_mutating_methods.get(class_name) {
                class_mutating_methods
                    .insert(class_name.clone(), methods.iter().cloned().collect());
            }
        }
    }

    (
        function_effects,
        class_method_effects,
        class_mutating_methods,
    )
}

fn compute_rewrite_context_fingerprint_for_unit(
    unit: &ParsedProjectUnit,
    entry_namespace: &str,
    ctx: &RewriteFingerprintContext<'_>,
) -> String {
    let mut relevant_namespaces: HashSet<String> = HashSet::new();

    let mut hasher = stable_hasher();
    entry_namespace.hash(&mut hasher);
    unit.namespace.hash(&mut hasher);
    hash_imports(&unit.imports, &mut hasher);
    let referenced_symbols: HashSet<String> = unit.referenced_symbols.iter().cloned().collect();
    let mut referenced_symbol_list = referenced_symbols.iter().collect::<Vec<_>>();
    referenced_symbol_list.sort();
    for symbol in referenced_symbol_list {
        if let Some(owner_file) = ctx
            .namespace_function_files
            .get(&unit.namespace)
            .and_then(|map| map.get(symbol))
        {
            if owner_file != &unit.file {
                hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
            }
        }
        if let Some(owner_file) = ctx
            .namespace_class_files
            .get(&unit.namespace)
            .and_then(|map| map.get(symbol))
        {
            if owner_file != &unit.file {
                hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
            }
        }
        if let Some(owner_file) = ctx
            .namespace_module_files
            .get(&unit.namespace)
            .and_then(|map| map.get(symbol))
        {
            if owner_file != &unit.file {
                hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
            }
        }
    }
    let empty_namespace_files_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let empty_namespace_function_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let empty_namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let empty_namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let dependency_ctx = DependencyResolutionContext {
        namespace_files_map: &empty_namespace_files_map,
        namespace_function_files: &empty_namespace_function_files,
        namespace_class_files: &empty_namespace_class_files,
        namespace_module_files: &empty_namespace_module_files,
        global_function_map: ctx.global_function_map,
        global_function_file_map: ctx.global_function_file_map,
        global_class_map: ctx.global_class_map,
        global_class_file_map: ctx.global_class_file_map,
        global_enum_map: ctx.global_enum_map,
        global_enum_file_map: ctx.global_enum_file_map,
        global_module_map: ctx.global_module_map,
        global_module_file_map: ctx.global_module_file_map,
    };

    for import in &unit.imports {
        if import.path.ends_with(".*") {
            let namespace = import.path.trim_end_matches(".*");
            let owner_files = resolve_symbol_owner_files_in_namespace(
                namespace,
                &referenced_symbols,
                &unit.qualified_symbol_refs,
                &dependency_ctx,
            );
            if owner_files.is_empty() {
                relevant_namespaces.insert(namespace.to_string());
                for prefix in namespace_prefixes(namespace) {
                    relevant_namespaces.insert(prefix);
                }
            } else {
                let mut owner_files = owner_files.into_iter().collect::<Vec<_>>();
                owner_files.sort();
                for owner_file in owner_files {
                    hash_file_api_fingerprint(ctx.file_api_fingerprints, &owner_file, &mut hasher);
                }
            }
            continue;
        }

        if ctx.namespace_api_fingerprints.contains_key(&import.path) {
            let import_key = import_lookup_key(import);
            let mut matched_owner_files = HashSet::new();
            for path in &unit.qualified_symbol_refs {
                if path.first().is_some_and(|part| part == &import_key) {
                    let rest = &path[1..];
                    if let Some(owner_file) = resolve_function_file_in_namespace_path(
                        &import.path,
                        rest,
                        ctx.global_function_map,
                        ctx.global_function_file_map,
                    ) {
                        matched_owner_files.insert(owner_file);
                    }
                }
            }
            if matched_owner_files.is_empty() {
                relevant_namespaces.insert(import.path.clone());
                for prefix in namespace_prefixes(&import.path) {
                    relevant_namespaces.insert(prefix);
                }
            } else {
                let mut matched_owner_files = matched_owner_files.into_iter().collect::<Vec<_>>();
                matched_owner_files.sort();
                for owner_file in matched_owner_files {
                    hash_file_api_fingerprint(ctx.file_api_fingerprints, &owner_file, &mut hasher);
                }
            }
            continue;
        }

        if let Some(owner_file) = import_path_owner_file(
            &import.path,
            ctx.global_function_map,
            ctx.global_function_file_map,
            ctx.global_class_map,
            ctx.global_class_file_map,
            ctx.global_enum_map,
            ctx.global_enum_file_map,
            ctx.global_module_map,
            ctx.global_module_file_map,
        ) {
            hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
            continue;
        }

        let imported_namespace = if import.path.contains('.') {
            import.path.rsplit_once('.').map(|(ns, _)| ns).unwrap_or("")
        } else {
            import.path.as_str()
        };
        if ctx
            .namespace_api_fingerprints
            .contains_key(imported_namespace)
        {
            relevant_namespaces.insert(imported_namespace.to_string());
            for prefix in namespace_prefixes(imported_namespace) {
                relevant_namespaces.insert(prefix);
            }
        }
    }

    hash_filtered_namespace_map(ctx.namespace_functions, &relevant_namespaces, &mut hasher);
    hash_filtered_global_map(ctx.global_function_map, &relevant_namespaces, &mut hasher);
    hash_filtered_namespace_map(ctx.namespace_classes, &relevant_namespaces, &mut hasher);
    hash_filtered_global_map(ctx.global_class_map, &relevant_namespaces, &mut hasher);
    hash_filtered_namespace_map(ctx.namespace_modules, &relevant_namespaces, &mut hasher);
    hash_filtered_global_map(ctx.global_module_map, &relevant_namespaces, &mut hasher);
    hash_namespace_api_fingerprints(
        ctx.namespace_api_fingerprints,
        &relevant_namespaces,
        &mut hasher,
    );
    format!("{:016x}", hasher.finish())
}

fn compute_semantic_project_fingerprint(
    config: &ProjectConfig,
    parsed_files: &[ParsedProjectUnit],
    emit_llvm: bool,
    do_check: bool,
) -> String {
    let mut hasher = stable_hasher();
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    config.name.hash(&mut hasher);
    config.version.hash(&mut hasher);
    config.entry.hash(&mut hasher);
    config.output.hash(&mut hasher);
    config.opt_level.hash(&mut hasher);
    config.target.hash(&mut hasher);
    format!("{:?}", config.output_kind).hash(&mut hasher);
    config.link_search.hash(&mut hasher);
    config.link_libs.hash(&mut hasher);
    config.link_args.hash(&mut hasher);
    emit_llvm.hash(&mut hasher);
    do_check.hash(&mut hasher);

    for unit in parsed_files {
        unit.file.hash(&mut hasher);
        unit.semantic_fingerprint.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

fn collect_active_symbols(program: &Program) -> HashSet<String> {
    fn collect_decl_active_symbols(
        decl: &Decl,
        module_prefix: Option<&str>,
        symbols: &mut HashSet<String>,
    ) {
        match decl {
            Decl::Function(func) => {
                let name = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, func.name))
                    .unwrap_or_else(|| func.name.clone());
                symbols.insert(name);
            }
            Decl::Class(class) => {
                let name = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, class.name))
                    .unwrap_or_else(|| class.name.clone());
                symbols.insert(name);
            }
            Decl::Enum(en) => {
                let name = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, en.name))
                    .unwrap_or_else(|| en.name.clone());
                symbols.insert(name);
            }
            Decl::Module(module) => {
                let module_name = module_prefix
                    .map(|prefix| format!("{}__{}", prefix, module.name))
                    .unwrap_or_else(|| module.name.clone());
                symbols.insert(module_name.clone());
                for inner in &module.declarations {
                    collect_decl_active_symbols(&inner.node, Some(&module_name), symbols);
                }
            }
            Decl::Import(_) | Decl::Interface(_) => {}
        }
    }

    let mut symbols = HashSet::new();
    for decl in &program.declarations {
        collect_decl_active_symbols(&decl.node, None, &mut symbols);
    }
    symbols
}

fn load_rewritten_file_cache(
    project_root: &Path,
    file: &Path,
    semantic_fingerprint: &str,
    rewrite_context_fingerprint: &str,
) -> Result<Option<Program>, String> {
    let path = rewritten_file_cache_path(project_root, file);
    let entry: RewrittenFileCacheEntry = match read_cache_blob(&path, "rewrite cache")? {
        Some(entry) => entry,
        None => return Ok(None),
    };

    if entry.schema != REWRITE_CACHE_SCHEMA
        || entry.compiler_version != env!("CARGO_PKG_VERSION")
        || entry.semantic_fingerprint != semantic_fingerprint
        || entry.rewrite_context_fingerprint != rewrite_context_fingerprint
    {
        return Ok(None);
    }

    Ok(Some(entry.rewritten_program))
}

fn save_rewritten_file_cache(
    project_root: &Path,
    file: &Path,
    semantic_fingerprint: &str,
    rewrite_context_fingerprint: &str,
    rewritten_program: &Program,
) -> Result<(), String> {
    let path = rewritten_file_cache_path(project_root, file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create rewrite cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }

    let entry = RewrittenFileCacheEntry {
        schema: REWRITE_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        semantic_fingerprint: semantic_fingerprint.to_string(),
        rewrite_context_fingerprint: rewrite_context_fingerprint.to_string(),
        rewritten_program: rewritten_program.clone(),
    };
    write_cache_blob(&path, "rewrite cache", &entry)
}

fn object_ext() -> &'static str {
    #[cfg(windows)]
    {
        "obj"
    }
    #[cfg(not(windows))]
    {
        "o"
    }
}

fn object_cache_object_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = stable_hasher();
    file.hash(&mut hasher);
    project_root
        .join(".apexcache")
        .join("objects")
        .join(format!("{:016x}.{}", hasher.finish(), object_ext()))
}

fn object_cache_meta_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = stable_hasher();
    file.hash(&mut hasher);
    project_root
        .join(".apexcache")
        .join("objects")
        .join(format!("{:016x}.json", hasher.finish()))
}

#[derive(Debug, Clone)]
struct ObjectCachePaths {
    object_path: PathBuf,
    meta_path: PathBuf,
}

fn object_cache_paths(project_root: &Path, file: &Path) -> ObjectCachePaths {
    ObjectCachePaths {
        object_path: object_cache_object_path(project_root, file),
        meta_path: object_cache_meta_path(project_root, file),
    }
}

fn link_manifest_cache_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".apexcache")
        .join("link")
        .join("latest.json")
}

fn compute_link_fingerprint(
    output_path: &Path,
    link_inputs: &[PathBuf],
    link: &LinkConfig<'_>,
) -> String {
    let mut hasher = stable_hasher();
    let linker = detect_linker_flavor()
        .map(|flavor| flavor.cache_key())
        .unwrap_or("missing");
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    output_path.hash(&mut hasher);
    link.opt_level.hash(&mut hasher);
    link.target.hash(&mut hasher);
    std::mem::discriminant(&link.output_kind).hash(&mut hasher);
    link.link_search.hash(&mut hasher);
    link.link_libs.hash(&mut hasher);
    link.link_args.hash(&mut hasher);
    link_inputs.hash(&mut hasher);
    linker.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn load_link_manifest_cache(project_root: &Path) -> Option<LinkManifestCache> {
    let path = link_manifest_cache_path(project_root);
    let cache: LinkManifestCache = read_cache_blob(&path, "link manifest cache").ok()??;
    if cache.schema != LINK_MANIFEST_CACHE_SCHEMA
        || cache.compiler_version != env!("CARGO_PKG_VERSION")
    {
        return None;
    }
    Some(cache)
}

fn save_link_manifest_cache(project_root: &Path, cache: &LinkManifestCache) -> Result<(), String> {
    let path = link_manifest_cache_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create link manifest cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }

    write_cache_blob(&path, "link manifest cache", cache)
}

fn should_skip_final_link(
    previous_manifest: Option<&LinkManifestCache>,
    current_manifest: &LinkManifestCache,
    output_path: &Path,
    object_cache_miss_count: usize,
) -> bool {
    object_cache_miss_count == 0
        && output_path.exists()
        && previous_manifest.is_some_and(|manifest| manifest == current_manifest)
}

fn compute_object_build_fingerprint(link: &LinkConfig<'_>) -> String {
    let mut hasher = stable_hasher();
    let linker = detect_linker_flavor()
        .map(|flavor| flavor.cache_key())
        .unwrap_or("missing");
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    link.opt_level.hash(&mut hasher);
    link.target.hash(&mut hasher);
    linker.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn load_object_cache_hit(
    cache_paths: &ObjectCachePaths,
    semantic_fingerprint: &str,
    rewrite_context_fingerprint: &str,
    object_build_fingerprint: &str,
) -> Result<Option<PathBuf>, String> {
    if !cache_paths.meta_path.exists() || !cache_paths.object_path.exists() {
        return Ok(None);
    }
    let meta: ObjectCacheEntry = match read_cache_blob(&cache_paths.meta_path, "object cache meta")?
    {
        Some(meta) => meta,
        None => return Ok(None),
    };

    if meta.schema != OBJECT_CACHE_SCHEMA
        || meta.compiler_version != env!("CARGO_PKG_VERSION")
        || meta.semantic_fingerprint != semantic_fingerprint
        || meta.rewrite_context_fingerprint != rewrite_context_fingerprint
        || meta.object_build_fingerprint != object_build_fingerprint
    {
        return Ok(None);
    }

    Ok(Some(cache_paths.object_path.clone()))
}

fn save_object_cache_meta(
    cache_paths: &ObjectCachePaths,
    semantic_fingerprint: &str,
    rewrite_context_fingerprint: &str,
    object_build_fingerprint: &str,
) -> Result<(), String> {
    if let Some(parent) = cache_paths.meta_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create object cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }

    let meta = ObjectCacheEntry {
        schema: OBJECT_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        semantic_fingerprint: semantic_fingerprint.to_string(),
        rewrite_context_fingerprint: rewrite_context_fingerprint.to_string(),
        object_build_fingerprint: object_build_fingerprint.to_string(),
    };
    write_cache_blob(&cache_paths.meta_path, "object cache meta", &meta)
}

fn parse_project_unit(project_root: &Path, file: &Path) -> Result<ParsedProjectUnit, String> {
    let filename = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown.apex");
    let file_metadata = current_file_metadata_stamp(file)?;
    let cached_entry = load_parsed_file_cache_entry(project_root, file)?;
    let (
        namespace,
        program,
        imports,
        api_fingerprint,
        semantic_fingerprint,
        source_fingerprint_for_cache,
        from_parse_cache,
    ) = if let Some(cache) = cached_entry.as_ref() {
        if cache.file_metadata == file_metadata {
            (
                cache.namespace.clone(),
                cache.program.clone(),
                cache.imports.clone(),
                cache.api_fingerprint.clone(),
                cache.semantic_fingerprint.clone(),
                None,
                true,
            )
        } else {
            let source = fs::read_to_string(file).map_err(|e| {
                format!(
                    "{}: Failed to read '{}': {}",
                    "error".red().bold(),
                    file.display(),
                    e
                )
            })?;
            let source_fp = source_fingerprint(&source);
            if cache.source_fingerprint == source_fp {
                (
                    cache.namespace.clone(),
                    cache.program.clone(),
                    cache.imports.clone(),
                    cache.api_fingerprint.clone(),
                    cache.semantic_fingerprint.clone(),
                    None,
                    true,
                )
            } else {
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

                (
                    namespace,
                    program,
                    imports,
                    api_fingerprint,
                    semantic_fingerprint,
                    Some(source_fp),
                    false,
                )
            }
        }
    } else {
        let source = fs::read_to_string(file).map_err(|e| {
            format!(
                "{}: Failed to read '{}': {}",
                "error".red().bold(),
                file.display(),
                e
            )
        })?;
        let source_fp = source_fingerprint(&source);
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

        (
            namespace,
            program,
            imports,
            api_fingerprint,
            semantic_fingerprint,
            Some(source_fp),
            false,
        )
    };

    let mut function_names = Vec::new();
    let mut class_names = Vec::new();
    let mut enum_names = Vec::new();
    let mut module_names = Vec::new();
    let mut referenced_symbols = HashSet::new();
    let mut qualified_symbol_refs: HashSet<Vec<String>> = HashSet::new();

    fn collect_function_names(decl: &Decl, module_prefix: Option<String>, out: &mut Vec<String>) {
        match decl {
            Decl::Function(func) => {
                if let Some(module_name) = module_prefix {
                    out.push(format!("{}__{}", module_name, func.name));
                } else {
                    out.push(func.name.clone());
                }
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner in &module.declarations {
                    collect_function_names(&inner.node, Some(next_prefix.clone()), out);
                }
            }
            Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }

    fn collect_class_names(decl: &Decl, module_prefix: Option<String>, out: &mut Vec<String>) {
        match decl {
            Decl::Class(class) => {
                if let Some(module_name) = module_prefix {
                    out.push(format!("{}__{}", module_name, class.name));
                } else {
                    out.push(class.name.clone());
                }
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner in &module.declarations {
                    collect_class_names(&inner.node, Some(next_prefix.clone()), out);
                }
            }
            Decl::Function(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }

    fn collect_enum_names(decl: &Decl, module_prefix: Option<String>, out: &mut Vec<String>) {
        match decl {
            Decl::Enum(en) => {
                if let Some(module_name) = module_prefix {
                    out.push(format!("{}__{}", module_name, en.name));
                } else {
                    out.push(en.name.clone());
                }
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner in &module.declarations {
                    collect_enum_names(&inner.node, Some(next_prefix.clone()), out);
                }
            }
            Decl::Function(_) | Decl::Class(_) | Decl::Interface(_) | Decl::Import(_) => {}
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

    fn collect_type_refs(ty: &ast::Type, out: &mut HashSet<String>) {
        match ty {
            ast::Type::Named(name) => {
                out.insert(name.clone());
            }
            ast::Type::Generic(name, args) => {
                out.insert(name.clone());
                for arg in args {
                    collect_type_refs(arg, out);
                }
            }
            ast::Type::Function(params, ret) => {
                for param in params {
                    collect_type_refs(param, out);
                }
                collect_type_refs(ret, out);
            }
            ast::Type::Option(inner) => {
                out.insert("Option".to_string());
                collect_type_refs(inner, out);
            }
            ast::Type::List(inner) => {
                out.insert("List".to_string());
                collect_type_refs(inner, out);
            }
            ast::Type::Set(inner) => {
                out.insert("Set".to_string());
                collect_type_refs(inner, out);
            }
            ast::Type::Box(inner) => {
                out.insert("Box".to_string());
                collect_type_refs(inner, out);
            }
            ast::Type::Rc(inner) => {
                out.insert("Rc".to_string());
                collect_type_refs(inner, out);
            }
            ast::Type::Arc(inner) => {
                out.insert("Arc".to_string());
                collect_type_refs(inner, out);
            }
            ast::Type::Ptr(inner) => {
                out.insert("Ptr".to_string());
                collect_type_refs(inner, out);
            }
            ast::Type::Task(inner) => {
                out.insert("Task".to_string());
                collect_type_refs(inner, out);
            }
            ast::Type::Range(inner) => {
                out.insert("Range".to_string());
                collect_type_refs(inner, out);
            }
            ast::Type::Ref(inner) | ast::Type::MutRef(inner) => collect_type_refs(inner, out),
            ast::Type::Result(ok, err) => {
                out.insert("Result".to_string());
                collect_type_refs(ok, out);
                collect_type_refs(err, out);
            }
            ast::Type::Map(ok, err) => {
                out.insert("Map".to_string());
                collect_type_refs(ok, out);
                collect_type_refs(err, out);
            }
            ast::Type::Integer
            | ast::Type::Float
            | ast::Type::Boolean
            | ast::Type::String
            | ast::Type::Char
            | ast::Type::None => {}
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
                    collect_type_refs(ty, out);
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
                out.insert(ty.clone());
                for arg in args {
                    collect_expr_refs(&arg.node, out, qualified_out);
                }
            }
            Expr::Lambda { params, body } => {
                for param in params {
                    collect_type_refs(&param.ty, out);
                }
                collect_expr_refs(&body.node, out, qualified_out);
            }
            Expr::Match { expr, arms } => {
                collect_expr_refs(&expr.node, out, qualified_out);
                for arm in arms {
                    collect_pattern_refs(&arm.pattern, out);
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

    fn collect_pattern_refs(pattern: &Pattern, out: &mut HashSet<String>) {
        if let Pattern::Variant(name, _) = pattern {
            out.insert(name.clone());
        }
    }

    fn collect_stmt_refs(
        stmt: &Stmt,
        out: &mut HashSet<String>,
        qualified_out: &mut HashSet<Vec<String>>,
    ) {
        match stmt {
            Stmt::Let { ty, value, .. } => {
                collect_type_refs(ty, out);
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
                    collect_type_refs(var_type, out);
                }
                collect_expr_refs(&iterable.node, out, qualified_out);
                collect_block_refs(body, out, qualified_out);
            }
            Stmt::Match { expr, arms } => {
                collect_expr_refs(&expr.node, out, qualified_out);
                for arm in arms {
                    collect_pattern_refs(&arm.pattern, out);
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
                for param in &func.params {
                    collect_type_refs(&param.ty, out);
                }
                collect_type_refs(&func.return_type, out);
                collect_block_refs(&func.body, out, qualified_out);
            }
            Decl::Class(class) => {
                if let Some(parent) = &class.extends {
                    out.insert(parent.clone());
                }
                out.extend(class.implements.iter().cloned());
                for field in &class.fields {
                    collect_type_refs(&field.ty, out);
                }
                if let Some(ctor) = &class.constructor {
                    for param in &ctor.params {
                        collect_type_refs(&param.ty, out);
                    }
                    collect_block_refs(&ctor.body, out, qualified_out);
                }
                if let Some(dtor) = &class.destructor {
                    collect_block_refs(&dtor.body, out, qualified_out);
                }
                for method in &class.methods {
                    for param in &method.params {
                        collect_type_refs(&param.ty, out);
                    }
                    collect_type_refs(&method.return_type, out);
                    collect_block_refs(&method.body, out, qualified_out);
                }
            }
            Decl::Enum(en) => {
                for variant in &en.variants {
                    for field in &variant.fields {
                        collect_type_refs(&field.ty, out);
                    }
                }
            }
            Decl::Interface(interface) => {
                out.extend(interface.extends.iter().cloned());
                for method in &interface.methods {
                    for param in &method.params {
                        collect_type_refs(&param.ty, out);
                    }
                    collect_type_refs(&method.return_type, out);
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
                Decl::Function(_) => collect_function_names(&decl.node, None, &mut function_names),
                Decl::Module(module) => {
                    module_names.push(module.name.clone());
                    collect_function_names(&decl.node, None, &mut function_names);
                    collect_class_names(&decl.node, None, &mut class_names);
                    collect_enum_names(&decl.node, None, &mut enum_names);
                }
                Decl::Class(class) => class_names.push(class.name.clone()),
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

/// Build the current project with proper namespace checking
fn build_project(
    _release: bool,
    emit_llvm: bool,
    do_check: bool,
    check_only: bool,
    show_timings: bool,
) -> Result<(), String> {
    let mut build_timings = BuildTimings::new(show_timings);
    let cwd = current_dir_checked()?;
    let project_root = find_project_root(&cwd)
        .ok_or_else(|| format!("{}: No apex.toml found. Are you in a project directory?\nRun `apex new <name>` to create a new project.",
            "error".red().bold()))?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;

    // Validate project
    config.validate(&project_root)?;

    let output_path = project_root.join(&config.output);
    let fingerprint = compute_project_fingerprint(&project_root, &config, emit_llvm, do_check)?;
    if !check_only {
        if let Some(cached) = load_cached_fingerprint(&project_root) {
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
    let files = config.get_source_files(&project_root);
    let mut parsed_files: Vec<ParsedProjectUnit> = Vec::new();
    let mut global_function_map: HashMap<String, String> = HashMap::new(); // func_name -> namespace
    let mut global_function_file_map: HashMap<String, PathBuf> = HashMap::new(); // func_name -> owner file
    let mut global_class_map: HashMap<String, String> = HashMap::new(); // class_name -> namespace
    let mut global_class_file_map: HashMap<String, PathBuf> = HashMap::new(); // class_name -> owner file
    let mut global_enum_map: HashMap<String, String> = HashMap::new(); // enum_name -> namespace
    let mut global_enum_file_map: HashMap<String, PathBuf> = HashMap::new(); // enum_name -> owner file
    let mut global_module_map: HashMap<String, String> = HashMap::new(); // module_name -> namespace
    let mut global_module_file_map: HashMap<String, PathBuf> = HashMap::new(); // module_name -> owner file
    let mut namespace_class_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_enum_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_module_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut function_collisions: Vec<(String, String, String)> = Vec::new();
    let mut class_collisions: Vec<(String, String, String)> = Vec::new();
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

    for unit in parsed_units {
        if unit.from_parse_cache {
            parse_cache_hits += 1;
        }

        // Extract symbol definitions for global maps
        let class_entry = namespace_class_map
            .entry(unit.namespace.clone())
            .or_default();
        let enum_entry = namespace_enum_map
            .entry(unit.namespace.clone())
            .or_default();
        let module_entry = namespace_module_map
            .entry(unit.namespace.clone())
            .or_default();

        for func_name in &unit.function_names {
            if let Some(existing_ns) = global_function_map.get(func_name) {
                if existing_ns != &unit.namespace {
                    function_collisions.push((
                        func_name.clone(),
                        existing_ns.clone(),
                        unit.namespace.clone(),
                    ));
                }
            } else {
                global_function_map.insert(func_name.clone(), unit.namespace.clone());
                global_function_file_map.insert(func_name.clone(), unit.file.clone());
            }
        }
        for class_name in &unit.class_names {
            class_entry.insert(class_name.clone());
            if let Some(existing_ns) = global_class_map.get(class_name) {
                if existing_ns != &unit.namespace {
                    class_collisions.push((
                        class_name.clone(),
                        existing_ns.clone(),
                        unit.namespace.clone(),
                    ));
                }
            } else {
                global_class_map.insert(class_name.clone(), unit.namespace.clone());
                global_class_file_map.insert(class_name.clone(), unit.file.clone());
            }
        }
        for enum_name in &unit.enum_names {
            enum_entry.insert(enum_name.clone());
            global_enum_map.insert(enum_name.clone(), unit.namespace.clone());
            global_enum_file_map.insert(enum_name.clone(), unit.file.clone());
        }
        for module_name in &unit.module_names {
            module_entry.insert(module_name.clone());
            if let Some(existing_ns) = global_module_map.get(module_name) {
                if existing_ns != &unit.namespace {
                    module_collisions.push((
                        module_name.clone(),
                        existing_ns.clone(),
                        unit.namespace.clone(),
                    ));
                }
            } else {
                global_module_map.insert(module_name.clone(), unit.namespace.clone());
                global_module_file_map.insert(module_name.clone(), unit.file.clone());
            }
        }

        parsed_files.push(unit);
    }

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

    let previous_dependency_graph = load_dependency_graph_cache(&project_root)?;
    let mut namespace_files_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut namespace_function_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let mut namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    for unit in &parsed_files {
        namespace_files_map
            .entry(unit.namespace.clone())
            .or_default()
            .push(unit.file.clone());
        let function_entry = namespace_function_files
            .entry(unit.namespace.clone())
            .or_default();
        for name in &unit.function_names {
            function_entry.insert(name.clone(), unit.file.clone());
        }
        let class_entry = namespace_class_files
            .entry(unit.namespace.clone())
            .or_default();
        for name in &unit.class_names {
            class_entry.insert(name.clone(), unit.file.clone());
        }
        let module_entry = namespace_module_files
            .entry(unit.namespace.clone())
            .or_default();
        for name in &unit.module_names {
            module_entry.insert(name.clone(), unit.file.clone());
        }
    }
    for files in namespace_files_map.values_mut() {
        files.sort();
    }

    let dependency_resolution_ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        namespace_function_files: &namespace_function_files,
        namespace_class_files: &namespace_class_files,
        namespace_module_files: &namespace_module_files,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
    };
    let (file_dependency_graph, dependency_graph_cache_hits) =
        build_timings.measure_value("dependency graph", || {
            build_file_dependency_graph_incremental(
                &parsed_files,
                &dependency_resolution_ctx,
                previous_dependency_graph.as_ref(),
            )
        });
    let reverse_file_dependency_graph = build_reverse_dependency_graph(&file_dependency_graph);
    let current_dependency_graph_cache =
        dependency_graph_cache_from_state(&parsed_files, &file_dependency_graph);
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
                load_semantic_cached_fingerprint(&project_root).is_some_and(|cached| {
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
    let mut namespace_functions: HashMap<String, HashSet<String>> = HashMap::new();
    for unit in &parsed_files {
        let entry = namespace_functions
            .entry(unit.namespace.clone())
            .or_default();
        for decl in &unit.program.declarations {
            if let Decl::Function(func) = &decl.node {
                entry.insert(func.name.clone());
            }
        }
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
    let rewrite_fingerprint_ctx = RewriteFingerprintContext {
        namespace_functions: &namespace_functions,
        namespace_function_files: &namespace_function_files,
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        namespace_classes: &namespace_class_map,
        namespace_class_files: &namespace_class_files,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        global_enum_map: &global_enum_map,
        global_enum_file_map: &global_enum_file_map,
        namespace_modules: &namespace_module_map,
        namespace_module_files: &namespace_module_files,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints,
        file_api_fingerprints: &file_api_fingerprints,
    };

    // Phase 2: Check imports for each file
    if do_check {
        println!("{} Checking imports...", "→".cyan());
        let shared_function_map = Arc::new(global_function_map.clone());
        let shared_known_namespace_paths = Arc::new(
            namespace_files_map
                .keys()
                .cloned()
                .collect::<HashSet<String>>(),
        );
        let import_check_cache_hits = std::sync::atomic::AtomicUsize::new(0);

        let import_results: Vec<Result<(), String>> =
            build_timings.measure("import check", || {
                Ok::<_, String>(
                    parsed_files
                        .par_iter()
                        .map(|unit| {
                            let rewrite_context_fingerprint =
                                compute_rewrite_context_fingerprint_for_unit(
                                    unit,
                                    &entry_namespace,
                                    &rewrite_fingerprint_ctx,
                                );
                            if load_import_check_cache_hit(
                                &project_root,
                                &unit.file,
                                &unit.import_check_fingerprint,
                                &rewrite_context_fingerprint,
                            )? {
                                import_check_cache_hits
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                return Ok(());
                            }

                            let mut checker = ImportChecker::new(
                                Arc::clone(&shared_function_map),
                                Arc::clone(&shared_known_namespace_paths),
                                unit.namespace.clone(),
                                unit.imports.clone(),
                                stdlib_registry(),
                            );

                            if let Err(errors) = checker.check_program(&unit.program) {
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
                            save_import_check_cache_hit(
                                &project_root,
                                &unit.file,
                                &unit.import_check_fingerprint,
                                &rewrite_context_fingerprint,
                            )?;
                            Ok(())
                        })
                        .collect(),
                )
            })?;

        for result in import_results {
            if let Err(rendered) = result {
                eprint!("{rendered}");
                return Err("Import check failed".to_string());
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
    }

    // Phase 3: Build combined AST with deterministic namespace mangling.
    let rewritten_results: Vec<Result<RewrittenProjectUnit, String>> =
        build_timings.measure("rewrite", || {
            Ok::<_, String>(
                parsed_files
                    .par_iter()
                    .map(|unit| {
                        let rewrite_context_fingerprint =
                            compute_rewrite_context_fingerprint_for_unit(
                                unit,
                                &entry_namespace,
                                &rewrite_fingerprint_ctx,
                            );
                        if let Some(cached) = load_rewritten_file_cache(
                            &project_root,
                            &unit.file,
                            &unit.semantic_fingerprint,
                            &rewrite_context_fingerprint,
                        )? {
                            let active_symbols = collect_active_symbols(&cached);
                            let api_program = api_projection_program(&cached);
                            return Ok(RewrittenProjectUnit {
                                file: unit.file.clone(),
                                program: cached,
                                api_program,
                                semantic_fingerprint: unit.semantic_fingerprint.clone(),
                                rewrite_context_fingerprint: rewrite_context_fingerprint.clone(),
                                active_symbols,
                                from_rewrite_cache: true,
                            });
                        }

                        let rewritten = project_rewrite::rewrite_program_for_project(
                            &unit.program,
                            &unit.namespace,
                            &entry_namespace,
                            &namespace_functions,
                            &global_function_map,
                            &namespace_class_map,
                            &global_class_map,
                            &namespace_enum_map,
                            &global_enum_map,
                            &namespace_module_map,
                            &global_module_map,
                            &unit.imports,
                        );
                        save_rewritten_file_cache(
                            &project_root,
                            &unit.file,
                            &unit.semantic_fingerprint,
                            &rewrite_context_fingerprint,
                            &rewritten,
                        )?;
                        let active_symbols = collect_active_symbols(&rewritten);
                        let api_program = api_projection_program(&rewritten);
                        Ok(RewrittenProjectUnit {
                            file: unit.file.clone(),
                            active_symbols,
                            api_program,
                            program: rewritten,
                            semantic_fingerprint: unit.semantic_fingerprint.clone(),
                            rewrite_context_fingerprint,
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
        let object_build_fingerprint = compute_object_build_fingerprint(&link);
        let previous_link_manifest = load_link_manifest_cache(&project_root);
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
        let codegen_reference_metadata: HashMap<PathBuf, CodegenReferenceMetadata> = parsed_files
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
        let file_namespaces: HashMap<PathBuf, String> = parsed_files
            .iter()
            .map(|unit| (unit.file.clone(), unit.namespace.clone()))
            .collect();
        let mut object_paths: Vec<Option<PathBuf>> = vec![None; rewritten_files.len()];
        let object_candidate_count = rewritten_files
            .iter()
            .filter(|unit| !unit.active_symbols.is_empty())
            .count();
        let cache_probe_results: Vec<Result<(usize, Option<PathBuf>), String>> = build_timings
            .measure("object cache probe", || {
                Ok::<_, String>(
                    rewritten_files
                        .par_iter()
                        .enumerate()
                        .map(|(index, unit)| {
                            if unit.active_symbols.is_empty() {
                                return Ok((index, None));
                            }
                            let cache_paths = object_cache_paths_by_file
                                .get(&unit.file)
                                .expect("object cache paths should exist for rewritten unit");
                            let cached_obj = load_object_cache_hit(
                                cache_paths,
                                &unit.semantic_fingerprint,
                                &unit.rewrite_context_fingerprint,
                                &object_build_fingerprint,
                            )?;
                            Ok((index, cached_obj))
                        })
                        .collect(),
                )
            })?;

        let mut object_cache_hits: usize = 0;
        let mut cache_misses: Vec<(usize, &RewrittenProjectUnit)> = Vec::new();
        for result in cache_probe_results {
            let (index, cached_obj) = result?;
            if let Some(cached_obj) = cached_obj {
                object_paths[index] = Some(cached_obj);
                object_cache_hits += 1;
            } else if !rewritten_files[index].active_symbols.is_empty() {
                cache_misses.push((index, &rewritten_files[index]));
            }
        }
        build_timings.record_counts(
            "object cache probe",
            &[
                ("candidates", object_candidate_count),
                ("reused", object_cache_hits),
                ("missed", cache_misses.len()),
            ],
        );

        let compiled_results: Vec<(usize, PathBuf)> =
            build_timings.measure("object codegen", || {
                cache_misses
                    .par_iter()
                    .map(|(index, unit)| {
                        let cache_paths = object_cache_paths_by_file
                            .get(&unit.file)
                            .expect("object cache paths should exist for rewritten unit");
                        let obj_path = cache_paths.object_path.clone();
                        let declaration_closure = declaration_symbols_for_unit(
                            &unit.file,
                            &unit.active_symbols,
                            &file_dependency_graph,
                            &codegen_reference_metadata,
                            &entry_namespace,
                            &global_function_map,
                            &global_function_file_map,
                            &global_class_map,
                            &global_class_file_map,
                            &global_enum_map,
                            &global_enum_file_map,
                            &global_module_map,
                            &global_module_file_map,
                        );
                        let codegen_program = codegen_program_for_unit(
                            &rewritten_files,
                            &rewritten_file_indices,
                            &unit.file,
                            Some(&declaration_closure.files),
                            Some(&declaration_closure.symbols),
                        );
                        let mut codegen_active_symbols = unit.active_symbols.clone();
                        codegen_active_symbols.extend(closure_body_symbols_for_unit(
                            &unit.file,
                            file_namespaces
                                .get(&unit.file)
                                .expect("namespace should exist for rewritten unit"),
                            &declaration_closure.symbols,
                            &global_function_file_map,
                            &global_class_file_map,
                        ));
                        compile_program_ast_to_object_filtered(
                            &codegen_program,
                            &unit.file,
                            &obj_path,
                            &link,
                            &codegen_active_symbols,
                            &declaration_closure.symbols,
                        )?;
                        save_object_cache_meta(
                            cache_paths,
                            &unit.semantic_fingerprint,
                            &unit.rewrite_context_fingerprint,
                            &object_build_fingerprint,
                        )?;
                        Ok::<(usize, PathBuf), String>((*index, obj_path))
                    })
                    .collect::<Result<Vec<_>, String>>()
            })?;
        build_timings.record_counts(
            "object codegen",
            &[
                ("candidates", object_candidate_count),
                ("reused", object_cache_hits),
                ("rebuilt", compiled_results.len()),
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

        let link_inputs: Vec<PathBuf> = object_paths.into_iter().flatten().collect();
        let current_link_manifest = LinkManifestCache {
            schema: LINK_MANIFEST_CACHE_SCHEMA.to_string(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            link_fingerprint: compute_link_fingerprint(&output_path, &link_inputs, &link),
            link_inputs: link_inputs.clone(),
        };

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
            save_link_manifest_cache(&project_root, &current_link_manifest)?;
        }
    }

    println!(
        "{} {} -> {}",
        "Built".green().bold(),
        config.name.cyan(),
        output_path.display()
    );

    if !check_only {
        save_cached_fingerprint(&project_root, &fingerprint)?;
        save_semantic_cached_fingerprint(&project_root, &semantic_fingerprint)?;
        save_dependency_graph_cache(&project_root, &current_dependency_graph_cache)?;
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
) -> Result<(), String> {
    let context = Context::create();
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let mut codegen = Codegen::new(&context, module_name);
    codegen
        .compile_filtered_with_decl_symbols(program, active_symbols, declaration_symbols)
        .map_err(|e| format!("{}: Codegen error: {}", "error".red().bold(), e.message))?;

    if let Some(parent) = object_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create object cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }
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
    Ok(())
}

/// Build and run the current project
fn run_project(
    args: &[String],
    release: bool,
    do_check: bool,
    show_timings: bool,
) -> Result<(), String> {
    build_project(release, false, do_check, false, show_timings)?;

    let cwd = current_dir_checked()?;
    let project_root = find_project_root(&cwd)
        .ok_or_else(|| format!("{}: No apex.toml found", "error".red().bold()))?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;
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

fn resolve_clang_opt_flag(opt_level: Option<&str>) -> &'static str {
    let normalized = opt_level
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match normalized.as_str() {
        "" | "3" => "-O3",
        "0" => "-O0",
        "1" => "-O1",
        "2" => "-O2",
        "s" => "-Os",
        "z" => "-Oz",
        "fast" => "-Ofast",
        _ => "-O3",
    }
}

struct LinkConfig<'a> {
    opt_level: Option<&'a str>,
    target: Option<&'a str>,
    output_kind: OutputKind,
    link_search: &'a [String],
    link_libs: &'a [String],
    link_args: &'a [String],
}

fn shutil_which(tool: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let candidate = dir.join(tool);
            if candidate.is_file() {
                return true;
            }
            #[cfg(windows)]
            {
                let exe = dir.join(format!("{}.exe", tool));
                exe.is_file()
            }
            #[cfg(not(windows))]
            {
                false
            }
        })
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LinkerFlavor {
    #[cfg(target_os = "linux")]
    Mold,
    #[cfg(any(target_os = "macos", windows))]
    Lld,
}

impl LinkerFlavor {
    fn clang_fuse_ld(self) -> &'static str {
        match self {
            #[cfg(target_os = "linux")]
            LinkerFlavor::Mold => "mold",
            #[cfg(any(target_os = "macos", windows))]
            LinkerFlavor::Lld => "lld",
        }
    }

    fn cache_key(self) -> &'static str {
        self.clang_fuse_ld()
    }
}

fn detect_linker_flavor() -> Result<LinkerFlavor, String> {
    #[cfg(target_os = "linux")]
    if shutil_which("mold") || shutil_which("ld.mold") {
        return Ok(LinkerFlavor::Mold);
    }

    #[cfg(target_os = "linux")]
    return Err(format!(
        "{}: Required linker 'mold' not found in PATH. Install mold and retry.",
        "error".red().bold()
    ));

    #[cfg(target_os = "macos")]
    if shutil_which("ld64.lld") || shutil_which("ld.lld") || shutil_which("lld") {
        return Ok(LinkerFlavor::Lld);
    }

    #[cfg(target_os = "macos")]
    return Err(format!(
        "{}: Required LLVM linker not found in PATH. Install lld/ld64.lld and retry.",
        "error".red().bold()
    ));

    #[cfg(windows)]
    if shutil_which("lld-link") || shutil_which("ld.lld") || shutil_which("lld") {
        return Ok(LinkerFlavor::Lld);
    }

    #[cfg(windows)]
    return Err(format!(
        "{}: Required LLVM linker not found in PATH. Install LLVM lld and retry.",
        "error".red().bold()
    ));

    #[allow(unreachable_code)]
    Err(format!(
        "{}: Unsupported host platform for linker detection.",
        "error".red().bold()
    ))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn should_force_no_pie(link: &LinkConfig<'_>) -> bool {
    if link.output_kind != OutputKind::Bin {
        return false;
    }

    match link.target {
        None => true,
        Some(target) => {
            let target = target.to_ascii_lowercase();
            !(target.contains("windows")
                || target.contains("mingw")
                || target.contains("darwin")
                || target.contains("apple"))
        }
    }
}

fn escape_response_file_arg(arg: &str) -> String {
    let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn write_link_response_file(path: &Path, objects: &[PathBuf]) -> Result<(), String> {
    let mut contents = String::new();
    for object in objects {
        contents.push_str(&escape_response_file_arg(&object.display().to_string()));
        contents.push('\n');
    }

    fs::write(path, contents).map_err(|e| {
        format!(
            "{}: Failed to write link response file '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })
}

/// Compile LLVM IR using clang
fn compile_ir(ir_path: &Path, output_path: &Path, link: &LinkConfig<'_>) -> Result<(), String> {
    let linker = detect_linker_flavor()?;
    let opt_flag = resolve_clang_opt_flag(link.opt_level);
    let run_clang = |march_native: bool, mtune_native: bool| {
        let mut cmd = Command::new("clang");
        cmd.arg(ir_path)
            .arg("-o")
            .arg(output_path)
            .arg("-Wno-override-module")
            .arg(opt_flag)
            .arg(format!("-fuse-ld={}", linker.clang_fuse_ld()));

        match link.output_kind {
            OutputKind::Bin => {}
            OutputKind::Shared => {
                cmd.arg("-shared");
            }
            OutputKind::Static => {
                cmd.arg("-c");
            }
        }

        if let Some(target_triple) = link.target {
            cmd.arg("--target").arg(target_triple);
        }

        if link.target.is_none() {
            if march_native {
                cmd.arg("-march=native");
            }
            if mtune_native {
                cmd.arg("-mtune=native");
            }
        }

        // Safe performance tweak: keep less frame bookkeeping in optimized binaries.
        cmd.arg("-fomit-frame-pointer");

        #[cfg(windows)]
        cmd.arg("-llegacy_stdio_definitions").arg("-lkernel32");

        #[cfg(not(windows))]
        cmd.arg("-lm").arg("-pthread");

        // GitHub Actions Ubuntu links executables as PIE by default; Apex bin objects/IR are
        // regular executable codegen, so request non-PIE explicitly on ELF toolchains.
        #[cfg(all(unix, not(target_os = "macos")))]
        if should_force_no_pie(link) {
            cmd.arg("-no-pie");
        }

        for path in link.link_search {
            cmd.arg(format!("-L{}", path));
        }

        for lib in link.link_libs {
            cmd.arg(format!("-l{}", lib));
        }

        for arg in link.link_args {
            cmd.arg(arg);
        }

        cmd.output()
    };

    // Keep aggressive native tuning, but degrade gracefully if one native flag is unsupported.
    let mut attempts: Vec<(bool, bool)> = vec![(true, true), (true, false), (false, false)];
    if link.target.is_some() {
        attempts = vec![(false, false)];
    }

    let mut last_stderr = String::new();
    for (march_native, mtune_native) in attempts {
        match run_clang(march_native, mtune_native) {
            Ok(output) if output.status.success() => {
                if link.output_kind == OutputKind::Static {
                    let object_path = output_path.with_extension("o");
                    fs::rename(output_path, &object_path).map_err(|e| {
                        format!(
                            "{}: Failed to stage object file for static archive: {}",
                            "error".red().bold(),
                            e
                        )
                    })?;
                    let status = Command::new("ar")
                        .arg("rcs")
                        .arg(output_path)
                        .arg(&object_path)
                        .status()
                        .map_err(|e| {
                            format!(
                                "{}: Failed to run ar for static library creation: {}",
                                "error".red().bold(),
                                e
                            )
                        })?;
                    let _ = fs::remove_file(&object_path);
                    if !status.success() {
                        return Err(format!(
                            "{}: ar failed while creating static library",
                            "error".red().bold()
                        ));
                    }
                }
                return Ok(());
            }
            Ok(output) => {
                last_stderr = String::from_utf8_lossy(&output.stderr).to_string();
            }
            Err(_) => {
                return Err(format!(
                    "{}: Clang not found. Install clang to compile.",
                    "error".red().bold()
                ));
            }
        }
    }

    Err(format!(
        "{}: Clang failed: {}",
        "error".red().bold(),
        last_stderr
    ))
}

fn link_objects(
    objects: &[PathBuf],
    output_path: &Path,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    let linker = detect_linker_flavor()?;
    if objects.is_empty() {
        return Err(format!(
            "{}: No object files generated for project build.",
            "error".red().bold()
        ));
    }

    let opt_flag = resolve_clang_opt_flag(link.opt_level);
    match link.output_kind {
        OutputKind::Static => {
            let status = Command::new("ar")
                .arg("rcs")
                .arg(output_path)
                .args(objects)
                .status()
                .map_err(|e| {
                    format!(
                        "{}: Failed to run ar for static library creation: {}",
                        "error".red().bold(),
                        e
                    )
                })?;
            if !status.success() {
                return Err(format!(
                    "{}: ar failed while creating static library",
                    "error".red().bold()
                ));
            }
            Ok(())
        }
        OutputKind::Bin | OutputKind::Shared => {
            let response_path = output_path.with_extension("link.rsp");
            write_link_response_file(&response_path, objects)?;
            let mut cmd = Command::new("clang");
            cmd.arg(format!("@{}", response_path.display()))
                .arg("-o")
                .arg(output_path)
                .arg(opt_flag)
                .arg(format!("-fuse-ld={}", linker.clang_fuse_ld()));

            if link.output_kind == OutputKind::Shared {
                cmd.arg("-shared");
            }
            if let Some(target_triple) = link.target {
                cmd.arg("--target").arg(target_triple);
            } else {
                cmd.arg("-march=native").arg("-mtune=native");
            }

            cmd.arg("-fomit-frame-pointer");

            #[cfg(windows)]
            cmd.arg("-llegacy_stdio_definitions").arg("-lkernel32");

            #[cfg(not(windows))]
            cmd.arg("-lm").arg("-pthread");

            // Avoid distro-dependent default PIE linking for normal executables on ELF hosts.
            #[cfg(all(unix, not(target_os = "macos")))]
            if should_force_no_pie(link) {
                cmd.arg("-no-pie");
            }

            for path in link.link_search {
                cmd.arg(format!("-L{}", path));
            }
            for lib in link.link_libs {
                cmd.arg(format!("-l{}", lib));
            }
            for arg in link.link_args {
                cmd.arg(arg);
            }

            let output = cmd.output().map_err(|_| {
                format!(
                    "{}: Clang not found. Install clang to compile.",
                    "error".red().bold()
                )
            })?;
            let _ = fs::remove_file(&response_path);
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Err(format!(
                    "{}: Clang failed while linking objects: {}",
                    "error".red().bold(),
                    stderr
                ))
            }
        }
    }
}

/// Check a single file
fn check_file(file: Option<&Path>) -> Result<(), String> {
    let file_path = if let Some(f) = file {
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
        return Ok(path.to_path_buf());
    }

    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    if let Some(project_root) = find_project_root(&current_dir) {
        let config = ProjectConfig::load(&project_root.join("apex.toml"))?;
        return Ok(config.get_entry_path(&project_root));
    }

    Err("No file specified and no apex.toml found in the current directory".to_string())
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
        if path.extension().and_then(|ext| ext.to_str()) == Some("apex") {
            return Ok(vec![path.to_path_buf()]);
        }
        return Err(format!("Path '{}' is not an .apex file", path.display()));
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
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_apex_files_recursive(&path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("apex") {
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
    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format!("{}: Parse error: {}", "error".red().bold(), e.message))?;

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

/// Run tests for a file or project
fn run_tests(
    test_path: Option<&Path>,
    list_only: bool,
    filter: Option<&str>,
) -> Result<(), String> {
    // Determine which file(s) to test
    let test_files = if let Some(path) = test_path {
        if path.is_file() {
            vec![path.to_path_buf()]
        } else {
            // Look for test files in directory
            find_test_files(path)?
        }
    } else {
        // Default: look for test files in current project
        let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
        find_test_files(&current_dir)?
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
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse_program()
            .map_err(|e| format!("Parse error: {}", e.message))?;

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

            // Create temporary file for test runner
            let runner_path = test_file.with_extension("test_runner.apex");
            fs::write(&runner_path, &runner_code)
                .map_err(|e| format!("Failed to write test runner: {}", e))?;

            // Compile and run the test runner
            let exe_path = test_file.with_extension("test_runner.exe");
            let result = compile_and_run_test(&runner_path, &exe_path);

            // Clean up temporary files
            let _ = fs::remove_file(&runner_path);
            let _ = fs::remove_file(&exe_path);

            result?;
        }
    }

    if !all_tests_found {
        println!("{}", "No tests discovered".yellow());
        println!("Mark functions with `@Test`:");
        println!("  {} function myTest(): None {{ ... }}", "@Test".cyan());
    }

    Ok(())
}

/// Find test files in a directory
fn find_test_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut test_files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "apex").unwrap_or(false) {
                // Check if file name suggests it's a test file
                let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if file_name.contains("test") || file_name.contains("spec") {
                    test_files.push(path);
                }
            }
        }
    }

    Ok(test_files)
}

/// Compile and run a test file
fn compile_and_run_test(source_path: &Path, exe_path: &Path) -> Result<(), String> {
    use std::process::Command;

    // Compile the test runner
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read test runner: {}", e))?;

    compile_source(&source, source_path, exe_path, false, true, None, None)?;

    // Run the compiled test
    println!("\n{}", "Running tests".cyan().bold());
    println!();

    let output = Command::new(exe_path)
        .output()
        .map_err(|e| format!("Failed to run test runner: {}", e))?;

    // Print output
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    // Check exit code
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

#[cfg(test)]
mod tests {
    use super::{
        api_program_fingerprint, build_file_dependency_graph_incremental, build_project,
        build_reverse_dependency_graph, check_command, codegen_program_for_unit, compile_source,
        component_fingerprint, compute_link_fingerprint, compute_namespace_api_fingerprints,
        compute_rewrite_context_fingerprint_for_unit, escape_response_file_arg, format_targets,
        parse_project_unit, precompute_all_transitive_dependencies,
        reusable_component_fingerprints, run_tests, semantic_program_fingerprint,
        should_skip_final_link, transitive_dependents, typecheck_summary_cache_from_state,
        typecheck_summary_cache_matches, DependencyGraphCache, DependencyGraphFileEntry,
        DependencyResolutionContext, LinkConfig, LinkManifestCache, OutputKind, ParsedProjectUnit,
        RewriteFingerprintContext, RewrittenProjectUnit, DEPENDENCY_GRAPH_CACHE_SCHEMA,
        LINK_MANIFEST_CACHE_SCHEMA,
    };
    use crate::ast::{Decl, FunctionDecl, ImportDecl, Program, Spanned, Type, Visibility};
    use crate::borrowck::BorrowChecker;
    use crate::formatter::format_program_canonical;
    use crate::parser::Parser;
    use crate::typeck::TypeChecker;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn parse_program(source: &str) -> Program {
        let tokens = crate::lexer::tokenize(source).expect("tokenize");
        let mut parser = Parser::new(tokens);
        parser.parse_program().expect("parse")
    }

    fn fingerprint_for(source: &str) -> String {
        let program = parse_program(source);
        semantic_program_fingerprint(&program)
    }

    fn assert_frontend_pipeline_ok(source: &str) {
        let program = parse_program(source);

        let mut type_checker = TypeChecker::new(source.to_string());
        if let Err(errors) = type_checker.check(&program) {
            panic!(
                "type check failed: {}",
                errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }

        let mut borrow_checker = BorrowChecker::new();
        if let Err(errors) = borrow_checker.check(&program) {
            panic!(
                "borrow check failed: {}",
                errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }

        let formatted = format_program_canonical(&program);
        let reparsed = parse_program(&formatted);

        let mut type_checker = TypeChecker::new(formatted.clone());
        if let Err(errors) = type_checker.check(&reparsed) {
            panic!(
                "type check after format failed: {}",
                errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }

        let mut borrow_checker = BorrowChecker::new();
        if let Err(errors) = borrow_checker.check(&reparsed) {
            panic!(
                "borrow check after format failed: {}",
                errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }

    fn make_temp_project_root(tag: &str) -> PathBuf {
        let temp_root = std::env::temp_dir().join(format!(
            "apex-project-smoke-{}-{}-{}",
            tag,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(temp_root.join("src")).expect("create temp project src dir");
        temp_root
    }

    fn cli_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct CwdRestore {
        previous: PathBuf,
    }

    impl Drop for CwdRestore {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.previous);
        }
    }

    fn with_current_dir<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
        let _lock = cli_test_lock().lock().expect("lock cwd test mutex");
        let previous = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(dir).expect("set current dir");
        let _restore = CwdRestore { previous };
        f()
    }

    fn write_test_project_config(root: &Path, files: &[&str], entry: &str, output: &str) {
        let files_toml = files
            .iter()
            .map(|file| format!("\"{}\"", file))
            .collect::<Vec<_>>()
            .join(", ");
        let config = format!(
            "name = \"smoke\"\nversion = \"0.1.0\"\nentry = \"{}\"\nfiles = [{}]\noutput = \"{}\"\n",
            entry, files_toml, output
        );
        fs::write(root.join("apex.toml"), config).expect("write apex.toml");
    }

    #[allow(clippy::type_complexity)]
    fn collect_project_symbol_maps(
        parsed_files: &[ParsedProjectUnit],
    ) -> (
        HashMap<String, Vec<PathBuf>>,
        HashMap<String, HashMap<String, PathBuf>>,
        HashMap<String, HashMap<String, PathBuf>>,
        HashMap<String, HashMap<String, PathBuf>>,
        HashMap<String, String>,
        HashMap<String, PathBuf>,
        HashMap<String, String>,
        HashMap<String, PathBuf>,
        HashMap<String, String>,
        HashMap<String, PathBuf>,
        HashMap<String, String>,
        HashMap<String, PathBuf>,
    ) {
        let mut namespace_files_map = HashMap::new();
        let mut namespace_function_files = HashMap::new();
        let mut namespace_class_files = HashMap::new();
        let mut namespace_module_files = HashMap::new();
        let mut global_function_map = HashMap::new();
        let mut global_function_file_map = HashMap::new();
        let mut global_class_map = HashMap::new();
        let mut global_class_file_map = HashMap::new();
        let mut global_enum_map = HashMap::new();
        let mut global_enum_file_map = HashMap::new();
        let mut global_module_map = HashMap::new();
        let mut global_module_file_map = HashMap::new();

        for unit in parsed_files {
            namespace_files_map
                .entry(unit.namespace.clone())
                .or_insert_with(Vec::new)
                .push(unit.file.clone());
            for name in &unit.function_names {
                namespace_function_files
                    .entry(unit.namespace.clone())
                    .or_insert_with(HashMap::new)
                    .insert(name.clone(), unit.file.clone());
                global_function_map.insert(name.clone(), unit.namespace.clone());
                global_function_file_map.insert(name.clone(), unit.file.clone());
            }
            for name in &unit.class_names {
                namespace_class_files
                    .entry(unit.namespace.clone())
                    .or_insert_with(HashMap::new)
                    .insert(name.clone(), unit.file.clone());
                global_class_map.insert(name.clone(), unit.namespace.clone());
                global_class_file_map.insert(name.clone(), unit.file.clone());
            }
            for name in &unit.enum_names {
                global_enum_map.insert(name.clone(), unit.namespace.clone());
                global_enum_file_map.insert(name.clone(), unit.file.clone());
            }
            for name in &unit.module_names {
                namespace_module_files
                    .entry(unit.namespace.clone())
                    .or_insert_with(HashMap::new)
                    .insert(name.clone(), unit.file.clone());
                global_module_map.insert(name.clone(), unit.namespace.clone());
                global_module_file_map.insert(name.clone(), unit.file.clone());
            }
        }

        for files in namespace_files_map.values_mut() {
            files.sort();
        }

        (
            namespace_files_map,
            namespace_function_files,
            namespace_class_files,
            namespace_module_files,
            global_function_map,
            global_function_file_map,
            global_class_map,
            global_class_file_map,
            global_enum_map,
            global_enum_file_map,
            global_module_map,
            global_module_file_map,
        )
    }

    #[test]
    fn semantic_program_fingerprint_ignores_comments_and_whitespace() {
        let a = r#"
import std.io.*;

function main(): None {
    println("hi");
    return None;
}
"#;
        let b = r#"
// top comment
import std.io.*;

function main(): None {
    // inside body
    println("hi");
    return None;
}
"#;

        assert_eq!(fingerprint_for(a), fingerprint_for(b));
    }

    #[test]
    fn semantic_program_fingerprint_changes_with_code_changes() {
        let a = r#"
function main(): None {
    println("hi");
    return None;
}
"#;
        let b = r#"
function main(): None {
    println("bye");
    return None;
}
"#;

        assert_ne!(fingerprint_for(a), fingerprint_for(b));
    }

    #[test]
    fn api_program_fingerprint_ignores_body_only_changes() {
        let a = r#"
function add(x: Integer): Integer {
    return x + 1;
}
"#;
        let b = r#"
function add(x: Integer): Integer {
    return x + 999;
}
"#;

        let pa = parse_program(a);
        let pb = parse_program(b);
        assert_eq!(api_program_fingerprint(&pa), api_program_fingerprint(&pb));
        assert_ne!(
            semantic_program_fingerprint(&pa),
            semantic_program_fingerprint(&pb)
        );
    }

    #[test]
    fn api_program_fingerprint_changes_with_signature_changes() {
        let a = r#"
function add(x: Integer): Integer {
    return x + 1;
}
"#;
        let b = r#"
function add(x: Float): Float {
    return x + 1.0;
}
"#;

        let pa = parse_program(a);
        let pb = parse_program(b);
        assert_ne!(api_program_fingerprint(&pa), api_program_fingerprint(&pb));
    }

    #[test]
    fn frontend_pipeline_corpus_survives_parse_check_borrow_and_format() {
        let corpus = [
            r#"
package demo.core;
import std.io.*;
function main(): None {
    println("hello");
    return None;
}
"#,
            r#"
function apply(f: () -> Integer): Integer {
    return f();
}

function one(): Integer {
    return 1;
}
"#,
            r#"
class Counter {
    mut value: Integer;

    constructor(start: Integer) {
        this.value = start;
    }

    function next(): Integer {
        this.value = this.value + 1;
        return this.value;
    }
}

function main(): None {
    mut c: Counter = Counter(1);
    x: Integer = c.next();
    println("count {x}");
    return None;
}
"#,
            r#"
enum MaybeInt {
    Some(value: Integer),
    Empty
}

function unwrap_or_zero(v: MaybeInt): Integer {
    match (v) {
        Some(value) => { return value; },
        _ => { return 0; },
    }
}
"#,
            r#"
module Math {
    function id<T>(value: T): T {
        return value;
    }
}

function main(): None {
    x: Integer = Math.id<Integer>(1);
    y: Integer = if (x == 1) { 10; } else { 20; };
    println("value {y}");
    return None;
}
"#,
        ];

        for source in corpus {
            assert_frontend_pipeline_ok(source);
        }
    }

    #[test]
    fn project_parse_cache_reuses_only_unchanged_files() {
        let temp_root = make_temp_project_root("parse-cache-selective");
        let src_dir = temp_root.join("src");
        let main_file = src_dir.join("main.apex");
        let lib_file = src_dir.join("lib.apex");

        fs::write(
            &main_file,
            "package app;\nimport lib.math;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main file");
        fs::write(
            &lib_file,
            "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
        )
        .expect("write lib file");

        let first_main = parse_project_unit(&temp_root, &main_file).expect("first main parse");
        let first_lib = parse_project_unit(&temp_root, &lib_file).expect("first lib parse");
        assert!(!first_main.from_parse_cache);
        assert!(!first_lib.from_parse_cache);

        thread::sleep(Duration::from_millis(5));
        fs::write(
            &lib_file,
            "package lib;\nfunction add(x: Integer): Integer { return x + 2; }\n",
        )
        .expect("rewrite lib file");

        let second_main = parse_project_unit(&temp_root, &main_file).expect("second main parse");
        let second_lib = parse_project_unit(&temp_root, &lib_file).expect("second lib parse");

        assert!(second_main.from_parse_cache);
        assert!(!second_lib.from_parse_cache);
        assert_eq!(
            first_main.semantic_fingerprint,
            second_main.semantic_fingerprint
        );
        assert_ne!(
            first_lib.semantic_fingerprint,
            second_lib.semantic_fingerprint
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_multi_file_import_graph_tracks_real_parsed_owner_file() {
        let temp_root = make_temp_project_root("import-graph");
        let src_dir = temp_root.join("src");
        let main_file = src_dir.join("main.apex");
        let math_file = src_dir.join("math.apex");

        fs::write(
            &main_file,
            "package app;\nimport lib.math;\nfunction main(): None { value: Integer = add(1); return None; }\n",
        )
        .expect("write main file");
        fs::write(
            &math_file,
            "package lib;\nfunction add(x: Integer): Integer { return x + 1; }\n",
        )
        .expect("write math file");

        let parsed_files = vec![
            parse_project_unit(&temp_root, &main_file).expect("parse main"),
            parse_project_unit(&temp_root, &math_file).expect("parse math"),
        ];

        let mut namespace_files_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
        let mut namespace_function_files: HashMap<String, HashMap<String, PathBuf>> =
            HashMap::new();
        let mut namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
        let mut namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
        let mut global_function_map: HashMap<String, String> = HashMap::new();
        let mut global_function_file_map: HashMap<String, PathBuf> = HashMap::new();
        let mut global_class_map: HashMap<String, String> = HashMap::new();
        let mut global_class_file_map: HashMap<String, PathBuf> = HashMap::new();
        let mut global_enum_map: HashMap<String, String> = HashMap::new();
        let mut global_enum_file_map: HashMap<String, PathBuf> = HashMap::new();
        let mut global_module_map: HashMap<String, String> = HashMap::new();
        let mut global_module_file_map: HashMap<String, PathBuf> = HashMap::new();

        for unit in &parsed_files {
            namespace_files_map
                .entry(unit.namespace.clone())
                .or_default()
                .push(unit.file.clone());
            for name in &unit.function_names {
                namespace_function_files
                    .entry(unit.namespace.clone())
                    .or_default()
                    .insert(name.clone(), unit.file.clone());
                global_function_map.insert(name.clone(), unit.namespace.clone());
                global_function_file_map.insert(name.clone(), unit.file.clone());
            }
            for name in &unit.class_names {
                namespace_class_files
                    .entry(unit.namespace.clone())
                    .or_default()
                    .insert(name.clone(), unit.file.clone());
                global_class_map.insert(name.clone(), unit.namespace.clone());
                global_class_file_map.insert(name.clone(), unit.file.clone());
            }
            for name in &unit.enum_names {
                global_enum_map.insert(name.clone(), unit.namespace.clone());
                global_enum_file_map.insert(name.clone(), unit.file.clone());
            }
            for name in &unit.module_names {
                namespace_module_files
                    .entry(unit.namespace.clone())
                    .or_default()
                    .insert(name.clone(), unit.file.clone());
                global_module_map.insert(name.clone(), unit.namespace.clone());
                global_module_file_map.insert(name.clone(), unit.file.clone());
            }
        }

        let ctx = DependencyResolutionContext {
            namespace_files_map: &namespace_files_map,
            namespace_function_files: &namespace_function_files,
            namespace_class_files: &namespace_class_files,
            namespace_module_files: &namespace_module_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
        };

        let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None);
        assert_eq!(
            graph.get(&main_file).cloned().unwrap_or_default(),
            HashSet::from([math_file.clone()])
        );
        assert!(graph
            .get(&math_file)
            .cloned()
            .unwrap_or_default()
            .is_empty());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_check_supports_cross_file_function_value_references() {
        let temp_root = make_temp_project_root("function-value-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("lib.apex"),
            "package app;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
        )
        .expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction main(): None { o: Option<(Integer) -> Integer> = Option.some(add1); r: Result<(Integer) -> Integer, String> = Result.ok(add1); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            check_command(None, false).expect("project check should support function value refs");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_imported_function_value_alias_references() {
        let temp_root = make_temp_project_root("function-value-import-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("lib.apex"),
            "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
        )
        .expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.add1 as inc;\nfunction main(): None { f: (Integer) -> Integer = inc; o: Option<(Integer) -> Integer> = Option.some(inc); x: Integer = f(2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support imported function value aliases");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_namespace_alias_function_values() {
        let temp_root = make_temp_project_root("function-value-namespace-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("lib.apex"),
            "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\nfunction twice(f: (Integer) -> Integer, x: Integer): Integer { return f(f(x)); }\n",
        )
        .expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { f: (Integer) -> Integer = u.add1; x: Integer = u.twice(f, 1); y: Integer = u.add1(2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support namespace alias function values");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_nested_namespace_alias_function_values() {
        let temp_root = make_temp_project_root("function-value-nested-namespace-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("lib.apex"),
            "package util;\nmodule M { function add1(x: Integer): Integer { return x + 1; } }\n",
        )
        .expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { f: (Integer) -> Integer = u.M.add1; x: Integer = u.M.add1(1); y: Integer = f(2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support nested namespace alias function values");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_namespace_alias_class_constructors() {
        let temp_root = make_temp_project_root("class-constructor-namespace-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("lib.apex"),
            "package util;\nclass Box { value: Integer; constructor(v: Integer) { this.value = v; } }\n",
        )
        .expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { u.Box(2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support namespace alias class constructors");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_if_expression_function_value_callees() {
        let temp_root = make_temp_project_root("ifexpr-function-callee-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction dec(x: Integer): Integer { return x - 1; }\nfunction main(): None { x: Integer = (if (true) { inc; } else { dec; })(1); require(x == 2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support if-expression function-value callees");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_unit_enum_variant_values() {
        let temp_root = make_temp_project_root("unit-enum-variant-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nenum E { A, B }\nfunction main(): None { e: E = E.A; match (e) { E.A => { } E.B => { } } return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support unit enum variant values");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_exact_imported_enum_variant_aliases() {
        let temp_root = make_temp_project_root("exact-enum-variant-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nenum E { A(Integer) B(Integer) }\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.E.B as Variant;\nfunction main(): None { e: E = Variant(2); match (e) { E.A(v) => { require(false); } E.B(v) => { require(v == 2); } } return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support exact imported enum variant aliases");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_exact_imported_enum_variant_alias_patterns() {
        let temp_root = make_temp_project_root("exact-enum-variant-alias-pattern-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nenum E { A(Integer) B(Integer) }\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.E.B as Variant;\nfunction main(): None { e: E = Variant(2); match (e) { Variant(v) => { require(v == 2); } E.A(v) => { require(false); } } return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support exact imported enum variant alias patterns");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_exact_imported_nested_enum_aliases() {
        let temp_root = make_temp_project_root("exact-nested-enum-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.E as Enum;\nfunction main(): None { e: Enum = Enum.B(2); match (e) { Enum.B(v) => { require(v == 2); } Enum.A(v) => { require(false); } } return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support exact imported nested enum aliases");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_exact_imported_nested_enum_variant_aliases() {
        let temp_root = make_temp_project_root("exact-nested-enum-variant-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.E.B as Variant;\nfunction main(): None { e: M.E = Variant(2); match (e) { Variant(v) => { require(v == 2); } M.E.A(v) => { require(false); } } return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support exact imported nested enum variant aliases");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_namespace_alias_nested_enums() {
        let temp_root = make_temp_project_root("namespace-alias-nested-enum-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M { enum E { A(Integer) B(Integer) } }\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app as u;\nfunction main(): None { e: u.M.E = u.M.E.B(2); match (e) { u.M.E.B(v) => { require(v == 2); } u.M.E.A(v) => { require(false); } } return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support namespace alias nested enums");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_exact_imported_nested_function_aliases_returning_classes() {
        let temp_root = make_temp_project_root("exact-nested-function-alias-class-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n    function mk(value: Integer): Box { return Box(value); }\n}\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.mk as mk;\nfunction main(): None { value: Integer = mk(2).get(); require(value == 2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support exact imported nested function aliases returning classes",
            );
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_exact_imported_nested_class_aliases() {
        let temp_root = make_temp_project_root("exact-nested-class-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n}\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.Box as Boxed;\nfunction main(): None { value: Integer = Boxed(2).get(); require(value == 2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support exact imported nested class aliases");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_local_qualified_nested_class_paths() {
        let temp_root = make_temp_project_root("local-qualified-nested-class-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n}\nfunction main(): None { b: M.Box = M.Box(2); require(b.get() == 2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support local qualified nested class paths");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_local_qualified_nested_generic_class_paths() {
        let temp_root = make_temp_project_root("local-qualified-nested-generic-class-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n}\nfunction main(): None { b: M.Box<Integer> = M.Box<Integer>(2); require(b.get() == 2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support local qualified nested generic class paths");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_exact_imported_nested_generic_class_aliases() {
        let temp_root = make_temp_project_root("exact-nested-generic-class-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n}\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.Box as Boxed;\nfunction main(): None { b: Boxed<Integer> = Boxed<Integer>(2); require(b.get() == 2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support exact imported nested generic class aliases");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_exact_imported_nested_generic_function_aliases_returning_classes() {
        let temp_root = make_temp_project_root("exact-nested-generic-function-alias-class-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n    function mk<T>(value: T): Box<T> { return Box<T>(value); }\n}\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.mk as mk;\nfunction main(): None { value: Integer = mk<Integer>(2).get(); require(value == 2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support exact imported nested generic function aliases returning classes",
            );
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_local_nested_generic_functions_returning_classes() {
        let temp_root = make_temp_project_root("local-nested-generic-function-runtime-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n    function mk<T>(value: T): Box<T> { return Box<T>(value); }\n}\nfunction main(): Integer { return M.mk<Integer>(2).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support local nested generic function returns");
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run compiled local nested generic function binary");
        assert_eq!(status.code(), Some(2));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_exact_imported_nested_generic_function_aliases_returning_classes() {
        let temp_root =
            make_temp_project_root("exact-nested-generic-function-alias-class-runtime-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function get(): T { return this.value; }\n    }\n    function mk<T>(value: T): Box<T> { return Box<T>(value); }\n}\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.mk as mk;\nfunction main(): Integer { return mk<Integer>(2).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support exact imported nested generic function aliases at runtime",
            );
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run compiled imported nested generic function binary");
        assert_eq!(status.code(), Some(2));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_nested_generic_methods_on_nested_generic_classes() {
        let temp_root = make_temp_project_root("nested-generic-method-runtime-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { b: M.Box<Integer> = M.Box<Integer>(2); return b.map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support nested generic methods on nested generic classes",
            );
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run compiled nested generic method binary");
        assert_eq!(status.code(), Some(3));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_nested_generic_method_alias_paths() {
        let temp_root = make_temp_project_root("nested-generic-method-alias-runtime-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.Box as Boxed;\nimport app.inc as inc;\nfunction main(): Integer { b: Boxed<Integer> = Boxed<Integer>(2); return b.map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support nested generic method alias paths");
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run compiled nested generic alias method binary");
        assert_eq!(status.code(), Some(3));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_namespace_alias_nested_generic_class_specializations() {
        let temp_root = make_temp_project_root("namespace-alias-nested-generic-class-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n        async function mk_async(value: Integer): Task<Box<Integer>> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nimport util.M.N.Box as B;\nfunction main(): Integer { return u.M.N.Box<Integer>(41).value + B<Integer>(1).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support namespace alias nested generic class specializations",
            );
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_namespace_alias_nested_generic_method_specializations() {
        let temp_root = make_temp_project_root("namespace-alias-nested-generic-method-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { return u.M.N.mk(46).map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support namespace alias nested generic method specializations",
            );
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_cross_package_nested_generic_function_returns_via_namespace_alias() {
        let temp_root =
            make_temp_project_root("cross-package-nested-generic-return-namespace-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function get(): T { return this.value; }\n        }\n        function mk(value: Integer): Box<Integer> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): Integer { return u.M.N.mk(42).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support cross-package nested generic returns via namespace alias",
            );
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run cross-package nested generic return project binary");
        assert_eq!(status.code(), Some(42));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_cross_package_nested_generic_async_returns_via_namespace_alias() {
        let temp_root = make_temp_project_root(
            "cross-package-nested-generic-async-return-namespace-alias-project",
        );
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package util;\nmodule M {\n    module N {\n        class Box<T> {\n            value: T;\n            constructor(value: T) { this.value = value; }\n            function get(): T { return this.value; }\n        }\n        async function mk_async(value: Integer): Task<Box<Integer>> { return Box<Integer>(value); }\n    }\n}\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): Integer { return await(u.M.N.mk_async(43)).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support cross-package nested generic async returns via namespace alias",
            );
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run cross-package nested generic async return project binary");
        assert_eq!(status.code(), Some(43));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_qualified_module_type_paths() {
        let temp_root = make_temp_project_root("qualified-module-type-path-runtime-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule util {\n    class Item {\n        value: Integer;\n        constructor(value: Integer) { this.value = value; }\n        function get(): Integer { return this.value; }\n    }\n    function mk(): Item { return Item(7); }\n}\nfunction main(): Integer {\n    item: util.Item = util.mk();\n    return item.get();\n}\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support qualified module type paths end-to-end");
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run compiled qualified module type path binary");
        assert_eq!(status.code(), Some(7));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_user_defined_generic_classes_named_like_builtins() {
        let temp_root =
            make_temp_project_root("user-defined-generic-class-named-like-builtin-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nclass Box<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function get(): T { return this.value; }\n}\nfunction mk(value: Integer): Box<Integer> {\n    return Box<Integer>(value);\n}\nfunction main(): Integer {\n    return mk(42).get();\n}\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should prefer user-defined generic classes over built-in container names",
            );
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run compiled user-defined builtin-named generic class binary");
        assert_eq!(status.code(), Some(42));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_nested_generic_methods_on_expression_receivers() {
        let temp_root = make_temp_project_root("nested-generic-method-expr-runtime-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n    function make<T>(value: T): Box<T> { return Box<T>(value); }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): Integer { return M.make<Integer>(2).map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support nested generic methods on expression receivers",
            );
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run compiled nested generic expression receiver binary");
        assert_eq!(status.code(), Some(3));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_nested_generic_method_imported_expression_receivers() {
        let temp_root =
            make_temp_project_root("nested-generic-method-imported-expr-runtime-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/util.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("util.apex"),
            "package app;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n        function map<U>(f: (T) -> U): Box<U> { return Box<U>(f(this.value)); }\n        function get(): T { return this.value; }\n    }\n    function make<T>(value: T): Box<T> { return Box<T>(value); }\n}\nfunction inc(x: Integer): Integer { return x + 1; }\n",
        )
        .expect("write util");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport app.M.make as make;\nimport app.inc as inc;\nfunction main(): Integer { return make<Integer>(2).map<Integer>(inc).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false).expect(
                "project build should support imported expression receivers for nested generic methods",
            );
        });

        let status = std::process::Command::new(temp_root.join("smoke"))
            .status()
            .expect("run compiled imported nested generic expression receiver binary");
        assert_eq!(status.code(), Some(3));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_async_block_import_alias_calls() {
        let temp_root = make_temp_project_root("async-block-import-alias-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("lib.apex"),
            "package util;\nfunction add1(x: Integer): Integer { return x + 1; }\n",
        )
        .expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.add1 as inc;\nfunction main(): None { task: Task<Integer> = async { return inc(1); }; value: Integer = await(task); require(value == 2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support async-block import alias calls");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_namespace_alias_unit_enum_values() {
        let temp_root = make_temp_project_root("namespace-alias-unit-enum-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(src_dir.join("lib.apex"), "package util;\nenum E { A, B }\n").expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { e: u.E = u.E.A; match (e) { u.E.A => { } u.E.B => { } } return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support namespace alias unit enum values");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_try_expression_function_value_callees() {
        let temp_root = make_temp_project_root("try-function-callee-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction choose(): Result<(Integer) -> Integer, String> { return Result.ok(inc); }\nfunction main(): Result<None, String> { value: Integer = (choose()?)(1); require(value == 2); return Result.ok(None); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support try-expression function-value callees");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_imported_explicit_generic_free_calls() {
        let temp_root = make_temp_project_root("imported-explicit-generic-free-call-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("lib.apex"),
            "package util;\nfunction id<T>(x: T): T { return x; }\n",
        )
        .expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.id;\nfunction main(): None { value: Integer = id<Integer>(1); require(value == 1); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support imported explicit generic free calls");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_imported_generic_class_instance_methods() {
        let temp_root = make_temp_project_root("imported-generic-class-method-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("lib.apex"),
            "package util;\nclass Boxed<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function get(): T { return this.value; }\n}\n",
        )
        .expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util.Boxed;\nfunction main(): None { value: Integer = Boxed<Integer>(7).get(); require(value == 7); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support imported generic class instance methods");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_method_calls_on_function_returned_objects() {
        let temp_root = make_temp_project_root("function-return-method-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nclass Boxed<T> {\n    value: T;\n    constructor(value: T) { this.value = value; }\n    function get(): T { return this.value; }\n}\nfunction make_box(): Boxed<Integer> { return Boxed<Integer>(9); }\nfunction main(): None { value: Integer = make_box().get(); require(value == 9); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support method calls on function-returned objects");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_check_supports_namespace_alias_nested_module_generic_class_constructors() {
        let temp_root = make_temp_project_root("namespace-alias-nested-generic-class-check");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/lib.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("lib.apex"),
            "package util;\nmodule M {\n    class Box<T> {\n        value: T;\n        constructor(value: T) { this.value = value; }\n    }\n}\n",
        )
        .expect("write lib");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nimport util as u;\nfunction main(): None { b: u.M.Box<Integer> = u.M.Box<Integer>(1); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            check_command(None, false).expect(
                "project check should support namespace alias nested-module generic class constructors",
            );
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_build_supports_dereferenced_function_value_callees() {
        let temp_root = make_temp_project_root("deref-function-callee-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): None { f: &(Integer) -> Integer = &inc; value: Integer = (*f)(1); require(value == 2); return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, true, false, false)
                .expect("project build should support dereferenced function-value callees");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_check_rejects_async_borrowed_reference_results() {
        let temp_root = make_temp_project_root("async-borrowed-result-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction inc(x: Integer): Integer { return x + 1; }\nfunction main(): None { task: Task<&(Integer) -> Integer> = async { return &inc; }; return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            let err = check_command(None, false)
                .expect_err("project check should reject async borrowed reference results");
            assert!(
                err.contains("Async block cannot return a value containing borrowed references"),
                "{err}"
            );
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_check_rejects_async_borrowed_reference_params_and_captures() {
        let temp_root = make_temp_project_root("async-borrowed-param-capture-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nasync function read_ref(r: &Integer): Task<Integer> { return *r; }\nfunction main(): None { x: Integer = 1; alias: &Integer = &x; task: Task<Integer> = async { return *alias; }; return None; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            let err = check_command(None, false).expect_err(
                "project check should reject async borrowed reference parameters and captures",
            );
            assert!(
                err.contains("Async function 'app__read_ref' cannot accept a parameter containing borrowed references"),
                "{err}"
            );
            assert!(
                err.contains("Async block cannot capture 'alias' because its type contains borrowed references"),
                "{err}"
            );
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn cli_check_command_succeeds_for_temp_project() {
        let temp_root = make_temp_project_root("cli-check");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/helper.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction main(): None { value: Integer = helper(); return None; }\n",
        )
        .expect("write main");
        fs::write(
            src_dir.join("helper.apex"),
            "package app;\nfunction helper(): Integer { return 1; }\n",
        )
        .expect("write helper");

        with_current_dir(&temp_root, || {
            check_command(None, false).expect("project check should pass");
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_supports_implicit_default_class_constructor() {
        let temp_root = make_temp_project_root("implicit-default-ctor");
        let source_path = temp_root.join("implicit_ctor.apex");
        let output_path = temp_root.join("implicit_ctor");
        let source = r#"
            class C {
                function value(): Integer { return 7; }
            }

            function main(): None {
                c: C = C();
                x: Integer = c.value();
                return None;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, true, true, None, None)
            .expect("implicit default constructor codegen should succeed");
        assert!(output_path.with_extension("ll").exists());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_supports_explicit_generic_method_calls() {
        let temp_root = make_temp_project_root("generic-method-codegen");
        let source_path = temp_root.join("generic_method.apex");
        let output_path = temp_root.join("generic_method");
        let source = r#"
            class C {
                function id<T>(x: T): T { return x; }
            }

            function main(): None {
                c: C = C();
                x: Integer = c.id<Integer>(1);
                return None;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, true, true, None, None)
            .expect("explicit generic method codegen should succeed");
        assert!(output_path.with_extension("ll").exists());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_supports_generic_class_instance_method_calls() {
        let temp_root = make_temp_project_root("generic-class-method-codegen");
        let source_path = temp_root.join("generic_class_method.apex");
        let output_path = temp_root.join("generic_class_method");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): None {
                b: Boxed<Integer> = Boxed<Integer>(7);
                x: Integer = b.get();
                return None;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, true, true, None, None)
            .expect("generic class instance method codegen should succeed");
        assert!(output_path.with_extension("ll").exists());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_generic_class_instance_methods() {
        let temp_root = make_temp_project_root("generic-class-method-runtime");
        let source_path = temp_root.join("generic_class_runtime.apex");
        let output_path = temp_root.join("generic_class_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                b: Boxed<Integer> = Boxed<Integer>(7);
                return b.get();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("generic class runtime codegen should succeed");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled generic class binary");
        assert_eq!(status.code(), Some(7));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_method_calls_on_function_returned_objects() {
        let temp_root = make_temp_project_root("function-return-method-runtime");
        let source_path = temp_root.join("function_return_method_runtime.apex");
        let output_path = temp_root.join("function_return_method_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function make_box(): Boxed<Integer> {
                return Boxed<Integer>(9);
            }

            function main(): Integer {
                return make_box().get();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("method call on function return value should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled function-return method binary");
        assert_eq!(status.code(), Some(9));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_method_calls_on_try_unwrapped_objects() {
        let temp_root = make_temp_project_root("try-object-method-runtime");
        let source_path = temp_root.join("try_object_method_runtime.apex");
        let output_path = temp_root.join("try_object_method_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function choose_box(): Result<Boxed<Integer>, String> {
                return Result.ok(Boxed<Integer>(21));
            }

            function use_box(): Result<Integer, String> {
                return Result.ok(choose_box()?.get());
            }

            function main(): Integer {
                result: Result<Integer, String> = use_box();
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("method call on try-unwrapped object should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled try-object method binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_method_calls_on_awaited_objects_without_extra_parentheses() {
        let temp_root = make_temp_project_root("await-object-method-runtime");
        let source_path = temp_root.join("await_object_method_runtime.apex");
        let output_path = temp_root.join("await_object_method_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            async function make_box(): Boxed<Integer> {
                return Boxed<Integer>(3);
            }

            async function run(): Integer {
                return await(make_box()).get();
            }

            function main(): Integer {
                t: Task<Integer> = run();
                return await(t);
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("awaited object method chain should parse and codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled awaited-object method binary");
        assert_eq!(status.code(), Some(3));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_negative_await_timeout() {
        let temp_root = make_temp_project_root("await-timeout-negative-runtime");
        let source_path = temp_root.join("await_timeout_negative_runtime.apex");
        let output_path = temp_root.join("await_timeout_negative_runtime");
        let source = r#"
            async function work(): Integer {
                return 7;
            }

            function main(): Integer {
                maybe: Option<Integer> = work().await_timeout(-1);
                if (maybe.is_some()) { return 99; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("negative await_timeout should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled negative await_timeout binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_prints_clean_option_unwrap_panic_message() {
        let temp_root = make_temp_project_root("option-unwrap-panic-message-runtime");
        let source_path = temp_root.join("option_unwrap_panic_message_runtime.apex");
        let output_path = temp_root.join("option_unwrap_panic_message_runtime");
        let source = r#"
            function main(): Integer {
                return Option.none().unwrap();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("Option.none unwrap panic path should codegen");

        let output = std::process::Command::new(&output_path)
            .output()
            .expect("run compiled Option.none unwrap binary");
        assert_eq!(output.status.code(), Some(1));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Option.unwrap() called on None\n"));
        assert!(!stdout.contains("\\n"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_prints_clean_result_unwrap_panic_message() {
        let temp_root = make_temp_project_root("result-unwrap-panic-message-runtime");
        let source_path = temp_root.join("result_unwrap_panic_message_runtime.apex");
        let output_path = temp_root.join("result_unwrap_panic_message_runtime");
        let source = r#"
            function main(): Integer {
                return Result.error("boom").unwrap();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("Result.error unwrap panic path should codegen");

        let output = std::process::Command::new(&output_path)
            .output()
            .expect("run compiled Result.error unwrap binary");
        assert_eq!(output.status.code(), Some(1));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Result.unwrap() called on Error\n"));
        assert!(!stdout.contains("\\n"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_if_expression_generic_constructor_branches() {
        let temp_root = make_temp_project_root("ifexpr-generic-ctor-runtime");
        let source_path = temp_root.join("ifexpr_generic_ctor_runtime.apex");
        let output_path = temp_root.join("ifexpr_generic_ctor_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function make(flag: Boolean): Boxed<Integer> {
                return if (flag) { Boxed<Integer>(1); } else { Boxed<Integer>(2); };
            }

            function main(): Integer {
                return make(true).get();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("if-expression generic constructors should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled if-expression generic constructor binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_method_calls_on_if_expression_objects() {
        let temp_root = make_temp_project_root("ifexpr-object-method-runtime");
        let source_path = temp_root.join("ifexpr_object_method_runtime.apex");
        let output_path = temp_root.join("ifexpr_object_method_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                return (if (true) { Boxed<Integer>(17); } else { Boxed<Integer>(18); }).get();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("method call on if-expression object should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled if-expression object binary");
        assert_eq!(status.code(), Some(17));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_field_access_on_match_expression_objects() {
        let temp_root = make_temp_project_root("match-object-field-runtime");
        let source_path = temp_root.join("match_object_field_runtime.apex");
        let output_path = temp_root.join("match_object_field_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                return (match (0) { 0 => { Boxed<Integer>(19); }, _ => { Boxed<Integer>(20); }, }).value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("field access on match-expression object should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled match-expression object binary");
        assert_eq!(status.code(), Some(19));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_method_calls_on_indexed_objects() {
        let temp_root = make_temp_project_root("index-object-method-runtime");
        let source_path = temp_root.join("index_object_method_runtime.apex");
        let output_path = temp_root.join("index_object_method_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                xs: List<Boxed<Integer>> = List<Boxed<Integer>>();
                xs.push(Boxed<Integer>(30));
                return xs[0].get();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("method call on indexed object should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled indexed-object method binary");
        assert_eq!(status.code(), Some(30));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_field_access_on_indexed_objects() {
        let temp_root = make_temp_project_root("index-object-field-runtime");
        let source_path = temp_root.join("index_object_field_runtime.apex");
        let output_path = temp_root.join("index_object_field_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed<Integer>> = List<Boxed<Integer>>();
                xs.push(Boxed<Integer>(31));
                return xs[0].value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("field access on indexed object should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled indexed-object field binary");
        assert_eq!(status.code(), Some(31));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_option_unwrap_method_chains_on_call_results() {
        let temp_root = make_temp_project_root("option-call-unwrap-method-runtime");
        let source_path = temp_root.join("option_call_unwrap_method_runtime.apex");
        let output_path = temp_root.join("option_call_unwrap_method_runtime");
        let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function choose(): Option<Boxed<Integer>> {
                return Option.some(Boxed<Integer>(32));
            }

            function main(): Integer {
                return choose().unwrap().get();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("option unwrap method chain on call result should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled option-unwrap method chain binary");
        assert_eq!(status.code(), Some(32));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_list_methods_on_call_results() {
        let temp_root = make_temp_project_root("list-call-method-runtime");
        let source_path = temp_root.join("list_call_method_runtime.apex");
        let output_path = temp_root.join("list_call_method_runtime");
        let source = r#"
            function make(): List<Integer> {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs.push(2);
                return xs;
            }

            function main(): Integer {
                return make().length();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("list method on call result should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled list-call method binary");
        assert_eq!(status.code(), Some(2));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_range_methods_on_call_results() {
        let temp_root = make_temp_project_root("range-call-method-runtime");
        let source_path = temp_root.join("range_call_method_runtime.apex");
        let output_path = temp_root.join("range_call_method_runtime");
        let source = r#"
            function mk(): Range<Integer> {
                return range(0, 10);
            }

            function main(): Integer {
                return if (mk().has_next()) { 1; } else { 2; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("range method on call result should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled range-call method binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_set_methods_on_call_results() {
        let temp_root = make_temp_project_root("set-call-method-runtime");
        let source_path = temp_root.join("set_call_method_runtime.apex");
        let output_path = temp_root.join("set_call_method_runtime");
        let source = r#"
            function build(): Set<Integer> {
                s: Set<Integer> = Set<Integer>();
                s.add(7);
                return s;
            }

            function main(): Integer {
                return if (build().contains(7)) { 1; } else { 2; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("set method on call result should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled set-call method binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_set_remove_on_call_results() {
        let temp_root = make_temp_project_root("set-remove-call-method-runtime");
        let source_path = temp_root.join("set_remove_call_method_runtime.apex");
        let output_path = temp_root.join("set_remove_call_method_runtime");
        let source = r#"
            function build(): Set<Integer> {
                s: Set<Integer> = Set<Integer>();
                s.add(7);
                return s;
            }

            function main(): Integer {
                return if (build().remove(7)) { 1; } else { 2; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("set remove on call result should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled set-remove call binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_set_contains_on_option_values() {
        let temp_root = make_temp_project_root("set-option-contains-runtime");
        let source_path = temp_root.join("set_option_contains_runtime.apex");
        let output_path = temp_root.join("set_option_contains_runtime");
        let source = r#"
            function main(): Integer {
                s: Set<Option<Integer>> = Set<Option<Integer>>();
                s.add(Option.some(7));
                return if (s.contains(Option.some(7))) { 1; } else { 2; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("set option contains should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled set-option contains binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_set_contains_on_result_values() {
        let temp_root = make_temp_project_root("set-result-contains-runtime");
        let source_path = temp_root.join("set_result_contains_runtime.apex");
        let output_path = temp_root.join("set_result_contains_runtime");
        let source = r#"
            function main(): Integer {
                s: Set<Result<Integer, Integer>> = Set<Result<Integer, Integer>>();
                s.add(Result.ok(7));
                return if (s.contains(Result.ok(7))) { 1; } else { 2; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("set result contains should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled set-result contains binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_methods_on_call_results() {
        let temp_root = make_temp_project_root("map-call-method-runtime");
        let source_path = temp_root.join("map_call_method_runtime.apex");
        let output_path = temp_root.join("map_call_method_runtime");
        let source = r#"
            function build(): Map<Integer, Integer> {
                m: Map<Integer, Integer> = Map<Integer, Integer>();
                m.set(1, 7);
                return m;
            }

            function main(): Integer {
                return if (build().contains(1)) { build().length(); } else { 9; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("map method on call result should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map-call method binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_growth_past_initial_capacity() {
        let temp_root = make_temp_project_root("map-growth-runtime");
        let source_path = temp_root.join("map_growth_runtime.apex");
        let output_path = temp_root.join("map_growth_runtime");
        let source = r#"
            function build(): Map<Integer, Integer> {
                m: Map<Integer, Integer> = Map<Integer, Integer>();
                m.set(0, 10);
                m.set(1, 11);
                m.set(2, 12);
                m.set(3, 13);
                m.set(4, 14);
                m.set(5, 15);
                m.set(6, 16);
                m.set(7, 17);
                m.set(8, 18);
                return m;
            }

            function main(): Integer {
                m: Map<Integer, Integer> = build();
                return if (m.contains(8)) { m.get(8); } else { 99; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("map growth should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map-growth binary");
        assert_eq!(status.code(), Some(18));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_option_growth_for_earlier_keys() {
        let temp_root = make_temp_project_root("map-option-growth-earlier-runtime");
        let source_path = temp_root.join("map_option_growth_earlier_runtime.apex");
        let output_path = temp_root.join("map_option_growth_earlier_runtime");
        let source = r#"
            function main(): Integer {
                m: Map<Option<Integer>, Integer> = Map<Option<Integer>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Option.some(i), i + 10);
                    i = i + 1;
                }
                return if (m.contains(Option.some(0)) && m.get(Option.some(0)) == 10 && m.contains(Option.some(8)) && m.get(Option.some(8)) == 18) { 0; } else { 1; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("map option growth should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map-option growth binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_option_updates_after_growth() {
        let temp_root = make_temp_project_root("map-option-update-runtime");
        let source_path = temp_root.join("map_option_update_runtime.apex");
        let output_path = temp_root.join("map_option_update_runtime");
        let source = r#"
            function main(): Integer {
                m: Map<Option<Integer>, Integer> = Map<Option<Integer>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Option.some(i), i + 10);
                    i = i + 1;
                }
                m.set(Option.some(4), 99);
                return if (m.length() == 9 && m.get(Option.some(4)) == 99 && m.get(Option.some(8)) == 18) { 0; } else { 1; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("map option update should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map-option update binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_set_option_remove_after_growth() {
        let temp_root = make_temp_project_root("set-option-remove-runtime");
        let source_path = temp_root.join("set_option_remove_runtime.apex");
        let output_path = temp_root.join("set_option_remove_runtime");
        let source = r#"
            function main(): Integer {
                s: Set<Option<Integer>> = Set<Option<Integer>>();
                mut i: Integer = 0;
                while (i < 9) {
                    s.add(Option.some(i));
                    i = i + 1;
                }
                removed: Boolean = s.remove(Option.some(4));
                return if (removed && !s.contains(Option.some(4)) && s.contains(Option.some(8)) && s.length() == 8) { 0; } else { 1; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("set option remove should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled set-option remove binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_result_growth_with_integer_error_keys() {
        let temp_root = make_temp_project_root("map-result-growth-runtime");
        let source_path = temp_root.join("map_result_growth_runtime.apex");
        let output_path = temp_root.join("map_result_growth_runtime");
        let source = r#"
            function main(): Integer {
                m: Map<Result<Integer, Integer>, Integer> = Map<Result<Integer, Integer>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Result.error(i), i + 10);
                    i = i + 1;
                }
                return if (m.contains(Result.error(0)) && m.get(Result.error(0)) == 10 && m.contains(Result.error(8)) && m.get(Result.error(8)) == 18) { 0; } else { 1; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("map result growth should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map-result growth binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_result_error_with_non_integer_ok_type() {
        let temp_root = make_temp_project_root("result-error-layout-runtime");
        let source_path = temp_root.join("result_error_layout_runtime.apex");
        let output_path = temp_root.join("result_error_layout_runtime");
        let source = r#"
            function bad(): Result<Float, String> {
                return Result.error("x");
            }

            function main(): Integer {
                r: Result<Float, String> = bad();
                return if (r.is_ok()) { 1; } else { 0; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("result error layout should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled result-error layout binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_with_class_pointer_keys() {
        let temp_root = make_temp_project_root("map-class-key-runtime");
        let source_path = temp_root.join("map_class_key_runtime.apex");
        let output_path = temp_root.join("map_class_key_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                a: Boxed = Boxed(1);
                b: Boxed = Boxed(2);
                m: Map<Boxed, Integer> = Map<Boxed, Integer>();
                m.set(a, 11);
                m.set(b, 12);
                return if (m.contains(a) && m.get(a) == 11 && m.get(b) == 12) { 0; } else { 1; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("map class key should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map-class-key binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_set_with_class_pointer_keys() {
        let temp_root = make_temp_project_root("set-class-key-runtime");
        let source_path = temp_root.join("set_class_key_runtime.apex");
        let output_path = temp_root.join("set_class_key_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                a: Boxed = Boxed(1);
                b: Boxed = Boxed(2);
                s: Set<Boxed> = Set<Boxed>();
                s.add(a);
                s.add(b);
                return if (s.contains(a) && s.contains(b)) { 0; } else { 1; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("set class key should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled set-class-key binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_with_nested_option_class_keys() {
        let temp_root = make_temp_project_root("map-option-class-key-runtime");
        let source_path = temp_root.join("map_option_class_key_runtime.apex");
        let output_path = temp_root.join("map_option_class_key_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                a: Boxed = Boxed(1);
                b: Boxed = Boxed(2);
                m: Map<Option<Boxed>, Integer> = Map<Option<Boxed>, Integer>();
                m.set(Option.some(a), 11);
                m.set(Option.some(b), 12);
                return if (m.contains(Option.some(a)) && m.get(Option.some(b)) == 12) { 0; } else { 1; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("nested option class key should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled nested option class key binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_with_multi_variant_enum_keys() {
        let temp_root = make_temp_project_root("map-enum-key-runtime");
        let source_path = temp_root.join("map_enum_key_runtime.apex");
        let output_path = temp_root.join("map_enum_key_runtime");
        let source = r#"
            enum E {
                A(Integer)
                B(Integer)
            }

            function main(): Integer {
                m: Map<E, Integer> = Map<E, Integer>();
                m.set(E.A(1), 11);
                m.set(E.B(2), 12);
                return if (m.contains(E.A(1)) && m.get(E.A(1)) == 11 && m.get(E.B(2)) == 12) { 0; } else { 1; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("map enum key should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map-enum-key binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_set_with_multi_variant_enum_keys() {
        let temp_root = make_temp_project_root("set-enum-key-runtime");
        let source_path = temp_root.join("set_enum_key_runtime.apex");
        let output_path = temp_root.join("set_enum_key_runtime");
        let source = r#"
            enum E {
                A(Integer)
                B(Integer)
            }

            function main(): Integer {
                s: Set<E> = Set<E>();
                s.add(E.A(1));
                s.add(E.B(2));
                return if (s.contains(E.A(1)) && s.contains(E.B(2))) { 0; } else { 1; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("set enum key should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled set-enum-key binary");
        assert_eq!(status.code(), Some(0));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_option_is_some_in_condition() {
        let temp_root = make_temp_project_root("option-is-some-condition-runtime");
        let source_path = temp_root.join("option_is_some_condition_runtime.apex");
        let output_path = temp_root.join("option_is_some_condition_runtime");
        let source = r#"
            function choose(): Option<Integer> {
                return Option.some(1);
            }

            function main(): Integer {
                return if (choose().is_some()) { 1; } else { 2; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("option is_some condition should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled option-is-some binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_result_is_ok_in_condition() {
        let temp_root = make_temp_project_root("result-is-ok-condition-runtime");
        let source_path = temp_root.join("result_is_ok_condition_runtime.apex");
        let output_path = temp_root.join("result_is_ok_condition_runtime");
        let source = r#"
            function choose(): Result<Integer, String> {
                return Result.ok(1);
            }

            function main(): Integer {
                return if (choose().is_ok()) { 1; } else { 2; };
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("result is_ok condition should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled result-is-ok binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_string_length_on_literal_receiver() {
        let temp_root = make_temp_project_root("string-length-literal-runtime");
        let source_path = temp_root.join("string_length_literal_runtime.apex");
        let output_path = temp_root.join("string_length_literal_runtime");
        let source = r#"
            function main(): Integer {
                return "abc".length();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("string length on literal receiver should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled string-length literal binary");
        assert_eq!(status.code(), Some(3));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_string_length_on_concatenation_receiver() {
        let temp_root = make_temp_project_root("string-length-concat-runtime");
        let source_path = temp_root.join("string_length_concat_runtime.apex");
        let output_path = temp_root.join("string_length_concat_runtime");
        let source = r#"
            function main(): Integer {
                return ("a" + "bc").length();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("string length on concatenation receiver should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled string-length concat binary");
        assert_eq!(status.code(), Some(3));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_string_length_on_interpolation_receiver() {
        let temp_root = make_temp_project_root("string-length-interp-runtime");
        let source_path = temp_root.join("string_length_interp_runtime.apex");
        let output_path = temp_root.join("string_length_interp_runtime");
        let source = r#"
            function main(): Integer {
                return ("a{1}c").length();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("string length on interpolation receiver should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled string-length interpolation binary");
        assert_eq!(status.code(), Some(3));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_field_access_on_list_get_object_results() {
        let temp_root = make_temp_project_root("list-get-object-field-runtime");
        let source_path = temp_root.join("list_get_object_field_runtime.apex");
        let output_path = temp_root.join("list_get_object_field_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                xs.push(Boxed(5));
                return xs.get(0).value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("field access on list.get object result should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled list-get object field binary");
        assert_eq!(status.code(), Some(5));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_field_access_on_map_get_object_results() {
        let temp_root = make_temp_project_root("map-get-object-field-runtime");
        let source_path = temp_root.join("map_get_object_field_runtime.apex");
        let output_path = temp_root.join("map_get_object_field_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(6));
                return m.get(1).value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("field access on map.get object result should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map-get object field binary");
        assert_eq!(status.code(), Some(6));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_missing_map_get_object_results() {
        let temp_root = make_temp_project_root("map-get-missing-object-runtime");
        let source_path = temp_root.join("map_get_missing_object_runtime.apex");
        let output_path = temp_root.join("map_get_missing_object_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                return m.get(1).value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("missing map.get object result should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled missing map.get object binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_field_access_on_map_index_object_results() {
        let temp_root = make_temp_project_root("map-index-object-field-runtime");
        let source_path = temp_root.join("map_index_object_field_runtime.apex");
        let output_path = temp_root.join("map_index_object_field_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(8));
                return m[1].value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("field access on map index object result should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map index object field binary");
        assert_eq!(status.code(), Some(8));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_index_assignment_with_string_keys() {
        let temp_root = make_temp_project_root("map-index-assign-runtime");
        let source_path = temp_root.join("map_index_assign_runtime.apex");
        let output_path = temp_root.join("map_index_assign_runtime");
        let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["x"] = 21;
                return m["x"];
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("map index assignment should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map index assignment binary");
        assert_eq!(status.code(), Some(21));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_out_of_bounds_list_index_assignment() {
        let temp_root = make_temp_project_root("list-index-assign-oob-runtime");
        let source_path = temp_root.join("list_index_assign_oob_runtime.apex");
        let output_path = temp_root.join("list_index_assign_oob_runtime");
        let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs[10] = 24;
                return 24;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("out-of-bounds list assignment should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled list assignment oob binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_negative_list_index_assignment() {
        let temp_root = make_temp_project_root("list-index-assign-negative-runtime");
        let source_path = temp_root.join("list_index_assign_negative_runtime.apex");
        let output_path = temp_root.join("list_index_assign_negative_runtime");
        let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs[-1] = 25;
                return 25;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("negative list assignment should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled negative list assignment binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_missing_map_index_object_results() {
        let temp_root = make_temp_project_root("map-index-missing-object-runtime");
        let source_path = temp_root.join("map_index_missing_object_runtime.apex");
        let output_path = temp_root.join("map_index_missing_object_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                return m[1].value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("missing map index object result should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled missing map index object binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_empty_list_get_object_results() {
        let temp_root = make_temp_project_root("list-get-empty-object-runtime");
        let source_path = temp_root.join("list_get_empty_object_runtime.apex");
        let output_path = temp_root.join("list_get_empty_object_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                return xs.get(0).value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("empty list.get object result should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled empty list.get object binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_empty_list_pop_object_results() {
        let temp_root = make_temp_project_root("list-pop-empty-object-runtime");
        let source_path = temp_root.join("list_pop_empty_object_runtime.apex");
        let output_path = temp_root.join("list_pop_empty_object_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                return xs.pop().value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("empty list.pop object result should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled empty list.pop object binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_negative_list_get_index() {
        let temp_root = make_temp_project_root("list-get-negative-index-runtime");
        let source_path = temp_root.join("list_get_negative_index_runtime.apex");
        let output_path = temp_root.join("list_get_negative_index_runtime");
        let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                return xs.get(-1);
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("negative list.get index should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled negative list.get binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_negative_list_index_operator() {
        let temp_root = make_temp_project_root("list-index-negative-runtime");
        let source_path = temp_root.join("list_index_negative_runtime.apex");
        let output_path = temp_root.join("list_index_negative_runtime");
        let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                return xs[-1];
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("negative list index operator should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled negative list index operator binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_string_index_operator() {
        let temp_root = make_temp_project_root("string-index-runtime");
        let source_path = temp_root.join("string_index_runtime.apex");
        let output_path = temp_root.join("string_index_runtime");
        let source = r#"
            function main(): Integer {
                c: Char = "abc"[1];
                if (c == 'b') { return 19; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("string index operator should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled string index binary");
        assert_eq!(status.code(), Some(19));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_out_of_bounds_string_index_operator() {
        let temp_root = make_temp_project_root("string-index-oob-runtime");
        let source_path = temp_root.join("string_index_oob_runtime.apex");
        let output_path = temp_root.join("string_index_oob_runtime");
        let source = r#"
            function main(): Integer {
                c: Char = "abc"[10];
                return 20;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("out-of-bounds string index should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled string index oob binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_string_equality_on_literals() {
        let temp_root = make_temp_project_root("string-eq-literal-runtime");
        let source_path = temp_root.join("string_eq_literal_runtime.apex");
        let output_path = temp_root.join("string_eq_literal_runtime");
        let source = r#"
            function main(): Integer {
                if ("b" == "b") { return 32; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("string literal equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled string equality literal binary");
        assert_eq!(status.code(), Some(32));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_string_equality_on_expression_results() {
        let temp_root = make_temp_project_root("string-eq-expr-runtime");
        let source_path = temp_root.join("string_eq_expr_runtime.apex");
        let output_path = temp_root.join("string_eq_expr_runtime");
        let source = r#"
            import std.string.*;
            function main(): Integer {
                if (Str.concat("a", "b") == "ab") { return 33; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("string expression equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled string equality expression binary");
        assert_eq!(status.code(), Some(33));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_list_identity_equality() {
        let temp_root = make_temp_project_root("list-eq-runtime");
        let source_path = temp_root.join("list_eq_runtime.apex");
        let output_path = temp_root.join("list_eq_runtime");
        let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                if (xs == xs) { return 34; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("list identity equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled list equality binary");
        assert_eq!(status.code(), Some(34));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_identity_equality() {
        let temp_root = make_temp_project_root("map-eq-runtime");
        let source_path = temp_root.join("map_eq_runtime.apex");
        let output_path = temp_root.join("map_eq_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                mut m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(2));
                if (m == m) { return 35; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("map identity equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled map equality binary");
        assert_eq!(status.code(), Some(35));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_class_identity_equality() {
        let temp_root = make_temp_project_root("class-eq-runtime");
        let source_path = temp_root.join("class_eq_runtime.apex");
        let output_path = temp_root.join("class_eq_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(2);
                if (b == b) { return 36; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("class identity equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled class equality binary");
        assert_eq!(status.code(), Some(36));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_option_unwrap_object_identity_equality() {
        let temp_root = make_temp_project_root("option-unwrap-object-eq-runtime");
        let source_path = temp_root.join("option_unwrap_object_eq_runtime.apex");
        let output_path = temp_root.join("option_unwrap_object_eq_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(3);
                x: Option<Boxed> = Option.some(b);
                if (x.unwrap() == b) { return 37; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("Option.unwrap object identity equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled Option.unwrap object equality binary");
        assert_eq!(status.code(), Some(37));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_map_get_object_identity_equality() {
        let temp_root = make_temp_project_root("map-get-object-eq-runtime");
        let source_path = temp_root.join("map_get_object_eq_runtime.apex");
        let output_path = temp_root.join("map_get_object_eq_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Boxed = Boxed(4);
                mut m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, b);
                if (m.get(1) == b) { return 38; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("Map.get object identity equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled Map.get object equality binary");
        assert_eq!(status.code(), Some(38));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_await_timeout_unwrap_object_identity_equality() {
        let temp_root = make_temp_project_root("await-timeout-unwrap-object-eq-runtime");
        let source_path = temp_root.join("await_timeout_unwrap_object_eq_runtime.apex");
        let output_path = temp_root.join("await_timeout_unwrap_object_eq_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            async function work(): Boxed {
                return Boxed(5);
            }

            function main(): Integer {
                b: Boxed = work().await_timeout(10).unwrap();
                if (b == b) { return 39; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("await_timeout unwrap object identity equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled await_timeout unwrap object equality binary");
        assert_eq!(status.code(), Some(39));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_direct_range_method_calls() {
        let temp_root = make_temp_project_root("direct-range-method-runtime");
        let source_path = temp_root.join("direct_range_method_runtime.apex");
        let output_path = temp_root.join("direct_range_method_runtime");
        let source = r#"
            function main(): Integer {
                if (range(0, 10).has_next()) { return 40; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("direct range method call should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled direct range method binary");
        assert_eq!(status.code(), Some(40));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_direct_option_some_method_chains() {
        let temp_root = make_temp_project_root("direct-option-some-method-runtime");
        let source_path = temp_root.join("direct_option_some_method_runtime.apex");
        let output_path = temp_root.join("direct_option_some_method_runtime");
        let source = r#"
            function main(): Integer {
                if (Option.some(12).unwrap() == 12) { return 41; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("direct Option.some method chain should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled direct Option.some method binary");
        assert_eq!(status.code(), Some(41));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_direct_option_some_object_method_chains() {
        let temp_root = make_temp_project_root("direct-option-some-object-method-runtime");
        let source_path = temp_root.join("direct_option_some_object_method_runtime.apex");
        let output_path = temp_root.join("direct_option_some_object_method_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Option.some(Boxed(14)).unwrap().value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("direct Option.some object method chain should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled direct Option.some object method binary");
        assert_eq!(status.code(), Some(14));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_direct_result_ok_method_chains() {
        let temp_root = make_temp_project_root("direct-result-ok-method-runtime");
        let source_path = temp_root.join("direct_result_ok_method_runtime.apex");
        let output_path = temp_root.join("direct_result_ok_method_runtime");
        let source = r#"
            function main(): Integer {
                if (Result.ok(12).unwrap() == 12) { return 42; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("direct Result.ok method chain should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled direct Result.ok method binary");
        assert_eq!(status.code(), Some(42));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_direct_result_ok_object_method_chains() {
        let temp_root = make_temp_project_root("direct-result-ok-object-method-runtime");
        let source_path = temp_root.join("direct_result_ok_object_method_runtime.apex");
        let output_path = temp_root.join("direct_result_ok_object_method_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Result.ok(Boxed(15)).unwrap().value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("direct Result.ok object method chain should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled direct Result.ok object method binary");
        assert_eq!(status.code(), Some(15));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_direct_constructor_method_calls() {
        let temp_root = make_temp_project_root("direct-ctor-method-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nclass Boxed { value: Integer; constructor(value: Integer) { this.value = value; } function get(): Integer { return this.value; } }\nfunction main(): Integer { return Boxed(23).get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, false, false, false)
                .expect("project build should support direct constructor method calls");
        });

        let output_path = temp_root.join("smoke");
        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run direct constructor method project binary");
        assert_eq!(status.code(), Some(23));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_local_qualified_nested_enum_match_expressions() {
        let temp_root = make_temp_project_root("local-nested-enum-match-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M { enum E { A(Integer), B(Integer) } class Box { value: Integer; constructor(value: Integer) { this.value = value; } } }\nfunction main(): Integer { return (match (M.E.A(42)) { M.E.A(v) => M.Box(v), M.E.B(v) => M.Box(v) }).value; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, false, false, false).expect(
                "project build should support local qualified nested enum match expressions",
            );
        });

        let output_path = temp_root.join("smoke");
        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run local nested enum match project binary");
        assert_eq!(status.code(), Some(42));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_module_local_qualified_async_function_paths() {
        let temp_root = make_temp_project_root("module-local-qualified-async-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M { class Box { value: Integer; constructor(value: Integer) { this.value = value; } } async function mk(): M.Box { return M.Box(43); } }\nfunction main(): Integer { return await(M.mk()).value; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, false, false, false)
                .expect("project build should support module-local qualified async function paths");
        });

        let output_path = temp_root.join("smoke");
        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run module-local qualified async project binary");
        assert_eq!(status.code(), Some(43));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_deeper_local_nested_module_function_paths() {
        let temp_root = make_temp_project_root("deeper-local-nested-module-function-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M { module N { class Box { value: Integer; constructor(value: Integer) { this.value = value; } function get(): Integer { return this.value; } } function mk(): Box { return Box(51); } } }\nfunction main(): Integer { return M.N.mk().get(); }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, false, false, false)
                .expect("project build should support deeper local nested module function paths");
        });

        let output_path = temp_root.join("smoke");
        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run deeper local nested module function project binary");
        assert_eq!(status.code(), Some(51));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_run_supports_deeper_local_nested_module_async_paths() {
        let temp_root = make_temp_project_root("deeper-local-nested-module-async-project");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nmodule M { module N { class Box { value: Integer; constructor(value: Integer) { this.value = value; } } async function mk(): Box { return Box(53); } } }\nfunction main(): Integer { return await(M.N.mk()).value; }\n",
        )
        .expect("write main");

        with_current_dir(&temp_root, || {
            build_project(false, false, false, false, false)
                .expect("project build should support deeper local nested module async paths");
        });

        let output_path = temp_root.join("smoke");
        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run deeper local nested module async project binary");
        assert_eq!(status.code(), Some(53));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_direct_constructor_method_calls() {
        let temp_root = make_temp_project_root("direct-ctor-method-runtime");
        let source_path = temp_root.join("direct_ctor_method_runtime.apex");
        let output_path = temp_root.join("direct_ctor_method_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function get(): Integer { return this.value; }
            }

            function main(): Integer {
                return Boxed(23).get();
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("direct constructor method call should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled direct constructor method binary");
        assert_eq!(status.code(), Some(23));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_direct_result_error_integer_equality() {
        let temp_root = make_temp_project_root("direct-result-error-int-eq-runtime");
        let source_path = temp_root.join("direct_result_error_int_eq_runtime.apex");
        let output_path = temp_root.join("direct_result_error_int_eq_runtime");
        let source = r#"
            function main(): Integer {
                e: Integer = 7;
                if (Result.error(e) == Result.error(e)) { return 43; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("direct Result.error integer equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled direct Result.error integer equality binary");
        assert_eq!(status.code(), Some(43));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_runs_direct_result_error_object_identity_equality() {
        let temp_root = make_temp_project_root("direct-result-error-object-eq-runtime");
        let source_path = temp_root.join("direct_result_error_object_eq_runtime.apex");
        let output_path = temp_root.join("direct_result_error_object_eq_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                e: Boxed = Boxed(9);
                if (Result.error(e) == Result.error(e)) { return 44; }
                return 0;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("direct Result.error object equality should codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled direct Result.error object equality binary");
        assert_eq!(status.code(), Some(44));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_fails_fast_on_empty_list_index_object_results() {
        let temp_root = make_temp_project_root("list-index-empty-object-runtime");
        let source_path = temp_root.join("list_index_empty_object_runtime.apex");
        let output_path = temp_root.join("list_index_empty_object_runtime");
        let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                return xs[0].value;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, false, true, None, None)
            .expect("empty list index object result should still codegen");

        let status = std::process::Command::new(&output_path)
            .status()
            .expect("run compiled empty list index object binary");
        assert_eq!(status.code(), Some(1));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_supports_lambda_callee_calls() {
        let temp_root = make_temp_project_root("lambda-callee-codegen");
        let source_path = temp_root.join("lambda_callee.apex");
        let output_path = temp_root.join("lambda_callee");
        let source = r#"
            function main(): None {
                x: Integer = ((y: Integer) => y + 1)(2);
                return None;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, true, true, None, None)
            .expect("lambda callee codegen should succeed");
        assert!(output_path.with_extension("ll").exists());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_supports_indexed_function_value_callees() {
        let temp_root = make_temp_project_root("indexed-function-callee-codegen");
        let source_path = temp_root.join("indexed_function_callee.apex");
        let output_path = temp_root.join("indexed_function_callee");
        let source = r#"
            function inc(x: Integer): Integer { return x + 1; }
            function dec(x: Integer): Integer { return x - 1; }

            function main(): None {
                fs: List<(Integer) -> Integer> = List<(Integer) -> Integer>();
                fs.push(inc);
                fs.push(dec);
                x: Integer = fs[0](1);
                return None;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, true, true, None, None)
            .expect("indexed function-value callee should codegen");
        assert!(output_path.with_extension("ll").exists());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_supports_if_expression_function_value_callees() {
        let temp_root = make_temp_project_root("ifexpr-function-callee-codegen");
        let source_path = temp_root.join("ifexpr_function_callee.apex");
        let output_path = temp_root.join("ifexpr_function_callee");
        let source = r#"
            function inc(x: Integer): Integer { return x + 1; }
            function dec(x: Integer): Integer { return x - 1; }

            function main(): None {
                x: Integer = (if (true) { inc; } else { dec; })(1);
                return None;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, true, true, None, None)
            .expect("if-expression function-value callee should codegen");
        assert!(output_path.with_extension("ll").exists());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_supports_function_valued_field_calls() {
        let temp_root = make_temp_project_root("function-field-call-codegen");
        let source_path = temp_root.join("function_field_call.apex");
        let output_path = temp_root.join("function_field_call");
        let source = r#"
            class C {
                f: (Integer) -> Integer;
                constructor() { this.f = (n: Integer) => n + 1; }
            }

            function main(): None {
                c: C = C();
                x: Integer = c.f(2);
                return None;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, true, true, None, None)
            .expect("function-valued field calls should codegen");
        assert!(output_path.with_extension("ll").exists());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn compile_source_supports_generic_method_returning_lambda() {
        let temp_root = make_temp_project_root("generic-method-lambda-codegen");
        let source_path = temp_root.join("generic_method_lambda.apex");
        let output_path = temp_root.join("generic_method_lambda");
        let source = r#"
            class C {
                function mk<T>(x: T): () -> T { return () => x; }
            }

            function main(): None {
                c: C = C();
                f: () -> Integer = c.mk<Integer>(7);
                x: Integer = f();
                return None;
            }
        "#;

        fs::write(&source_path, source).expect("write source");
        compile_source(source, &source_path, &output_path, true, true, None, None)
            .expect("generic method returning lambda should codegen");
        assert!(output_path.with_extension("ll").exists());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn cli_format_targets_checks_and_formats_project_files() {
        let temp_root = make_temp_project_root("cli-fmt");
        let src_dir = temp_root.join("src");
        write_test_project_config(&temp_root, &["src/main.apex"], "src/main.apex", "smoke");
        let main_file = src_dir.join("main.apex");
        fs::write(
            &main_file,
            "function main(): None {println(\"hi\");return None;}\n",
        )
        .expect("write unformatted file");

        with_current_dir(&temp_root, || {
            let err = format_targets(None, true).expect_err("format check should fail before fmt");
            assert!(err.contains("format check failed"), "{err}");
            format_targets(None, false).expect("format should succeed");
            format_targets(None, true).expect("format check should pass after fmt");
        });

        let formatted = fs::read_to_string(&main_file).expect("read formatted file");
        assert!(
            formatted.contains("function main(): None {\n"),
            "{formatted}"
        );
        assert!(formatted.contains("    println(\"hi\");\n"), "{formatted}");

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn cli_run_tests_lists_filtered_tests_in_directory() {
        let temp_root = make_temp_project_root("cli-test-list");
        let test_file = temp_root.join("smoke_test.apex");
        fs::write(
            &test_file,
            r#"
                @Test
                function smokeAlpha(): None { return None; }

                @Test
                function otherBeta(): None { return None; }
            "#,
        )
        .expect("write test file");

        run_tests(Some(&temp_root), true, Some("smoke")).expect("test listing should succeed");

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn cli_check_command_reports_cross_file_type_errors() {
        let temp_root = make_temp_project_root("cli-check-type-error");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/helper.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction main(): None { value: Integer = helper(); return None; }\n",
        )
        .expect("write main");
        fs::write(
            src_dir.join("helper.apex"),
            "package app;\nfunction helper(): String { return \"oops\"; }\n",
        )
        .expect("write helper");

        with_current_dir(&temp_root, || {
            let err = check_command(None, false).expect_err("project check should fail");
            assert!(
                err.contains("Type mismatch")
                    || err.contains("expected Integer")
                    || err.contains("Expected Integer"),
                "{err}"
            );
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn cli_check_command_reports_cross_file_borrow_errors() {
        let temp_root = make_temp_project_root("cli-check-borrow-error");
        let src_dir = temp_root.join("src");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/helper.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            src_dir.join("main.apex"),
            "package app;\nfunction main(): None { s: String = \"hello\"; consume(s); t: String = s; return None; }\n",
        )
        .expect("write main");
        fs::write(
            src_dir.join("helper.apex"),
            "package app;\nfunction consume(owned s: String): None { return None; }\n",
        )
        .expect("write helper");

        with_current_dir(&temp_root, || {
            let err = check_command(None, false).expect_err("project check should fail");
            assert!(
                err.contains("Use of moved value 's'") || err.contains("moved value 's'"),
                "{err}"
            );
        });

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_rewrite_fingerprint_ignores_body_only_dependency_change() {
        let temp_root = make_temp_project_root("rewrite-fp-body-only");
        let src_dir = temp_root.join("src");
        let main_file = src_dir.join("main.apex");
        let helper_file = src_dir.join("helper.apex");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/helper.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            &main_file,
            "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
        )
        .expect("write main");
        fs::write(
            &helper_file,
            "package lib;\nfunction foo(): Integer { return 1; }\n",
        )
        .expect("write helper");
        let parsed_before = vec![
            parse_project_unit(&temp_root, &main_file).expect("parse main before"),
            parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
        ];
        let (
            _namespace_files_map_before,
            namespace_function_files_before,
            namespace_class_files_before,
            namespace_module_files_before,
            global_function_map_before,
            global_function_file_map_before,
            global_class_map_before,
            global_class_file_map_before,
            global_enum_map_before,
            global_enum_file_map_before,
            global_module_map_before,
            global_module_file_map_before,
        ) = collect_project_symbol_maps(&parsed_before);
        let namespace_functions_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.function_names.iter().cloned());
                acc
            },
        );
        let namespace_classes_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.class_names.iter().cloned());
                acc
            },
        );
        let namespace_modules_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.module_names.iter().cloned());
                acc
            },
        );
        let namespace_api_fingerprints_before = compute_namespace_api_fingerprints(&parsed_before);
        let file_api_fingerprints_before = parsed_before
            .iter()
            .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
            .collect::<HashMap<_, _>>();
        let rewrite_ctx_before = RewriteFingerprintContext {
            namespace_functions: &namespace_functions_before,
            namespace_function_files: &namespace_function_files_before,
            global_function_map: &global_function_map_before,
            global_function_file_map: &global_function_file_map_before,
            namespace_classes: &namespace_classes_before,
            namespace_class_files: &namespace_class_files_before,
            global_class_map: &global_class_map_before,
            global_class_file_map: &global_class_file_map_before,
            global_enum_map: &global_enum_map_before,
            global_enum_file_map: &global_enum_file_map_before,
            namespace_modules: &namespace_modules_before,
            namespace_module_files: &namespace_module_files_before,
            global_module_map: &global_module_map_before,
            global_module_file_map: &global_module_file_map_before,
            namespace_api_fingerprints: &namespace_api_fingerprints_before,
            file_api_fingerprints: &file_api_fingerprints_before,
        };
        let main_before = parsed_before
            .iter()
            .find(|u| u.file == main_file)
            .expect("main");
        let rewrite_fp_before =
            compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

        thread::sleep(Duration::from_millis(5));
        fs::write(
            &helper_file,
            "package lib;\nfunction foo(): Integer { return 2; }\n",
        )
        .expect("rewrite helper body");

        let parsed_files = vec![
            parse_project_unit(&temp_root, &main_file).expect("parse main after"),
            parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
        ];
        let (
            namespace_files_map,
            namespace_function_files,
            namespace_class_files,
            namespace_module_files,
            global_function_map,
            global_function_file_map,
            global_class_map,
            global_class_file_map,
            global_enum_map,
            global_enum_file_map,
            global_module_map,
            global_module_file_map,
        ) = collect_project_symbol_maps(&parsed_files);
        let namespace_functions = parsed_files.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.function_names.iter().cloned());
                acc
            },
        );
        let namespace_classes = parsed_files.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.class_names.iter().cloned());
                acc
            },
        );
        let namespace_modules = parsed_files.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.module_names.iter().cloned());
                acc
            },
        );
        let namespace_api_fingerprints = compute_namespace_api_fingerprints(&parsed_files);
        let file_api_fingerprints = parsed_files
            .iter()
            .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
            .collect::<HashMap<_, _>>();
        let rewrite_ctx = RewriteFingerprintContext {
            namespace_functions: &namespace_functions,
            namespace_function_files: &namespace_function_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            namespace_class_files: &namespace_class_files,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            namespace_modules: &namespace_modules,
            namespace_module_files: &namespace_module_files,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
            namespace_api_fingerprints: &namespace_api_fingerprints,
            file_api_fingerprints: &file_api_fingerprints,
        };
        let main_unit = parsed_files
            .iter()
            .find(|u| u.file == main_file)
            .expect("main");
        let rewrite_fp_after =
            compute_rewrite_context_fingerprint_for_unit(main_unit, "app", &rewrite_ctx);
        let _ = namespace_files_map;

        assert_eq!(rewrite_fp_before, rewrite_fp_after);

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn project_rewrite_fingerprint_changes_on_import_breaking_api_change() {
        let temp_root = make_temp_project_root("rewrite-fp-api-change");
        let src_dir = temp_root.join("src");
        let main_file = src_dir.join("main.apex");
        let helper_file = src_dir.join("helper.apex");
        write_test_project_config(
            &temp_root,
            &["src/main.apex", "src/helper.apex"],
            "src/main.apex",
            "smoke",
        );
        fs::write(
            &main_file,
            "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
        )
        .expect("write main");
        fs::write(
            &helper_file,
            "package lib;\nfunction foo(): Integer { return 1; }\n",
        )
        .expect("write helper");
        let parsed_before = vec![
            parse_project_unit(&temp_root, &main_file).expect("parse main before"),
            parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
        ];
        let (
            _namespace_files_map_before,
            namespace_function_files_before,
            namespace_class_files_before,
            namespace_module_files_before,
            global_function_map_before,
            global_function_file_map_before,
            global_class_map_before,
            global_class_file_map_before,
            global_enum_map_before,
            global_enum_file_map_before,
            global_module_map_before,
            global_module_file_map_before,
        ) = collect_project_symbol_maps(&parsed_before);
        let namespace_functions_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.function_names.iter().cloned());
                acc
            },
        );
        let namespace_classes_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.class_names.iter().cloned());
                acc
            },
        );
        let namespace_modules_before = parsed_before.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.module_names.iter().cloned());
                acc
            },
        );
        let namespace_api_fingerprints_before = compute_namespace_api_fingerprints(&parsed_before);
        let file_api_fingerprints_before = parsed_before
            .iter()
            .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
            .collect::<HashMap<_, _>>();
        let rewrite_ctx_before = RewriteFingerprintContext {
            namespace_functions: &namespace_functions_before,
            namespace_function_files: &namespace_function_files_before,
            global_function_map: &global_function_map_before,
            global_function_file_map: &global_function_file_map_before,
            namespace_classes: &namespace_classes_before,
            namespace_class_files: &namespace_class_files_before,
            global_class_map: &global_class_map_before,
            global_class_file_map: &global_class_file_map_before,
            global_enum_map: &global_enum_map_before,
            global_enum_file_map: &global_enum_file_map_before,
            namespace_modules: &namespace_modules_before,
            namespace_module_files: &namespace_module_files_before,
            global_module_map: &global_module_map_before,
            global_module_file_map: &global_module_file_map_before,
            namespace_api_fingerprints: &namespace_api_fingerprints_before,
            file_api_fingerprints: &file_api_fingerprints_before,
        };
        let main_before = parsed_before
            .iter()
            .find(|u| u.file == main_file)
            .expect("main");
        let rewrite_fp_before =
            compute_rewrite_context_fingerprint_for_unit(main_before, "app", &rewrite_ctx_before);

        thread::sleep(Duration::from_millis(5));
        fs::write(
            &helper_file,
            "package lib;\nfunction bar(): Integer { return 1; }\n",
        )
        .expect("rewrite helper api");

        let parsed_files = vec![
            parse_project_unit(&temp_root, &main_file).expect("parse main"),
            parse_project_unit(&temp_root, &helper_file).expect("parse helper"),
        ];
        let (
            _namespace_files_map,
            namespace_function_files,
            namespace_class_files,
            namespace_module_files,
            global_function_map,
            global_function_file_map,
            global_class_map,
            global_class_file_map,
            global_enum_map,
            global_enum_file_map,
            global_module_map,
            global_module_file_map,
        ) = collect_project_symbol_maps(&parsed_files);
        let namespace_functions = parsed_files.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.function_names.iter().cloned());
                acc
            },
        );
        let namespace_classes = parsed_files.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.class_names.iter().cloned());
                acc
            },
        );
        let namespace_modules = parsed_files.iter().fold(
            HashMap::<String, HashSet<String>>::new(),
            |mut acc, unit| {
                acc.entry(unit.namespace.clone())
                    .or_default()
                    .extend(unit.module_names.iter().cloned());
                acc
            },
        );
        let namespace_api_fingerprints = compute_namespace_api_fingerprints(&parsed_files);
        let file_api_fingerprints = parsed_files
            .iter()
            .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
            .collect::<HashMap<_, _>>();
        let rewrite_ctx = RewriteFingerprintContext {
            namespace_functions: &namespace_functions,
            namespace_function_files: &namespace_function_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            namespace_class_files: &namespace_class_files,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            namespace_modules: &namespace_modules,
            namespace_module_files: &namespace_module_files,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
            namespace_api_fingerprints: &namespace_api_fingerprints,
            file_api_fingerprints: &file_api_fingerprints,
        };
        let main_unit = parsed_files
            .iter()
            .find(|u| u.file == main_file)
            .expect("main");
        let rewrite_fp_after =
            compute_rewrite_context_fingerprint_for_unit(main_unit, "app", &rewrite_ctx);

        assert_ne!(rewrite_fp_before, rewrite_fp_after);

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn generated_project_rewrite_fingerprint_matrix_matches_expected_invalidation() {
        let body_only_variants = [
            "package lib;\nfunction foo(): Integer { return 2; }\n",
            "package lib;\nfunction foo(): Integer { return 99; }\n",
        ];
        let import_breaking_variants = [
            "package lib;\nfunction bar(): Integer { return 1; }\n",
            "package lib;\nfunction foo(x: Integer): Integer { return x; }\n",
        ];

        for helper_after in body_only_variants {
            let temp_root = make_temp_project_root("generated-rewrite-body");
            let src_dir = temp_root.join("src");
            let main_file = src_dir.join("main.apex");
            let helper_file = src_dir.join("helper.apex");
            write_test_project_config(
                &temp_root,
                &["src/main.apex", "src/helper.apex"],
                "src/main.apex",
                "smoke",
            );
            fs::write(
                &main_file,
                "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
            )
            .expect("write main");
            fs::write(
                &helper_file,
                "package lib;\nfunction foo(): Integer { return 1; }\n",
            )
            .expect("write helper");

            let parsed_before = vec![
                parse_project_unit(&temp_root, &main_file).expect("parse main before"),
                parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
            ];
            let (
                _namespace_files_map_before,
                namespace_function_files_before,
                namespace_class_files_before,
                namespace_module_files_before,
                global_function_map_before,
                global_function_file_map_before,
                global_class_map_before,
                global_class_file_map_before,
                global_enum_map_before,
                global_enum_file_map_before,
                global_module_map_before,
                global_module_file_map_before,
            ) = collect_project_symbol_maps(&parsed_before);
            let namespace_functions_before = parsed_before.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.function_names.iter().cloned());
                    acc
                },
            );
            let namespace_classes_before = parsed_before.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.class_names.iter().cloned());
                    acc
                },
            );
            let namespace_modules_before = parsed_before.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.module_names.iter().cloned());
                    acc
                },
            );
            let namespace_api_fingerprints_before =
                compute_namespace_api_fingerprints(&parsed_before);
            let file_api_fingerprints_before = parsed_before
                .iter()
                .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
                .collect::<HashMap<_, _>>();
            let rewrite_ctx_before = RewriteFingerprintContext {
                namespace_functions: &namespace_functions_before,
                namespace_function_files: &namespace_function_files_before,
                global_function_map: &global_function_map_before,
                global_function_file_map: &global_function_file_map_before,
                namespace_classes: &namespace_classes_before,
                namespace_class_files: &namespace_class_files_before,
                global_class_map: &global_class_map_before,
                global_class_file_map: &global_class_file_map_before,
                global_enum_map: &global_enum_map_before,
                global_enum_file_map: &global_enum_file_map_before,
                namespace_modules: &namespace_modules_before,
                namespace_module_files: &namespace_module_files_before,
                global_module_map: &global_module_map_before,
                global_module_file_map: &global_module_file_map_before,
                namespace_api_fingerprints: &namespace_api_fingerprints_before,
                file_api_fingerprints: &file_api_fingerprints_before,
            };
            let main_before = parsed_before
                .iter()
                .find(|u| u.file == main_file)
                .expect("main");
            let rewrite_fp_before = compute_rewrite_context_fingerprint_for_unit(
                main_before,
                "app",
                &rewrite_ctx_before,
            );

            fs::write(&helper_file, helper_after).expect("rewrite helper body variant");
            let parsed_after = vec![
                parse_project_unit(&temp_root, &main_file).expect("parse main after"),
                parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
            ];
            let (
                _namespace_files_map_after,
                namespace_function_files_after,
                namespace_class_files_after,
                namespace_module_files_after,
                global_function_map_after,
                global_function_file_map_after,
                global_class_map_after,
                global_class_file_map_after,
                global_enum_map_after,
                global_enum_file_map_after,
                global_module_map_after,
                global_module_file_map_after,
            ) = collect_project_symbol_maps(&parsed_after);
            let namespace_functions_after = parsed_after.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.function_names.iter().cloned());
                    acc
                },
            );
            let namespace_classes_after = parsed_after.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.class_names.iter().cloned());
                    acc
                },
            );
            let namespace_modules_after = parsed_after.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.module_names.iter().cloned());
                    acc
                },
            );
            let namespace_api_fingerprints_after =
                compute_namespace_api_fingerprints(&parsed_after);
            let file_api_fingerprints_after = parsed_after
                .iter()
                .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
                .collect::<HashMap<_, _>>();
            let rewrite_ctx_after = RewriteFingerprintContext {
                namespace_functions: &namespace_functions_after,
                namespace_function_files: &namespace_function_files_after,
                global_function_map: &global_function_map_after,
                global_function_file_map: &global_function_file_map_after,
                namespace_classes: &namespace_classes_after,
                namespace_class_files: &namespace_class_files_after,
                global_class_map: &global_class_map_after,
                global_class_file_map: &global_class_file_map_after,
                global_enum_map: &global_enum_map_after,
                global_enum_file_map: &global_enum_file_map_after,
                namespace_modules: &namespace_modules_after,
                namespace_module_files: &namespace_module_files_after,
                global_module_map: &global_module_map_after,
                global_module_file_map: &global_module_file_map_after,
                namespace_api_fingerprints: &namespace_api_fingerprints_after,
                file_api_fingerprints: &file_api_fingerprints_after,
            };
            let main_after = parsed_after
                .iter()
                .find(|u| u.file == main_file)
                .expect("main");
            let rewrite_fp_after =
                compute_rewrite_context_fingerprint_for_unit(main_after, "app", &rewrite_ctx_after);

            assert_eq!(rewrite_fp_before, rewrite_fp_after);
            let _ = fs::remove_dir_all(temp_root);
        }

        for helper_after in import_breaking_variants {
            let temp_root = make_temp_project_root("generated-rewrite-api");
            let src_dir = temp_root.join("src");
            let main_file = src_dir.join("main.apex");
            let helper_file = src_dir.join("helper.apex");
            write_test_project_config(
                &temp_root,
                &["src/main.apex", "src/helper.apex"],
                "src/main.apex",
                "smoke",
            );
            fs::write(
                &main_file,
                "package app;\nimport lib.foo;\nfunction main(): None { value: Integer = foo(); return None; }\n",
            )
            .expect("write main");
            fs::write(
                &helper_file,
                "package lib;\nfunction foo(): Integer { return 1; }\n",
            )
            .expect("write helper");

            let parsed_before = vec![
                parse_project_unit(&temp_root, &main_file).expect("parse main before"),
                parse_project_unit(&temp_root, &helper_file).expect("parse helper before"),
            ];
            let (
                _namespace_files_map_before,
                namespace_function_files_before,
                namespace_class_files_before,
                namespace_module_files_before,
                global_function_map_before,
                global_function_file_map_before,
                global_class_map_before,
                global_class_file_map_before,
                global_enum_map_before,
                global_enum_file_map_before,
                global_module_map_before,
                global_module_file_map_before,
            ) = collect_project_symbol_maps(&parsed_before);
            let namespace_functions_before = parsed_before.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.function_names.iter().cloned());
                    acc
                },
            );
            let namespace_classes_before = parsed_before.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.class_names.iter().cloned());
                    acc
                },
            );
            let namespace_modules_before = parsed_before.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.module_names.iter().cloned());
                    acc
                },
            );
            let namespace_api_fingerprints_before =
                compute_namespace_api_fingerprints(&parsed_before);
            let file_api_fingerprints_before = parsed_before
                .iter()
                .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
                .collect::<HashMap<_, _>>();
            let rewrite_ctx_before = RewriteFingerprintContext {
                namespace_functions: &namespace_functions_before,
                namespace_function_files: &namespace_function_files_before,
                global_function_map: &global_function_map_before,
                global_function_file_map: &global_function_file_map_before,
                namespace_classes: &namespace_classes_before,
                namespace_class_files: &namespace_class_files_before,
                global_class_map: &global_class_map_before,
                global_class_file_map: &global_class_file_map_before,
                global_enum_map: &global_enum_map_before,
                global_enum_file_map: &global_enum_file_map_before,
                namespace_modules: &namespace_modules_before,
                namespace_module_files: &namespace_module_files_before,
                global_module_map: &global_module_map_before,
                global_module_file_map: &global_module_file_map_before,
                namespace_api_fingerprints: &namespace_api_fingerprints_before,
                file_api_fingerprints: &file_api_fingerprints_before,
            };
            let main_before = parsed_before
                .iter()
                .find(|u| u.file == main_file)
                .expect("main");
            let rewrite_fp_before = compute_rewrite_context_fingerprint_for_unit(
                main_before,
                "app",
                &rewrite_ctx_before,
            );

            fs::write(&helper_file, helper_after).expect("rewrite helper api variant");
            let parsed_after = vec![
                parse_project_unit(&temp_root, &main_file).expect("parse main after"),
                parse_project_unit(&temp_root, &helper_file).expect("parse helper after"),
            ];
            let (
                _namespace_files_map_after,
                namespace_function_files_after,
                namespace_class_files_after,
                namespace_module_files_after,
                global_function_map_after,
                global_function_file_map_after,
                global_class_map_after,
                global_class_file_map_after,
                global_enum_map_after,
                global_enum_file_map_after,
                global_module_map_after,
                global_module_file_map_after,
            ) = collect_project_symbol_maps(&parsed_after);
            let namespace_functions_after = parsed_after.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.function_names.iter().cloned());
                    acc
                },
            );
            let namespace_classes_after = parsed_after.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.class_names.iter().cloned());
                    acc
                },
            );
            let namespace_modules_after = parsed_after.iter().fold(
                HashMap::<String, HashSet<String>>::new(),
                |mut acc, unit| {
                    acc.entry(unit.namespace.clone())
                        .or_default()
                        .extend(unit.module_names.iter().cloned());
                    acc
                },
            );
            let namespace_api_fingerprints_after =
                compute_namespace_api_fingerprints(&parsed_after);
            let file_api_fingerprints_after = parsed_after
                .iter()
                .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
                .collect::<HashMap<_, _>>();
            let rewrite_ctx_after = RewriteFingerprintContext {
                namespace_functions: &namespace_functions_after,
                namespace_function_files: &namespace_function_files_after,
                global_function_map: &global_function_map_after,
                global_function_file_map: &global_function_file_map_after,
                namespace_classes: &namespace_classes_after,
                namespace_class_files: &namespace_class_files_after,
                global_class_map: &global_class_map_after,
                global_class_file_map: &global_class_file_map_after,
                global_enum_map: &global_enum_map_after,
                global_enum_file_map: &global_enum_file_map_after,
                namespace_modules: &namespace_modules_after,
                namespace_module_files: &namespace_module_files_after,
                global_module_map: &global_module_map_after,
                global_module_file_map: &global_module_file_map_after,
                namespace_api_fingerprints: &namespace_api_fingerprints_after,
                file_api_fingerprints: &file_api_fingerprints_after,
            };
            let main_after = parsed_after
                .iter()
                .find(|u| u.file == main_file)
                .expect("main");
            let rewrite_fp_after =
                compute_rewrite_context_fingerprint_for_unit(main_after, "app", &rewrite_ctx_after);

            assert_ne!(rewrite_fp_before, rewrite_fp_after);
            let _ = fs::remove_dir_all(temp_root);
        }
    }

    fn make_unit(file: &str, namespace: &str, imports: &[&str]) -> ParsedProjectUnit {
        ParsedProjectUnit {
            file: PathBuf::from(file),
            namespace: namespace.to_string(),
            program: Program {
                package: Some(namespace.to_string()),
                declarations: Vec::new(),
            },
            imports: imports
                .iter()
                .map(|path| ImportDecl {
                    path: (*path).to_string(),
                    alias: None,
                })
                .collect(),
            api_fingerprint: "api".to_string(),
            semantic_fingerprint: "sem".to_string(),
            import_check_fingerprint: "import".to_string(),
            function_names: Vec::new(),
            class_names: Vec::new(),
            enum_names: Vec::new(),
            module_names: Vec::new(),
            referenced_symbols: Vec::new(),
            qualified_symbol_refs: Vec::new(),
            api_referenced_symbols: Vec::new(),
            from_parse_cache: false,
        }
    }

    #[test]
    fn rewrite_context_for_specific_import_ignores_unrelated_namespace_api_changes() {
        let unit = make_unit("src/main.apex", "app", &["lib.foo"]);

        let namespace_functions = HashMap::from([(
            "lib".to_string(),
            HashSet::from(["foo".to_string(), "bar".to_string()]),
        )]);
        let global_function_map = HashMap::from([
            ("foo".to_string(), "lib".to_string()),
            ("bar".to_string(), "lib".to_string()),
        ]);
        let global_function_file_map = HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]);
        let namespace_function_files = HashMap::from([(
            "lib".to_string(),
            HashMap::from([
                ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
                ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
            ]),
        )]);
        let namespace_classes = HashMap::new();
        let namespace_class_files = HashMap::new();
        let global_class_map = HashMap::new();
        let global_class_file_map = HashMap::new();
        let global_enum_map = HashMap::new();
        let global_enum_file_map = HashMap::new();
        let namespace_modules = HashMap::new();
        let namespace_module_files = HashMap::new();
        let global_module_map = HashMap::new();
        let global_module_file_map = HashMap::new();
        let namespace_api_fingerprints = HashMap::from([("lib".to_string(), "ns-v1".to_string())]);
        let file_api_fingerprints = HashMap::from([
            (PathBuf::from("src/lib_foo.apex"), "file-foo-v1".to_string()),
            (PathBuf::from("src/lib_bar.apex"), "file-bar-v1".to_string()),
        ]);
        let ctx_a = RewriteFingerprintContext {
            namespace_functions: &namespace_functions,
            namespace_function_files: &namespace_function_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            namespace_class_files: &namespace_class_files,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            namespace_modules: &namespace_modules,
            namespace_module_files: &namespace_module_files,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
            namespace_api_fingerprints: &namespace_api_fingerprints,
            file_api_fingerprints: &file_api_fingerprints,
        };

        let fp_a = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_a);
        let namespace_api_fingerprints_b =
            HashMap::from([("lib".to_string(), "ns-v2".to_string())]);
        let file_api_fingerprints_b = HashMap::from([
            (PathBuf::from("src/lib_foo.apex"), "file-foo-v1".to_string()),
            (PathBuf::from("src/lib_bar.apex"), "file-bar-v2".to_string()),
        ]);
        let ctx_b = RewriteFingerprintContext {
            namespace_functions: &namespace_functions,
            namespace_function_files: &namespace_function_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            namespace_class_files: &namespace_class_files,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            namespace_modules: &namespace_modules,
            namespace_module_files: &namespace_module_files,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
            namespace_api_fingerprints: &namespace_api_fingerprints_b,
            file_api_fingerprints: &file_api_fingerprints_b,
        };
        let fp_b = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_b);

        assert_eq!(fp_a, fp_b);
    }

    #[test]
    fn rewrite_context_for_wildcard_import_tracks_namespace_api_changes() {
        let unit = make_unit("src/main.apex", "app", &["lib.*"]);

        let namespace_functions = HashMap::from([(
            "lib".to_string(),
            HashSet::from(["foo".to_string(), "bar".to_string()]),
        )]);
        let global_function_map = HashMap::from([
            ("foo".to_string(), "lib".to_string()),
            ("bar".to_string(), "lib".to_string()),
        ]);
        let global_function_file_map = HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]);
        let namespace_function_files = HashMap::from([(
            "lib".to_string(),
            HashMap::from([
                ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
                ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
            ]),
        )]);
        let namespace_classes = HashMap::new();
        let namespace_class_files = HashMap::new();
        let global_class_map = HashMap::new();
        let global_class_file_map = HashMap::new();
        let global_enum_map = HashMap::new();
        let global_enum_file_map = HashMap::new();
        let namespace_modules = HashMap::new();
        let namespace_module_files = HashMap::new();
        let global_module_map = HashMap::new();
        let global_module_file_map = HashMap::new();
        let namespace_api_fingerprints_a =
            HashMap::from([("lib".to_string(), "ns-v1".to_string())]);
        let ctx_a = RewriteFingerprintContext {
            namespace_functions: &namespace_functions,
            namespace_function_files: &namespace_function_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            namespace_class_files: &namespace_class_files,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            namespace_modules: &namespace_modules,
            namespace_module_files: &namespace_module_files,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
            namespace_api_fingerprints: &namespace_api_fingerprints_a,
            file_api_fingerprints: &HashMap::new(),
        };
        let fp_a = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_a);
        let namespace_api_fingerprints_b =
            HashMap::from([("lib".to_string(), "ns-v2".to_string())]);
        let ctx_b = RewriteFingerprintContext {
            namespace_functions: &namespace_functions,
            namespace_function_files: &namespace_function_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            namespace_class_files: &namespace_class_files,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            namespace_modules: &namespace_modules,
            namespace_module_files: &namespace_module_files,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
            namespace_api_fingerprints: &namespace_api_fingerprints_b,
            file_api_fingerprints: &HashMap::new(),
        };
        let fp_b = compute_rewrite_context_fingerprint_for_unit(&unit, "app", &ctx_b);

        assert_ne!(fp_a, fp_b);
    }

    #[test]
    fn dependency_graph_tracks_specific_symbol_owner_file_only() {
        let app = make_unit("src/main.apex", "app", &["lib.foo"]);
        let foo = make_unit("src/lib_foo.apex", "lib", &[]);
        let bar = make_unit("src/lib_bar.apex", "lib", &[]);
        let parsed_files = vec![app.clone(), foo, bar];
        let namespace_files_map = HashMap::from([
            ("app".to_string(), vec![PathBuf::from("src/main.apex")]),
            (
                "lib".to_string(),
                vec![
                    PathBuf::from("src/lib_bar.apex"),
                    PathBuf::from("src/lib_foo.apex"),
                ],
            ),
        ]);

        let global_function_map = HashMap::from([
            ("foo".to_string(), "lib".to_string()),
            ("bar".to_string(), "lib".to_string()),
        ]);
        let global_function_file_map = HashMap::from([
            ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
            ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
        ]);
        let global_class_map = HashMap::new();
        let global_class_file_map = HashMap::new();
        let global_enum_map = HashMap::new();
        let global_enum_file_map = HashMap::new();
        let global_module_map = HashMap::new();
        let global_module_file_map = HashMap::new();
        let namespace_function_files = HashMap::from([(
            "lib".to_string(),
            HashMap::from([
                ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
                ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
            ]),
        )]);
        let namespace_class_files = HashMap::new();
        let namespace_module_files = HashMap::new();
        let ctx = DependencyResolutionContext {
            namespace_files_map: &namespace_files_map,
            namespace_function_files: &namespace_function_files,
            namespace_class_files: &namespace_class_files,
            namespace_module_files: &namespace_module_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
        };
        let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None);

        assert_eq!(
            graph.get(&app.file).cloned().unwrap_or_default(),
            HashSet::from([PathBuf::from("src/lib_foo.apex")])
        );
    }

    #[test]
    fn dependency_graph_tracks_same_namespace_symbol_references() {
        let mut app = make_unit("src/app.apex", "app", &[]);
        app.referenced_symbols = vec!["helper".to_string()];
        let mut helper = make_unit("src/helper.apex", "app", &[]);
        helper.function_names = vec!["helper".to_string()];
        let parsed_files = vec![app.clone(), helper.clone()];
        let namespace_files_map = HashMap::from([(
            "app".to_string(),
            vec![
                PathBuf::from("src/app.apex"),
                PathBuf::from("src/helper.apex"),
            ],
        )]);
        let namespace_function_files = HashMap::from([(
            "app".to_string(),
            HashMap::from([("helper".to_string(), PathBuf::from("src/helper.apex"))]),
        )]);
        let namespace_class_files = HashMap::new();
        let namespace_module_files = HashMap::new();
        let global_function_map = HashMap::from([("helper".to_string(), "app".to_string())]);
        let global_function_file_map =
            HashMap::from([("helper".to_string(), PathBuf::from("src/helper.apex"))]);
        let global_class_map = HashMap::new();
        let global_class_file_map = HashMap::new();
        let global_enum_map = HashMap::new();
        let global_enum_file_map = HashMap::new();
        let global_module_map = HashMap::new();
        let global_module_file_map = HashMap::new();
        let ctx = DependencyResolutionContext {
            namespace_files_map: &namespace_files_map,
            namespace_function_files: &namespace_function_files,
            namespace_class_files: &namespace_class_files,
            namespace_module_files: &namespace_module_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
        };

        let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None);

        assert_eq!(
            graph.get(&app.file).cloned().unwrap_or_default(),
            HashSet::from([PathBuf::from("src/helper.apex")])
        );
        assert!(graph
            .get(&helper.file)
            .cloned()
            .unwrap_or_default()
            .is_empty());
    }

    #[test]
    fn dependency_graph_limits_wildcard_imports_to_used_owner_files() {
        let mut app = make_unit("src/main.apex", "app", &["lib.*"]);
        app.referenced_symbols = vec!["foo".to_string()];
        let mut foo = make_unit("src/lib_foo.apex", "lib", &[]);
        foo.function_names = vec!["foo".to_string()];
        let mut bar = make_unit("src/lib_bar.apex", "lib", &[]);
        bar.function_names = vec!["bar".to_string()];
        let parsed_files = vec![app.clone(), foo, bar];
        let namespace_files_map = HashMap::from([
            ("app".to_string(), vec![PathBuf::from("src/main.apex")]),
            (
                "lib".to_string(),
                vec![
                    PathBuf::from("src/lib_bar.apex"),
                    PathBuf::from("src/lib_foo.apex"),
                ],
            ),
        ]);
        let namespace_function_files = HashMap::from([(
            "lib".to_string(),
            HashMap::from([
                ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
                ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
            ]),
        )]);
        let ctx = DependencyResolutionContext {
            namespace_files_map: &namespace_files_map,
            namespace_function_files: &namespace_function_files,
            namespace_class_files: &HashMap::new(),
            namespace_module_files: &HashMap::new(),
            global_function_map: &HashMap::from([
                ("foo".to_string(), "lib".to_string()),
                ("bar".to_string(), "lib".to_string()),
            ]),
            global_function_file_map: &HashMap::from([
                ("foo".to_string(), PathBuf::from("src/lib_foo.apex")),
                ("bar".to_string(), PathBuf::from("src/lib_bar.apex")),
            ]),
            global_class_map: &HashMap::new(),
            global_class_file_map: &HashMap::new(),
            global_enum_map: &HashMap::new(),
            global_enum_file_map: &HashMap::new(),
            global_module_map: &HashMap::new(),
            global_module_file_map: &HashMap::new(),
        };

        let (graph, _) = build_file_dependency_graph_incremental(&parsed_files, &ctx, None);
        assert_eq!(
            graph.get(&app.file).cloned().unwrap_or_default(),
            HashSet::from([PathBuf::from("src/lib_foo.apex")])
        );
    }

    #[test]
    fn dependency_graph_recomputes_direct_neighbors_after_api_change() {
        let mut app = make_unit("src/app.apex", "app", &["lib.foo"]);
        app.api_fingerprint = "app-v1".to_string();
        app.semantic_fingerprint = "app-v1".to_string();
        let mut foo = make_unit("src/lib_foo.apex", "lib", &[]);
        foo.function_names = vec!["foo".to_string()];
        foo.api_fingerprint = "foo-v2".to_string();
        foo.semantic_fingerprint = "foo-v2".to_string();

        let previous = DependencyGraphCache {
            schema: DEPENDENCY_GRAPH_CACHE_SCHEMA.to_string(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            files: vec![
                DependencyGraphFileEntry {
                    file: PathBuf::from("src/app.apex"),
                    semantic_fingerprint: "app-v1".to_string(),
                    api_fingerprint: "app-v1".to_string(),
                    direct_dependencies: vec![PathBuf::from("src/lib_foo.apex")],
                },
                DependencyGraphFileEntry {
                    file: PathBuf::from("src/lib_foo.apex"),
                    semantic_fingerprint: "foo-v1".to_string(),
                    api_fingerprint: "foo-v1".to_string(),
                    direct_dependencies: vec![],
                },
            ],
        };

        let parsed_files = vec![app.clone(), foo.clone()];
        let namespace_files_map = HashMap::from([
            ("app".to_string(), vec![PathBuf::from("src/app.apex")]),
            ("lib".to_string(), vec![PathBuf::from("src/lib_foo.apex")]),
        ]);
        let namespace_function_files = HashMap::from([(
            "lib".to_string(),
            HashMap::from([("foo".to_string(), PathBuf::from("src/lib_foo.apex"))]),
        )]);
        let ctx = DependencyResolutionContext {
            namespace_files_map: &namespace_files_map,
            namespace_function_files: &namespace_function_files,
            namespace_class_files: &HashMap::new(),
            namespace_module_files: &HashMap::new(),
            global_function_map: &HashMap::from([("foo".to_string(), "lib".to_string())]),
            global_function_file_map: &HashMap::from([(
                "foo".to_string(),
                PathBuf::from("src/lib_foo.apex"),
            )]),
            global_class_map: &HashMap::new(),
            global_class_file_map: &HashMap::new(),
            global_enum_map: &HashMap::new(),
            global_enum_file_map: &HashMap::new(),
            global_module_map: &HashMap::new(),
            global_module_file_map: &HashMap::new(),
        };

        let (_, reused) =
            build_file_dependency_graph_incremental(&parsed_files, &ctx, Some(&previous));
        assert_eq!(reused, 0);
    }

    #[test]
    fn typecheck_summary_cache_matches_identical_component_fingerprints() {
        let current = HashMap::from([
            (PathBuf::from("a.apex"), "sem-a".to_string()),
            (PathBuf::from("b.apex"), "sem-b".to_string()),
        ]);
        let components = vec![vec![PathBuf::from("a.apex")], vec![PathBuf::from("b.apex")]];
        let cache = typecheck_summary_cache_from_state(&current, &components);

        assert!(typecheck_summary_cache_matches(
            &cache,
            &current,
            &components
        ));
    }

    #[test]
    fn reusable_component_fingerprints_allows_partial_semantic_reuse() {
        let previous = typecheck_summary_cache_from_state(
            &HashMap::from([
                (PathBuf::from("a.apex"), "sem-a".to_string()),
                (PathBuf::from("b.apex"), "sem-b".to_string()),
                (PathBuf::from("c.apex"), "sem-c-old".to_string()),
            ]),
            &[
                vec![PathBuf::from("a.apex"), PathBuf::from("b.apex")],
                vec![PathBuf::from("c.apex")],
            ],
        );
        let current = HashMap::from([
            (PathBuf::from("a.apex"), "sem-a".to_string()),
            (PathBuf::from("b.apex"), "sem-b".to_string()),
            (PathBuf::from("c.apex"), "sem-c-new".to_string()),
        ]);
        let components = vec![
            vec![PathBuf::from("a.apex"), PathBuf::from("b.apex")],
            vec![PathBuf::from("c.apex")],
        ];

        let reusable = reusable_component_fingerprints(&previous, &current, &components);

        assert_eq!(reusable.len(), 1);
        assert!(reusable.contains(&component_fingerprint(&components[0], &current)));
    }

    #[test]
    fn reverse_dependency_graph_returns_only_transitive_dependents() {
        let reverse = build_reverse_dependency_graph(&HashMap::from([
            (
                PathBuf::from("a.apex"),
                HashSet::from([PathBuf::from("b.apex")]),
            ),
            (
                PathBuf::from("c.apex"),
                HashSet::from([PathBuf::from("a.apex")]),
            ),
            (PathBuf::from("d.apex"), HashSet::new()),
        ]));

        let impacted = transitive_dependents(&reverse, &HashSet::from([PathBuf::from("b.apex")]));

        assert_eq!(
            impacted,
            HashSet::from([
                PathBuf::from("b.apex"),
                PathBuf::from("a.apex"),
                PathBuf::from("c.apex"),
            ])
        );
    }

    #[test]
    fn link_manifest_skip_requires_exact_manifest_match_and_no_object_misses() {
        let output_path = PathBuf::from("build/app");
        let link_inputs = vec![PathBuf::from("a.o"), PathBuf::from("b.o")];
        let link = LinkConfig {
            opt_level: Some("3"),
            target: None,
            output_kind: OutputKind::Bin,
            link_search: &[],
            link_libs: &[],
            link_args: &[],
        };
        let current = LinkManifestCache {
            schema: LINK_MANIFEST_CACHE_SCHEMA.to_string(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            link_fingerprint: compute_link_fingerprint(&output_path, &link_inputs, &link),
            link_inputs: link_inputs.clone(),
        };

        assert!(!should_skip_final_link(None, &current, &output_path, 0));
        assert!(!should_skip_final_link(
            Some(&current),
            &current,
            &output_path,
            1
        ));
    }

    #[test]
    fn link_manifest_skip_allows_relink_elision_for_identical_cached_inputs() {
        let temp =
            std::env::temp_dir().join(format!("apex-link-manifest-test-{}", std::process::id()));
        fs::write(&temp, b"bin").expect("write output placeholder");
        let link_inputs = vec![PathBuf::from("a.o"), PathBuf::from("b.o")];
        let link = LinkConfig {
            opt_level: Some("3"),
            target: None,
            output_kind: OutputKind::Bin,
            link_search: &[],
            link_libs: &[],
            link_args: &[],
        };
        let current = LinkManifestCache {
            schema: LINK_MANIFEST_CACHE_SCHEMA.to_string(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            link_fingerprint: compute_link_fingerprint(&temp, &link_inputs, &link),
            link_inputs,
        };

        assert!(should_skip_final_link(Some(&current), &current, &temp, 0));

        let _ = fs::remove_file(temp);
    }

    #[test]
    fn parse_cache_reuses_same_content_even_after_metadata_change() {
        let temp_root = std::env::temp_dir().join(format!(
            "apex-parse-cache-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let src_dir = temp_root.join("src");
        fs::create_dir_all(&src_dir).expect("create temp src dir");
        let file = src_dir.join("main.apex");
        let source = "function main(): None { return None; }\n";
        fs::write(&file, source).expect("write source");

        let first = parse_project_unit(&temp_root, &file).expect("first parse");
        assert!(!first.from_parse_cache);

        thread::sleep(Duration::from_millis(5));
        fs::write(&file, source).expect("rewrite identical source");

        let second = parse_project_unit(&temp_root, &file).expect("second parse");
        assert!(second.from_parse_cache);
        assert_eq!(first.semantic_fingerprint, second.semantic_fingerprint);

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn response_file_args_escape_quotes_and_backslashes() {
        assert_eq!(
            escape_response_file_arg("C:\\tmp\\a \"b\".o"),
            "\"C:\\\\tmp\\\\a \\\"b\\\".o\""
        );
    }

    #[test]
    fn precompute_transitive_dependencies_matches_expected_closure() {
        let graph = HashMap::from([
            (
                PathBuf::from("a.apex"),
                HashSet::from([PathBuf::from("b.apex"), PathBuf::from("c.apex")]),
            ),
            (
                PathBuf::from("b.apex"),
                HashSet::from([PathBuf::from("d.apex")]),
            ),
            (
                PathBuf::from("c.apex"),
                HashSet::from([PathBuf::from("d.apex")]),
            ),
            (PathBuf::from("d.apex"), HashSet::new()),
        ]);

        let all = precompute_all_transitive_dependencies(&graph);
        assert_eq!(
            all.get(&PathBuf::from("a.apex"))
                .cloned()
                .unwrap_or_default(),
            HashSet::from([
                PathBuf::from("b.apex"),
                PathBuf::from("c.apex"),
                PathBuf::from("d.apex"),
            ])
        );
    }

    #[test]
    fn codegen_program_for_unit_uses_full_program_for_relevant_dependency_files() {
        let make_function = |name: &str| {
            Spanned::new(
                Decl::Function(FunctionDecl {
                    name: name.to_string(),
                    params: Vec::new(),
                    return_type: Type::None,
                    body: Vec::new(),
                    generic_params: Vec::new(),
                    visibility: Visibility::Public,
                    is_async: false,
                    is_extern: false,
                    extern_abi: None,
                    extern_link_name: None,
                    attributes: Vec::new(),
                    is_variadic: false,
                }),
                0..0,
            )
        };
        let make_unit = |file: &str, body_name: &str, api_name: &str| RewrittenProjectUnit {
            file: PathBuf::from(file),
            program: Program {
                package: None,
                declarations: vec![make_function(body_name)],
            },
            api_program: Program {
                package: None,
                declarations: vec![make_function(api_name)],
            },
            semantic_fingerprint: "sem".to_string(),
            rewrite_context_fingerprint: "rw".to_string(),
            active_symbols: HashSet::from([body_name.to_string()]),
            from_rewrite_cache: false,
        };

        let rewritten_files = vec![
            make_unit("a.apex", "fa", "fa_api"),
            make_unit("b.apex", "fb", "fb_api"),
            make_unit("c.apex", "fc", "fc_api"),
        ];
        let rewritten_file_indices = HashMap::from([
            (PathBuf::from("a.apex"), 0usize),
            (PathBuf::from("b.apex"), 1usize),
            (PathBuf::from("c.apex"), 2usize),
        ]);
        let closure = HashSet::from([PathBuf::from("b.apex")]);
        let declaration_symbols = HashSet::from(["fb".to_string()]);

        let program = codegen_program_for_unit(
            &rewritten_files,
            &rewritten_file_indices,
            Path::new("a.apex"),
            Some(&closure),
            Some(&declaration_symbols),
        );

        let names = program
            .declarations
            .iter()
            .filter_map(|decl| match &decl.node {
                Decl::Function(func) => Some(func.name.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["fa".to_string(), "fb".to_string()]);
    }
}
