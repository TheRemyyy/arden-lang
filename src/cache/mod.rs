use crate::ast::{Decl, ImportDecl, Program, Spanned};
use crate::dependency::*;
use crate::formatter;
use crate::linker::*;
use crate::project::ProjectConfig;
use crate::typeck::{ClassMethodEffectsSummary, FunctionEffectsSummary};
use colored::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use std::time::UNIX_EPOCH;
use twox_hash::XxHash64;
pub(crate) fn project_cache_file(project_root: &Path) -> PathBuf {
    project_root.join(".ardencache").join("build_fingerprint")
}

pub(crate) fn semantic_project_cache_file(project_root: &Path) -> PathBuf {
    project_root
        .join(".ardencache")
        .join("semantic_build_fingerprint")
}

pub(crate) fn stable_hasher() -> XxHash64 {
    XxHash64::with_seed(0)
}

pub(crate) fn read_cache_blob_raw(path: &Path, label: &str) -> Result<Option<Vec<u8>>, String> {
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

pub(crate) fn read_cache_blob<T: DeserializeOwned>(
    path: &Path,
    label: &str,
) -> Result<Option<T>, String> {
    let Some(raw) = read_cache_blob_raw(path, label)? else {
        return Ok(None);
    };
    let value = match bincode::deserialize(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "{}: Ignoring invalid {} '{}': {}",
                "warning".yellow().bold(),
                label,
                path.display(),
                error
            );
            return Ok(None);
        }
    };
    Ok(Some(value))
}

pub(crate) fn read_cache_blob_with_timing<T: DeserializeOwned>(
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
    let value = match bincode::deserialize(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "{}: Ignoring invalid {} '{}': {}",
                "warning".yellow().bold(),
                label,
                path.display(),
                error
            );
            totals
                .load_ns
                .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
            totals.bytes_read.fetch_add(byte_len, Ordering::Relaxed);
            totals.load_count.fetch_add(1, Ordering::Relaxed);
            return Ok(None);
        }
    };
    totals
        .load_ns
        .fetch_add(elapsed_nanos_u64(started_at), Ordering::Relaxed);
    totals.bytes_read.fetch_add(byte_len, Ordering::Relaxed);
    totals.load_count.fetch_add(1, Ordering::Relaxed);
    Ok(Some(value))
}

