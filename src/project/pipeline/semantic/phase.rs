use crate::borrowck::BorrowChecker;
use crate::cache::{
    save_semantic_summary_cache, save_typecheck_summary_cache, semantic_seed_data_from_cache,
    BuildTimings, ParsedProjectUnit, RewrittenProjectUnit, SemanticSummaryCache,
    TypecheckSummaryCache,
};
use crate::cli::output::{format_cli_path, print_cli_cache, print_cli_step};
use crate::dependency::{
    component_fingerprint, merge_reusable_component_semantic_data, reusable_component_fingerprints,
    semantic_check_components, semantic_program_for_component, semantic_summary_cache_from_state,
    typecheck_summary_cache_from_state, typecheck_summary_cache_matches,
};
use crate::diagnostics::{render_borrow_errors, render_type_errors};
use crate::typeck::{ClassMethodEffectsSummary, FunctionEffectsSummary, TypeChecker};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum SemanticPhaseError {
    CacheReuseGuard(String),
    SemanticCheck(String),
    ComponentCheck(String),
    Diagnostic(String),
    SemanticCacheSave(String),
    TypecheckCacheSave(String),
}

impl fmt::Display for SemanticPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CacheReuseGuard(message)
            | Self::SemanticCheck(message)
            | Self::ComponentCheck(message)
            | Self::Diagnostic(message)
            | Self::SemanticCacheSave(message)
            | Self::TypecheckCacheSave(message) => write!(f, "{message}"),
        }
    }
}

impl From<SemanticPhaseError> for String {
    fn from(value: SemanticPhaseError) -> Self {
        value.to_string()
    }
}

impl From<String> for SemanticPhaseError {
    fn from(value: String) -> Self {
        Self::SemanticCheck(value)
    }
}

pub(crate) struct SemanticPhaseInputs<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) parsed_files: &'a [ParsedProjectUnit],
    pub(crate) rewritten_files: &'a [RewrittenProjectUnit],
    pub(crate) file_dependency_graph: &'a HashMap<PathBuf, HashSet<PathBuf>>,
    pub(crate) previous_dependency_graph_exists: bool,
    pub(crate) previous_semantic_summary: Option<&'a SemanticSummaryCache>,
    pub(crate) previous_typecheck_summary: Option<&'a TypecheckSummaryCache>,
    pub(crate) body_only_changed: &'a HashSet<PathBuf>,
    pub(crate) api_changed: &'a HashSet<PathBuf>,
    pub(crate) dependent_api_impact: &'a HashSet<PathBuf>,
}

pub(crate) fn run_semantic_phase(
    build_timings: &mut BuildTimings,
    inputs: SemanticPhaseInputs<'_>,
) -> Result<(), String> {
    run_semantic_phase_impl(build_timings, inputs).map_err(Into::into)
}

