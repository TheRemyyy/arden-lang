use crate::cache::{compute_namespace_api_fingerprints, DependencyGraphCache, ParsedProjectUnit};
use crate::dependency::can_reuse_safe_rewrite_cache;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(crate) struct RewritePreparation {
    pub(crate) namespace_functions: HashMap<String, HashSet<String>>,
    pub(crate) entry_namespace: String,
    pub(crate) namespace_api_fingerprints: HashMap<String, String>,
    pub(crate) file_api_fingerprints: HashMap<PathBuf, String>,
    pub(crate) safe_rewrite_cache_files: HashSet<PathBuf>,
}

pub(crate) fn prepare_rewrite_inputs(
    parsed_files: &[ParsedProjectUnit],
    entry_path: &Path,
    previous_dependency_graph: Option<&DependencyGraphCache>,
    body_only_changed: &HashSet<PathBuf>,
    api_changed: &HashSet<PathBuf>,
    dependent_api_impact: &HashSet<PathBuf>,
) -> RewritePreparation {
    let mut namespace_functions: HashMap<String, HashSet<String>> = HashMap::new();
    for unit in parsed_files {
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
    let namespace_api_fingerprints = compute_namespace_api_fingerprints(parsed_files);
    let file_api_fingerprints: HashMap<PathBuf, String> = parsed_files
        .iter()
        .map(|unit| (unit.file.clone(), unit.api_fingerprint.clone()))
        .collect();

    let safe_rewrite_cache_files =
        if can_reuse_safe_rewrite_cache(previous_dependency_graph, &entry_namespace) {
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

    RewritePreparation {
        namespace_functions,
        entry_namespace,
        namespace_api_fingerprints,
        file_api_fingerprints,
        safe_rewrite_cache_files,
    }
}
