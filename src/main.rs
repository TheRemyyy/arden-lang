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
#[command(author = "TheRemyyy")]
#[command(version = "1.3.5")]
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
        } => build_project(release, emit_llvm, !no_check, false),
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
        Commands::Check { file } => check_command(file.as_deref()),
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

const PARSE_CACHE_SCHEMA: &str = "v4";

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
    api_fingerprint: String,
    semantic_fingerprint: String,
    function_names: Vec<String>,
    class_names: Vec<String>,
    module_names: Vec<String>,
    referenced_symbols: Vec<String>,
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
    match &decl.node {
        Decl::Function(func) => declaration_symbols
            .contains(&func.name)
            .then(|| decl.clone()),
        Decl::Class(class) => declaration_symbols
            .contains(&class.name)
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
    active_file: &Path,
    dependency_closure: Option<&HashSet<PathBuf>>,
    declaration_symbols: Option<&HashSet<String>>,
) -> Program {
    let mut program = Program {
        package: None,
        declarations: Vec::new(),
    };

    for unit in rewritten_files {
        if let Some(closure) = dependency_closure {
            if unit.file != active_file && !closure.contains(&unit.file) {
                continue;
            }
        }
        let source_program = if unit.file == active_file {
            unit.program.clone()
        } else {
            declaration_symbols
                .map(|symbols| filter_codegen_program_by_symbols(&unit.api_program, symbols))
                .unwrap_or_else(|| unit.api_program.clone())
        };
        program.declarations.extend(source_program.declarations);
    }

    program
}

