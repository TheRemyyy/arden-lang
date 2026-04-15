use crate::cache::{
    compute_semantic_project_fingerprint, load_semantic_cached_fingerprint,
    project_build_artifact_exists, BuildTimings, DependencyGraphCache, ParsedProjectUnit,
};
use crate::cli::output::print_cli_cache;
use crate::dependency::transitive_dependents;
use crate::project::ProjectConfig;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum SemanticGateError {
    CacheLookup(String),
}

impl fmt::Display for SemanticGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CacheLookup(message) => write!(f, "{message}"),
        }
    }
}

impl From<SemanticGateError> for String {
    fn from(value: SemanticGateError) -> Self {
        value.to_string()
    }
}

pub(crate) struct ProjectChangeImpact {
    pub(crate) body_only_changed: HashSet<PathBuf>,
    pub(crate) api_changed: HashSet<PathBuf>,
    pub(crate) dependent_api_impact: HashSet<PathBuf>,
}

pub(crate) struct SemanticGateInputs<'a> {
    pub(crate) config: &'a ProjectConfig,
    pub(crate) parsed_files: &'a [ParsedProjectUnit],
    pub(crate) emit_llvm: bool,
    pub(crate) do_check: bool,
    pub(crate) check_only: bool,
    pub(crate) project_root: &'a Path,
    pub(crate) output_path: &'a Path,
    pub(crate) impact: &'a ProjectChangeImpact,
}

pub(crate) fn compute_project_change_impact(
    previous_dependency_graph: Option<&DependencyGraphCache>,
    parsed_files: &[ParsedProjectUnit],
    reverse_file_dependency_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> ProjectChangeImpact {
    let mut body_only_changed = HashSet::new();
    let mut api_changed = HashSet::new();
    let mut dependent_api_impact = HashSet::new();

    if let Some(previous) = previous_dependency_graph {
        let previous_files = previous
            .files
            .iter()
            .map(|entry| (&entry.file, entry))
            .collect::<HashMap<_, _>>();

        for unit in parsed_files {
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
            let mut impacted = transitive_dependents(reverse_file_dependency_graph, &api_changed);
            for changed in &api_changed {
                impacted.remove(changed);
            }
            impacted
        };

        if !body_only_changed.is_empty() || !api_changed.is_empty() {
            print_cli_cache(format!(
                "Impact graph: {} body-only, {} API, {} downstream dependents",
                body_only_changed.len(),
                api_changed.len(),
                dependent_api_impact.len()
            ));
        }
    }

    ProjectChangeImpact {
        body_only_changed,
        api_changed,
        dependent_api_impact,
    }
}

pub(crate) fn evaluate_semantic_cache_gate(
    build_timings: &mut BuildTimings,
    inputs: SemanticGateInputs<'_>,
) -> Result<(String, bool), String> {
    evaluate_semantic_cache_gate_impl(build_timings, inputs).map_err(Into::into)
}

fn evaluate_semantic_cache_gate_impl(
    build_timings: &mut BuildTimings,
    inputs: SemanticGateInputs<'_>,
) -> Result<(String, bool), SemanticGateError> {
    let (semantic_fingerprint, semantic_cache_hit) =
        build_timings.measure("semantic cache gate", || {
            let semantic_fingerprint = compute_semantic_project_fingerprint(
                inputs.config,
                inputs.parsed_files,
                inputs.emit_llvm,
                inputs.do_check,
            );
            let semantic_cache_hit = if !inputs.check_only {
                load_semantic_cached_fingerprint(inputs.project_root)
                    .map_err(SemanticGateError::CacheLookup)?
                    .is_some_and(|cached| {
                        cached == semantic_fingerprint
                            && project_build_artifact_exists(inputs.output_path, inputs.emit_llvm)
                    })
            } else {
                false
            };
            Ok::<_, SemanticGateError>((semantic_fingerprint, semantic_cache_hit))
        })?;

    build_timings.record_counts(
        "semantic cache gate",
        &[
            ("files", inputs.parsed_files.len()),
            ("body_only", inputs.impact.body_only_changed.len()),
            ("api", inputs.impact.api_changed.len()),
            ("downstream", inputs.impact.dependent_api_impact.len()),
            ("hit", usize::from(semantic_cache_hit)),
        ],
    );

    Ok((semantic_fingerprint, semantic_cache_hit))
}
