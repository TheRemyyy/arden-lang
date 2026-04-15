use crate::cache::{
    collect_active_symbols, compute_rewrite_context_fingerprint_for_unit_impl, elapsed_nanos_u64,
    load_rewritten_file_cache, load_rewritten_file_cache_if_semantic_matches,
    save_rewritten_file_cache, BuildTimings, ParsedProjectUnit, PipelineRewriteTimingTotals,
    RewriteFingerprintTimingTotals, RewrittenFileCachePayload, RewrittenProjectUnit,
    REWRITE_CACHE_TIMING_TOTALS,
};
use crate::cli::output::print_cli_cache;
use crate::dependency::RewriteFingerprintContext;
use crate::project_rewrite;
use crate::specialization::{
    api_projection_program, program_has_codegen_specialization_demand,
    specialization_projection_program,
};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
enum RewritePhaseError {
    PhaseRun(String),
    UnitCollection(String),
}

impl fmt::Display for RewritePhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhaseRun(message) | Self::UnitCollection(message) => write!(f, "{message}"),
        }
    }
}

impl From<RewritePhaseError> for String {
    fn from(value: RewritePhaseError) -> Self {
        value.to_string()
    }
}

impl From<String> for RewritePhaseError {
    fn from(value: String) -> Self {
        Self::PhaseRun(value)
    }
}

pub(crate) struct RewritePhaseInputs<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) parsed_files: &'a [ParsedProjectUnit],
    pub(crate) safe_rewrite_cache_files: &'a HashSet<PathBuf>,
    pub(crate) entry_namespace: &'a str,
    pub(crate) rewrite_fingerprint_ctx: &'a RewriteFingerprintContext<'a>,
    pub(crate) namespace_functions: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_function_map: &'a HashMap<String, String>,
    pub(crate) namespace_class_map: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_class_map: &'a HashMap<String, String>,
    pub(crate) namespace_interface_map: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_interface_map: &'a HashMap<String, String>,
    pub(crate) namespace_enum_map: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_enum_map: &'a HashMap<String, String>,
    pub(crate) namespace_module_map: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_module_map: &'a HashMap<String, String>,
}

pub(crate) fn run_rewrite_phase(
    build_timings: &mut BuildTimings,
    inputs: RewritePhaseInputs<'_>,
) -> Result<Vec<RewrittenProjectUnit>, String> {
    run_rewrite_phase_impl(build_timings, inputs).map_err(Into::into)
}

fn run_rewrite_phase_impl(
    build_timings: &mut BuildTimings,
    inputs: RewritePhaseInputs<'_>,
) -> Result<Vec<RewrittenProjectUnit>, RewritePhaseError> {
    project_rewrite::reset_rewrite_timings();
    let rewrite_timing_totals = Arc::new(PipelineRewriteTimingTotals::default());
    let rewrite_fingerprint_timing_totals = Arc::new(RewriteFingerprintTimingTotals::default());
    let rewritten_results: Vec<Result<RewrittenProjectUnit, String>> = build_timings
        .measure("rewrite", || {
            Ok::<_, String>(
                inputs
                    .parsed_files
                    .par_iter()
                    .map(|unit| {
                        if inputs.safe_rewrite_cache_files.contains(&unit.file) {
                            let cache_lookup_started_at = Instant::now();
                            if let Some(cached_entry) =
                                load_rewritten_file_cache_if_semantic_matches(
                                    inputs.project_root,
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
                                inputs.entry_namespace,
                                inputs.rewrite_fingerprint_ctx,
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
                            inputs.project_root,
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
                            &project_rewrite::ProjectRewriteContext {
                                current_namespace: &unit.namespace,
                                entry_namespace: inputs.entry_namespace,
                                namespace_functions: inputs.namespace_functions,
                                global_function_map: inputs.global_function_map,
                                namespace_classes: inputs.namespace_class_map,
                                global_class_map: inputs.global_class_map,
                                namespace_interfaces: inputs.namespace_interface_map,
                                global_interface_map: inputs.global_interface_map,
                                namespace_enums: inputs.namespace_enum_map,
                                global_enum_map: inputs.global_enum_map,
                                namespace_modules: inputs.namespace_module_map,
                                global_module_map: inputs.global_module_map,
                                imports: &unit.imports,
                            },
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
                            inputs.project_root,
                            &unit.file,
                            RewrittenFileCachePayload {
                                semantic_fingerprint: &unit.semantic_fingerprint,
                                rewrite_context_fingerprint: &rewrite_context_fingerprint,
                                rewritten_program: &rewritten,
                                api_program: &api_program,
                                specialization_projection: &specialization_projection,
                                active_symbols: &active_symbols,
                                has_specialization_demand,
                            },
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
        })
        .map_err(RewritePhaseError::PhaseRun)?;

    let mut rewritten_files: Vec<RewrittenProjectUnit> = Vec::new();
    for result in rewritten_results {
        rewritten_files.push(result.map_err(RewritePhaseError::UnitCollection)?);
    }
    rewritten_files.sort_by(|a, b| a.file.cmp(&b.file));

    let rewrite_cache_hits = rewritten_files
        .iter()
        .filter(|unit| unit.from_rewrite_cache)
        .count();
    if rewrite_cache_hits > 0 {
        print_cli_cache(format!(
            "Reused rewrite cache for {}/{} files",
            rewrite_cache_hits,
            rewritten_files.len()
        ));
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

    Ok(rewritten_files)
}