fn semantic_program_for_files(
    rewritten_files: &[RewrittenProjectUnit],
    full_files: &HashSet<PathBuf>,
) -> Program {
    let mut program = Program {
        package: None,
        declarations: Vec::new(),
    };

    for unit in rewritten_files {
        let source_program = if full_files.contains(&unit.file) {
            unit.program.clone()
        } else {
            unit.api_program.clone()
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
    referenced_symbols: Vec<String>,
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
        global_module_map.get(symbol),
        global_module_file_map.get(symbol),
    ) {
        push_owner(owner_ns, owner_file);
    }
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
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &HashMap<String, PathBuf>,
) -> HashSet<String> {
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
                global_module_map,
                global_module_file_map,
            );
        }
    }

    declaration_symbols
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

    let entry: ParsedFileCacheEntry = match serde_json::from_str(&raw) {
        Ok(entry) => entry,
        Err(_) => return Ok(None),
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

const IMPORT_CHECK_CACHE_SCHEMA: &str = "v1";
const DEPENDENCY_GRAPH_CACHE_SCHEMA: &str = "v1";
const SEMANTIC_SUMMARY_CACHE_SCHEMA: &str = "v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImportCheckCacheEntry {
    schema: String,
    compiler_version: String,
    semantic_fingerprint: String,
    rewrite_context_fingerprint: String,
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
    semantic_fingerprint: &str,
    rewrite_context_fingerprint: &str,
) -> Result<bool, String> {
    let path = import_check_cache_path(project_root, file);
    if !path.exists() {
        return Ok(false);
    }

    let raw = fs::read_to_string(&path).map_err(|e| {
        format!(
            "{}: Failed to read import-check cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    let entry: ImportCheckCacheEntry = match serde_json::from_str(&raw) {
        Ok(entry) => entry,
        Err(_) => return Ok(false),
    };

    Ok(entry.schema == IMPORT_CHECK_CACHE_SCHEMA
        && entry.compiler_version == env!("CARGO_PKG_VERSION")
        && entry.semantic_fingerprint == semantic_fingerprint
        && entry.rewrite_context_fingerprint == rewrite_context_fingerprint)
}

fn save_import_check_cache_hit(
    project_root: &Path,
    file: &Path,
    semantic_fingerprint: &str,
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
        semantic_fingerprint: semantic_fingerprint.to_string(),
        rewrite_context_fingerprint: rewrite_context_fingerprint.to_string(),
    };
    let json = serde_json::to_string(&entry).map_err(|e| {
        format!(
            "{}: Failed to serialize import-check cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    fs::write(&path, json).map_err(|e| {
        format!(
            "{}: Failed to write import-check cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })
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
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path).map_err(|e| {
        format!(
            "{}: Failed to read dependency graph cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    let cache: DependencyGraphCache = match serde_json::from_str(&raw) {
        Ok(cache) => cache,
        Err(_) => return Ok(None),
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

    let json = serde_json::to_string(cache).map_err(|e| {
        format!(
            "{}: Failed to serialize dependency graph cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    fs::write(&path, json).map_err(|e| {
        format!(
            "{}: Failed to write dependency graph cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })
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
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path).map_err(|e| {
        format!(
            "{}: Failed to read semantic summary cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    let cache: SemanticSummaryCache = match serde_json::from_str(&raw) {
        Ok(cache) => cache,
        Err(_) => return Ok(None),
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

    let json = serde_json::to_string(cache).map_err(|e| {
        format!(
            "{}: Failed to serialize semantic summary cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    fs::write(&path, json).map_err(|e| {
        format!(
            "{}: Failed to write semantic summary cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })
}

const REWRITE_CACHE_SCHEMA: &str = "v2";

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

fn import_path_owner_file<'a>(
    path: &str,
    global_function_map: &HashMap<String, String>,
    global_function_file_map: &'a HashMap<String, PathBuf>,
    global_class_map: &HashMap<String, String>,
    global_class_file_map: &'a HashMap<String, PathBuf>,
    global_module_map: &HashMap<String, String>,
    global_module_file_map: &'a HashMap<String, PathBuf>,
) -> Option<&'a PathBuf> {
    let (namespace, symbol) = path.rsplit_once('.')?;

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
    global_function_map: &'a HashMap<String, String>,
    global_function_file_map: &'a HashMap<String, PathBuf>,
    namespace_classes: &'a HashMap<String, HashSet<String>>,
    global_class_map: &'a HashMap<String, String>,
    global_class_file_map: &'a HashMap<String, PathBuf>,
    namespace_modules: &'a HashMap<String, HashSet<String>>,
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
    global_module_map: &'a HashMap<String, String>,
    global_module_file_map: &'a HashMap<String, PathBuf>,
}

fn resolve_import_dependency_files(
    import: &ImportDecl,
    ctx: &DependencyResolutionContext<'_>,
) -> HashSet<PathBuf> {
    let mut deps = HashSet::new();

    if import.path.ends_with(".*") {
        if let Some(files) = ctx
            .namespace_files_map
            .get(import.path.trim_end_matches(".*"))
        {
            deps.extend(files.iter().cloned());
        }
        return deps;
    }

    if let Some(files) = ctx.namespace_files_map.get(&import.path) {
        deps.extend(files.iter().cloned());
        return deps;
    }

    if let Some(owner_file) = import_path_owner_file(
        &import.path,
        ctx.global_function_map,
        ctx.global_function_file_map,
        ctx.global_class_map,
        ctx.global_class_file_map,
        ctx.global_module_map,
        ctx.global_module_file_map,
    ) {
        deps.insert(owner_file.clone());
        return deps;
    }

    if let Some((namespace, _)) = import.path.rsplit_once('.') {
        if let Some(files) = ctx.namespace_files_map.get(namespace) {
            deps.extend(files.iter().cloned());
        }
    }

    deps
}

fn build_file_dependency_graph(
    parsed_files: &[ParsedProjectUnit],
    ctx: &DependencyResolutionContext<'_>,
) -> HashMap<PathBuf, HashSet<PathBuf>> {
    let mut graph = HashMap::new();

    for unit in parsed_files {
        let mut deps = HashSet::new();

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
            deps.extend(resolve_import_dependency_files(import, ctx));
        }

        deps.remove(&unit.file);
        graph.insert(unit.file.clone(), deps);
    }

    graph
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

    SemanticSummaryCache {
        schema: SEMANTIC_SUMMARY_CACHE_SCHEMA.to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        files,
        function_effects,
        class_method_effects,
        class_mutating_methods,
    }
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
    let mut relevant_namespaces: HashSet<String> =
        namespace_prefixes(&unit.namespace).into_iter().collect();
    relevant_namespaces.insert(unit.namespace.clone());

    let mut hasher = stable_hasher();
    entry_namespace.hash(&mut hasher);
    unit.namespace.hash(&mut hasher);
    hash_imports(&unit.imports, &mut hasher);

    for import in &unit.imports {
        if import.path.ends_with(".*") {
            let namespace = import.path.trim_end_matches(".*");
            relevant_namespaces.insert(namespace.to_string());
            for prefix in namespace_prefixes(namespace) {
                relevant_namespaces.insert(prefix);
            }
            continue;
        }

        if ctx.namespace_api_fingerprints.contains_key(&import.path) {
            relevant_namespaces.insert(import.path.clone());
            for prefix in namespace_prefixes(&import.path) {
                relevant_namespaces.insert(prefix);
            }
            continue;
        }

        if let Some(owner_file) = import_path_owner_file(
            &import.path,
            ctx.global_function_map,
            ctx.global_function_file_map,
            ctx.global_class_map,
            ctx.global_class_file_map,
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
    semantic_fingerprint: &str,
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

    let entry: RewrittenFileCacheEntry = match serde_json::from_str(&raw) {
        Ok(entry) => entry,
        Err(_) => return Ok(None),
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
    let raw = fs::read_to_string(path).ok()?;
    let cache: LinkManifestCache = serde_json::from_str(&raw).ok()?;
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

    let json = serde_json::to_string(cache).map_err(|e| {
        format!(
            "{}: Failed to serialize link manifest cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    fs::write(&path, json).map_err(|e| {
        format!(
            "{}: Failed to write link manifest cache '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })
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
    project_root: &Path,
    file: &Path,
    semantic_fingerprint: &str,
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
    let meta: ObjectCacheEntry = match serde_json::from_str(&raw) {
        Ok(meta) => meta,
        Err(_) => return Ok(None),
    };

    if meta.schema != OBJECT_CACHE_SCHEMA
        || meta.compiler_version != env!("CARGO_PKG_VERSION")
        || meta.semantic_fingerprint != semantic_fingerprint
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
    semantic_fingerprint: &str,
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
        semantic_fingerprint: semantic_fingerprint.to_string(),
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
    let filename = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown.apex");
    let file_metadata = current_file_metadata_stamp(file)?;
    let cached_entry = load_parsed_file_cache_entry(project_root, file)?;
    let (namespace, program, imports, api_fingerprint, semantic_fingerprint, from_parse_cache) =
        if let Some(cache) = cached_entry.as_ref() {
            if cache.file_metadata == file_metadata {
                (
                    cache.namespace.clone(),
                    cache.program.clone(),
                    cache.imports.clone(),
                    cache.api_fingerprint.clone(),
                    cache.semantic_fingerprint.clone(),
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

                    let cache_entry = ParsedFileCacheEntry {
                        schema: PARSE_CACHE_SCHEMA.to_string(),
                        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
                        file_metadata: file_metadata.clone(),
                        source_fingerprint: source_fp,
                        api_fingerprint: api_fingerprint.clone(),
                        semantic_fingerprint: semantic_fingerprint.clone(),
                        namespace: namespace.clone(),
                        program: program.clone(),
                        imports: imports.clone(),
                    };
                    save_parsed_file_cache(project_root, file, &cache_entry)?;

                    (
                        namespace,
                        program,
                        imports,
                        api_fingerprint,
                        semantic_fingerprint,
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

            let cache_entry = ParsedFileCacheEntry {
                schema: PARSE_CACHE_SCHEMA.to_string(),
                compiler_version: env!("CARGO_PKG_VERSION").to_string(),
                file_metadata,
                source_fingerprint: source_fp,
                api_fingerprint: api_fingerprint.clone(),
                semantic_fingerprint: semantic_fingerprint.clone(),
                namespace: namespace.clone(),
                program: program.clone(),
                imports: imports.clone(),
            };
            save_parsed_file_cache(project_root, file, &cache_entry)?;

            (
                namespace,
                program,
                imports,
                api_fingerprint,
                semantic_fingerprint,
                false,
            )
        };

    let mut function_names = Vec::new();
    let mut class_names = Vec::new();
    let mut module_names = Vec::new();
    let mut referenced_symbols = HashSet::new();

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
            ast::Type::Option(inner)
            | ast::Type::List(inner)
            | ast::Type::Set(inner)
            | ast::Type::Ref(inner)
            | ast::Type::MutRef(inner)
            | ast::Type::Box(inner)
            | ast::Type::Rc(inner)
            | ast::Type::Arc(inner)
            | ast::Type::Ptr(inner)
            | ast::Type::Task(inner)
            | ast::Type::Range(inner) => collect_type_refs(inner, out),
            ast::Type::Result(ok, err) | ast::Type::Map(ok, err) => {
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

    fn collect_expr_refs(expr: &Expr, out: &mut HashSet<String>) {
        match expr {
            Expr::Literal(_) | Expr::This => {}
            Expr::Ident(_) => {}
            Expr::Binary { left, right, .. } => {
                collect_expr_refs(&left.node, out);
                collect_expr_refs(&right.node, out);
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => collect_expr_refs(&expr.node, out),
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
                }
                collect_expr_refs(&callee.node, out);
                for arg in args {
                    collect_expr_refs(&arg.node, out);
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
                }
                collect_expr_refs(&object.node, out);
            }
            Expr::Index { object, index } => {
                collect_expr_refs(&object.node, out);
                collect_expr_refs(&index.node, out);
            }
            Expr::Construct { ty, args } => {
                out.insert(ty.clone());
                for arg in args {
                    collect_expr_refs(&arg.node, out);
                }
            }
            Expr::Lambda { params, body } => {
                for param in params {
                    collect_type_refs(&param.ty, out);
                }
                collect_expr_refs(&body.node, out);
            }
            Expr::Match { expr, arms } => {
                collect_expr_refs(&expr.node, out);
                for arm in arms {
                    collect_pattern_refs(&arm.pattern, out);
                    collect_block_refs(&arm.body, out);
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let ast::StringPart::Expr(expr) = part {
                        collect_expr_refs(&expr.node, out);
                    }
                }
            }
            Expr::AsyncBlock(body) | Expr::Block(body) => collect_block_refs(body, out),
            Expr::Require { condition, message } => {
                collect_expr_refs(&condition.node, out);
                if let Some(message) = message {
                    collect_expr_refs(&message.node, out);
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(start) = start {
                    collect_expr_refs(&start.node, out);
                }
                if let Some(end) = end {
                    collect_expr_refs(&end.node, out);
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_expr_refs(&condition.node, out);
                collect_block_refs(then_branch, out);
                if let Some(else_branch) = else_branch {
                    collect_block_refs(else_branch, out);
                }
            }
        }
    }

    fn collect_pattern_refs(pattern: &Pattern, out: &mut HashSet<String>) {
        if let Pattern::Variant(name, _) = pattern {
            out.insert(name.clone());
        }
    }

    fn collect_stmt_refs(stmt: &Stmt, out: &mut HashSet<String>) {
        match stmt {
            Stmt::Let { ty, value, .. } => {
                collect_type_refs(ty, out);
                collect_expr_refs(&value.node, out);
            }
            Stmt::Assign { target, value } => {
                collect_expr_refs(&target.node, out);
                collect_expr_refs(&value.node, out);
            }
            Stmt::Expr(expr) => collect_expr_refs(&expr.node, out),
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    collect_expr_refs(&expr.node, out);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_expr_refs(&condition.node, out);
                collect_block_refs(then_block, out);
                if let Some(else_block) = else_block {
                    collect_block_refs(else_block, out);
                }
            }
            Stmt::While { condition, body } => {
                collect_expr_refs(&condition.node, out);
                collect_block_refs(body, out);
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
                collect_expr_refs(&iterable.node, out);
                collect_block_refs(body, out);
            }
            Stmt::Match { expr, arms } => {
                collect_expr_refs(&expr.node, out);
                for arm in arms {
                    collect_pattern_refs(&arm.pattern, out);
                    collect_block_refs(&arm.body, out);
                }
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn collect_block_refs(block: &Block, out: &mut HashSet<String>) {
        for stmt in block {
            collect_stmt_refs(&stmt.node, out);
        }
    }

    fn collect_decl_refs(decl: &Decl, out: &mut HashSet<String>) {
        match decl {
            Decl::Function(func) => {
                for param in &func.params {
                    collect_type_refs(&param.ty, out);
                }
                collect_type_refs(&func.return_type, out);
                collect_block_refs(&func.body, out);
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
                    collect_block_refs(&ctor.body, out);
                }
                if let Some(dtor) = &class.destructor {
                    collect_block_refs(&dtor.body, out);
                }
                for method in &class.methods {
                    for param in &method.params {
                        collect_type_refs(&param.ty, out);
                    }
                    collect_type_refs(&method.return_type, out);
                    collect_block_refs(&method.body, out);
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
                        collect_block_refs(body, out);
                    }
                }
            }
            Decl::Module(module) => {
                out.insert(module.name.clone());
                for inner in &module.declarations {
                    collect_decl_refs(&inner.node, out);
                }
            }
            Decl::Import(_) => {}
        }
    }

    for decl in &program.declarations {
        match &decl.node {
            Decl::Function(_) => collect_function_names(&decl.node, None, &mut function_names),
            Decl::Module(module) => {
                module_names.push(module.name.clone());
                collect_function_names(&decl.node, None, &mut function_names);
            }
            Decl::Class(class) => class_names.push(class.name.clone()),
            _ => {}
        }
        collect_decl_refs(&decl.node, &mut referenced_symbols);
    }
    let projected_program = api_projection_program(&program);
    let mut api_referenced_symbols = HashSet::new();
    for decl in &projected_program.declarations {
        collect_decl_refs(&decl.node, &mut api_referenced_symbols);
    }

    Ok(ParsedProjectUnit {
        file: file.to_path_buf(),
        namespace,
        program,
        imports,
        api_fingerprint,
        semantic_fingerprint,
        function_names,
        class_names,
        module_names,
        referenced_symbols: referenced_symbols.into_iter().collect(),
        api_referenced_symbols: api_referenced_symbols.into_iter().collect(),
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
fn build_project(
    _release: bool,
    emit_llvm: bool,
    do_check: bool,
    check_only: bool,
) -> Result<(), String> {
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
                    "{} {} ({})",
                    "Up to date".green().bold(),
                    config.name.cyan(),
                    "build cache".dimmed()
                );
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
    let mut global_module_map: HashMap<String, String> = HashMap::new(); // module_name -> namespace
    let mut global_module_file_map: HashMap<String, PathBuf> = HashMap::new(); // module_name -> owner file
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
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
    };
    let file_dependency_graph =
        build_file_dependency_graph(&parsed_files, &dependency_resolution_ctx);
    let reverse_file_dependency_graph = build_reverse_dependency_graph(&file_dependency_graph);
    let current_dependency_graph_cache =
        dependency_graph_cache_from_state(&parsed_files, &file_dependency_graph);

    let previous_semantic_summary = load_semantic_summary_cache(&project_root)?;
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

    let semantic_fingerprint =
        compute_semantic_project_fingerprint(&config, &parsed_files, emit_llvm, do_check);
    if !check_only {
        if let Some(cached) = load_semantic_cached_fingerprint(&project_root) {
            if cached == semantic_fingerprint
                && project_build_artifact_exists(&output_path, emit_llvm)
            {
                println!(
                    "{} {} ({})",
                    "Up to date".green().bold(),
                    config.name.cyan(),
                    "semantic cache".dimmed()
                );
                save_cached_fingerprint(&project_root, &fingerprint)?;
                return Ok(());
            }
        }
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
        global_function_map: &global_function_map,
        global_function_file_map: &global_function_file_map,
        namespace_classes: &namespace_class_map,
        global_class_map: &global_class_map,
        global_class_file_map: &global_class_file_map,
        namespace_modules: &namespace_module_map,
        global_module_map: &global_module_map,
        global_module_file_map: &global_module_file_map,
        namespace_api_fingerprints: &namespace_api_fingerprints,
        file_api_fingerprints: &file_api_fingerprints,
    };

    // Phase 2: Check imports for each file
    if do_check {
        println!("{} Checking imports...", "→".cyan());
        let shared_function_map = Arc::new(global_function_map.clone());
        let import_check_cache_hits = std::sync::atomic::AtomicUsize::new(0);

        let import_results: Vec<Result<(), String>> = parsed_files
            .par_iter()
            .map(|unit| {
                let rewrite_context_fingerprint = compute_rewrite_context_fingerprint_for_unit(
                    unit,
                    &entry_namespace,
                    &rewrite_fingerprint_ctx,
                );
                if load_import_check_cache_hit(
                    &project_root,
                    &unit.file,
                    &unit.semantic_fingerprint,
                    &rewrite_context_fingerprint,
                )? {
                    import_check_cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return Ok(());
                }

                let mut checker = ImportChecker::new(
                    Arc::clone(&shared_function_map),
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
                    let mut rendered =
                        format!("{} Import errors in {}:\n", "error".red().bold(), filename);
                    for err in errors {
                        rendered.push_str(&format!("  → {}\n", err.format()));
                    }
                    return Err(rendered);
                }
                save_import_check_cache_hit(
                    &project_root,
                    &unit.file,
                    &unit.semantic_fingerprint,
                    &rewrite_context_fingerprint,
                )?;
                Ok(())
            })
            .collect();

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
    }

    // Phase 3: Build combined AST with deterministic namespace mangling.
    let rewritten_results: Vec<Result<RewrittenProjectUnit, String>> = parsed_files
        .par_iter()
        .map(|unit| {
            let rewrite_context_fingerprint = compute_rewrite_context_fingerprint_for_unit(
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

        let semantic_program = semantic_program_for_files(&rewritten_files, &semantic_full_files);
        let current_semantic_fingerprints: HashMap<PathBuf, String> = parsed_files
            .iter()
            .map(|unit| (unit.file.clone(), unit.semantic_fingerprint.clone()))
            .collect();
        let (seeded_function_effects, seeded_class_method_effects, seeded_class_mutating_methods) =
            previous_semantic_summary
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

        let mut type_checker = TypeChecker::new(String::new());
        if let Err(errors) = type_checker.check_with_effect_seeds(
            &semantic_program,
            &seeded_function_effects,
            &seeded_class_method_effects,
        ) {
            let mut rendered = String::new();
            for error in errors {
                rendered.push_str(&format!("\x1b[1;31merror\x1b[0m: {}\n", error.message));
            }
            return Err(rendered);
        }

        let mut borrow_checker = BorrowChecker::new();
        if let Err(errors) = borrow_checker
            .check_with_mutating_method_seeds(&semantic_program, &seeded_class_mutating_methods)
        {
            let mut rendered = String::new();
            for error in errors {
                rendered.push_str(&format!(
                    "\x1b[1;31merror[E0505]\x1b[0m: {}\n",
                    error.message
                ));
            }
            return Err(rendered);
        }
        let (function_effects, class_method_effects) = type_checker.export_effect_summary();

        save_semantic_summary_cache(
            &project_root,
            &semantic_summary_cache_from_state(
                &parsed_files,
                function_effects,
                class_method_effects,
                borrow_checker.export_class_mutating_method_summary(),
            ),
        )?;
    }

    if check_only {
        println!("{} {}", "Checked".green().bold(), config.name.cyan());
        println!("{}", "No errors found.".green());
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
        compile_program_ast(
            &combined_program,
            &entry_path,
            &output_path,
            emit_llvm,
            &link,
        )?;
    } else {
        let object_build_fingerprint = compute_object_build_fingerprint(&link);
        let previous_link_manifest = load_link_manifest_cache(&project_root);
        let codegen_reference_metadata: HashMap<PathBuf, CodegenReferenceMetadata> = parsed_files
            .iter()
            .map(|unit| {
                (
                    unit.file.clone(),
                    CodegenReferenceMetadata {
                        referenced_symbols: unit.referenced_symbols.clone(),
                        api_referenced_symbols: unit.api_referenced_symbols.clone(),
                    },
                )
            })
            .collect();
        let mut object_paths: Vec<Option<PathBuf>> = vec![None; rewritten_files.len()];
        let mut object_cache_hits: usize = 0;
        let object_candidate_count = rewritten_files
            .iter()
            .filter(|unit| !unit.active_symbols.is_empty())
            .count();
        let mut cache_misses: Vec<(usize, &RewrittenProjectUnit)> = Vec::new();

        for (index, unit) in rewritten_files.iter().enumerate() {
            if unit.active_symbols.is_empty() {
                continue;
            }

            if let Some(cached_obj) = load_object_cache_hit(
                &project_root,
                &unit.file,
                &unit.semantic_fingerprint,
                &unit.rewrite_context_fingerprint,
                &object_build_fingerprint,
            )? {
                object_paths[index] = Some(cached_obj);
                object_cache_hits += 1;
                continue;
            }
            cache_misses.push((index, unit));
        }

        let compiled_results: Vec<(usize, PathBuf)> = cache_misses
            .par_iter()
            .map(|(index, unit)| {
                let obj_path = object_cache_object_path(&project_root, &unit.file);
                let dependency_closure =
                    transitive_dependencies(&file_dependency_graph, &unit.file);
                let declaration_symbols = declaration_symbols_for_unit(
                    &unit.file,
                    &unit.active_symbols,
                    &file_dependency_graph,
                    &codegen_reference_metadata,
                    &entry_namespace,
                    &global_function_map,
                    &global_function_file_map,
                    &global_class_map,
                    &global_class_file_map,
                    &global_module_map,
                    &global_module_file_map,
                );
                let codegen_program = codegen_program_for_unit(
                    &rewritten_files,
                    &unit.file,
                    Some(&dependency_closure),
                    Some(&declaration_symbols),
                );
                compile_program_ast_to_object_filtered(
                    &codegen_program,
                    &unit.file,
                    &obj_path,
                    &link,
                    &unit.active_symbols,
                    &declaration_symbols,
                )?;
                save_object_cache_meta(
                    &project_root,
                    &unit.file,
                    &unit.semantic_fingerprint,
                    &unit.rewrite_context_fingerprint,
                    &object_build_fingerprint,
                )?;
                Ok::<(usize, PathBuf), String>((*index, obj_path))
            })
            .collect::<Result<Vec<_>, String>>()?;

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
        } else {
            link_objects(&link_inputs, &output_path, &link)?;
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
fn run_project(args: &[String], release: bool, do_check: bool) -> Result<(), String> {
    build_project(release, false, do_check, false)?;

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

fn check_command(file: Option<&Path>) -> Result<(), String> {
    if file.is_none() {
        if let Some(cwd_project_root) = find_project_root(&current_dir_checked()?) {
            let _ = cwd_project_root;
            return build_project(false, false, true, true);
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
        let function_namespaces = import_check::extract_function_namespaces(&program, &namespace);
        let mut import_checker = ImportChecker::new(
            Arc::new(function_namespaces),
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
    Mold,
    Lld,
}

impl LinkerFlavor {
    fn clang_fuse_ld(self) -> &'static str {
        match self {
            LinkerFlavor::Mold => "mold",
            LinkerFlavor::Lld => "lld",
        }
    }

    fn cache_key(self) -> &'static str {
        self.clang_fuse_ld()
    }
}

fn detect_linker_flavor() -> Result<LinkerFlavor, String> {
    #[cfg(not(windows))]
    if shutil_which("mold") || shutil_which("ld.mold") {
        return Ok(LinkerFlavor::Mold);
    }
    if shutil_which("ld.lld") || shutil_which("lld") {
        return Ok(LinkerFlavor::Lld);
    }
    Err(format!(
        "{}: No supported linker found in PATH. Install mold (preferred) or lld and retry.",
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
        cmd.arg("-llegacy_stdio_definitions");

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
            cmd.arg("-llegacy_stdio_definitions");

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
    let mut import_checker = ImportChecker::new(
        Arc::new(function_namespaces),
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

#[cfg(test)]
mod tests {
    use super::{
        api_program_fingerprint, build_file_dependency_graph, build_reverse_dependency_graph,
        compute_link_fingerprint, compute_rewrite_context_fingerprint_for_unit,
        escape_response_file_arg, parse_project_unit, semantic_program_fingerprint,
        should_skip_final_link, transitive_dependents, DependencyResolutionContext, LinkConfig,
        LinkManifestCache, OutputKind, ParsedProjectUnit, RewriteFingerprintContext,
        LINK_MANIFEST_CACHE_SCHEMA,
    };
    use crate::ast::{ImportDecl, Program};
    use crate::parser::Parser;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::path::PathBuf;
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
            function_names: Vec::new(),
            class_names: Vec::new(),
            module_names: Vec::new(),
            referenced_symbols: Vec::new(),
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
        let namespace_classes = HashMap::new();
        let global_class_map = HashMap::new();
        let global_class_file_map = HashMap::new();
        let namespace_modules = HashMap::new();
        let global_module_map = HashMap::new();
        let global_module_file_map = HashMap::new();
        let namespace_api_fingerprints = HashMap::from([("lib".to_string(), "ns-v1".to_string())]);
        let file_api_fingerprints = HashMap::from([
            (PathBuf::from("src/lib_foo.apex"), "file-foo-v1".to_string()),
            (PathBuf::from("src/lib_bar.apex"), "file-bar-v1".to_string()),
        ]);
        let ctx_a = RewriteFingerprintContext {
            namespace_functions: &namespace_functions,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            namespace_modules: &namespace_modules,
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
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            namespace_modules: &namespace_modules,
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
        let namespace_classes = HashMap::new();
        let global_class_map = HashMap::new();
        let global_class_file_map = HashMap::new();
        let namespace_modules = HashMap::new();
        let global_module_map = HashMap::new();
        let global_module_file_map = HashMap::new();
        let namespace_api_fingerprints_a =
            HashMap::from([("lib".to_string(), "ns-v1".to_string())]);
        let ctx_a = RewriteFingerprintContext {
            namespace_functions: &namespace_functions,
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            namespace_modules: &namespace_modules,
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
            global_function_map: &global_function_map,
            global_function_file_map: &global_function_file_map,
            namespace_classes: &namespace_classes,
            global_class_map: &global_class_map,
            global_class_file_map: &global_class_file_map,
            namespace_modules: &namespace_modules,
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
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
        };
        let graph = build_file_dependency_graph(&parsed_files, &ctx);

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
            global_module_map: &global_module_map,
            global_module_file_map: &global_module_file_map,
        };

        let graph = build_file_dependency_graph(&parsed_files, &ctx);

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
}
