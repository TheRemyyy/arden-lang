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
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use std::time::UNIX_EPOCH;
use twox_hash::XxHash64;

use crate::ast::{Block, Decl, Expr, ImportDecl, Pattern, Program, Spanned, Stmt, Type};
use crate::borrowck::BorrowChecker;
use crate::codegen::Codegen;
use crate::import_check::ImportChecker;
use crate::parser::{parse_type_source, Parser};
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

fn read_cache_blob_raw(path: &Path, label: &str) -> Result<Option<Vec<u8>>, String> {
    let raw = match fs::read(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "{}: Failed to read {} '{}': {}",
                "error".red().bold(),
                label,
                path.display(),
                error
            ));
        }
    };
    Ok(Some(raw))
}

fn read_cache_blob<T: DeserializeOwned>(path: &Path, label: &str) -> Result<Option<T>, String> {
    let Some(raw) = read_cache_blob_raw(path, label)? else {
        return Ok(None);
    };
    let value = bincode::deserialize(&raw).map_err(|e| {
        format!(
            "{}: Failed to decode {} '{}': {}",
            "error".red().bold(),
            label,
            path.display(),
            e
        )
    })?;
    Ok(Some(value))
}

fn read_cache_blob_with_timing<T: DeserializeOwned>(
    path: &Path,
    label: &str,
    totals: &CacheIoTimingTotals,
) -> Result<Option<T>, String> {
    let started_at = Instant::now();
    let Some(raw) = read_cache_blob_raw(path, label)? else {
        totals
            .load_ns
            .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
        return Ok(None);
    };
    let byte_len = raw.len() as u64;
    let value = bincode::deserialize(&raw).map_err(|e| {
        format!(
            "{}: Failed to decode {} '{}': {}",
            "error".red().bold(),
            label,
            path.display(),
            e
        )
    })?;
    totals
        .load_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    totals.bytes_read.fetch_add(byte_len, Ordering::Relaxed);
    totals.load_count.fetch_add(1, Ordering::Relaxed);
    Ok(Some(value))
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

fn write_cache_blob_with_timing<T: Serialize>(
    path: &Path,
    label: &str,
    value: &T,
    totals: &CacheIoTimingTotals,
) -> Result<(), String> {
    let bytes = bincode::serialize(value).map_err(|e| {
        format!(
            "{}: Failed to serialize {} '{}': {}",
            "error".red().bold(),
            label,
            path.display(),
            e
        )
    })?;
    let byte_len = bytes.len() as u64;
    let started_at = Instant::now();
    fs::write(path, bytes).map_err(|e| {
        format!(
            "{}: Failed to write {} '{}': {}",
            "error".red().bold(),
            label,
            path.display(),
            e
        )
    })?;
    totals
        .save_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    totals.bytes_written.fetch_add(byte_len, Ordering::Relaxed);
    totals.save_count.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

fn project_build_artifact_exists(output_path: &Path, emit_llvm: bool) -> bool {
    if emit_llvm {
        output_path.with_extension("ll").exists()
    } else {
        output_path.exists()
    }
}

fn ensure_output_parent_dir(output_path: &Path) -> Result<(), String> {
    let Some(parent) = output_path.parent() else {
        return Ok(());
    };

    if parent.as_os_str().is_empty() || parent == Path::new(".") {
        return Ok(());
    }

    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "{}: Failed to create output directory '{}': {}",
            "error".red().bold(),
            parent.display(),
            e
        )
    })
}

fn compute_project_fingerprint(
    files: &[PathBuf],
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

    for file in files {
        file.hash(&mut hasher);
        let contents = fs::read(file).map_err(|e| {
            format!(
                "{}: Failed to read source for '{}': {}",
                "error".red().bold(),
                file.display(),
                e
            )
        })?;
        contents.hash(&mut hasher);
    }

    Ok(format!("{:016x}", hasher.finish()))
}

fn load_cached_fingerprint(project_root: &Path) -> Result<Option<String>, String> {
    let cache_file = project_cache_file(project_root);
    if !cache_file.exists() {
        return Ok(None);
    }

    let fingerprint = fs::read_to_string(&cache_file).map_err(|e| {
        format!(
            "{}: Failed to read build cache '{}': {}",
            "error".red().bold(),
            cache_file.display(),
            e
        )
    })?;
    let fingerprint = fingerprint.trim().to_string();
    if fingerprint.is_empty() {
        return Ok(None);
    }
    Ok(Some(fingerprint))
}

