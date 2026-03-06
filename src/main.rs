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
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use twox_hash::XxHash64;

use crate::ast::{Decl, ImportDecl, Program};
use crate::borrowck::BorrowChecker;
use crate::codegen::Codegen;
use crate::import_check::ImportChecker;
use crate::parser::Parser;
use crate::project::{find_project_root, OutputKind, ProjectConfig};
use crate::test_runner::{discover_tests, generate_test_runner_with_source, print_discovery};
use crate::typeck::TypeChecker;

#[derive(ClapParser)]
#[command(name = "apex")]
#[command(author = "Remyyy")]
#[command(version = "1.3.1")]
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
        /// Optimization level: 0,1,2,3,s,z,fast (default: 3)
        #[arg(long)]
        opt_level: Option<String>,
        /// Target triple passed to clang (example: x86_64-unknown-linux-gnu)
        #[arg(long)]
        target: Option<String>,
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
    /// Run static lint checks
    Lint {
        /// File to lint (default: project entry point)
        path: Option<PathBuf>,
    },
    /// Apply safe automated fixes
    Fix {
        /// File to fix (default: project entry point)
        path: Option<PathBuf>,
    },
    /// Format Apex source files
    Fmt {
        /// File or directory to format (default: current project or current directory)
        path: Option<PathBuf>,
        /// Check formatting without writing changes
        #[arg(long)]
        check: bool,
    },
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
    /// Start LSP server
    Lsp,
    /// Run tests
    Test {
        /// Input file or directory (default: project test files)
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// List tests without running them
        #[arg(short, long)]
        list: bool,
        /// Filter tests by name pattern
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Generate extern bindings from a C header file
    Bindgen {
        /// Input C header file
        header: PathBuf,
        /// Output Apex file (prints to stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Benchmark execution time
    Bench {
        /// Input file (optional, runs project if not specified)
        file: Option<PathBuf>,
        /// Number of measured runs
        #[arg(short, long, default_value_t = 5)]
        iterations: usize,
    },
    /// Profile a single execution
    Profile {
        /// Input file (optional, runs project if not specified)
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
        Commands::Check { file } => check_file(file.as_deref()),
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

fn stable_hasher() -> XxHash64 {
    XxHash64::with_seed(0)
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

const PARSE_CACHE_SCHEMA: &str = "v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedFileCacheEntry {
    schema: String,
    compiler_version: String,
    source_fingerprint: String,
    namespace: String,
    program: Program,
    imports: Vec<ImportDecl>,
}

#[derive(Debug, Clone)]
struct ParsedProjectUnit {
    file: PathBuf,
    namespace: String,
    program: Program,
    imports: Vec<ImportDecl>,
    source_fingerprint: String,
    function_names: Vec<String>,
    class_names: Vec<String>,
    module_names: Vec<String>,
    from_parse_cache: bool,
}

#[derive(Debug, Clone)]
struct RewrittenProjectUnit {
    file: PathBuf,
    program: Program,
    source_fingerprint: String,
    active_symbols: HashSet<String>,
    from_rewrite_cache: bool,
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

fn load_parsed_file_cache(
    project_root: &Path,
    file: &Path,
    source_fp: &str,
) -> Result<Option<ParsedFileCacheEntry>, String> {
    let path = parsed_file_cache_path(project_root, file);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path).map_err(|e| {
        format!(
            "{}: Failed to read parse cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;

    let entry: ParsedFileCacheEntry = serde_json::from_str(&raw).map_err(|e| {
        format!(
            "{}: Failed to parse cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;

    if entry.schema != PARSE_CACHE_SCHEMA
        || entry.compiler_version != env!("CARGO_PKG_VERSION")
        || entry.source_fingerprint != source_fp
    {
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

    let json = serde_json::to_string(entry).map_err(|e| {
        format!(
            "{}: Failed to serialize parse cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;

    fs::write(&path, json).map_err(|e| {
        format!(
            "{}: Failed to write parse cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })
}

const REWRITE_CACHE_SCHEMA: &str = "v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RewrittenFileCacheEntry {
    schema: String,
    compiler_version: String,
    source_fingerprint: String,
    rewrite_context_fingerprint: String,
    rewritten_program: Program,
}

const OBJECT_CACHE_SCHEMA: &str = "v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObjectCacheEntry {
    schema: String,
    compiler_version: String,
    source_fingerprint: String,
    rewrite_context_fingerprint: String,
    object_build_fingerprint: String,
}

fn rewritten_file_cache_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    file.hash(&mut hasher);
    project_root
        .join(".apexcache")
        .join("rewritten")
        .join(format!("{:016x}.json", hasher.finish()))
}

fn hash_sorted_map(map: &HashMap<String, String>, hasher: &mut impl Hasher) {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.cmp(b.1)));
    for (k, v) in entries {
        k.hash(hasher);
        v.hash(hasher);
    }
}

fn hash_sorted_map_of_sets(map: &HashMap<String, HashSet<String>>, hasher: &mut impl Hasher) {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (k, set) in entries {
        k.hash(hasher);
        let mut values = set.iter().collect::<Vec<_>>();
        values.sort();
        for v in values {
            v.hash(hasher);
        }
    }
}

fn compute_rewrite_context_fingerprint(
    entry_namespace: &str,
    namespace_functions: &HashMap<String, HashSet<String>>,
    global_function_map: &HashMap<String, String>,
    namespace_classes: &HashMap<String, HashSet<String>>,
    global_class_map: &HashMap<String, String>,
    namespace_modules: &HashMap<String, HashSet<String>>,
    global_module_map: &HashMap<String, String>,
) -> String {
    let mut hasher = stable_hasher();
    entry_namespace.hash(&mut hasher);
    hash_sorted_map_of_sets(namespace_functions, &mut hasher);
    hash_sorted_map(global_function_map, &mut hasher);
    hash_sorted_map_of_sets(namespace_classes, &mut hasher);
    hash_sorted_map(global_class_map, &mut hasher);
    hash_sorted_map_of_sets(namespace_modules, &mut hasher);
    hash_sorted_map(global_module_map, &mut hasher);
    format!("{:016x}", hasher.finish())
}

fn collect_active_symbols(program: &Program) -> HashSet<String> {
    let mut symbols = HashSet::new();
    for decl in &program.declarations {
        match &decl.node {
            Decl::Function(func) => {
                symbols.insert(func.name.clone());
            }
            Decl::Class(class) => {
                symbols.insert(class.name.clone());
            }
            Decl::Enum(en) => {
                symbols.insert(en.name.clone());
            }
            Decl::Module(module) => {
                symbols.insert(module.name.clone());
                for inner in &module.declarations {
                    match &inner.node {
                        Decl::Function(func) => {
                            symbols.insert(format!("{}__{}", module.name, func.name));
                        }
                        Decl::Class(class) => {
                            symbols.insert(format!("{}__{}", module.name, class.name));
                        }
                        Decl::Enum(en) => {
                            symbols.insert(format!("{}__{}", module.name, en.name));
                        }
                        _ => {}
                    }
                }
            }
            Decl::Import(_) | Decl::Interface(_) => {}
        }
    }
    symbols
}

fn load_rewritten_file_cache(
    project_root: &Path,
    file: &Path,
    source_fingerprint: &str,
    rewrite_context_fingerprint: &str,
) -> Result<Option<Program>, String> {
    let path = rewritten_file_cache_path(project_root, file);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path).map_err(|e| {
        format!(
            "{}: Failed to read rewrite cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;

    let entry: RewrittenFileCacheEntry = serde_json::from_str(&raw).map_err(|e| {
        format!(
            "{}: Failed to parse rewrite cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;

    if entry.schema != REWRITE_CACHE_SCHEMA
        || entry.compiler_version != env!("CARGO_PKG_VERSION")
        || entry.source_fingerprint != source_fingerprint
        || entry.rewrite_context_fingerprint != rewrite_context_fingerprint
    {
        return Ok(None);
    }

    Ok(Some(entry.rewritten_program))
}

fn save_rewritten_file_cache(
    project_root: &Path,
    file: &Path,
    source_fingerprint: &str,
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
        source_fingerprint: source_fingerprint.to_string(),
        rewrite_context_fingerprint: rewrite_context_fingerprint.to_string(),
        rewritten_program: rewritten_program.clone(),
    };
    let json = serde_json::to_string(&entry).map_err(|e| {
        format!(
            "{}: Failed to serialize rewrite cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;

    fs::write(&path, json).map_err(|e| {
        format!(
            "{}: Failed to write rewrite cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })
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

fn compute_object_build_fingerprint(link: &LinkConfig<'_>) -> String {
    let mut hasher = stable_hasher();
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    link.opt_level.hash(&mut hasher);
    link.target.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn load_object_cache_hit(
    project_root: &Path,
    file: &Path,
    source_fingerprint: &str,
    rewrite_context_fingerprint: &str,
    object_build_fingerprint: &str,
) -> Result<Option<PathBuf>, String> {
    let meta_path = object_cache_meta_path(project_root, file);
    let obj_path = object_cache_object_path(project_root, file);
    if !meta_path.exists() || !obj_path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&meta_path).map_err(|e| {
        format!(
            "{}: Failed to read object cache meta '{}': {}",
            "error".red().bold(),
            meta_path.display(),
            e
        )
    })?;
    let meta: ObjectCacheEntry = serde_json::from_str(&raw).map_err(|e| {
        format!(
            "{}: Failed to parse object cache meta '{}': {}",
            "error".red().bold(),
            meta_path.display(),
            e
        )
    })?;

    if meta.schema != OBJECT_CACHE_SCHEMA
        || meta.compiler_version != env!("CARGO_PKG_VERSION")
        || meta.source_fingerprint != source_fingerprint
        || meta.rewrite_context_fingerprint != rewrite_context_fingerprint
        || meta.object_build_fingerprint != object_build_fingerprint
    {
        return Ok(None);
    }

    Ok(Some(obj_path))
}

fn save_object_cache_meta(
    project_root: &Path,
    file: &Path,
    source_fingerprint: &str,
    rewrite_context_fingerprint: &str,
    object_build_fingerprint: &str,
) -> Result<(), String> {
    let meta_path = object_cache_meta_path(project_root, file);
    if let Some(parent) = meta_path.parent() {
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
        source_fingerprint: source_fingerprint.to_string(),
        rewrite_context_fingerprint: rewrite_context_fingerprint.to_string(),
        object_build_fingerprint: object_build_fingerprint.to_string(),
    };
    let json = serde_json::to_string(&meta).map_err(|e| {
        format!(
            "{}: Failed to serialize object cache meta '{}': {}",
            "error".red().bold(),
            meta_path.display(),
            e
        )
    })?;
    fs::write(&meta_path, json).map_err(|e| {
        format!(
            "{}: Failed to write object cache meta '{}': {}",
            "error".red().bold(),
            meta_path.display(),
            e
        )
    })
}

fn parse_project_unit(project_root: &Path, file: &Path) -> Result<ParsedProjectUnit, String> {
    let source = fs::read_to_string(file).map_err(|e| {
        format!(
            "{}: Failed to read '{}': {}",
            "error".red().bold(),
            file.display(),
            e
        )
    })?;

    let filename = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown.apex");

    let source_fp = source_fingerprint(&source);
    let (namespace, program, imports, from_parse_cache) =
        if let Some(cache) = load_parsed_file_cache(project_root, file, &source_fp)? {
            (cache.namespace, cache.program, cache.imports, true)
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

            let cache_entry = ParsedFileCacheEntry {
                schema: PARSE_CACHE_SCHEMA.to_string(),
                compiler_version: env!("CARGO_PKG_VERSION").to_string(),
                source_fingerprint: source_fp.clone(),
                namespace: namespace.clone(),
                program: program.clone(),
                imports: imports.clone(),
            };
            save_parsed_file_cache(project_root, file, &cache_entry)?;

            (namespace, program, imports, false)
        };

    let mut function_names = Vec::new();
    let mut class_names = Vec::new();
    let mut module_names = Vec::new();
    for decl in &program.declarations {
        match &decl.node {
            Decl::Function(func) => function_names.push(func.name.clone()),
            Decl::Class(class) => class_names.push(class.name.clone()),
            Decl::Module(module) => module_names.push(module.name.clone()),
            _ => {}
        }
    }

    Ok(ParsedProjectUnit {
        file: file.to_path_buf(),
        namespace,
        program,
        imports,
        source_fingerprint: source_fp,
        function_names,
        class_names,
        module_names,
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

// Welcome to {}!
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
    if let Some(cached) = load_cached_fingerprint(&project_root) {
        if cached == fingerprint && project_build_artifact_exists(&output_path, emit_llvm) {
            println!(
                "{} {} ({})",
                "Up to date".green().bold(),
                config.name.cyan(),
                "build cache".dimmed()
            );
            return Ok(());
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
    let mut global_class_map: HashMap<String, String> = HashMap::new(); // class_name -> namespace
    let mut global_module_map: HashMap<String, String> = HashMap::new(); // module_name -> namespace
    let mut namespace_class_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_module_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut function_collisions: Vec<(String, String, String)> = Vec::new();
    let mut class_collisions: Vec<(String, String, String)> = Vec::new();
    let mut module_collisions: Vec<(String, String, String)> = Vec::new();
    let mut parse_cache_hits: usize = 0;

    let mut parsed_units: Vec<ParsedProjectUnit> = files
        .par_iter()
        .map(|file| parse_project_unit(&project_root, file))
        .collect::<Result<Vec<_>, String>>()?;
    parsed_units.sort_by(|a, b| a.file.cmp(&b.file));

    for unit in parsed_units {
        if unit.from_parse_cache {
            parse_cache_hits += 1;
        }

        // Extract symbol definitions for global maps
        let class_entry = namespace_class_map
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
            }
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
                "  â†’ '{}' is defined in both '{}' and '{}'",
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
                "  â†’ '{}' is defined in both '{}' and '{}'",
                name, ns_a, ns_b
            );
        }
        return Err(
            "Project contains colliding top-level module names. Use unique module names per project."
                .to_string(),
        );
    }

    // Phase 2: Check imports for each file
    if do_check {
        println!("{} Checking imports...", "→".cyan());

        let import_results: Vec<Result<(), String>> = parsed_files
            .par_iter()
            .map(|unit| {
                let mut checker = ImportChecker::new(
                    global_function_map.clone(),
                    unit.namespace.clone(),
                    unit.imports.clone(),
                );

                if let Err(errors) = checker.check_program(&unit.program) {
                    let filename = unit
                        .file
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    let mut rendered =
                        format!("{} Import errors in {}:\n", "error".red().bold(), filename);
                    for err in errors {
                        rendered.push_str(&format!("  → {}\n", err.format()));
                    }
                    return Err(rendered);
                }
                Ok(())
            })
            .collect();

        for result in import_results {
            if let Err(rendered) = result {
                eprint!("{rendered}");
                return Err("Import check failed".to_string());
            }
        }
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

    let rewrite_context_fingerprint = compute_rewrite_context_fingerprint(
        &entry_namespace,
        &namespace_functions,
        &global_function_map,
        &namespace_class_map,
        &global_class_map,
        &namespace_module_map,
        &global_module_map,
    );

    // Phase 3: Build combined AST with deterministic namespace mangling.
    let rewritten_results: Vec<Result<RewrittenProjectUnit, String>> = parsed_files
        .par_iter()
        .map(|unit| {
            if let Some(cached) = load_rewritten_file_cache(
                &project_root,
                &unit.file,
                &unit.source_fingerprint,
                &rewrite_context_fingerprint,
            )? {
                let active_symbols = collect_active_symbols(&cached);
                return Ok(RewrittenProjectUnit {
                    file: unit.file.clone(),
                    program: cached,
                    source_fingerprint: unit.source_fingerprint.clone(),
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
                &namespace_module_map,
                &global_module_map,
                &unit.imports,
            );
            save_rewritten_file_cache(
                &project_root,
                &unit.file,
                &unit.source_fingerprint,
                &rewrite_context_fingerprint,
                &rewritten,
            )?;
            Ok(RewrittenProjectUnit {
                file: unit.file.clone(),
                active_symbols: collect_active_symbols(&rewritten),
                program: rewritten,
                source_fingerprint: unit.source_fingerprint.clone(),
                from_rewrite_cache: false,
            })
        })
        .collect();

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

    let mut combined_program = Program {
        package: None,
        declarations: Vec::new(),
    };
    for unit in &rewritten_files {
        combined_program
            .declarations
            .extend(unit.program.declarations.iter().cloned());
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
        compile_program_ast(
            &combined_program,
            &entry_path,
            &output_path,
            emit_llvm,
            &link,
        )?;
    } else {
        let object_build_fingerprint = compute_object_build_fingerprint(&link);
        let mut object_paths: Vec<PathBuf> = Vec::new();
        let mut object_cache_hits: usize = 0;
        let object_candidate_count = rewritten_files
            .iter()
            .filter(|unit| !unit.active_symbols.is_empty())
            .count();

        for unit in &rewritten_files {
            if unit.active_symbols.is_empty() {
                continue;
            }

            if let Some(cached_obj) = load_object_cache_hit(
                &project_root,
                &unit.file,
                &unit.source_fingerprint,
                &rewrite_context_fingerprint,
                &object_build_fingerprint,
            )? {
                object_paths.push(cached_obj);
                object_cache_hits += 1;
                continue;
            }

            let obj_path = object_cache_object_path(&project_root, &unit.file);
            compile_program_ast_to_object_filtered(
                &combined_program,
                &unit.file,
                &obj_path,
                &link,
                &unit.active_symbols,
            )?;
            save_object_cache_meta(
                &project_root,
                &unit.file,
                &unit.source_fingerprint,
                &rewrite_context_fingerprint,
                &object_build_fingerprint,
            )?;
            object_paths.push(obj_path);
        }

        if object_cache_hits > 0 {
            println!(
                "{} Reused object cache for {}/{} files",
                "→".cyan(),
                object_cache_hits,
                object_candidate_count
            );
        }

        link_objects(&object_paths, &output_path, &link)?;
    }

    println!(
        "{} {} -> {}",
        "Built".green().bold(),
        config.name.cyan(),
        output_path.display()
    );

    save_cached_fingerprint(&project_root, &fingerprint)?;

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
) -> Result<(), String> {
    let context = Context::create();
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let mut codegen = Codegen::new(&context, module_name);
    codegen
        .compile_filtered(program, active_symbols)
        .map_err(|e| format!("{}: Codegen error: {}", "error".red().bold(), e.message))?;

    let ir_path = object_path.with_extension("ll");
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
    codegen.write_ir(&ir_path)?;
    compile_ir_to_object(&ir_path, object_path, link)?;
    let _ = fs::remove_file(&ir_path);
    Ok(())
}

/// Build and run the current project
fn run_project(args: &[String], release: bool, do_check: bool) -> Result<(), String> {
    build_project(release, false, do_check)?;

    let cwd = current_dir_checked()?;
    let project_root = find_project_root(&cwd)
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

    compile_file(file, Some(&output), false, do_check, None, None)?;

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
    opt_level: Option<&str>,
    target: Option<&str>,
) -> Result<(), String> {
    // Check if we're in a project
    if let Some(project_root) = find_project_root(&current_dir_checked()?) {
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

    compile_source(
        &source,
        file,
        &output_path,
        emit_llvm,
        do_check,
        opt_level,
        target,
    )?;

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

/// Compile LLVM IR using clang
fn compile_ir(ir_path: &Path, output_path: &Path, link: &LinkConfig<'_>) -> Result<(), String> {
    let opt_flag = resolve_clang_opt_flag(link.opt_level);
    let run_clang = |march_native: bool, mtune_native: bool| {
        let mut cmd = Command::new("clang");
        cmd.arg(ir_path)
            .arg("-o")
            .arg(output_path)
            .arg("-Wno-override-module")
            .arg(opt_flag);

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
        cmd.arg("-llegacy_stdio_definitions");

        #[cfg(not(windows))]
        cmd.arg("-lm").arg("-pthread");

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

fn compile_ir_to_object(
    ir_path: &Path,
    object_path: &Path,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    let opt_flag = resolve_clang_opt_flag(link.opt_level);
    let mut cmd = Command::new("clang");
    cmd.arg("-c")
        .arg(ir_path)
        .arg("-o")
        .arg(object_path)
        .arg("-Wno-override-module")
        .arg(opt_flag);

    if let Some(target_triple) = link.target {
        cmd.arg("--target").arg(target_triple);
    } else {
        cmd.arg("-march=native").arg("-mtune=native");
    }

    cmd.arg("-fomit-frame-pointer");

    let output = cmd.output().map_err(|_| {
        format!(
            "{}: Clang not found. Install clang to compile.",
            "error".red().bold()
        )
    })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Err(format!(
        "{}: Clang failed while compiling object: {}",
        "error".red().bold(),
        stderr
    ))
}

fn link_objects(
    objects: &[PathBuf],
    output_path: &Path,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
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
            let mut cmd = Command::new("clang");
            cmd.args(objects).arg("-o").arg(output_path).arg(opt_flag);

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
            cmd.arg("-llegacy_stdio_definitions");

            #[cfg(not(windows))]
            cmd.arg("-lm").arg("-pthread");

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
    let project_root = find_project_root(&current_dir_checked()?).ok_or_else(|| {
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
    println!("  {}: {:?}", "Output Kind".dimmed(), config.output_kind);
    println!("  {}: {}", "Opt Level".dimmed(), config.opt_level);
    println!(
        "  {}: {}",
        "Target".dimmed(),
        config.target.as_deref().unwrap_or("native/default")
    );
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

    if !config.link_search.is_empty() {
        println!("\n{}", "Link Search Paths:".dimmed());
        for path in &config.link_search {
            println!("  - {}", path);
        }
    }

    if !config.link_libs.is_empty() {
        println!("\n{}", "Link Libraries:".dimmed());
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
        return Err("No Apex files found to format.".to_string());
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
            println!("{}", "All Apex files are properly formatted.".green());
            return Ok(());
        }

        eprintln!("{} Formatting differences found:", "error".red().bold());
        for file in changed {
            eprintln!("  - {}", file.display());
        }
        return Err("Formatting check failed".to_string());
    }

    if changed.is_empty() {
        println!("{}", "No formatting changes needed.".green());
    } else {
        println!("{} {} file(s).", "Formatted".green().bold(), changed.len());
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

    Err("No file specified and no apex.toml found in current directory.".to_string())
}

fn lint_target(path: Option<&Path>) -> Result<(), String> {
    let file = resolve_default_file(path)?;
    let source = fs::read_to_string(&file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;
    let result = lint::lint_source(&source, false)
        .map_err(|e| format!("{} in '{}': {}", "error".red().bold(), file.display(), e))?;

    if result.findings.is_empty() {
        println!("{}", "No lint findings.".green());
        return Ok(());
    }

    eprintln!(
        "{} Lint findings in {}:",
        "warning".yellow().bold(),
        file.display()
    );
    for finding in result.findings {
        eprintln!("  {}", finding.format());
    }
    Err("Lint failed".to_string())
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
        println!("{}", "No safe fixes needed.".green());
        return Ok(());
    }

    fs::write(&file, formatted_source)
        .map_err(|e| format!("{}: Failed to write file: {}", "error".red().bold(), e))?;
    println!("{} {}", "Fixed".green().bold(), file.display());
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
            run_project(&[], false, true)?;
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

    println!("{}", "Benchmark Summary".cyan().bold());
    println!("  Runs: {}", samples_ms.len());
    println!("  Min:  {:.3} ms", min);
    println!("  Mean: {:.3} ms", mean);
    println!("  Max:  {:.3} ms", max);
    Ok(())
}

fn profile_target(file: Option<&Path>) -> Result<(), String> {
    let start = Instant::now();
    if let Some(file) = file {
        run_single_file(file, &[], false, true)?;
    } else {
        run_project(&[], false, true)?;
    }
    let elapsed = start.elapsed();

    println!("{}", "Profile Summary".cyan().bold());
    println!("  Wall time: {:.3} ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  CPU/RSS profiling is not wired yet; this command currently reports execution time."
    );
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
        println!("{}", "No test files found.".yellow());
        println!("Create test files with functions marked with @Test attribute.");
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
                "{}: No tests matching filter '{}'",
                test_file.display(),
                filter.unwrap_or("")
            );
            continue;
        }

        let filtered_total_tests: usize = filtered_suites.iter().map(|s| s.tests.len()).sum();
        let filtered_ignored_tests: usize = filtered_suites
            .iter()
            .map(|s| s.tests.iter().filter(|t| t.ignore_reason.is_some()).count())
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
        println!("{}", "No tests found in any test files.".yellow());
        println!("Mark functions with @Test to create tests:");
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
    println!("\n{}", "Running tests...".cyan().bold());
    println!();

    let output = Command::new(exe_path)
        .output()
        .map_err(|e| format!("Failed to run tests: {}", e))?;

    // Print output
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    // Check exit code
    if !output.status.success() {
        return Err("Tests failed".to_string());
    }

    Ok(())
}

fn bindgen_header(header: &Path, output: Option<&Path>) -> Result<(), String> {
    let count = bindgen::generate_bindings(header, output)?;
    if let Some(out) = output {
        println!(
            "{} Generated {} binding(s) -> {}",
            "OK".green().bold(),
            count,
            out.display()
        );
    } else {
        eprintln!("{} Generated {} binding(s)", "OK".green().bold(), count);
    }
    Ok(())
}
