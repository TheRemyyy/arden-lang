use crate::cache::{
    collect_known_namespace_paths_for_units, compute_rewrite_context_fingerprint_for_unit_impl,
    elapsed_nanos_u64, load_import_check_cache_hit, save_import_check_cache_hit, BuildTimings,
    ImportCheckTimingTotals, ParsedProjectUnit,
};
use crate::cli::output::format_cli_path;
use crate::dependency::RewriteFingerprintContext;
use crate::import_check::ImportChecker;
use crate::stdlib::stdlib_registry;
use colored::Colorize;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
enum ImportCheckPhaseError {
    CheckRun(String),
    ResultCollect(String),
}

impl fmt::Display for ImportCheckPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CheckRun(message) | Self::ResultCollect(message) => write!(f, "{message}"),
        }
    }
}

impl From<ImportCheckPhaseError> for String {
    fn from(value: ImportCheckPhaseError) -> Self {
        value.to_string()
    }
}

impl From<String> for ImportCheckPhaseError {
    fn from(value: String) -> Self {
        Self::CheckRun(value)
    }
}

pub(crate) struct ImportCheckInputs<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) parsed_files: &'a [ParsedProjectUnit],
    pub(crate) global_function_map: &'a HashMap<String, String>,
    pub(crate) entry_namespace: &'a str,
    pub(crate) rewrite_fingerprint_ctx: &'a RewriteFingerprintContext<'a>,
}

pub(crate) fn run_import_check_phase(
    build_timings: &mut BuildTimings,
    inputs: ImportCheckInputs<'_>,
) -> Result<(), String> {
    run_import_check_phase_impl(build_timings, inputs).map_err(Into::into)
}

fn run_import_check_phase_impl(
    build_timings: &mut BuildTimings,
    inputs: ImportCheckInputs<'_>,
) -> Result<(), ImportCheckPhaseError> {
    let shared_function_map = Arc::new(inputs.global_function_map.clone());
    let shared_known_namespace_paths =
        Arc::new(collect_known_namespace_paths_for_units(inputs.parsed_files));
    let import_check_cache_hits = AtomicUsize::new(0);
    let import_check_timing_totals = Arc::new(ImportCheckTimingTotals::default());

    let import_results: Vec<Result<(), String>> = build_timings
        .measure("import check", || {
            Ok::<_, String>(
                inputs
                    .parsed_files
                    .par_iter()
                    .map(|unit| {
                        let fingerprint_started_at = Instant::now();
                        let rewrite_context_fingerprint =
                            compute_rewrite_context_fingerprint_for_unit_impl(
                                unit,
                                inputs.entry_namespace,
                                inputs.rewrite_fingerprint_ctx,
                                None,
                            );
                        import_check_timing_totals
                            .rewrite_context_fingerprint_ns
                            .fetch_add(elapsed_nanos_u64(fingerprint_started_at), Ordering::Relaxed);

                        let cache_lookup_started_at = Instant::now();
                        if load_import_check_cache_hit(
                            inputs.project_root,
                            &unit.file,
                            &unit.import_check_fingerprint,
                            &rewrite_context_fingerprint,
                        )? {
                            import_check_timing_totals.cache_lookup_ns.fetch_add(
                                elapsed_nanos_u64(cache_lookup_started_at),
                                Ordering::Relaxed,
                            );
                            import_check_cache_hits.fetch_add(1, Ordering::Relaxed);
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
                            crate::extract_top_level_imports(&unit.program),
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
                            let filename = format_cli_path(&unit.file);
                            let source = fs::read_to_string(&unit.file);
                            let mut rendered = String::new();
                            for error in errors {
                                if let Ok(source) = source.as_deref() {
                                    rendered.push_str(&error.format_with_source(source, &filename));
                                } else {
                                    let source_read_error = source
                                        .as_ref()
                                        .err()
                                        .map(std::string::ToString::to_string)
                                        .unwrap_or_else(|| "unknown read error".to_string());
                                    rendered.push_str(&format!(
                                        "{}: {}\n{}: Failed to read '{}' while formatting import errors: {}",
                                        "error".red().bold(),
                                        error.format(),
                                        "error".red().bold(),
                                        format_cli_path(&unit.file),
                                        source_read_error,
                                    ));
                                }
                                rendered.push('\n');
                            }
                            return Err(rendered.trim_end().to_string());
                        }
                        import_check_timing_totals
                            .checker_run_ns
                            .fetch_add(elapsed_nanos_u64(checker_run_started_at), Ordering::Relaxed);

                        let cache_save_started_at = Instant::now();
                        save_import_check_cache_hit(
                            inputs.project_root,
                            &unit.file,
                            &unit.import_check_fingerprint,
                            &rewrite_context_fingerprint,
                        )?;
                        import_check_timing_totals
                            .cache_save_ns
                            .fetch_add(elapsed_nanos_u64(cache_save_started_at), Ordering::Relaxed);
                        Ok(())
                    })
                    .collect(),
            )
        })
        .map_err(ImportCheckPhaseError::CheckRun)?;

    for result in import_results {
        if let Err(rendered) = result {
            return Err(ImportCheckPhaseError::ResultCollect(format!(
                "Import check failed\n{rendered}"
            )));
        }
    }

    let import_check_cache_hits = import_check_cache_hits.load(Ordering::Relaxed);
    if import_check_cache_hits > 0 {
        crate::cli::output::print_cli_cache(format!(
            "Reused import-check cache for {}/{} files",
            import_check_cache_hits,
            inputs.parsed_files.len()
        ));
    }

    build_timings.record_counts(
        "import check",
        &[
            ("considered", inputs.parsed_files.len()),
            ("reused", import_check_cache_hits),
            (
                "checked",
                inputs
                    .parsed_files
                    .len()
                    .saturating_sub(import_check_cache_hits),
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

    Ok(())
}