pub(crate) fn write_cache_blob<T: Serialize>(
    path: &Path,
    label: &str,
    value: &T,
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

pub(crate) fn write_cache_blob_with_timing<T: Serialize>(
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

pub(crate) fn project_build_artifact_exists(output_path: &Path, emit_llvm: bool) -> bool {
    if emit_llvm {
        output_path.with_extension("ll").exists()
    } else {
        output_path.exists()
    }
}

pub(crate) fn ensure_output_parent_dir(output_path: &Path) -> Result<(), String> {
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

pub(crate) fn compute_project_fingerprint(
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

pub(crate) fn load_cached_fingerprint(project_root: &Path) -> Result<Option<String>, String> {
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

pub(crate) fn load_semantic_cached_fingerprint(
    project_root: &Path,
) -> Result<Option<String>, String> {
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

pub(crate) fn save_cached_fingerprint(
    project_root: &Path,
    fingerprint: &str,
) -> Result<(), String> {
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

pub(crate) fn save_semantic_cached_fingerprint(
    project_root: &Path,
    fingerprint: &str,
) -> Result<(), String> {
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

pub(crate) const PARSE_CACHE_SCHEMA: &str = "v9";
pub(crate) const DEPENDENCY_GRAPH_CACHE_SCHEMA: &str = "v3";
pub(crate) const SEMANTIC_SUMMARY_CACHE_SCHEMA: &str = "v2";
pub(crate) const TYPECHECK_SUMMARY_CACHE_SCHEMA: &str = "v4";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FileMetadataStamp {
    pub(crate) len: u64,
    pub(crate) modified_secs: u64,
    pub(crate) modified_nanos: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ParsedFileCacheEntry {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) file_metadata: FileMetadataStamp,
    pub(crate) source_fingerprint: String,
    pub(crate) api_fingerprint: String,
    pub(crate) semantic_fingerprint: String,
    pub(crate) import_check_fingerprint: String,
    pub(crate) namespace: String,
    pub(crate) program: Program,
    pub(crate) imports: Vec<ImportDecl>,
    pub(crate) function_names: Vec<String>,
    pub(crate) class_names: Vec<String>,
    #[serde(default)]
    pub(crate) interface_names: Vec<String>,
    pub(crate) enum_names: Vec<String>,
    pub(crate) module_names: Vec<String>,
    pub(crate) referenced_symbols: Vec<String>,
    pub(crate) qualified_symbol_refs: Vec<Vec<String>>,
    pub(crate) api_referenced_symbols: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedProjectUnit {
    pub(crate) file: PathBuf,
    pub(crate) namespace: String,
    pub(crate) program: Program,
    pub(crate) imports: Vec<ImportDecl>,
    pub(crate) api_fingerprint: String,
    pub(crate) semantic_fingerprint: String,
    pub(crate) import_check_fingerprint: String,
    pub(crate) function_names: Vec<String>,
    pub(crate) class_names: Vec<String>,
    pub(crate) interface_names: Vec<String>,
    pub(crate) enum_names: Vec<String>,
    pub(crate) module_names: Vec<String>,
    pub(crate) referenced_symbols: Vec<String>,
    pub(crate) qualified_symbol_refs: Vec<Vec<String>>,
    pub(crate) api_referenced_symbols: Vec<String>,
    pub(crate) from_parse_cache: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RewrittenProjectUnit {
    pub(crate) file: PathBuf,
    pub(crate) program: Program,
    pub(crate) api_program: Program,
    pub(crate) specialization_projection: Program,
    pub(crate) semantic_fingerprint: String,
    pub(crate) rewrite_context_fingerprint: String,
    pub(crate) active_symbols: HashSet<String>,
    pub(crate) has_specialization_demand: bool,
    pub(crate) from_rewrite_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DependencyGraphCache {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) entry_namespace: String,
    pub(crate) files: Vec<DependencyGraphFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DependencyGraphFileEntry {
    pub(crate) file: PathBuf,
    pub(crate) semantic_fingerprint: String,
    pub(crate) api_fingerprint: String,
    pub(crate) direct_dependencies: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SemanticSummaryCache {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) files: Vec<SemanticSummaryFileEntry>,
    pub(crate) components: Vec<SemanticSummaryComponentEntry>,
    pub(crate) function_effects: HashMap<String, Vec<String>>,
    pub(crate) class_method_effects: HashMap<String, HashMap<String, Vec<String>>>,
    pub(crate) class_mutating_methods: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SemanticSummaryFileEntry {
    pub(crate) file: PathBuf,
    pub(crate) semantic_fingerprint: String,
    pub(crate) function_names: Vec<String>,
    pub(crate) class_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SemanticSummaryComponentEntry {
    pub(crate) component_fingerprint: String,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) function_names: Vec<String>,
    pub(crate) class_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TypecheckSummaryCache {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) files: Vec<TypecheckSummaryFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TypecheckSummaryFileEntry {
    pub(crate) file: PathBuf,
    pub(crate) semantic_fingerprint: String,
    pub(crate) component_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolLookupResolution {
    pub(crate) owner_namespace: String,
    pub(crate) symbol_name: String,
    pub(crate) owner_file: PathBuf,
}

pub(crate) type SharedSymbolLookupResolution = Arc<SymbolLookupResolution>;
pub(crate) type ExactSymbolLookup = HashMap<String, Option<SharedSymbolLookupResolution>>;
pub(crate) type WildcardMemberLookup =
    HashMap<String, HashMap<String, Option<SharedSymbolLookupResolution>>>;

#[derive(Debug, Clone)]
pub(crate) struct ProjectSymbolLookup {
    pub(crate) exact: ExactSymbolLookup,
    pub(crate) wildcard_members: WildcardMemberLookup,
}

pub(crate) struct BuildTimingPhase {
    pub(crate) label: String,
    pub(crate) ms: f64,
    pub(crate) counters: Vec<(String, usize)>,
}

pub(crate) struct BuildTimings {
    pub(crate) enabled: bool,
    pub(crate) started_at: Instant,
    pub(crate) phases: Vec<BuildTimingPhase>,
}

impl BuildTimings {
    fn format_seconds(ms: f64) -> String {
        format!("{:.6} s", ms / 1000.0)
    }

    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            started_at: Instant::now(),
            phases: Vec::new(),
        }
    }

    pub(crate) fn measure<T, E, F>(&mut self, label: &str, f: F) -> Result<T, E>
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

    pub(crate) fn measure_value<T, F>(&mut self, label: &str, f: F) -> T
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

    pub(crate) fn measure_step<T, F>(&mut self, label: &str, f: F) -> T
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

    pub(crate) fn record_counts(&mut self, label: &str, counters: &[(&str, usize)]) {
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

    pub(crate) fn record_duration_ns(&mut self, label: &str, nanos: u64) {
        if !self.enabled {
            return;
        }

        self.phases.push(BuildTimingPhase {
            label: label.to_string(),
            ms: nanos as f64 / 1_000_000.0,
            counters: Vec::new(),
        });
    }

    pub(crate) fn print(&self) {
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
            println!(
                "  {:<28} {:>10}{}",
                phase.label,
                Self::format_seconds(phase.ms),
                counters
            );
        }
        println!(
            "  {:<28} {:>10}",
            "total",
            Self::format_seconds(self.started_at.elapsed().as_secs_f64() * 1000.0)
        );
    }
}

pub(crate) fn elapsed_nanos_u64(started_at: Instant) -> u64 {
    started_at.elapsed().as_nanos() as u64
}

#[derive(Default)]
pub(crate) struct DependencyGraphTimingTotals {
    pub(crate) cache_validation_ns: AtomicU64,
    pub(crate) direct_symbol_refs_ns: AtomicU64,
    pub(crate) import_exact_ns: AtomicU64,
    pub(crate) import_wildcard_ns: AtomicU64,
    pub(crate) import_namespace_alias_ns: AtomicU64,
    pub(crate) import_parent_namespace_ns: AtomicU64,
    pub(crate) namespace_fallback_ns: AtomicU64,
    pub(crate) owner_lookup_ns: AtomicU64,
    pub(crate) namespace_files_ns: AtomicU64,
    pub(crate) files_reused: AtomicUsize,
    pub(crate) files_rebuilt: AtomicUsize,
    pub(crate) direct_symbol_ref_count: AtomicUsize,
    pub(crate) import_exact_count: AtomicUsize,
    pub(crate) import_wildcard_count: AtomicUsize,
    pub(crate) import_namespace_alias_count: AtomicUsize,
    pub(crate) import_parent_namespace_count: AtomicUsize,
    pub(crate) namespace_fallback_count: AtomicUsize,
    pub(crate) qualified_ref_count: AtomicUsize,
}

#[derive(Default)]
pub(crate) struct RewriteFingerprintTimingTotals {
    pub(crate) local_symbol_refs_ns: AtomicU64,
    pub(crate) wildcard_imports_ns: AtomicU64,
    pub(crate) namespace_alias_imports_ns: AtomicU64,
    pub(crate) exact_imports_ns: AtomicU64,
    pub(crate) relevant_namespace_prefixes_ns: AtomicU64,
    pub(crate) namespace_hashing_ns: AtomicU64,
    pub(crate) local_symbol_ref_count: AtomicUsize,
    pub(crate) wildcard_import_count: AtomicUsize,
    pub(crate) namespace_alias_import_count: AtomicUsize,
    pub(crate) exact_import_count: AtomicUsize,
    pub(crate) prefix_expand_count: AtomicUsize,
}

#[derive(Default)]
pub(crate) struct DeclarationClosureTimingTotals {
    pub(crate) closure_seed_ns: AtomicU64,
    pub(crate) metadata_lookup_ns: AtomicU64,
    pub(crate) wildcard_imports_ns: AtomicU64,
    pub(crate) exact_imports_ns: AtomicU64,
    pub(crate) qualified_refs_ns: AtomicU64,
    pub(crate) reference_symbols_ns: AtomicU64,
    pub(crate) visited_file_count: AtomicUsize,
    pub(crate) wildcard_import_count: AtomicUsize,
    pub(crate) exact_import_count: AtomicUsize,
    pub(crate) qualified_ref_count: AtomicUsize,
    pub(crate) reference_symbol_count: AtomicUsize,
}

#[derive(Default)]
pub(crate) struct ObjectEmitTimingTotals {
    pub(crate) context_create_ns: AtomicU64,
    pub(crate) codegen_new_ns: AtomicU64,
    pub(crate) compile_filtered_ns: AtomicU64,
    pub(crate) object_dir_setup_ns: AtomicU64,
    pub(crate) write_object_ns: AtomicU64,
    pub(crate) active_symbol_count: AtomicUsize,
    pub(crate) declaration_symbol_count: AtomicUsize,
    pub(crate) program_decl_count: AtomicUsize,
}

#[derive(Default)]
pub(crate) struct ImportCheckTimingTotals {
    pub(crate) rewrite_context_fingerprint_ns: AtomicU64,
    pub(crate) cache_lookup_ns: AtomicU64,
    pub(crate) checker_init_ns: AtomicU64,
    pub(crate) checker_run_ns: AtomicU64,
    pub(crate) cache_save_ns: AtomicU64,
}

#[derive(Default)]
pub(crate) struct PipelineRewriteTimingTotals {
    pub(crate) rewrite_context_fingerprint_ns: AtomicU64,
    pub(crate) cache_lookup_ns: AtomicU64,
    pub(crate) rewrite_program_ns: AtomicU64,
    pub(crate) cache_save_ns: AtomicU64,
    pub(crate) active_symbols_ns: AtomicU64,
    pub(crate) api_projection_ns: AtomicU64,
    pub(crate) specialization_projection_ns: AtomicU64,
    pub(crate) specialization_demand_ns: AtomicU64,
}

#[derive(Default)]
pub(crate) struct ObjectCodegenTimingTotals {
    pub(crate) declaration_closure_ns: AtomicU64,
    pub(crate) codegen_program_ns: AtomicU64,
    pub(crate) closure_body_symbols_ns: AtomicU64,
    pub(crate) llvm_emit_ns: AtomicU64,
    pub(crate) cache_save_ns: AtomicU64,
}

#[derive(Default)]
pub(crate) struct CacheIoTimingTotals {
    pub(crate) load_ns: AtomicU64,
    pub(crate) save_ns: AtomicU64,
    pub(crate) bytes_read: AtomicU64,
    pub(crate) bytes_written: AtomicU64,
    pub(crate) load_count: AtomicUsize,
    pub(crate) save_count: AtomicUsize,
}

pub(crate) static PARSE_CACHE_TIMING_TOTALS: CacheIoTimingTotals = CacheIoTimingTotals {
    load_ns: AtomicU64::new(0),
    save_ns: AtomicU64::new(0),
    bytes_read: AtomicU64::new(0),
    bytes_written: AtomicU64::new(0),
    load_count: AtomicUsize::new(0),
    save_count: AtomicUsize::new(0),
};

pub(crate) static REWRITE_CACHE_TIMING_TOTALS: CacheIoTimingTotals = CacheIoTimingTotals {
    load_ns: AtomicU64::new(0),
    save_ns: AtomicU64::new(0),
    bytes_read: AtomicU64::new(0),
    bytes_written: AtomicU64::new(0),
    load_count: AtomicUsize::new(0),
    save_count: AtomicUsize::new(0),
};

pub(crate) static OBJECT_CACHE_META_TIMING_TOTALS: CacheIoTimingTotals = CacheIoTimingTotals {
    load_ns: AtomicU64::new(0),
    save_ns: AtomicU64::new(0),
    bytes_read: AtomicU64::new(0),
    bytes_written: AtomicU64::new(0),
    load_count: AtomicUsize::new(0),
    save_count: AtomicUsize::new(0),
};

pub(crate) fn reset_cache_io_timing_totals(totals: &CacheIoTimingTotals) {
    totals.load_ns.store(0, Ordering::Relaxed);
    totals.save_ns.store(0, Ordering::Relaxed);
    totals.bytes_read.store(0, Ordering::Relaxed);
    totals.bytes_written.store(0, Ordering::Relaxed);
    totals.load_count.store(0, Ordering::Relaxed);
    totals.save_count.store(0, Ordering::Relaxed);
}

pub(crate) fn parsed_file_cache_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = stable_hasher();
    file.hash(&mut hasher);
    project_root
        .join(".ardencache")
        .join("parsed")
        .join(format!("{:016x}.json", hasher.finish()))
}

pub(crate) fn source_fingerprint(source: &str) -> String {
    let mut hasher = stable_hasher();
    source.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub(crate) fn semantic_program_fingerprint(program: &Program) -> String {
    let canonical = formatter::format_program_canonical(program);
    source_fingerprint(&canonical)
}

pub(crate) fn current_file_metadata_stamp(file: &Path) -> Result<FileMetadataStamp, String> {
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

pub(crate) fn load_parsed_file_cache_entry(
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

pub(crate) fn save_parsed_file_cache(
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

pub(crate) const IMPORT_CHECK_CACHE_SCHEMA: &str = "v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ImportCheckCacheEntry {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) import_check_fingerprint: String,
    pub(crate) rewrite_context_fingerprint: String,
}

pub(crate) fn compute_import_check_fingerprint(
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

pub(crate) fn import_check_cache_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = stable_hasher();
    file.hash(&mut hasher);
    project_root
        .join(".ardencache")
        .join("import_check")
        .join(format!("{:016x}.json", hasher.finish()))
}

pub(crate) fn load_import_check_cache_hit(
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

pub(crate) fn save_import_check_cache_hit(
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

pub(crate) fn dependency_graph_cache_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".ardencache")
        .join("dependency_graph")
        .join("latest.json")
}

pub(crate) fn load_dependency_graph_cache(
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

pub(crate) fn save_dependency_graph_cache(
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

pub(crate) fn semantic_summary_cache_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".ardencache")
        .join("semantic_summary")
        .join("latest.json")
}

pub(crate) fn load_semantic_summary_cache(
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

pub(crate) fn save_semantic_summary_cache(
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

pub(crate) fn typecheck_summary_cache_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".ardencache")
        .join("typecheck_summary")
        .join("latest.json")
}

pub(crate) fn load_typecheck_summary_cache(
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

pub(crate) fn save_typecheck_summary_cache(
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

pub(crate) const REWRITE_CACHE_SCHEMA: &str = "v9";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RewrittenFileCacheEntry {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) semantic_fingerprint: String,
    pub(crate) rewrite_context_fingerprint: String,
    pub(crate) rewritten_program: Program,
    pub(crate) api_program: Program,
    pub(crate) specialization_projection: Program,
    pub(crate) active_symbols: Vec<String>,
    pub(crate) has_specialization_demand: bool,
}

pub(crate) const OBJECT_CACHE_SCHEMA: &str = "v3";
pub(crate) const OBJECT_SHARD_CACHE_SCHEMA: &str = "v1";
pub(crate) const LINK_MANIFEST_CACHE_SCHEMA: &str = "v1";
pub(crate) const OBJECT_CODEGEN_SHARD_SIZE: usize = 8;
// Large projects pay a disproportionate fixed cost per LLVM module/object emit.
// Sharding only kicks in once the project is big enough that the cold-build win
// outweighs the coarser invalidation granularity for object cache reuse.
pub(crate) const OBJECT_CODEGEN_SHARD_THRESHOLD: usize = 256;

pub(crate) fn env_usize_override(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

pub(crate) fn object_codegen_shard_size() -> usize {
    env_usize_override("ARDEN_OBJECT_SHARD_SIZE", OBJECT_CODEGEN_SHARD_SIZE)
}

pub(crate) fn object_codegen_shard_threshold() -> usize {
    env_usize_override(
        "ARDEN_OBJECT_SHARD_THRESHOLD",
        OBJECT_CODEGEN_SHARD_THRESHOLD,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ObjectCacheEntry {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) semantic_fingerprint: String,
    pub(crate) rewrite_context_fingerprint: String,
    pub(crate) object_build_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ObjectShardMemberFingerprint {
    pub(crate) file: PathBuf,
    pub(crate) semantic_fingerprint: String,
    pub(crate) rewrite_context_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ObjectShardCacheEntry {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) object_build_fingerprint: String,
    pub(crate) members: Vec<ObjectShardMemberFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LinkManifestCache {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) link_fingerprint: String,
    pub(crate) link_inputs: Vec<PathBuf>,
}

pub(crate) fn rewritten_file_cache_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    file.hash(&mut hasher);
    project_root
        .join(".ardencache")
        .join("rewritten")
        .join(format!("{:016x}.json", hasher.finish()))
}

pub(crate) fn semantic_seed_data_from_cache(
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

pub(crate) fn compute_rewrite_context_fingerprint_for_unit_impl(
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
    let dependency_ctx = DependencyResolutionContext {
        namespace_files_map: &empty_namespace_files_map,
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
pub(crate) fn compute_rewrite_context_fingerprint_for_unit(
    unit: &ParsedProjectUnit,
    entry_namespace: &str,
    ctx: &RewriteFingerprintContext<'_>,
) -> String {
    compute_rewrite_context_fingerprint_for_unit_impl(unit, entry_namespace, ctx, None)
}

pub(crate) fn compute_semantic_project_fingerprint(
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

pub(crate) fn collect_active_symbols(program: &Program) -> HashSet<String> {
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

pub(crate) fn load_rewritten_file_cache(
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

pub(crate) fn load_rewritten_file_cache_entry(
    project_root: &Path,
    file: &Path,
) -> Result<Option<RewrittenFileCacheEntry>, String> {
    let path = rewritten_file_cache_path(project_root, file);
    read_cache_blob_with_timing(&path, "rewrite cache", &REWRITE_CACHE_TIMING_TOTALS)
}

pub(crate) fn load_rewritten_file_cache_if_semantic_matches(
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
pub(crate) fn save_rewritten_file_cache(
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

pub(crate) fn object_ext() -> &'static str {
    #[cfg(windows)]
    {
        "obj"
    }
    #[cfg(not(windows))]
    {
        "o"
    }
}

pub(crate) fn object_cache_object_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = stable_hasher();
    file.hash(&mut hasher);
    project_root
        .join(".ardencache")
        .join("objects")
        .join(format!("{:016x}.{}", hasher.finish(), object_ext()))
}

pub(crate) fn object_cache_meta_path(project_root: &Path, file: &Path) -> PathBuf {
    let mut hasher = stable_hasher();
    file.hash(&mut hasher);
    project_root
        .join(".ardencache")
        .join("objects")
        .join(format!("{:016x}.json", hasher.finish()))
}

#[derive(Debug, Clone)]
pub(crate) struct ObjectCachePaths {
    pub(crate) object_path: PathBuf,
    pub(crate) meta_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ObjectShardCachePaths {
    pub(crate) object_path: PathBuf,
    pub(crate) meta_path: PathBuf,
}

pub(crate) fn object_cache_paths(project_root: &Path, file: &Path) -> ObjectCachePaths {
    ObjectCachePaths {
        object_path: object_cache_object_path(project_root, file),
        meta_path: object_cache_meta_path(project_root, file),
    }
}

pub(crate) fn object_shard_cache_key(files: &[PathBuf]) -> String {
    let mut normalized_files = files.to_vec();
    normalized_files.sort();
    let mut hasher = stable_hasher();
    for file in &normalized_files {
        file.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

pub(crate) fn normalized_object_shard_members(
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

pub(crate) fn object_shard_cache_paths(
    project_root: &Path,
    files: &[PathBuf],
) -> ObjectShardCachePaths {
    let key = object_shard_cache_key(files);
    ObjectShardCachePaths {
        object_path: project_root
            .join(".ardencache")
            .join("object_shards")
            .join(format!("{key}.{}", object_ext())),
        meta_path: project_root
            .join(".ardencache")
            .join("object_shards")
            .join(format!("{key}.json")),
    }
}

pub(crate) fn link_manifest_cache_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".ardencache")
        .join("link")
        .join("latest.json")
}

pub(crate) fn compute_link_fingerprint(
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

pub(crate) fn dedupe_link_inputs(link_inputs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(link_inputs.len());
    for path in link_inputs {
        if seen.insert(path.clone()) {
            deduped.push(path);
        }
    }
    deduped
}

pub(crate) fn load_link_manifest_cache(
    project_root: &Path,
) -> Result<Option<LinkManifestCache>, String> {
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

pub(crate) fn save_link_manifest_cache(
    project_root: &Path,
    cache: &LinkManifestCache,
) -> Result<(), String> {
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

pub(crate) fn should_skip_final_link(
    previous_manifest: Option<&LinkManifestCache>,
    current_manifest: &LinkManifestCache,
    output_path: &Path,
    object_cache_miss_count: usize,
) -> bool {
    object_cache_miss_count == 0
        && output_path.exists()
        && previous_manifest.is_some_and(|manifest| manifest == current_manifest)
}

pub(crate) fn compute_object_build_fingerprint(link: &LinkConfig<'_>) -> String {
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

pub(crate) fn load_object_cache_hit(
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

pub(crate) fn save_object_cache_meta(
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

pub(crate) fn load_object_shard_cache_hit(
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

pub(crate) fn save_object_shard_cache_meta(
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

pub(crate) fn hash_imports(imports: &[ImportDecl], hasher: &mut impl Hasher) {
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

pub(crate) fn hash_filtered_namespace_map(
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

pub(crate) fn hash_filtered_global_map(
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

pub(crate) fn compute_namespace_api_fingerprints(
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

pub(crate) fn collect_known_namespace_paths_for_units(
    parsed_files: &[ParsedProjectUnit],
) -> HashSet<String> {
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

pub(crate) fn hash_namespace_api_fingerprints(
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

pub(crate) fn hash_file_api_fingerprint(
    file_api_fingerprints: &HashMap<PathBuf, String>,
    file: &Path,
    hasher: &mut impl Hasher,
) {
    if let Some(fingerprint) = file_api_fingerprints.get(file) {
        file.hash(hasher);
        fingerprint.hash(hasher);
    }
}