fn run_semantic_phase_impl(
    build_timings: &mut BuildTimings,
    inputs: SemanticPhaseInputs<'_>,
) -> Result<(), SemanticPhaseError> {
    print_cli_step("Running semantic checks");

    let mut semantic_full_files: HashSet<PathBuf> = inputs
        .parsed_files
        .iter()
        .map(|unit| unit.file.clone())
        .collect();
    if inputs.previous_dependency_graph_exists && inputs.previous_semantic_summary.is_some() {
        semantic_full_files = inputs
            .body_only_changed
            .union(inputs.api_changed)
            .cloned()
            .collect::<HashSet<_>>();
        semantic_full_files.extend(inputs.dependent_api_impact.iter().cloned());
        if semantic_full_files.is_empty() {
            semantic_full_files.extend(inputs.parsed_files.iter().map(|unit| unit.file.clone()));
        }
    }

    let current_semantic_fingerprints: HashMap<PathBuf, String> = inputs
        .parsed_files
        .iter()
        .map(|unit| (unit.file.clone(), unit.semantic_fingerprint.clone()))
        .collect();
    let semantic_components =
        semantic_check_components(inputs.parsed_files, inputs.file_dependency_graph);
    let reusable_component_fps = inputs
        .previous_typecheck_summary
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
            inputs.previous_typecheck_summary.ok_or_else(|| {
                SemanticPhaseError::CacheReuseGuard(
                    "error: semantic cache reuse was attempted without a previous typecheck summary"
                        .to_string(),
                )
            })?,
            &current_semantic_fingerprints,
            &semantic_components,
        );

    if reusable_typecheck_cache {
        print_cli_cache(format!(
            "Reused typecheck/borrowck cache for {}/{} files",
            current_semantic_fingerprints.len(),
            inputs.parsed_files.len()
        ));
        build_timings.record_counts(
            "semantic",
            &[
                ("components", semantic_components.len()),
                ("reused_components", semantic_components.len()),
                ("checked_components", 0),
                ("reused_files", inputs.parsed_files.len()),
                ("checked_files", 0),
            ],
        );
        return Ok(());
    }

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

    let (seeded_function_effects, seeded_class_method_effects, seeded_class_mutating_methods) =
        inputs
            .previous_semantic_summary
            .as_ref()
            .map(|cache| {
                semantic_seed_data_from_cache(
                    cache,
                    &current_semantic_fingerprints,
                    &semantic_full_files,
                )
            })
            .unwrap_or_else(|| (HashMap::new(), HashMap::new(), HashMap::new()));

    if semantic_full_files.len() < inputs.parsed_files.len() {
        print_cli_cache(format!(
            "Semantic delta: checking {}/{} files with full bodies",
            semantic_full_files.len(),
            inputs.parsed_files.len()
        ));
    }
    if semantic_components.len() > 1 {
        print_cli_cache(format!(
            "Parallel semantic check across {} independent components",
            semantic_components.len()
        ));
    }
    if !reusable_component_files.is_empty() {
        print_cli_cache(format!(
            "Reused semantic component cache for {}/{} files",
            reusable_component_files.len(),
            inputs.parsed_files.len()
        ));
    }

    struct ComponentSemanticCheckResult {
        function_effects: FunctionEffectsSummary,
        class_method_effects: ClassMethodEffectsSummary,
        class_mutating_methods: HashMap<String, HashSet<String>>,
    }

    let semantic_results: Vec<Result<ComponentSemanticCheckResult, SemanticPhaseError>> =
        build_timings
            .measure("semantic", || {
                Ok::<_, String>(
                    checked_components
                        .par_iter()
                        .map(|component| {
                            let component_files: HashSet<PathBuf> = component.iter().cloned().collect();
                            let component_sources = component
                                .iter()
                                .map(|file| {
                                    fs::read_to_string(file)
                                        .map(|source| (file.clone(), source))
                                        .map_err(|error| {
                                            SemanticPhaseError::ComponentCheck(format!(
                                                "error: Failed to read '{}' during semantic checks: {}",
                                                format_cli_path(file),
                                                error
                                            ))
                                        })
                                })
                                .collect::<Result<Vec<_>, SemanticPhaseError>>()?;
                            let semantic_program = semantic_program_for_component(
                                inputs.rewritten_files,
                                &component_files,
                                &semantic_full_files,
                            );

                            let mut type_checker = TypeChecker::new();
                            if let Err(errors) = type_checker.check_with_effect_seeds(
                                &semantic_program,
                                &seeded_function_effects,
                                &seeded_class_method_effects,
                            ) {
                                return Err(SemanticPhaseError::ComponentCheck(
                                    render_type_errors(errors, &component_sources),
                                ));
                            }

                            let mut borrow_checker = BorrowChecker::new();
                            if let Err(errors) = borrow_checker.check_with_mutating_method_seeds(
                                &semantic_program,
                                &seeded_class_mutating_methods,
                            ) {
                                return Err(SemanticPhaseError::ComponentCheck(
                                    render_borrow_errors(errors, &component_sources),
                                ));
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
            })
            .map_err(SemanticPhaseError::SemanticCheck)?;

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
                inputs
                    .parsed_files
                    .len()
                    .saturating_sub(reusable_component_files.len()),
            ),
            ("full_body_files", semantic_full_files.len()),
        ],
    );

    let mut rendered_errors = String::new();
    let (mut function_effects, mut class_method_effects, mut class_mutating_methods) = inputs
        .previous_semantic_summary
        .as_ref()
        .map(|cache| merge_reusable_component_semantic_data(cache, &reusable_component_fps))
        .unwrap_or_else(|| (HashMap::new(), HashMap::new(), HashMap::new()));

    for result in semantic_results {
        match result {
            Ok(component) => {
                function_effects.extend(component.function_effects);
                class_method_effects.extend(component.class_method_effects);
                class_mutating_methods.extend(component.class_mutating_methods);
            }
            Err(SemanticPhaseError::ComponentCheck(errors)) => rendered_errors.push_str(&errors),
            Err(error) => return Err(error),
        }
    }

    if !rendered_errors.is_empty() {
        return Err(SemanticPhaseError::Diagnostic(rendered_errors));
    }

    save_semantic_summary_cache(
        inputs.project_root,
        &semantic_summary_cache_from_state(
            inputs.parsed_files,
            &semantic_components,
            function_effects,
            class_method_effects,
            class_mutating_methods,
        ),
    )
    .map_err(SemanticPhaseError::SemanticCacheSave)?;
    save_typecheck_summary_cache(
        inputs.project_root,
        &typecheck_summary_cache_from_state(&current_semantic_fingerprints, &semantic_components),
    )
    .map_err(SemanticPhaseError::TypecheckCacheSave)?;

    Ok(())
}