fn load_semantic_cached_fingerprint(project_root: &Path) -> Result<Option<String>, String> {
    let cache_file = semantic_project_cache_file(project_root);
    if !cache_file.exists() {
        return Ok(None);
    }

    let fingerprint = fs::read_to_string(&cache_file).map_err(|e| {
        format!(
            "{}: Failed to read semantic build cache '{}': {}",
            "error".red().bold(),
            cache_file.display(),
            e
        )
    })?;
    let fingerprint = fingerprint.trim().to_string();
    if fingerprint.is_empty() {
        return Ok(None);
    }
    Ok(Some(fingerprint))
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

const PARSE_CACHE_SCHEMA: &str = "v9";
const DEPENDENCY_GRAPH_CACHE_SCHEMA: &str = "v3";
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
    #[serde(default)]
    interface_names: Vec<String>,
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
    interface_names: Vec<String>,
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
    specialization_projection: Program,
    semantic_fingerprint: String,
    rewrite_context_fingerprint: String,
    active_symbols: HashSet<String>,
    has_specialization_demand: bool,
    from_rewrite_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DependencyGraphCache {
    schema: String,
    compiler_version: String,
    entry_namespace: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct SymbolLookupResolution {
    owner_namespace: String,
    symbol_name: String,
    owner_file: PathBuf,
}

type SharedSymbolLookupResolution = Arc<SymbolLookupResolution>;
type ExactSymbolLookup = HashMap<String, Option<SharedSymbolLookupResolution>>;
type WildcardMemberLookup = HashMap<String, HashMap<String, Option<SharedSymbolLookupResolution>>>;

#[derive(Debug, Clone)]
struct ProjectSymbolLookup {
    exact: ExactSymbolLookup,
    wildcard_members: WildcardMemberLookup,
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

    fn measure_step<T, F>(&mut self, label: &str, f: F) -> T
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

    fn record_duration_ns(&mut self, label: &str, nanos: u64) {
        if !self.enabled {
            return;
        }

        self.phases.push(BuildTimingPhase {
            label: label.to_string(),
            ms: nanos as f64 / 1_000_000.0,
            counters: Vec::new(),
        });
    }

    fn print(&self) {
        if !self.enabled {
            return;
        }

        println!("{}", "Build timings".cyan().bold());
        if self.phases.iter().any(|phase| phase.label.contains('/')) {
            println!(
                "  note: subphase timings are cumulative worker time for parallel sections and can exceed parent wall time"
            );
        }
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

fn elapsed_nanos_u64(started_at: Instant) -> u64 {
    started_at.elapsed().as_nanos() as u64
}

#[derive(Default)]
struct DependencyGraphTimingTotals {
    cache_validation_ns: AtomicU64,
    direct_symbol_refs_ns: AtomicU64,
    import_exact_ns: AtomicU64,
    import_wildcard_ns: AtomicU64,
    import_namespace_alias_ns: AtomicU64,
    import_parent_namespace_ns: AtomicU64,
    namespace_fallback_ns: AtomicU64,
    owner_lookup_ns: AtomicU64,
    namespace_files_ns: AtomicU64,
    files_reused: AtomicUsize,
    files_rebuilt: AtomicUsize,
    direct_symbol_ref_count: AtomicUsize,
    import_exact_count: AtomicUsize,
    import_wildcard_count: AtomicUsize,
    import_namespace_alias_count: AtomicUsize,
    import_parent_namespace_count: AtomicUsize,
    namespace_fallback_count: AtomicUsize,
    qualified_ref_count: AtomicUsize,
}

#[derive(Default)]
struct RewriteFingerprintTimingTotals {
    local_symbol_refs_ns: AtomicU64,
    wildcard_imports_ns: AtomicU64,
    namespace_alias_imports_ns: AtomicU64,
    exact_imports_ns: AtomicU64,
    relevant_namespace_prefixes_ns: AtomicU64,
    namespace_hashing_ns: AtomicU64,
    local_symbol_ref_count: AtomicUsize,
    wildcard_import_count: AtomicUsize,
    namespace_alias_import_count: AtomicUsize,
    exact_import_count: AtomicUsize,
    prefix_expand_count: AtomicUsize,
}

#[derive(Default)]
struct DeclarationClosureTimingTotals {
    closure_seed_ns: AtomicU64,
    metadata_lookup_ns: AtomicU64,
    wildcard_imports_ns: AtomicU64,
    exact_imports_ns: AtomicU64,
    qualified_refs_ns: AtomicU64,
    reference_symbols_ns: AtomicU64,
    visited_file_count: AtomicUsize,
    wildcard_import_count: AtomicUsize,
    exact_import_count: AtomicUsize,
    qualified_ref_count: AtomicUsize,
    reference_symbol_count: AtomicUsize,
}

#[derive(Default)]
struct ObjectEmitTimingTotals {
    context_create_ns: AtomicU64,
    codegen_new_ns: AtomicU64,
    compile_filtered_ns: AtomicU64,
    object_dir_setup_ns: AtomicU64,
    write_object_ns: AtomicU64,
    active_symbol_count: AtomicUsize,
    declaration_symbol_count: AtomicUsize,
    program_decl_count: AtomicUsize,
}

#[derive(Default)]
struct ImportCheckTimingTotals {
    rewrite_context_fingerprint_ns: AtomicU64,
    cache_lookup_ns: AtomicU64,
    checker_init_ns: AtomicU64,
    checker_run_ns: AtomicU64,
    cache_save_ns: AtomicU64,
}

#[derive(Default)]
struct RewriteTimingTotals {
    rewrite_context_fingerprint_ns: AtomicU64,
    cache_lookup_ns: AtomicU64,
    rewrite_program_ns: AtomicU64,
    cache_save_ns: AtomicU64,
    active_symbols_ns: AtomicU64,
    api_projection_ns: AtomicU64,
    specialization_projection_ns: AtomicU64,
    specialization_demand_ns: AtomicU64,
}

#[derive(Default)]
struct ObjectCodegenTimingTotals {
    declaration_closure_ns: AtomicU64,
    codegen_program_ns: AtomicU64,
    closure_body_symbols_ns: AtomicU64,
    llvm_emit_ns: AtomicU64,
    cache_save_ns: AtomicU64,
}

#[derive(Default)]
struct CacheIoTimingTotals {
    load_ns: AtomicU64,
    save_ns: AtomicU64,
    bytes_read: AtomicU64,
    bytes_written: AtomicU64,
    load_count: AtomicUsize,
    save_count: AtomicUsize,
}

static PARSE_CACHE_TIMING_TOTALS: CacheIoTimingTotals = CacheIoTimingTotals {
    load_ns: AtomicU64::new(0),
    save_ns: AtomicU64::new(0),
    bytes_read: AtomicU64::new(0),
    bytes_written: AtomicU64::new(0),
    load_count: AtomicUsize::new(0),
    save_count: AtomicUsize::new(0),
};

static REWRITE_CACHE_TIMING_TOTALS: CacheIoTimingTotals = CacheIoTimingTotals {
    load_ns: AtomicU64::new(0),
    save_ns: AtomicU64::new(0),
    bytes_read: AtomicU64::new(0),
    bytes_written: AtomicU64::new(0),
    load_count: AtomicUsize::new(0),
    save_count: AtomicUsize::new(0),
};

static OBJECT_CACHE_META_TIMING_TOTALS: CacheIoTimingTotals = CacheIoTimingTotals {
    load_ns: AtomicU64::new(0),
    save_ns: AtomicU64::new(0),
    bytes_read: AtomicU64::new(0),
    bytes_written: AtomicU64::new(0),
    load_count: AtomicUsize::new(0),
    save_count: AtomicUsize::new(0),
};

fn reset_cache_io_timing_totals(totals: &CacheIoTimingTotals) {
    totals.load_ns.store(0, Ordering::Relaxed);
    totals.save_ns.store(0, Ordering::Relaxed);
    totals.bytes_read.store(0, Ordering::Relaxed);
    totals.bytes_written.store(0, Ordering::Relaxed);
    totals.load_count.store(0, Ordering::Relaxed);
    totals.save_count.store(0, Ordering::Relaxed);
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

fn api_program_fingerprint(program: &Program) -> String {
    let projected = api_projection_program(program);
    let canonical = formatter::format_program_canonical(&projected);
    source_fingerprint(&canonical)
}

fn type_has_codegen_specialization_demand(ty: &Type) -> bool {
    match ty {
        Type::Generic(_, _) => true,
        Type::Function(params, ret) => {
            params.iter().any(type_has_codegen_specialization_demand)
                || type_has_codegen_specialization_demand(ret)
        }
        Type::Option(inner)
        | Type::Result(inner, _)
        | Type::List(inner)
        | Type::Set(inner)
        | Type::Ref(inner)
        | Type::MutRef(inner)
        | Type::Box(inner)
        | Type::Rc(inner)
        | Type::Arc(inner)
        | Type::Ptr(inner)
        | Type::Task(inner)
        | Type::Range(inner) => type_has_codegen_specialization_demand(inner),
        Type::Map(key, value) => {
            type_has_codegen_specialization_demand(key)
                || type_has_codegen_specialization_demand(value)
        }
        Type::Integer
        | Type::Float
        | Type::Boolean
        | Type::String
        | Type::Char
        | Type::None
        | Type::Named(_) => false,
    }
}

fn expr_has_codegen_specialization_demand(expr: &Expr) -> bool {
    match expr {
        Expr::Call {
            callee,
            args,
            type_args,
        } => {
            !type_args.is_empty()
                || expr_has_codegen_specialization_demand(&callee.node)
                || args
                    .iter()
                    .any(|arg| expr_has_codegen_specialization_demand(&arg.node))
                || type_args.iter().any(type_has_codegen_specialization_demand)
        }
        Expr::GenericFunctionValue { callee, type_args } => {
            !type_args.is_empty()
                || expr_has_codegen_specialization_demand(&callee.node)
                || type_args.iter().any(type_has_codegen_specialization_demand)
        }
        Expr::Construct { ty, args } => {
            parse_type_source(ty)
                .ok()
                .is_some_and(|ty| type_has_codegen_specialization_demand(&ty))
                || args
                    .iter()
                    .any(|arg| expr_has_codegen_specialization_demand(&arg.node))
        }
        Expr::Binary { left, right, .. } => {
            expr_has_codegen_specialization_demand(&left.node)
                || expr_has_codegen_specialization_demand(&right.node)
        }
        Expr::Unary { expr, .. }
        | Expr::Try(expr)
        | Expr::Borrow(expr)
        | Expr::MutBorrow(expr)
        | Expr::Deref(expr)
        | Expr::Await(expr) => expr_has_codegen_specialization_demand(&expr.node),
        Expr::Field { object, .. } => expr_has_codegen_specialization_demand(&object.node),
        Expr::Index { object, index } => {
            expr_has_codegen_specialization_demand(&object.node)
                || expr_has_codegen_specialization_demand(&index.node)
        }
        Expr::Lambda { params, body } => {
            params
                .iter()
                .any(|param| type_has_codegen_specialization_demand(&param.ty))
                || expr_has_codegen_specialization_demand(&body.node)
        }
        Expr::Match { expr, arms } => {
            expr_has_codegen_specialization_demand(&expr.node)
                || arms.iter().any(|arm| {
                    arm.body
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Expr::StringInterp(parts) => parts.iter().any(|part| match part {
            ast::StringPart::Literal(_) => false,
            ast::StringPart::Expr(expr) => expr_has_codegen_specialization_demand(&expr.node),
        }),
        Expr::AsyncBlock(block) | Expr::Block(block) => block
            .iter()
            .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node)),
        Expr::Require { condition, message } => {
            expr_has_codegen_specialization_demand(&condition.node)
                || message
                    .as_ref()
                    .is_some_and(|msg| expr_has_codegen_specialization_demand(&msg.node))
        }
        Expr::Range { start, end, .. } => {
            start
                .as_ref()
                .is_some_and(|expr| expr_has_codegen_specialization_demand(&expr.node))
                || end
                    .as_ref()
                    .is_some_and(|expr| expr_has_codegen_specialization_demand(&expr.node))
        }
        Expr::IfExpr {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_has_codegen_specialization_demand(&condition.node)
                || then_branch
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                || else_branch.as_ref().is_some_and(|block| {
                    block
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::This => false,
    }
}

fn stmt_has_codegen_specialization_demand(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Let { ty, value, .. } => {
            type_has_codegen_specialization_demand(ty)
                || expr_has_codegen_specialization_demand(&value.node)
        }
        Stmt::Assign { target, value } => {
            expr_has_codegen_specialization_demand(&target.node)
                || expr_has_codegen_specialization_demand(&value.node)
        }
        Stmt::Expr(expr) => expr_has_codegen_specialization_demand(&expr.node),
        Stmt::Return(expr) => expr
            .as_ref()
            .is_some_and(|expr| expr_has_codegen_specialization_demand(&expr.node)),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_has_codegen_specialization_demand(&condition.node)
                || then_block
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                || else_block.as_ref().is_some_and(|block| {
                    block
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Stmt::While { condition, body } => {
            expr_has_codegen_specialization_demand(&condition.node)
                || body
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
        }
        Stmt::For {
            var_type,
            iterable,
            body,
            ..
        } => {
            var_type
                .as_ref()
                .is_some_and(type_has_codegen_specialization_demand)
                || expr_has_codegen_specialization_demand(&iterable.node)
                || body
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
        }
        Stmt::Match { expr, arms } => {
            expr_has_codegen_specialization_demand(&expr.node)
                || arms.iter().any(|arm| {
                    arm.body
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Stmt::Break | Stmt::Continue => false,
    }
}

fn specialization_projection_stmt(stmt: &Stmt) -> Option<Stmt> {
    match stmt {
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            let projected_then = then_block
                .iter()
                .filter_map(|stmt| {
                    specialization_projection_stmt(&stmt.node)
                        .map(|node| Spanned::new(node, stmt.span.clone()))
                })
                .collect::<Vec<_>>();
            let projected_else = else_block.as_ref().map(|block| {
                block
                    .iter()
                    .filter_map(|stmt| {
                        specialization_projection_stmt(&stmt.node)
                            .map(|node| Spanned::new(node, stmt.span.clone()))
                    })
                    .collect::<Vec<_>>()
            });
            if expr_has_codegen_specialization_demand(&condition.node)
                || !projected_then.is_empty()
                || projected_else
                    .as_ref()
                    .is_some_and(|block| !block.is_empty())
            {
                Some(Stmt::If {
                    condition: condition.clone(),
                    then_block: projected_then,
                    else_block: projected_else.filter(|block| !block.is_empty()),
                })
            } else {
                None
            }
        }
        Stmt::While { condition, body } => {
            let projected_body = body
                .iter()
                .filter_map(|stmt| {
                    specialization_projection_stmt(&stmt.node)
                        .map(|node| Spanned::new(node, stmt.span.clone()))
                })
                .collect::<Vec<_>>();
            if expr_has_codegen_specialization_demand(&condition.node) || !projected_body.is_empty()
            {
                Some(Stmt::While {
                    condition: condition.clone(),
                    body: projected_body,
                })
            } else {
                None
            }
        }
        Stmt::For {
            var,
            var_type,
            iterable,
            body,
        } => {
            let projected_body = body
                .iter()
                .filter_map(|stmt| {
                    specialization_projection_stmt(&stmt.node)
                        .map(|node| Spanned::new(node, stmt.span.clone()))
                })
                .collect::<Vec<_>>();
            if var_type
                .as_ref()
                .is_some_and(type_has_codegen_specialization_demand)
                || expr_has_codegen_specialization_demand(&iterable.node)
                || !projected_body.is_empty()
            {
                Some(Stmt::For {
                    var: var.clone(),
                    var_type: var_type.clone(),
                    iterable: iterable.clone(),
                    body: projected_body,
                })
            } else {
                None
            }
        }
        Stmt::Match { expr, arms } => {
            let projected_arms = arms
                .iter()
                .filter_map(|arm| {
                    let projected_body = arm
                        .body
                        .iter()
                        .filter_map(|stmt| {
                            specialization_projection_stmt(&stmt.node)
                                .map(|node| Spanned::new(node, stmt.span.clone()))
                        })
                        .collect::<Vec<_>>();
                    if !projected_body.is_empty() {
                        Some(ast::MatchArm {
                            pattern: arm.pattern.clone(),
                            body: projected_body,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if expr_has_codegen_specialization_demand(&expr.node) || !projected_arms.is_empty() {
                Some(Stmt::Match {
                    expr: expr.clone(),
                    arms: projected_arms,
                })
            } else {
                None
            }
        }
        _ if stmt_has_codegen_specialization_demand(stmt) => Some(stmt.clone()),
        _ => None,
    }
}

fn specialization_projection_decl(decl: &Spanned<Decl>) -> Spanned<Decl> {
    let projected = match &decl.node {
        Decl::Function(func) => {
            let mut func = func.clone();
            if !func.is_extern {
                func.body = func
                    .body
                    .iter()
                    .filter_map(|stmt| {
                        specialization_projection_stmt(&stmt.node)
                            .map(|node| Spanned::new(node, stmt.span.clone()))
                    })
                    .collect();
            }
            Decl::Function(func)
        }
        Decl::Class(class) => {
            let mut class = class.clone();
            if let Some(constructor) = &mut class.constructor {
                constructor.body = constructor
                    .body
                    .iter()
                    .filter_map(|stmt| {
                        specialization_projection_stmt(&stmt.node)
                            .map(|node| Spanned::new(node, stmt.span.clone()))
                    })
                    .collect();
            }
            if let Some(destructor) = &mut class.destructor {
                destructor.body = destructor
                    .body
                    .iter()
                    .filter_map(|stmt| {
                        specialization_projection_stmt(&stmt.node)
                            .map(|node| Spanned::new(node, stmt.span.clone()))
                    })
                    .collect();
            }
            class.methods = class
                .methods
                .into_iter()
                .map(|mut method| {
                    method.body = method
                        .body
                        .iter()
                        .filter_map(|stmt| {
                            specialization_projection_stmt(&stmt.node)
                                .map(|node| Spanned::new(node, stmt.span.clone()))
                        })
                        .collect();
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
                    method.default_impl = method.default_impl.map(|body| {
                        body.iter()
                            .filter_map(|stmt| {
                                specialization_projection_stmt(&stmt.node)
                                    .map(|node| Spanned::new(node, stmt.span.clone()))
                            })
                            .collect()
                    });
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
                .map(specialization_projection_decl)
                .collect();
            Decl::Module(module)
        }
        Decl::Enum(en) => Decl::Enum(en.clone()),
        Decl::Import(import) => Decl::Import(import.clone()),
    };
    Spanned::new(projected, decl.span.clone())
}

fn specialization_projection_program(program: &Program) -> Program {
    Program {
        package: program.package.clone(),
        declarations: program
            .declarations
            .iter()
            .map(specialization_projection_decl)
            .collect(),
    }
}

fn decl_has_codegen_specialization_demand(decl: &Decl) -> bool {
    match decl {
        Decl::Function(func) => {
            func.params
                .iter()
                .any(|param| type_has_codegen_specialization_demand(&param.ty))
                || type_has_codegen_specialization_demand(&func.return_type)
                || func
                    .body
                    .iter()
                    .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
        }
        Decl::Class(class) => {
            class
                .fields
                .iter()
                .any(|field| type_has_codegen_specialization_demand(&field.ty))
                || class.constructor.as_ref().is_some_and(|ctor| {
                    ctor.params
                        .iter()
                        .any(|param| type_has_codegen_specialization_demand(&param.ty))
                        || ctor
                            .body
                            .iter()
                            .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
                || class.destructor.as_ref().is_some_and(|dtor| {
                    dtor.body
                        .iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
                || class.methods.iter().any(|method| {
                    method
                        .params
                        .iter()
                        .any(|param| type_has_codegen_specialization_demand(&param.ty))
                        || type_has_codegen_specialization_demand(&method.return_type)
                        || method
                            .body
                            .iter()
                            .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }
        Decl::Enum(en) => en.variants.iter().any(|variant| {
            variant
                .fields
                .iter()
                .any(|field| type_has_codegen_specialization_demand(&field.ty))
        }),
        Decl::Interface(interface) => interface.methods.iter().any(|method| {
            method
                .params
                .iter()
                .any(|param| type_has_codegen_specialization_demand(&param.ty))
                || type_has_codegen_specialization_demand(&method.return_type)
                || method.default_impl.as_ref().is_some_and(|body| {
                    body.iter()
                        .any(|stmt| stmt_has_codegen_specialization_demand(&stmt.node))
                })
        }),
        Decl::Module(module) => module
            .declarations
            .iter()
            .any(|decl| decl_has_codegen_specialization_demand(&decl.node)),
        Decl::Import(_) => false,
    }
}

fn program_has_codegen_specialization_demand(program: &Program) -> bool {
    program
        .declarations
        .iter()
        .any(|decl| decl_has_codegen_specialization_demand(&decl.node))
}

#[cfg(test)]
fn codegen_program_for_unit(
    rewritten_files: &[RewrittenProjectUnit],
    rewritten_file_indices: &HashMap<PathBuf, usize>,
    active_file: &Path,
    _dependency_closure: Option<&HashSet<PathBuf>>,
    _declaration_symbols: Option<&HashSet<String>>,
) -> Program {
    codegen_program_for_units(
        rewritten_files,
        rewritten_file_indices,
        &[active_file.to_path_buf()],
        _dependency_closure,
    )
}

fn codegen_program_for_units(
    rewritten_files: &[RewrittenProjectUnit],
    rewritten_file_indices: &HashMap<PathBuf, usize>,
    active_files: &[PathBuf],
    dependency_closure: Option<&HashSet<PathBuf>>,
) -> Program {
    fn merge_codegen_declarations(
        output: &mut Vec<Spanned<Decl>>,
        incoming: &[Spanned<Decl>],
        seen_specializations: &mut HashSet<String>,
    ) {
        for decl in incoming {
            match &decl.node {
                Decl::Function(func) => {
                    if func.name.contains("__spec__")
                        && !seen_specializations.insert(func.name.clone())
                    {
                        continue;
                    }
                    output.push(decl.clone());
                }
                Decl::Class(class) => {
                    if class.name.contains("__spec__")
                        && !seen_specializations.insert(class.name.clone())
                    {
                        continue;
                    }
                    output.push(decl.clone());
                }
                Decl::Enum(en) => {
                    if en.name.contains("__spec__") && !seen_specializations.insert(en.name.clone())
                    {
                        continue;
                    }
                    output.push(decl.clone());
                }
                Decl::Module(module) => {
                    if let Some(existing_module) =
                        output
                            .iter_mut()
                            .find_map(|existing| match &mut existing.node {
                                Decl::Module(existing_module)
                                    if existing_module.name == module.name =>
                                {
                                    Some(existing_module)
                                }
                                _ => None,
                            })
                    {
                        merge_codegen_declarations(
                            &mut existing_module.declarations,
                            &module.declarations,
                            seen_specializations,
                        );
                    } else {
                        let mut merged_module = module.clone();
                        merged_module.declarations.clear();
                        merge_codegen_declarations(
                            &mut merged_module.declarations,
                            &module.declarations,
                            seen_specializations,
                        );
                        output.push(Spanned::new(Decl::Module(merged_module), decl.span.clone()));
                    }
                }
                Decl::Interface(_) | Decl::Import(_) => output.push(decl.clone()),
            }
        }
    }

    let mut program = Program {
        package: None,
        declarations: Vec::new(),
    };
    let mut seen_specializations = HashSet::new();
    let active_file_set = active_files.iter().cloned().collect::<HashSet<_>>();

    let specialization_demand_files = rewritten_files
        .iter()
        .filter(|unit| unit.has_specialization_demand)
        .map(|unit| unit.file.clone())
        .collect::<HashSet<_>>();
    let active_file_has_specialization_demand = active_files.iter().any(|active_file| {
        rewritten_file_indices
            .get(active_file)
            .and_then(|index| rewritten_files.get(*index))
            .is_some_and(|unit| unit.has_specialization_demand)
    });
    let mut relevant_files = dependency_closure
        .map(|closure| closure.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_else(|| {
            rewritten_files
                .iter()
                .map(|unit| unit.file.clone())
                .collect::<Vec<_>>()
        });
    relevant_files.extend(
        specialization_demand_files
            .iter()
            .filter(|file| !active_file_set.contains(*file))
            .cloned(),
    );
    for active_file in active_files {
        if !relevant_files.iter().any(|file| file == active_file) {
            relevant_files.push(active_file.clone());
        }
    }
    relevant_files.sort();
    relevant_files.dedup();

    for file in relevant_files {
        let Some(index) = rewritten_file_indices.get(&file).copied() else {
            continue;
        };
        let unit = &rewritten_files[index];
        let source_program = if active_file_set.contains(&file) {
            unit.program.clone()
        } else if active_file_has_specialization_demand {
            // Explicit generic specialization in the active unit may depend on full generic
            // template bodies from dependency files; projections are not sufficient here.
            unit.program.clone()
        } else if specialization_demand_files.contains(&file) {
            unit.specialization_projection.clone()
        } else {
            unit.api_program.clone()
        };
        merge_codegen_declarations(
            &mut program.declarations,
            &source_program.declarations,
            &mut seen_specializations,
        );
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

fn mangle_project_nominal_symbol_for_codegen(namespace: &str, name: &str) -> String {
    format!("{}__{}", namespace.replace('.', "__"), name)
}

#[allow(clippy::too_many_arguments)]
fn insert_declaration_symbol_for_owner(
    symbol: &str,
    owner_ns: &str,
    owner_file: &Path,
    entry_namespace: &str,
    declaration_symbols: &mut HashSet<String>,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_interface_map: &HashMap<String, String>,
    global_interface_file_map: &HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
) {
    let is_function_owner = global_function_map
        .get(symbol)
        .is_some_and(|ns| ns == owner_ns)
        && global_function_file_map
            .get(symbol)
            .is_some_and(|path| path == owner_file);
    if is_function_owner {
        declaration_symbols.insert(mangle_project_symbol_for_codegen(
            owner_ns,
            entry_namespace,
            symbol,
        ));
    }

    let is_nominal_owner = [
        global_class_map
            .get(symbol)
            .is_some_and(|ns| ns == owner_ns)
            && global_class_file_map
                .get(symbol)
                .is_some_and(|path| path == owner_file),
        global_interface_map
            .get(symbol)
            .is_some_and(|ns| ns == owner_ns)
            && global_interface_file_map
                .get(symbol)
                .is_some_and(|path| path == owner_file),
        global_enum_map.get(symbol).is_some_and(|ns| ns == owner_ns)
            && global_enum_file_map
                .get(symbol)
                .is_some_and(|path| path == owner_file),
        global_module_map
            .get(symbol)
            .is_some_and(|ns| ns == owner_ns)
            && global_module_file_map
                .get(symbol)
                .is_some_and(|path| path == owner_file),
    ]
    .into_iter()
    .any(|matched| matched);

    if is_nominal_owner || !is_function_owner {
        declaration_symbols.insert(mangle_project_nominal_symbol_for_codegen(owner_ns, symbol));
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
    current_file: &Path,
    prefer_local_owner: bool,
    symbol: &str,
    entry_namespace: &str,
    declaration_symbols: &mut HashSet<String>,
    stack: &mut Vec<PathBuf>,
    closure_files: &HashSet<PathBuf>,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_interface_map: &HashMap<String, String>,
    global_interface_file_map: &HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
) {
    let mut push_owner = |owner_ns: &str, owner_file: &Path| {
        if closure_files.contains(owner_file) {
            insert_declaration_symbol_for_owner(
                symbol,
                owner_ns,
                owner_file,
                entry_namespace,
                declaration_symbols,
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
            );
            stack.push(owner_file.to_path_buf());
        }
    };

    if prefer_local_owner
        && global_function_file_map
            .get(symbol)
            .is_none_or(|owner_file| owner_file != current_file)
    {
        if let (Some(owner_ns), Some(owner_file)) = (
            global_class_map.get(symbol),
            global_class_file_map.get(symbol),
        ) {
            if owner_file == current_file {
                push_owner(owner_ns, owner_file);
                return;
            }
        }
        if let (Some(owner_ns), Some(owner_file)) = (
            global_interface_map.get(symbol),
            global_interface_file_map.get(symbol),
        ) {
            if owner_file == current_file {
                push_owner(owner_ns, owner_file);
                return;
            }
        }
        if let (Some(owner_ns), Some(owner_file)) = (
            global_enum_map.get(symbol),
            global_enum_file_map.get(symbol),
        ) {
            if owner_file == current_file {
                push_owner(owner_ns, owner_file);
                return;
            }
        }
        if let (Some(owner_ns), Some(owner_file)) = (
            global_module_map.get(symbol),
            global_module_file_map.get(symbol),
        ) {
            if owner_file == current_file {
                push_owner(owner_ns, owner_file);
                return;
            }
        }
    }

    if let Some((owner_symbol, _member)) = symbol.rsplit_once("__") {
        if let (Some(owner_ns), Some(owner_file)) = (
            global_class_map.get(owner_symbol),
            global_class_file_map.get(owner_symbol),
        ) {
            push_owner(owner_ns, owner_file);
        }
        // Try deeper split for nested modules
        let mut parts = symbol.split("__").collect::<Vec<_>>();
        while parts.len() > 1 {
            parts.pop();
            let parent = parts.join("__");
            if let (Some(owner_ns), Some(owner_file)) = (
                global_class_map.get(&parent),
                global_class_file_map.get(&parent),
            ) {
                push_owner(owner_ns, owner_file);
            }
            if let (Some(owner_ns), Some(owner_file)) = (
                global_module_map.get(&parent),
                global_module_file_map.get(&parent),
            ) {
                push_owner(owner_ns, owner_file);
            }
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
        global_interface_map.get(symbol),
        global_interface_file_map.get(symbol),
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
    symbol_lookup: &'a ProjectSymbolLookup,
) -> Option<(String, String, &'a PathBuf)> {
    let full_path = qualified_symbol_path(namespace_path, symbol_name);
    exact_symbol_resolution(symbol_lookup, &full_path).map(|resolution| {
        (
            resolution.owner_namespace.clone(),
            resolution.symbol_name.clone(),
            &resolution.owner_file,
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_exact_imported_symbol_owner<'a>(
    namespace_path: &str,
    symbol_name: &str,
    symbol_lookup: &'a ProjectSymbolLookup,
) -> Option<(String, String, &'a PathBuf)> {
    resolve_exact_imported_symbol_file(namespace_path, symbol_name, symbol_lookup)
}

#[allow(clippy::too_many_arguments)]
fn extend_declaration_symbols_for_exact_import(
    import: &ImportDecl,
    entry_namespace: &str,
    declaration_symbols: &mut HashSet<String>,
    stack: &mut Vec<PathBuf>,
    closure_files: &HashSet<PathBuf>,
    symbol_lookup: &ProjectSymbolLookup,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_interface_map: &HashMap<String, String>,
    global_interface_file_map: &HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
) {
    let Some((namespace, symbol)) = import.path.rsplit_once('.') else {
        return;
    };

    if let Some((owner_ns, symbol_name, owner_file)) =
        resolve_exact_imported_symbol_owner(namespace, symbol, symbol_lookup)
    {
        if closure_files.contains(owner_file) {
            insert_declaration_symbol_for_owner(
                &symbol_name,
                &owner_ns,
                owner_file,
                entry_namespace,
                declaration_symbols,
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
            );
            stack.push(owner_file.clone());
        }
        return;
    }

    if let Some((enum_namespace, enum_name)) = namespace.rsplit_once('.') {
        if let Some((owner_ns, resolved_enum_name, owner_file)) =
            resolve_exact_imported_symbol_file(enum_namespace, enum_name, symbol_lookup)
        {
            if closure_files.contains(owner_file) {
                declaration_symbols.insert(mangle_project_nominal_symbol_for_codegen(
                    &owner_ns,
                    &resolved_enum_name,
                ));
                stack.push(owner_file.clone());
            }
            return;
        }
    }

    let mut push_owner = |owner_ns: &str, owner_file: &Path| {
        if owner_ns == namespace && closure_files.contains(owner_file) {
            insert_declaration_symbol_for_owner(
                symbol,
                owner_ns,
                owner_file,
                entry_namespace,
                declaration_symbols,
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
            );
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
        global_interface_map.get(symbol),
        global_interface_file_map.get(symbol),
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
    precomputed_dependency_closures: &PrecomputedDependencyClosures,
    reference_metadata: &HashMap<PathBuf, CodegenReferenceMetadata>,
    entry_namespace: &str,
    symbol_lookup: &ProjectSymbolLookup,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_interface_map: &HashMap<String, String>,
    global_interface_file_map: &HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
    timings: Option<&DeclarationClosureTimingTotals>,
) -> DeclarationClosure {
    let closure_seed_started_at = Instant::now();
    let mut closure_files =
        transitive_dependencies_from_precomputed(precomputed_dependency_closures, root_file);
    closure_files.insert(root_file.to_path_buf());
    if let Some(timings) = timings {
        timings.closure_seed_ns.fetch_add(
            elapsed_nanos_u64(closure_seed_started_at),
            Ordering::Relaxed,
        );
    }

    let mut declaration_symbols = root_active_symbols.clone();
    let mut visited_files = HashSet::new();
    let mut stack = vec![root_file.to_path_buf()];

    while let Some(file) = stack.pop() {
        if !visited_files.insert(file.clone()) {
            continue;
        }
        if let Some(timings) = timings {
            timings.visited_file_count.fetch_add(1, Ordering::Relaxed);
        }

        let metadata_lookup_started_at = Instant::now();
        let Some(metadata) = reference_metadata.get(&file) else {
            if let Some(timings) = timings {
                timings.metadata_lookup_ns.fetch_add(
                    elapsed_nanos_u64(metadata_lookup_started_at),
                    Ordering::Relaxed,
                );
            }
            continue;
        };
        if let Some(timings) = timings {
            timings.metadata_lookup_ns.fetch_add(
                elapsed_nanos_u64(metadata_lookup_started_at),
                Ordering::Relaxed,
            );
        }

        for import in &metadata.imports {
            if import.path.ends_with(".*") {
                if let Some(timings) = timings {
                    timings
                        .wildcard_import_count
                        .fetch_add(1, Ordering::Relaxed);
                }
                let wildcard_started_at = Instant::now();
                let namespace = import.path.trim_end_matches(".*");
                for symbol in &metadata.referenced_symbols {
                    if let Some(timings) = timings {
                        timings
                            .reference_symbol_count
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    if let Some((owner_ns, candidate)) = resolve_symbol_in_namespace_path(
                        namespace,
                        std::slice::from_ref(symbol),
                        symbol_lookup,
                    ) {
                        let owner_file = global_function_file_map
                            .get(&candidate)
                            .or_else(|| global_class_file_map.get(&candidate))
                            .or_else(|| global_interface_file_map.get(&candidate))
                            .or_else(|| global_enum_file_map.get(&candidate))
                            .or_else(|| global_module_file_map.get(&candidate));
                        if let Some(owner_file) = owner_file {
                            if closure_files.contains(owner_file) {
                                insert_declaration_symbol_for_owner(
                                    &candidate,
                                    &owner_ns,
                                    owner_file,
                                    entry_namespace,
                                    &mut declaration_symbols,
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
                                );
                                stack.push(owner_file.to_path_buf());
                            }
                        }
                    }
                }
                if let Some(timings) = timings {
                    timings
                        .wildcard_imports_ns
                        .fetch_add(elapsed_nanos_u64(wildcard_started_at), Ordering::Relaxed);
                }
                continue;
            }

            if let Some(timings) = timings {
                timings.exact_import_count.fetch_add(1, Ordering::Relaxed);
            }
            let exact_started_at = Instant::now();
            extend_declaration_symbols_for_exact_import(
                import,
                entry_namespace,
                &mut declaration_symbols,
                &mut stack,
                &closure_files,
                symbol_lookup,
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
            );
            if let Some(timings) = timings {
                timings
                    .exact_imports_ns
                    .fetch_add(elapsed_nanos_u64(exact_started_at), Ordering::Relaxed);
            }

            let import_key = import_lookup_key(import);
            let qualified_started_at = Instant::now();
            for path in &metadata.qualified_symbol_refs {
                if path.first().is_some_and(|part| part == &import_key) {
                    if let Some(timings) = timings {
                        timings.qualified_ref_count.fetch_add(1, Ordering::Relaxed);
                    }
                    let rest = &path[1..];
                    if let Some((owner_ns, candidate)) =
                        resolve_symbol_in_namespace_path(&import.path, rest, symbol_lookup)
                    {
                        let owner_file = global_function_file_map
                            .get(&candidate)
                            .or_else(|| global_class_file_map.get(&candidate))
                            .or_else(|| global_interface_file_map.get(&candidate))
                            .or_else(|| global_enum_file_map.get(&candidate))
                            .or_else(|| global_module_file_map.get(&candidate));
                        if let Some(owner_file) = owner_file {
                            if closure_files.contains(owner_file) {
                                insert_declaration_symbol_for_owner(
                                    &candidate,
                                    &owner_ns,
                                    owner_file,
                                    entry_namespace,
                                    &mut declaration_symbols,
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
                                );
                                stack.push(owner_file.to_path_buf());
                            }
                        }
                    }
                }
            }
            if let Some(timings) = timings {
                timings
                    .qualified_refs_ns
                    .fetch_add(elapsed_nanos_u64(qualified_started_at), Ordering::Relaxed);
            }
        }

        let symbols = if file == root_file {
            &metadata.referenced_symbols
        } else {
            &metadata.api_referenced_symbols
        };
        let reference_symbols_started_at = Instant::now();
        for symbol in symbols {
            if let Some(timings) = timings {
                timings
                    .reference_symbol_count
                    .fetch_add(1, Ordering::Relaxed);
            }
            extend_declaration_symbols_for_reference(
                &file,
                file != root_file,
                symbol,
                entry_namespace,
                &mut declaration_symbols,
                &mut stack,
                &closure_files,
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
            );
        }
        if let Some(timings) = timings {
            timings.reference_symbols_ns.fetch_add(
                elapsed_nanos_u64(reference_symbols_started_at),
                Ordering::Relaxed,
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
    declaration_symbols: &HashSet<String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_module_file_map: &HashMap<String, PathBuf>,
) -> HashSet<String> {
    declaration_symbols
        .iter()
        .filter(|symbol| {
            if symbol.as_str() == "main" {
                return global_function_file_map
                    .get("main")
                    .is_some_and(|owner_file| owner_file == root_file);
            }

            if global_function_file_map
                .get(symbol.as_str())
                .is_some_and(|owner_file| owner_file == root_file)
            {
                return true;
            }

            if global_class_file_map
                .get(symbol.as_str())
                .is_some_and(|owner_file| owner_file == root_file)
            {
                return true;
            }

            if global_module_file_map
                .get(symbol.as_str())
                .is_some_and(|owner_file| owner_file == root_file)
            {
                return true;
            }

            if let Some(owner) = symbol.strip_suffix("__new") {
                return global_class_file_map
                    .get(owner)
                    .is_some_and(|owner_file| owner_file == root_file);
            }

            if let Some((owner, _)) = symbol.rsplit_once("__") {
                if global_class_file_map
                    .get(owner)
                    .is_some_and(|owner_file| owner_file == root_file)
                {
                    return true;
                }

                if global_module_file_map
                    .get(owner)
                    .is_some_and(|owner_file| owner_file == root_file)
                {
                    return true;
                }
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
    let entry: ParsedFileCacheEntry =
        match read_cache_blob_with_timing(&path, "parse cache", &PARSE_CACHE_TIMING_TOTALS)? {
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
    write_cache_blob_with_timing(&path, "parse cache", entry, &PARSE_CACHE_TIMING_TOTALS)
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

const REWRITE_CACHE_SCHEMA: &str = "v9";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RewrittenFileCacheEntry {
    schema: String,
    compiler_version: String,
    semantic_fingerprint: String,
    rewrite_context_fingerprint: String,
    rewritten_program: Program,
    api_program: Program,
    specialization_projection: Program,
    active_symbols: Vec<String>,
    has_specialization_demand: bool,
}

const OBJECT_CACHE_SCHEMA: &str = "v3";
const OBJECT_SHARD_CACHE_SCHEMA: &str = "v1";
const LINK_MANIFEST_CACHE_SCHEMA: &str = "v1";
const OBJECT_CODEGEN_SHARD_SIZE: usize = 8;
const OBJECT_CODEGEN_SHARD_THRESHOLD: usize = usize::MAX;

fn env_usize_override(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn object_codegen_shard_size() -> usize {
    env_usize_override("APEX_OBJECT_SHARD_SIZE", OBJECT_CODEGEN_SHARD_SIZE)
}

fn object_codegen_shard_threshold() -> usize {
    env_usize_override(
        "APEX_OBJECT_SHARD_THRESHOLD",
        OBJECT_CODEGEN_SHARD_THRESHOLD,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObjectCacheEntry {
    schema: String,
    compiler_version: String,
    semantic_fingerprint: String,
    rewrite_context_fingerprint: String,
    object_build_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ObjectShardMemberFingerprint {
    file: PathBuf,
    semantic_fingerprint: String,
    rewrite_context_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObjectShardCacheEntry {
    schema: String,
    compiler_version: String,
    object_build_fingerprint: String,
    members: Vec<ObjectShardMemberFingerprint>,
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
        for module_name in &unit.module_names {
            grouped
                .entry(format!(
                    "{}.{}",
                    unit.namespace,
                    module_name.replace("__", ".")
                ))
                .or_default()
                .push((&unit.file, unit.api_fingerprint.as_str()));
        }
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

fn collect_known_namespace_paths_for_units(parsed_files: &[ParsedProjectUnit]) -> HashSet<String> {
    let mut paths = HashSet::new();
    fn collect_enum_variant_paths(
        paths: &mut HashSet<String>,
        declarations: &[Spanned<Decl>],
        namespace: &str,
        module_prefix: Option<&str>,
    ) {
        for decl in declarations {
            match &decl.node {
                Decl::Enum(en) => {
                    let enum_path = if let Some(prefix) = module_prefix {
                        format!("{}.{}", prefix, en.name)
                    } else {
                        format!("{}.{}", namespace, en.name)
                    };
                    for variant in &en.variants {
                        paths.insert(format!("{}.{}", enum_path, variant.name));
                    }
                }
                Decl::Module(module) => {
                    let next_prefix = if let Some(prefix) = module_prefix {
                        format!("{}.{}", prefix, module.name)
                    } else {
                        format!("{}.{}", namespace, module.name)
                    };
                    collect_enum_variant_paths(
                        paths,
                        &module.declarations,
                        namespace,
                        Some(&next_prefix),
                    );
                }
                Decl::Function(_) | Decl::Class(_) | Decl::Interface(_) | Decl::Import(_) => {}
            }
        }
    }

    for unit in parsed_files {
        paths.insert(unit.namespace.clone());
        for class_name in &unit.class_names {
            let class_path = class_name.replace("__", ".");
            paths.insert(format!("{}.{}", unit.namespace, class_path));
        }
        for interface_name in &unit.interface_names {
            let interface_path = interface_name.replace("__", ".");
            paths.insert(format!("{}.{}", unit.namespace, interface_path));
        }
        for enum_name in &unit.enum_names {
            let enum_path = enum_name.replace("__", ".");
            paths.insert(format!("{}.{}", unit.namespace, enum_path));
        }
        for module_name in &unit.module_names {
            let module_path = module_name.replace("__", ".");
            paths.insert(format!("{}.{}", unit.namespace, module_path));
        }
        collect_enum_variant_paths(
            &mut paths,
            &unit.program.declarations,
            &unit.namespace,
            None,
        );
    }
    paths
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

fn qualified_symbol_path(namespace: &str, symbol_name: &str) -> String {
    let separator_count = symbol_name.matches("__").count();
    let mut path = String::with_capacity(namespace.len() + symbol_name.len() + separator_count + 1);
    if !namespace.is_empty() {
        path.push_str(namespace);
        path.push('.');
    }
    if separator_count == 0 {
        path.push_str(symbol_name);
        return path;
    }

    let mut remaining = symbol_name;
    while let Some(index) = remaining.find("__") {
        path.push_str(&remaining[..index]);
        path.push('.');
        remaining = &remaining[index + 2..];
    }
    path.push_str(remaining);
    path
}

fn qualified_symbol_path_for_parts(namespace: &str, member_parts: &[String]) -> Option<String> {
    if member_parts.is_empty() {
        return None;
    }

    Some(if namespace.is_empty() {
        member_parts.join(".")
    } else {
        format!("{}.{}", namespace, member_parts.join("."))
    })
}

fn wildcard_member_import_path(owner_namespace: &str, symbol_name: &str) -> (String, String) {
    let Some(last_separator) = symbol_name.rfind("__") else {
        return (owner_namespace.to_string(), symbol_name.to_string());
    };

    let member_name = symbol_name[last_separator + 2..].to_string();
    let prefix = &symbol_name[..last_separator];
    let separator_count = prefix.matches("__").count();
    let mut import_namespace =
        String::with_capacity(owner_namespace.len() + prefix.len() + separator_count + 1);
    import_namespace.push_str(owner_namespace);
    import_namespace.push('.');
    if separator_count == 0 {
        import_namespace.push_str(prefix);
        return (import_namespace, member_name);
    }

    let mut remaining = prefix;
    while let Some(index) = remaining.find("__") {
        import_namespace.push_str(&remaining[..index]);
        import_namespace.push('.');
        remaining = &remaining[index + 2..];
    }
    import_namespace.push_str(remaining);
    (import_namespace, member_name)
}

fn insert_lookup_resolution(
    target: &mut HashMap<String, Option<SharedSymbolLookupResolution>>,
    key: String,
    resolution: SharedSymbolLookupResolution,
) {
    match target.entry(key) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(Some(resolution));
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let unchanged = entry
                .get()
                .as_ref()
                .is_some_and(|current| current.as_ref() == resolution.as_ref());
            if !unchanged {
                entry.insert(None);
            }
        }
    }
}

fn insert_symbol_lookup_entry(
    exact_lookup: &mut ExactSymbolLookup,
    wildcard_lookup: &mut WildcardMemberLookup,
    owner_namespace: &str,
    symbol_name: &str,
    owner_file: &Path,
) {
    let resolution = Arc::new(SymbolLookupResolution {
        owner_namespace: owner_namespace.to_string(),
        symbol_name: symbol_name.to_string(),
        owner_file: owner_file.to_path_buf(),
    });
    insert_lookup_resolution(
        exact_lookup,
        qualified_symbol_path(owner_namespace, symbol_name),
        Arc::clone(&resolution),
    );

    let (import_namespace, member_name) = wildcard_member_import_path(owner_namespace, symbol_name);
    insert_lookup_resolution(
        wildcard_lookup.entry(import_namespace).or_default(),
        member_name,
        resolution,
    );
}

#[allow(clippy::too_many_arguments)]
fn register_global_symbol(
    symbol_name: &str,
    owner_namespace: &str,
    owner_file: &Path,
    global_map: &mut HashMap<String, String>,
    global_file_map: &mut HashMap<String, PathBuf>,
    collisions: &mut Vec<(String, String, String)>,
    exact_lookup: &mut ExactSymbolLookup,
    wildcard_lookup: &mut WildcardMemberLookup,
    build_symbol_lookup: bool,
) {
    match global_map.entry(symbol_name.to_string()) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(owner_namespace.to_string());
            global_file_map.insert(symbol_name.to_string(), owner_file.to_path_buf());
            if build_symbol_lookup {
                insert_symbol_lookup_entry(
                    exact_lookup,
                    wildcard_lookup,
                    owner_namespace,
                    symbol_name,
                    owner_file,
                );
            }
        }
        std::collections::hash_map::Entry::Occupied(entry) => {
            if entry.get() != owner_namespace {
                collisions.push((
                    symbol_name.to_string(),
                    entry.get().clone(),
                    owner_namespace.to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn build_project_symbol_lookup(
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &HashMap<String, PathBuf>,
    global_interface_map: &HashMap<String, String>,
    global_interface_file_map: &HashMap<String, PathBuf>,
    global_enum_map: &HashMap<String, String>,
    global_enum_file_map: &HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
) -> ProjectSymbolLookup {
    let symbol_count = global_function_map.len()
        + global_class_map.len()
        + global_interface_map.len()
        + global_enum_map.len()
        + global_module_map.len();
    let mut exact = HashMap::with_capacity(symbol_count);
    let mut wildcard_members = HashMap::with_capacity(symbol_count);

    for (symbol_name, owner_namespace) in global_function_map {
        if let Some(owner_file) = global_function_file_map.get(symbol_name) {
            insert_symbol_lookup_entry(
                &mut exact,
                &mut wildcard_members,
                owner_namespace,
                symbol_name,
                owner_file,
            );
        }
    }
    for (symbol_name, owner_namespace) in global_class_map {
        if let Some(owner_file) = global_class_file_map.get(symbol_name) {
            insert_symbol_lookup_entry(
                &mut exact,
                &mut wildcard_members,
                owner_namespace,
                symbol_name,
                owner_file,
            );
        }
    }
    for (symbol_name, owner_namespace) in global_interface_map {
        if let Some(owner_file) = global_interface_file_map.get(symbol_name) {
            insert_symbol_lookup_entry(
                &mut exact,
                &mut wildcard_members,
                owner_namespace,
                symbol_name,
                owner_file,
            );
        }
    }
    for (symbol_name, owner_namespace) in global_enum_map {
        if let Some(owner_file) = global_enum_file_map.get(symbol_name) {
            insert_symbol_lookup_entry(
                &mut exact,
                &mut wildcard_members,
                owner_namespace,
                symbol_name,
                owner_file,
            );
        }
    }
    for (symbol_name, owner_namespace) in global_module_map {
        if let Some(owner_file) = global_module_file_map.get(symbol_name) {
            insert_symbol_lookup_entry(
                &mut exact,
                &mut wildcard_members,
                owner_namespace,
                symbol_name,
                owner_file,
            );
        }
    }

    ProjectSymbolLookup {
        exact,
        wildcard_members,
    }
}

fn exact_symbol_resolution<'a>(
    lookup: &'a ProjectSymbolLookup,
    qualified_path: &str,
) -> Option<&'a SymbolLookupResolution> {
    lookup.exact.get(qualified_path).and_then(Option::as_deref)
}

fn wildcard_symbol_resolution<'a>(
    lookup: &'a ProjectSymbolLookup,
    import_namespace: &str,
    member_name: &str,
) -> Option<&'a SymbolLookupResolution> {
    lookup
        .wildcard_members
        .get(import_namespace)
        .and_then(|members| members.get(member_name))
        .and_then(Option::as_deref)
}

#[allow(clippy::too_many_arguments)]
fn import_path_owner_file<'a>(
    path: &str,
    symbol_lookup: &'a ProjectSymbolLookup,
) -> Option<&'a PathBuf> {
    if let Some(resolution) = exact_symbol_resolution(symbol_lookup, path) {
        return Some(&resolution.owner_file);
    }

    if let Some((enum_path, _)) = path.rsplit_once('.') {
        if let Some(resolution) = exact_symbol_resolution(symbol_lookup, enum_path) {
            return Some(&resolution.owner_file);
        }
    }

    None
}

#[allow(dead_code)]
struct RewriteFingerprintContext<'a> {
    namespace_functions: &'a HashMap<String, HashSet<String>>,
    namespace_function_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    global_function_map: &'a HashMap<String, String>,
    global_function_file_map: &'a HashMap<String, PathBuf>,
    namespace_classes: &'a HashMap<String, HashSet<String>>,
    namespace_class_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    global_class_map: &'a HashMap<String, String>,
    global_class_file_map: &'a HashMap<String, PathBuf>,
    namespace_interface_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    global_interface_map: &'a HashMap<String, String>,
    global_interface_file_map: &'a HashMap<String, PathBuf>,
    global_enum_map: &'a HashMap<String, String>,
    global_enum_file_map: &'a HashMap<String, PathBuf>,
    namespace_modules: &'a HashMap<String, HashSet<String>>,
    namespace_module_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    global_module_map: &'a HashMap<String, String>,
    global_module_file_map: &'a HashMap<String, PathBuf>,
    namespace_api_fingerprints: &'a HashMap<String, String>,
    file_api_fingerprints: &'a HashMap<PathBuf, String>,
    symbol_lookup: Arc<ProjectSymbolLookup>,
}

#[allow(dead_code)]
struct DependencyResolutionContext<'a> {
    namespace_files_map: &'a HashMap<String, Vec<PathBuf>>,
    namespace_function_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    namespace_class_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    namespace_interface_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    namespace_module_files: &'a HashMap<String, HashMap<String, PathBuf>>,
    global_function_map: &'a HashMap<String, String>,
    global_function_file_map: &'a HashMap<String, PathBuf>,
    global_class_map: &'a HashMap<String, String>,
    global_class_file_map: &'a HashMap<String, PathBuf>,
    global_interface_map: &'a HashMap<String, String>,
    global_interface_file_map: &'a HashMap<String, PathBuf>,
    global_enum_map: &'a HashMap<String, String>,
    global_enum_file_map: &'a HashMap<String, PathBuf>,
    global_module_map: &'a HashMap<String, String>,
    global_module_file_map: &'a HashMap<String, PathBuf>,
    symbol_lookup: Arc<ProjectSymbolLookup>,
}

fn import_lookup_key(import: &ImportDecl) -> String {
    import
        .alias
        .as_ref()
        .cloned()
        .unwrap_or_else(|| import.path.rsplit('.').next().unwrap_or("").to_string())
}

fn resolve_symbol_file_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    symbol_lookup: &ProjectSymbolLookup,
) -> Option<PathBuf> {
    if member_parts.len() == 1 {
        if let Some(resolution) =
            wildcard_symbol_resolution(symbol_lookup, namespace_path, &member_parts[0])
        {
            return Some(resolution.owner_file.clone());
        }
    }

    for prefix_len in (1..=member_parts.len()).rev() {
        let prefix = &member_parts[..prefix_len];
        let dotted_path = qualified_symbol_path_for_parts(namespace_path, prefix)?;
        if let Some(resolution) = exact_symbol_resolution(symbol_lookup, &dotted_path) {
            return Some(resolution.owner_file.clone());
        }
        if prefix_len > 1 {
            let mangled_prefix = prefix.join("__");
            let mangled_path = if namespace_path.is_empty() {
                mangled_prefix
            } else {
                format!("{}.{}", namespace_path, mangled_prefix)
            };
            if let Some(resolution) = exact_symbol_resolution(symbol_lookup, &mangled_path) {
                return Some(resolution.owner_file.clone());
            }
        }
    }

    None
}

fn resolve_symbol_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    symbol_lookup: &ProjectSymbolLookup,
) -> Option<(String, String)> {
    if member_parts.len() == 1 {
        if let Some(resolution) =
            wildcard_symbol_resolution(symbol_lookup, namespace_path, &member_parts[0])
        {
            return Some((
                resolution.owner_namespace.clone(),
                resolution.symbol_name.clone(),
            ));
        }
    }

    for prefix_len in (1..=member_parts.len()).rev() {
        let prefix = &member_parts[..prefix_len];
        let dotted_path = qualified_symbol_path_for_parts(namespace_path, prefix)?;
        if let Some(resolution) = exact_symbol_resolution(symbol_lookup, &dotted_path) {
            return Some((
                resolution.owner_namespace.clone(),
                resolution.symbol_name.clone(),
            ));
        }
        if prefix_len > 1 {
            let mangled_prefix = prefix.join("__");
            let mangled_path = if namespace_path.is_empty() {
                mangled_prefix
            } else {
                format!("{}.{}", namespace_path, mangled_prefix)
            };
            if let Some(resolution) = exact_symbol_resolution(symbol_lookup, &mangled_path) {
                return Some((
                    resolution.owner_namespace.clone(),
                    resolution.symbol_name.clone(),
                ));
            }
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn resolve_owner_file_in_namespace_path(
    namespace_path: &str,
    member_parts: &[String],
    symbol_lookup: &ProjectSymbolLookup,
) -> Option<PathBuf> {
    resolve_symbol_file_in_namespace_path(namespace_path, member_parts, symbol_lookup)
}

fn resolve_symbol_owner_files_in_namespace(
    namespace: &str,
    referenced_symbols: &HashSet<String>,
    qualified_symbol_refs: &[Vec<String>],
    ctx: &DependencyResolutionContext<'_>,
    timings: Option<&DependencyGraphTimingTotals>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();

    for symbol in referenced_symbols {
        if let Some(timings) = timings {
            timings
                .direct_symbol_ref_count
                .fetch_add(1, Ordering::Relaxed);
        }
        let lookup_started_at = Instant::now();
        if let Some(file) = resolve_owner_file_in_namespace_path(
            namespace,
            std::slice::from_ref(symbol),
            ctx.symbol_lookup.as_ref(),
        ) {
            deps.insert(file);
        }
        if let Some(timings) = timings {
            timings
                .owner_lookup_ns
                .fetch_add(elapsed_nanos_u64(lookup_started_at), Ordering::Relaxed);
        }
    }

    for path in qualified_symbol_refs {
        if let Some(timings) = timings {
            timings.qualified_ref_count.fetch_add(1, Ordering::Relaxed);
        }
        let lookup_started_at = Instant::now();
        if let Some(file) =
            resolve_owner_file_in_namespace_path(namespace, path, ctx.symbol_lookup.as_ref())
        {
            deps.insert(file);
        }
        if let Some(timings) = timings {
            timings
                .owner_lookup_ns
                .fetch_add(elapsed_nanos_u64(lookup_started_at), Ordering::Relaxed);
        }
    }

    deps
}

fn namespace_dependency_files(
    namespace: &str,
    ctx: &DependencyResolutionContext<'_>,
    timings: Option<&DependencyGraphTimingTotals>,
) -> HashSet<PathBuf> {
    let started_at = Instant::now();
    let deps = ctx
        .namespace_files_map
        .get(namespace)
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    if let Some(timings) = timings {
        timings
            .namespace_files_ns
            .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    }
    deps
}

fn resolve_import_dependency_files(
    unit: &ParsedProjectUnit,
    import: &ImportDecl,
    referenced_symbols: &HashSet<String>,
    qualified_symbol_refs: &[Vec<String>],
    ctx: &DependencyResolutionContext<'_>,
    timings: Option<&DependencyGraphTimingTotals>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();

    if import.path.ends_with(".*") {
        if let Some(timings) = timings {
            timings
                .import_wildcard_count
                .fetch_add(1, Ordering::Relaxed);
        }
        let started_at = Instant::now();
        let namespace = import.path.trim_end_matches(".*");
        let owner_files = resolve_symbol_owner_files_in_namespace(
            namespace,
            referenced_symbols,
            qualified_symbol_refs,
            ctx,
            timings,
        );
        if owner_files.is_empty() {
            let fallback_started_at = Instant::now();
            let deps = namespace_dependency_files(namespace, ctx, timings);
            if let Some(timings) = timings {
                timings
                    .namespace_fallback_count
                    .fetch_add(1, Ordering::Relaxed);
                timings
                    .namespace_fallback_ns
                    .fetch_add(elapsed_nanos_u64(fallback_started_at), Ordering::Relaxed);
                timings
                    .import_wildcard_ns
                    .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
            }
            return deps;
        }
        if let Some(timings) = timings {
            timings
                .import_wildcard_ns
                .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
        }
        return owner_files;
    }

    let exact_started_at = Instant::now();
    if let Some(owner_file) = import_path_owner_file(&import.path, ctx.symbol_lookup.as_ref()) {
        if let Some(timings) = timings {
            timings.import_exact_count.fetch_add(1, Ordering::Relaxed);
            timings
                .import_exact_ns
                .fetch_add(elapsed_nanos_u64(exact_started_at), Ordering::Relaxed);
        }
        deps.insert(owner_file.clone());
        return deps;
    }
    if let Some(timings) = timings {
        timings.import_exact_count.fetch_add(1, Ordering::Relaxed);
        timings
            .import_exact_ns
            .fetch_add(elapsed_nanos_u64(exact_started_at), Ordering::Relaxed);
    }

    let import_key = import_lookup_key(import);
    let namespace_like_import = ctx.namespace_files_map.contains_key(&import.path)
        || unit
            .imports
            .iter()
            .any(|candidate| candidate.path == import.path && candidate.alias.is_some());
    if namespace_like_import {
        if let Some(timings) = timings {
            timings
                .import_namespace_alias_count
                .fetch_add(1, Ordering::Relaxed);
        }
        let started_at = Instant::now();
        for path in qualified_symbol_refs {
            if path.first().is_some_and(|part| part == &import_key) {
                if let Some(timings) = timings {
                    timings.qualified_ref_count.fetch_add(1, Ordering::Relaxed);
                }
                let lookup_started_at = Instant::now();
                let rest = &path[1..];
                if let Some(file) = resolve_owner_file_in_namespace_path(
                    &import.path,
                    rest,
                    ctx.symbol_lookup.as_ref(),
                ) {
                    deps.insert(file);
                }
                if let Some(timings) = timings {
                    timings
                        .owner_lookup_ns
                        .fetch_add(elapsed_nanos_u64(lookup_started_at), Ordering::Relaxed);
                }
            }
        }
        if deps.is_empty() {
            let fallback_started_at = Instant::now();
            let exact_import_namespace_fallback =
                namespace_dependency_files(&import.path, ctx, timings);
            if !exact_import_namespace_fallback.is_empty() {
                if let Some(timings) = timings {
                    timings
                        .namespace_fallback_count
                        .fetch_add(1, Ordering::Relaxed);
                    timings
                        .namespace_fallback_ns
                        .fetch_add(elapsed_nanos_u64(fallback_started_at), Ordering::Relaxed);
                    timings
                        .import_namespace_alias_ns
                        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
                }
                return exact_import_namespace_fallback;
            }
            if let Some((namespace, _)) = import.path.rsplit_once('.') {
                let parent_started_at = Instant::now();
                let deps = namespace_dependency_files(namespace, ctx, timings);
                if let Some(timings) = timings {
                    timings
                        .import_parent_namespace_count
                        .fetch_add(1, Ordering::Relaxed);
                    timings
                        .import_parent_namespace_ns
                        .fetch_add(elapsed_nanos_u64(parent_started_at), Ordering::Relaxed);
                    timings
                        .import_namespace_alias_ns
                        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
                }
                return deps;
            }
        }
        if let Some(timings) = timings {
            timings
                .import_namespace_alias_ns
                .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
        }
        return deps;
    }

    if let Some((namespace, _)) = import.path.rsplit_once('.') {
        if let Some(timings) = timings {
            timings
                .import_parent_namespace_count
                .fetch_add(1, Ordering::Relaxed);
        }
        let started_at = Instant::now();
        let owner_files = resolve_symbol_owner_files_in_namespace(
            namespace,
            referenced_symbols,
            qualified_symbol_refs,
            ctx,
            timings,
        );
        if owner_files.is_empty() {
            let fallback_started_at = Instant::now();
            deps.extend(namespace_dependency_files(namespace, ctx, timings));
            if let Some(timings) = timings {
                timings
                    .namespace_fallback_count
                    .fetch_add(1, Ordering::Relaxed);
                timings
                    .namespace_fallback_ns
                    .fetch_add(elapsed_nanos_u64(fallback_started_at), Ordering::Relaxed);
            }
        } else {
            deps.extend(owner_files);
        }
        if let Some(timings) = timings {
            timings
                .import_parent_namespace_ns
                .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
        }
    }

    deps
}

fn resolve_direct_dependencies_for_unit(
    unit: &ParsedProjectUnit,
    ctx: &DependencyResolutionContext<'_>,
    timings: Option<&DependencyGraphTimingTotals>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();
    let referenced_symbols: HashSet<String> = unit.referenced_symbols.iter().cloned().collect();

    let direct_started_at = Instant::now();
    for symbol in &unit.referenced_symbols {
        if let Some(timings) = timings {
            timings
                .direct_symbol_ref_count
                .fetch_add(1, Ordering::Relaxed);
        }
        if ctx
            .global_function_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_function_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
        if ctx
            .global_class_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_class_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
        if ctx
            .global_interface_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_interface_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
        if ctx
            .global_enum_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_enum_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
        if ctx
            .global_module_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_module_file_map.get(symbol) {
                if owner_file != &unit.file {
                    deps.insert(owner_file.clone());
                }
            }
        }
    }
    if let Some(timings) = timings {
        timings
            .direct_symbol_refs_ns
            .fetch_add(elapsed_nanos_u64(direct_started_at), Ordering::Relaxed);
    }

    for import in &unit.imports {
        deps.extend(resolve_import_dependency_files(
            unit,
            import,
            &referenced_symbols,
            &unit.qualified_symbol_refs,
            ctx,
            timings,
        ));
    }

    deps.remove(&unit.file);
    deps
}

fn build_file_dependency_graph_incremental(
    parsed_files: &[ParsedProjectUnit],
    ctx: &DependencyResolutionContext<'_>,
    previous: Option<&DependencyGraphCache>,
    timings: Option<&DependencyGraphTimingTotals>,
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
            let cache_started_at = Instant::now();
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
            if let Some(timings) = timings {
                timings
                    .cache_validation_ns
                    .fetch_add(elapsed_nanos_u64(cache_started_at), Ordering::Relaxed);
            }

            if previous_entry.semantic_fingerprint == unit.semantic_fingerprint
                && previous_entry.api_fingerprint == unit.api_fingerprint
                && !direct_dependency_api_changed
                && !direct_dependent_api_changed
            {
                reused += 1;
                if let Some(timings) = timings {
                    timings.files_reused.fetch_add(1, Ordering::Relaxed);
                }
                previous_entry
                    .direct_dependencies
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>()
            } else {
                if let Some(timings) = timings {
                    timings.files_rebuilt.fetch_add(1, Ordering::Relaxed);
                }
                resolve_direct_dependencies_for_unit(unit, ctx, timings)
            }
        } else {
            if let Some(timings) = timings {
                timings.files_rebuilt.fetch_add(1, Ordering::Relaxed);
            }
            resolve_direct_dependencies_for_unit(unit, ctx, timings)
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

struct PrecomputedDependencyClosures {
    files: Vec<PathBuf>,
    file_indices: HashMap<PathBuf, usize>,
    closures: Vec<Vec<u64>>,
}

fn precompute_all_transitive_dependencies(
    forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> PrecomputedDependencyClosures {
    fn empty_words(word_count: usize) -> Vec<u64> {
        vec![0; word_count]
    }

    fn set_bit(words: &mut [u64], index: usize) {
        let word = index / 64;
        let bit = index % 64;
        words[word] |= 1u64 << bit;
    }

    fn union_words(dst: &mut [u64], src: &[u64]) {
        for (dst_word, src_word) in dst.iter_mut().zip(src.iter()) {
            *dst_word |= *src_word;
        }
    }

    fn visit(
        file: &PathBuf,
        forward_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
        file_indices: &HashMap<PathBuf, usize>,
        word_count: usize,
        memo: &mut HashMap<PathBuf, Vec<u64>>,
        visiting: &mut HashSet<PathBuf>,
    ) -> Vec<u64> {
        if let Some(cached) = memo.get(file) {
            return cached.clone();
        }
        if !visiting.insert(file.clone()) {
            return empty_words(word_count);
        }

        let mut closure = empty_words(word_count);
        if let Some(deps) = forward_graph.get(file) {
            for dep in deps {
                if let Some(dep_index) = file_indices.get(dep) {
                    set_bit(&mut closure, *dep_index);
                }
                let dep_closure =
                    visit(dep, forward_graph, file_indices, word_count, memo, visiting);
                union_words(&mut closure, &dep_closure);
            }
        }

        visiting.remove(file);
        memo.insert(file.clone(), closure.clone());
        closure
    }

    let mut files: Vec<PathBuf> = forward_graph.keys().cloned().collect();
    files.sort();
    let file_indices = files
        .iter()
        .enumerate()
        .map(|(index, file)| (file.clone(), index))
        .collect::<HashMap<_, _>>();
    let word_count = files.len().div_ceil(64);
    let mut memo = HashMap::new();
    let mut visiting = HashSet::new();

    for file in &files {
        visit(
            file,
            forward_graph,
            &file_indices,
            word_count,
            &mut memo,
            &mut visiting,
        );
    }

    let closures = files
        .iter()
        .map(|file| memo.remove(file).unwrap_or_else(|| empty_words(word_count)))
        .collect();

    PrecomputedDependencyClosures {
        files,
        file_indices,
        closures,
    }
}

fn transitive_dependencies_from_precomputed(
    precomputed: &PrecomputedDependencyClosures,
    root: &Path,
) -> HashSet<PathBuf> {
    let Some(root_index) = precomputed.file_indices.get(root).copied() else {
        return HashSet::new();
    };
    let Some(words) = precomputed.closures.get(root_index) else {
        return HashSet::new();
    };
    let mut out = HashSet::new();
    for (word_index, word) in words.iter().copied().enumerate() {
        if word == 0 {
            continue;
        }
        let base = word_index * 64;
        for bit in 0..64 {
            if (word & (1u64 << bit)) == 0 {
                continue;
            }
            let file_index = base + bit;
            if let Some(file) = precomputed.files.get(file_index) {
                out.insert(file.clone());
            }
        }
    }
    out
}

fn dependency_graph_cache_from_state(
    entry_namespace: &str,
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
        entry_namespace: entry_namespace.to_string(),
        files,
    }
}

fn can_reuse_safe_rewrite_cache(
    previous_dependency_graph: Option<&DependencyGraphCache>,
    entry_namespace: &str,
) -> bool {
    previous_dependency_graph.is_some_and(|cache| cache.entry_namespace == entry_namespace)
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

fn compute_rewrite_context_fingerprint_for_unit_impl(
    unit: &ParsedProjectUnit,
    entry_namespace: &str,
    ctx: &RewriteFingerprintContext<'_>,
    timings: Option<&RewriteFingerprintTimingTotals>,
) -> String {
    let mut relevant_namespaces: HashSet<String> = HashSet::new();

    let mut hasher = stable_hasher();
    entry_namespace.hash(&mut hasher);
    unit.namespace.hash(&mut hasher);
    hash_imports(&unit.imports, &mut hasher);
    let referenced_symbols: HashSet<String> = unit.referenced_symbols.iter().cloned().collect();
    let mut referenced_symbol_list = referenced_symbols.iter().collect::<Vec<_>>();
    referenced_symbol_list.sort();
    let local_refs_started_at = Instant::now();
    for symbol in referenced_symbol_list {
        if let Some(timings) = timings {
            timings
                .local_symbol_ref_count
                .fetch_add(1, Ordering::Relaxed);
        }
        if ctx
            .global_function_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_function_file_map.get(symbol) {
                if owner_file != &unit.file {
                    hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
                }
            }
        }
        if ctx
            .global_class_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_class_file_map.get(symbol) {
                if owner_file != &unit.file {
                    hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
                }
            }
        }
        if ctx
            .global_interface_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_interface_file_map.get(symbol) {
                if owner_file != &unit.file {
                    hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
                }
            }
        }
        if ctx
            .global_enum_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_enum_file_map.get(symbol) {
                if owner_file != &unit.file {
                    hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
                }
            }
        }
        if ctx
            .global_module_map
            .get(symbol)
            .is_some_and(|owner_namespace| owner_namespace == &unit.namespace)
        {
            if let Some(owner_file) = ctx.global_module_file_map.get(symbol) {
                if owner_file != &unit.file {
                    hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
                }
            }
        }
    }
    if let Some(timings) = timings {
        timings
            .local_symbol_refs_ns
            .fetch_add(elapsed_nanos_u64(local_refs_started_at), Ordering::Relaxed);
    }
    let empty_namespace_files_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let empty_namespace_function_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let empty_namespace_class_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let empty_namespace_interface_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let empty_namespace_module_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
    let dependency_ctx = DependencyResolutionContext {
        namespace_files_map: &empty_namespace_files_map,
        namespace_function_files: &empty_namespace_function_files,
        namespace_class_files: &empty_namespace_class_files,
        namespace_interface_files: &empty_namespace_interface_files,
        namespace_module_files: &empty_namespace_module_files,
        global_function_map: ctx.global_function_map,
        global_function_file_map: ctx.global_function_file_map,
        global_class_map: ctx.global_class_map,
        global_class_file_map: ctx.global_class_file_map,
        global_interface_map: ctx.global_interface_map,
        global_interface_file_map: ctx.global_interface_file_map,
        global_enum_map: ctx.global_enum_map,
        global_enum_file_map: ctx.global_enum_file_map,
        global_module_map: ctx.global_module_map,
        global_module_file_map: ctx.global_module_file_map,
        symbol_lookup: Arc::clone(&ctx.symbol_lookup),
    };

    for import in &unit.imports {
        if import.path.ends_with(".*") {
            if let Some(timings) = timings {
                timings
                    .wildcard_import_count
                    .fetch_add(1, Ordering::Relaxed);
            }
            let wildcard_started_at = Instant::now();
            let namespace = import.path.trim_end_matches(".*");
            let owner_files = resolve_symbol_owner_files_in_namespace(
                namespace,
                &referenced_symbols,
                &unit.qualified_symbol_refs,
                &dependency_ctx,
                None,
            );
            if owner_files.is_empty() {
                relevant_namespaces.insert(namespace.to_string());
                let prefixes_started_at = Instant::now();
                let prefixes = namespace_prefixes(namespace);
                if let Some(timings) = timings {
                    timings
                        .prefix_expand_count
                        .fetch_add(prefixes.len(), Ordering::Relaxed);
                }
                for prefix in prefixes {
                    relevant_namespaces.insert(prefix);
                }
                if let Some(timings) = timings {
                    timings
                        .relevant_namespace_prefixes_ns
                        .fetch_add(elapsed_nanos_u64(prefixes_started_at), Ordering::Relaxed);
                }
            } else {
                let mut owner_files = owner_files.into_iter().collect::<Vec<_>>();
                owner_files.sort();
                for owner_file in owner_files {
                    hash_file_api_fingerprint(ctx.file_api_fingerprints, &owner_file, &mut hasher);
                }
            }
            if let Some(timings) = timings {
                timings
                    .wildcard_imports_ns
                    .fetch_add(elapsed_nanos_u64(wildcard_started_at), Ordering::Relaxed);
            }
            continue;
        }

        if ctx.namespace_api_fingerprints.contains_key(&import.path) {
            if let Some(timings) = timings {
                timings
                    .namespace_alias_import_count
                    .fetch_add(1, Ordering::Relaxed);
            }
            let namespace_alias_started_at = Instant::now();
            if let Some(namespace_api_fingerprint) =
                ctx.namespace_api_fingerprints.get(&import.path)
            {
                import.path.hash(&mut hasher);
                namespace_api_fingerprint.hash(&mut hasher);
            }
            let import_key = import_lookup_key(import);
            let mut matched_owner_files = HashSet::new();
            for path in &unit.qualified_symbol_refs {
                if path.first().is_some_and(|part| part == &import_key) {
                    let rest = &path[1..];
                    if let Some(owner_file) = resolve_owner_file_in_namespace_path(
                        &import.path,
                        rest,
                        ctx.symbol_lookup.as_ref(),
                    ) {
                        matched_owner_files.insert(owner_file);
                    }
                }
            }
            if matched_owner_files.is_empty() {
                relevant_namespaces.insert(import.path.clone());
                let prefixes_started_at = Instant::now();
                let prefixes = namespace_prefixes(&import.path);
                if let Some(timings) = timings {
                    timings
                        .prefix_expand_count
                        .fetch_add(prefixes.len(), Ordering::Relaxed);
                }
                for prefix in prefixes {
                    relevant_namespaces.insert(prefix);
                }
                if let Some(timings) = timings {
                    timings
                        .relevant_namespace_prefixes_ns
                        .fetch_add(elapsed_nanos_u64(prefixes_started_at), Ordering::Relaxed);
                }
            } else {
                let mut matched_owner_files = matched_owner_files.into_iter().collect::<Vec<_>>();
                matched_owner_files.sort();
                for owner_file in matched_owner_files {
                    hash_file_api_fingerprint(ctx.file_api_fingerprints, &owner_file, &mut hasher);
                }
            }
            if let Some(timings) = timings {
                timings.namespace_alias_imports_ns.fetch_add(
                    elapsed_nanos_u64(namespace_alias_started_at),
                    Ordering::Relaxed,
                );
            }
            continue;
        }

        if let Some(timings) = timings {
            timings.exact_import_count.fetch_add(1, Ordering::Relaxed);
        }
        let exact_import_started_at = Instant::now();
        if let Some(owner_file) = import_path_owner_file(&import.path, ctx.symbol_lookup.as_ref()) {
            hash_file_api_fingerprint(ctx.file_api_fingerprints, owner_file, &mut hasher);
            if let Some(timings) = timings {
                timings.exact_imports_ns.fetch_add(
                    elapsed_nanos_u64(exact_import_started_at),
                    Ordering::Relaxed,
                );
            }
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
            let prefixes_started_at = Instant::now();
            let prefixes = namespace_prefixes(imported_namespace);
            if let Some(timings) = timings {
                timings
                    .prefix_expand_count
                    .fetch_add(prefixes.len(), Ordering::Relaxed);
            }
            for prefix in prefixes {
                relevant_namespaces.insert(prefix);
            }
            if let Some(timings) = timings {
                timings
                    .relevant_namespace_prefixes_ns
                    .fetch_add(elapsed_nanos_u64(prefixes_started_at), Ordering::Relaxed);
            }
        }
        if let Some(timings) = timings {
            timings.exact_imports_ns.fetch_add(
                elapsed_nanos_u64(exact_import_started_at),
                Ordering::Relaxed,
            );
        }
    }

    let namespace_hashing_started_at = Instant::now();
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
    if let Some(timings) = timings {
        timings.namespace_hashing_ns.fetch_add(
            elapsed_nanos_u64(namespace_hashing_started_at),
            Ordering::Relaxed,
        );
    }
    format!("{:016x}", hasher.finish())
}

#[cfg_attr(not(test), allow(dead_code))]
fn compute_rewrite_context_fingerprint_for_unit(
    unit: &ParsedProjectUnit,
    entry_namespace: &str,
    ctx: &RewriteFingerprintContext<'_>,
) -> String {
    compute_rewrite_context_fingerprint_for_unit_impl(unit, entry_namespace, ctx, None)
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
) -> Result<Option<RewrittenFileCacheEntry>, String> {
    let entry = match load_rewritten_file_cache_entry(project_root, file)? {
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

    Ok(Some(entry))
}

fn load_rewritten_file_cache_entry(
    project_root: &Path,
    file: &Path,
) -> Result<Option<RewrittenFileCacheEntry>, String> {
    let path = rewritten_file_cache_path(project_root, file);
    read_cache_blob_with_timing(&path, "rewrite cache", &REWRITE_CACHE_TIMING_TOTALS)
}

fn load_rewritten_file_cache_if_semantic_matches(
    project_root: &Path,
    file: &Path,
    semantic_fingerprint: &str,
) -> Result<Option<RewrittenFileCacheEntry>, String> {
    let entry = match load_rewritten_file_cache_entry(project_root, file)? {
        Some(entry) => entry,
        None => return Ok(None),
    };

    if entry.schema != REWRITE_CACHE_SCHEMA
        || entry.compiler_version != env!("CARGO_PKG_VERSION")
        || entry.semantic_fingerprint != semantic_fingerprint
    {
        return Ok(None);
    }

    Ok(Some(entry))
}

#[allow(clippy::too_many_arguments)]
fn save_rewritten_file_cache(
    project_root: &Path,
    file: &Path,
    semantic_fingerprint: &str,
    rewrite_context_fingerprint: &str,
    rewritten_program: &Program,
    api_program: &Program,
    specialization_projection: &Program,
    active_symbols: &HashSet<String>,
    has_specialization_demand: bool,
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
        api_program: api_program.clone(),
        specialization_projection: specialization_projection.clone(),
        active_symbols: {
            let mut symbols = active_symbols.iter().cloned().collect::<Vec<_>>();
            symbols.sort();
            symbols
        },
        has_specialization_demand,
    };
    write_cache_blob_with_timing(&path, "rewrite cache", &entry, &REWRITE_CACHE_TIMING_TOTALS)
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

#[derive(Debug, Clone)]
struct ObjectShardCachePaths {
    object_path: PathBuf,
    meta_path: PathBuf,
}

fn object_cache_paths(project_root: &Path, file: &Path) -> ObjectCachePaths {
    ObjectCachePaths {
        object_path: object_cache_object_path(project_root, file),
        meta_path: object_cache_meta_path(project_root, file),
    }
}

fn object_shard_cache_key(files: &[PathBuf]) -> String {
    let mut normalized_files = files.to_vec();
    normalized_files.sort();
    let mut hasher = stable_hasher();
    for file in &normalized_files {
        file.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn normalized_object_shard_members(
    members: &[ObjectShardMemberFingerprint],
) -> Vec<ObjectShardMemberFingerprint> {
    let mut normalized = members.to_vec();
    normalized.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then_with(|| left.semantic_fingerprint.cmp(&right.semantic_fingerprint))
            .then_with(|| {
                left.rewrite_context_fingerprint
                    .cmp(&right.rewrite_context_fingerprint)
            })
    });
    normalized
}

fn object_shard_cache_paths(project_root: &Path, files: &[PathBuf]) -> ObjectShardCachePaths {
    let key = object_shard_cache_key(files);
    ObjectShardCachePaths {
        object_path: project_root
            .join(".apexcache")
            .join("object_shards")
            .join(format!("{key}.{}", object_ext())),
        meta_path: project_root
            .join(".apexcache")
            .join("object_shards")
            .join(format!("{key}.json")),
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

fn dedupe_link_inputs(link_inputs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(link_inputs.len());
    for path in link_inputs {
        if seen.insert(path.clone()) {
            deduped.push(path);
        }
    }
    deduped
}

fn load_link_manifest_cache(project_root: &Path) -> Result<Option<LinkManifestCache>, String> {
    let path = link_manifest_cache_path(project_root);
    let cache: LinkManifestCache = match read_cache_blob(&path, "link manifest cache")? {
        Some(cache) => cache,
        None => return Ok(None),
    };
    if cache.schema != LINK_MANIFEST_CACHE_SCHEMA
        || cache.compiler_version != env!("CARGO_PKG_VERSION")
    {
        return Ok(None);
    }
    Ok(Some(cache))
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
    let meta: ObjectCacheEntry = match read_cache_blob_with_timing(
        &cache_paths.meta_path,
        "object cache meta",
        &OBJECT_CACHE_META_TIMING_TOTALS,
    )? {
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
    write_cache_blob_with_timing(
        &cache_paths.meta_path,
        "object cache meta",
        &meta,
        &OBJECT_CACHE_META_TIMING_TOTALS,
    )
}

fn load_object_shard_cache_hit(
    cache_paths: &ObjectShardCachePaths,
    members: &[ObjectShardMemberFingerprint],
    object_build_fingerprint: &str,
) -> Result<Option<PathBuf>, String> {
    if !cache_paths.meta_path.exists() || !cache_paths.object_path.exists() {
        return Ok(None);
    }
    let meta: ObjectShardCacheEntry = match read_cache_blob_with_timing(
        &cache_paths.meta_path,
        "object shard cache meta",
        &OBJECT_CACHE_META_TIMING_TOTALS,
    )? {
        Some(meta) => meta,
        None => return Ok(None),
    };

    let normalized_members = normalized_object_shard_members(members);

    if meta.schema != OBJECT_SHARD_CACHE_SCHEMA
        || meta.compiler_version != env!("CARGO_PKG_VERSION")
        || meta.object_build_fingerprint != object_build_fingerprint
        || normalized_object_shard_members(&meta.members) != normalized_members
    {
        return Ok(None);
    }

    Ok(Some(cache_paths.object_path.clone()))
}

fn save_object_shard_cache_meta(
    cache_paths: &ObjectShardCachePaths,
    members: &[ObjectShardMemberFingerprint],
    object_build_fingerprint: &str,
) -> Result<(), String> {
    if let Some(parent) = cache_paths.meta_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "{}: Failed to create object shard cache directory '{}': {}",
                "error".red().bold(),
                parent.display(),
                e
            )
        })?;
    }

    let meta = ObjectShardCacheEntry {
        schema: OBJECT_SHARD_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        object_build_fingerprint: object_build_fingerprint.to_string(),
        members: normalized_object_shard_members(members),
    };
    write_cache_blob_with_timing(
        &cache_paths.meta_path,
        "object shard cache meta",
        &meta,
        &OBJECT_CACHE_META_TIMING_TOTALS,
    )
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
        let source = fs::read_to_string(file).map_err(|e| {
            format!(
                "{}: Failed to read '{}': {}",
                "error".red().bold(),
                file.display(),
                e
            )
        })?;
        let source_fp = source_fingerprint(&source);
        if cache.file_metadata == file_metadata && cache.source_fingerprint == source_fp {
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
    let mut interface_names = Vec::new();
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

    fn collect_interface_names(decl: &Decl, module_prefix: Option<String>, out: &mut Vec<String>) {
        match decl {
            Decl::Interface(interface) => {
                if let Some(module_name) = module_prefix {
                    out.push(format!("{}__{}", module_name, interface.name));
                } else {
                    out.push(interface.name.clone());
                }
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner in &module.declarations {
                    collect_interface_names(&inner.node, Some(next_prefix.clone()), out);
                }
            }
            Decl::Function(_) | Decl::Class(_) | Decl::Enum(_) | Decl::Import(_) => {}
        }
    }

    fn collect_module_names(decl: &Decl, module_prefix: Option<String>, out: &mut Vec<String>) {
        if let Decl::Module(module) = decl {
            let full_name = if let Some(prefix) = module_prefix {
                format!("{}__{}", prefix, module.name)
            } else {
                module.name.clone()
            };
            out.push(full_name.clone());
            for inner in &module.declarations {
                collect_module_names(&inner.node, Some(full_name.clone()), out);
            }
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
                Decl::Function(_) => collect_function_names(&decl.node, None, &mut function_names),
                Decl::Module(_) => {
                    collect_module_names(&decl.node, None, &mut module_names);
                    collect_function_names(&decl.node, None, &mut function_names);
                    collect_class_names(&decl.node, None, &mut class_names);
                    collect_interface_names(&decl.node, None, &mut interface_names);
                    collect_enum_names(&decl.node, None, &mut enum_names);
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
                    &mut global_function_map,
                    &mut global_function_file_map,
                    &mut function_collisions,
                    &mut project_symbol_lookup_exact,
                    &mut project_symbol_lookup_wildcard_members,
                    needs_project_symbol_lookup,
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
                        &mut global_class_map,
                        &mut global_class_file_map,
                        &mut class_collisions,
                        &mut project_symbol_lookup_exact,
                        &mut project_symbol_lookup_wildcard_members,
                        needs_project_symbol_lookup,
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
                        &mut global_interface_map,
                        &mut global_interface_file_map,
                        &mut interface_collisions,
                        &mut project_symbol_lookup_exact,
                        &mut project_symbol_lookup_wildcard_members,
                        needs_project_symbol_lookup,
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
                        &mut global_enum_map,
                        &mut global_enum_file_map,
                        &mut enum_collisions,
                        &mut project_symbol_lookup_exact,
                        &mut project_symbol_lookup_wildcard_members,
                        needs_project_symbol_lookup,
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
                        &mut global_module_map,
                        &mut global_module_file_map,
                        &mut module_collisions,
                        &mut project_symbol_lookup_exact,
                        &mut project_symbol_lookup_wildcard_members,
                        needs_project_symbol_lookup,
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
    let rewrite_timing_totals = Arc::new(RewriteTimingTotals::default());
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
                            &unit.namespace,
                            &entry_namespace,
                            &namespace_functions,
                            &global_function_map,
                            &namespace_class_map,
                            &global_class_map,
                            &namespace_interface_map,
                            &global_interface_map,
                            &namespace_enum_map,
                            &global_enum_map,
                            &namespace_module_map,
                            &global_module_map,
                            &unit.imports,
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
        #[derive(Clone)]
        struct ObjectCodegenShard {
            member_indices: Vec<usize>,
            member_files: Vec<PathBuf>,
            member_fingerprints: Vec<ObjectShardMemberFingerprint>,
            cache_paths: Option<ObjectShardCachePaths>,
        }

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
                        for index in &shard.member_indices {
                            let unit = &rewritten_files[*index];
                            let declaration_closure = declaration_symbols_for_unit(
                                &unit.file,
                                &unit.active_symbols,
                                &precomputed_dependency_closures,
                                &codegen_reference_metadata,
                                &entry_namespace,
                                &project_symbol_lookup,
                                &global_function_map,
                                &global_function_file_map,
                                &global_class_map,
                                &global_class_file_map,
                                &global_interface_map,
                                &global_interface_file_map,
                                &global_enum_map,
                                &global_enum_file_map,
                                &global_module_map,
                                &global_module_file_map,
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

fn validate_opt_level(opt_level: Option<&str>) -> Result<(), String> {
    let Some(raw) = opt_level else {
        return Ok(());
    };

    let normalized = raw.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "0" | "1" | "2" | "3" | "s" | "z" | "fast"
    ) {
        return Ok(());
    }

    Err(format!(
        "{}: Invalid optimization level '{}'. Expected one of: 0, 1, 2, 3, s, z, fast.",
        "error".red().bold(),
        raw
    ))
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

#[cfg(test)]
mod tests {
    use super::{
        api_program_fingerprint, build_file_dependency_graph_incremental, build_project,
        build_project_symbol_lookup, build_reverse_dependency_graph, can_reuse_safe_rewrite_cache,
        check_command, check_file, codegen_program_for_unit, compile_file, compile_source,
        component_fingerprint, compute_link_fingerprint, compute_namespace_api_fingerprints,
        compute_rewrite_context_fingerprint_for_unit, dedupe_link_inputs, escape_response_file_arg,
        find_test_files, fix_target, format_targets, lex_file, lint_target,
        load_cached_fingerprint, load_link_manifest_cache, load_object_shard_cache_hit,
        load_semantic_cached_fingerprint, new_project, object_shard_cache_key,
        object_shard_cache_paths, parse_file, parse_project_unit,
        precompute_all_transitive_dependencies, read_cache_blob, reusable_component_fingerprints,
        run_project, run_tests, save_object_shard_cache_meta, semantic_program_fingerprint,
        should_skip_final_link, show_project_info, transitive_dependencies_from_precomputed,
        transitive_dependents, typecheck_summary_cache_from_state, typecheck_summary_cache_matches,
        DependencyGraphCache, DependencyGraphFileEntry, DependencyResolutionContext, LinkConfig,
        LinkManifestCache, ObjectShardMemberFingerprint, OutputKind, ParsedFileCacheEntry,
        ParsedProjectUnit, RewriteFingerprintContext, RewrittenProjectUnit,
        DEPENDENCY_GRAPH_CACHE_SCHEMA, LINK_MANIFEST_CACHE_SCHEMA,
    };
    use crate::ast::Program;
    use crate::borrowck::BorrowChecker;
    use crate::formatter::format_program_canonical;
    use crate::parser::Parser;
    use crate::typeck::TypeChecker;
    use std::collections::{HashMap, HashSet};
    use std::fs;

    use std::path::Path;
    use std::path::PathBuf;

    use std::sync::{Arc, Mutex, OnceLock};

    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn parse_program(source: &str) -> Program {
        let tokens = crate::lexer::tokenize(source).expect("tokenize");
        let mut parser = Parser::new(tokens);
        parser.parse_program().expect("parse")
    }

    pub fn fingerprint_for(source: &str) -> String {
        let program = parse_program(source);
        semantic_program_fingerprint(&program)
    }

    pub fn rewrite_fingerprint_for_test_unit(
        parsed_files: &[ParsedProjectUnit],
        target_file: &Path,
        entry_namespace: &str,
    ) -> String {
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
        ) = collect_project_symbol_maps(parsed_files);
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
        let namespace_api_fingerprints = compute_namespace_api_fingerprints(parsed_files);
        let file_api_fingerprints = parsed_files
            .iter()
            .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
            .collect::<HashMap<_, _>>();
        let namespace_interface_files: HashMap<String, HashMap<String, PathBuf>> = HashMap::new();
        let global_interface_map: HashMap<String, String> = HashMap::new();
        let global_interface_file_map: HashMap<String, PathBuf> = HashMap::new();
        let symbol_lookup = Arc::new(build_project_symbol_lookup(
            &global_function_map,
            &global_function_file_map,
            &global_class_map,
            &global_class_file_map,
            &global_interface_map,
            &global_interface_file_map,
            &global_enum_map,
            &global_enum_file_map,
            &global_module_map,
            &global_module_file_map,
        ));
        let rewrite_ctx = RewriteFingerprintContext {
            namespace_functions: &namespace_functions,
            namespace_function_files: &namespace_function_files,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            namespace_class_files: &namespace_class_files,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            namespace_interface_files: &namespace_interface_files,
            global_interface_map: &global_interface_map,
            global_interface_file_map: &global_interface_file_map,
            global_enum_map: &global_enum_map,
            global_enum_file_map: &global_enum_file_map,
            namespace_modules: &namespace_modules,
            namespace_module_files: &namespace_module_files,
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
            namespace_api_fingerprints: &namespace_api_fingerprints,
            file_api_fingerprints: &file_api_fingerprints,
            symbol_lookup: Arc::clone(&symbol_lookup),
        };
        let target_unit = parsed_files
            .iter()
            .find(|u| u.file == target_file)
            .expect("target unit");
        compute_rewrite_context_fingerprint_for_unit(target_unit, entry_namespace, &rewrite_ctx)
    }

    pub fn assert_frontend_pipeline_ok(source: &str) {
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

    pub fn make_temp_project_root(tag: &str) -> PathBuf {
        let base_temp = std::env::temp_dir()
            .canonicalize()
            .unwrap_or_else(|_| std::env::temp_dir());
        let temp_root = base_temp.join(format!(
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

    pub fn cli_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    pub struct CwdRestore {
        previous: PathBuf,
    }

    impl Drop for CwdRestore {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.previous);
        }
    }

    pub fn with_current_dir<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
        let _lock = cli_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(dir).expect("set current dir");
        let _restore = CwdRestore { previous };
        f()
    }

    pub fn write_test_project_config(root: &Path, files: &[&str], entry: &str, output: &str) {
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
    pub fn collect_project_symbol_maps(
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
            for module_name in &unit.module_names {
                namespace_files_map
                    .entry(format!(
                        "{}.{}",
                        unit.namespace,
                        module_name.replace("__", ".")
                    ))
                    .or_insert_with(Vec::new)
                    .push(unit.file.clone());
            }
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
            files.dedup();
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

    mod cli;
    mod compile_source;
    mod project;
    mod typeck_frontend;
}
