use crate::cache::{BuildTimings, PARSE_CACHE_TIMING_TOTALS};
use crate::cli::output::print_cli_cache;
use std::sync::atomic::Ordering;

pub(crate) struct ParseIndexMetrics {
    pub(crate) files_len: usize,
    pub(crate) parse_cache_hits: usize,
    pub(crate) total_function_names: usize,
    pub(crate) total_class_names: usize,
    pub(crate) total_interface_names: usize,
    pub(crate) total_enum_names: usize,
    pub(crate) total_module_names: usize,
    pub(crate) needs_project_symbol_lookup: bool,
    pub(crate) parse_index_namespace_sets_ns: u64,
    pub(crate) parse_index_function_register_ns: u64,
    pub(crate) parse_index_class_register_ns: u64,
    pub(crate) parse_index_interface_register_ns: u64,
    pub(crate) parse_index_enum_register_ns: u64,
    pub(crate) parse_index_module_register_ns: u64,
    pub(crate) parse_index_parsed_file_push_ns: u64,
}

pub(crate) fn record_parse_index_metrics(
    build_timings: &mut BuildTimings,
    metrics: ParseIndexMetrics,
) {
    if metrics.parse_cache_hits > 0 {
        print_cli_cache(format!(
            "Reused parse cache for {}/{} files",
            metrics.parse_cache_hits, metrics.files_len
        ));
    }
    build_timings.record_counts(
        "parse + symbol scan",
        &[
            ("considered", metrics.files_len),
            ("reused", metrics.parse_cache_hits),
            (
                "parsed",
                metrics.files_len.saturating_sub(metrics.parse_cache_hits),
            ),
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
    build_timings.record_duration_ns(
        "parse index/namespace sets",
        metrics.parse_index_namespace_sets_ns,
    );
    build_timings.record_duration_ns(
        "parse index/register functions",
        metrics.parse_index_function_register_ns,
    );
    build_timings.record_duration_ns(
        "parse index/register classes",
        metrics.parse_index_class_register_ns,
    );
    build_timings.record_duration_ns(
        "parse index/register interfaces",
        metrics.parse_index_interface_register_ns,
    );
    build_timings.record_duration_ns(
        "parse index/register enums",
        metrics.parse_index_enum_register_ns,
    );
    build_timings.record_duration_ns(
        "parse index/register modules",
        metrics.parse_index_module_register_ns,
    );
    build_timings.record_duration_ns(
        "parse index/push units",
        metrics.parse_index_parsed_file_push_ns,
    );
    build_timings.record_counts(
        "parse index/details",
        &[
            ("functions", metrics.total_function_names),
            ("classes", metrics.total_class_names),
            ("interfaces", metrics.total_interface_names),
            ("enums", metrics.total_enum_names),
            ("modules", metrics.total_module_names),
            (
                "project_symbol_lookup",
                usize::from(metrics.needs_project_symbol_lookup),
            ),
        ],
    );
}
