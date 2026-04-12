use crate::cache::{BuildTimings, DependencyGraphTimingTotals};
use crate::cli::output::print_cli_cache;
use std::sync::atomic::Ordering;

pub(crate) fn record_dependency_graph_metrics(
    build_timings: &mut BuildTimings,
    timings: &DependencyGraphTimingTotals,
    parsed_file_count: usize,
    dependency_graph_cache_hits: usize,
) {
    if dependency_graph_cache_hits > 0 {
        print_cli_cache(format!(
            "Reused dependency graph entries for {}/{} files",
            dependency_graph_cache_hits, parsed_file_count
        ));
    }
    build_timings.record_counts(
        "dependency graph",
        &[
            ("considered", parsed_file_count),
            ("reused", dependency_graph_cache_hits),
            (
                "rebuilt",
                parsed_file_count.saturating_sub(dependency_graph_cache_hits),
            ),
        ],
    );
    build_timings.record_duration_ns(
        "dependency graph/cache validation",
        timings.cache_validation_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/direct refs",
        timings.direct_symbol_refs_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/import exact",
        timings.import_exact_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/import wildcard",
        timings.import_wildcard_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/import namespace alias",
        timings.import_namespace_alias_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/import parent namespace",
        timings.import_parent_namespace_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/namespace fallback",
        timings.namespace_fallback_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/owner lookup",
        timings.owner_lookup_ns.load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "dependency graph/namespace files",
        timings.namespace_files_ns.load(Ordering::Relaxed),
    );
    build_timings.record_counts(
        "dependency graph/details",
        &[
            ("reused_files", timings.files_reused.load(Ordering::Relaxed)),
            (
                "rebuilt_files",
                timings.files_rebuilt.load(Ordering::Relaxed),
            ),
            (
                "direct_symbol_refs",
                timings.direct_symbol_ref_count.load(Ordering::Relaxed),
            ),
            (
                "exact_imports",
                timings.import_exact_count.load(Ordering::Relaxed),
            ),
            (
                "wildcard_imports",
                timings.import_wildcard_count.load(Ordering::Relaxed),
            ),
            (
                "namespace_alias_imports",
                timings.import_namespace_alias_count.load(Ordering::Relaxed),
            ),
            (
                "parent_namespace_imports",
                timings
                    .import_parent_namespace_count
                    .load(Ordering::Relaxed),
            ),
            (
                "namespace_fallbacks",
                timings.namespace_fallback_count.load(Ordering::Relaxed),
            ),
            (
                "qualified_refs",
                timings.qualified_ref_count.load(Ordering::Relaxed),
            ),
        ],
    );
}
