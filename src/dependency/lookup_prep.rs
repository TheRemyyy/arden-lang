use crate::cache::{elapsed_nanos_u64, BuildTimings, ParsedProjectUnit};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

pub(crate) fn build_namespace_files_lookup(
    build_timings: &mut BuildTimings,
    parsed_files: &[ParsedProjectUnit],
    total_module_names: usize,
) -> HashMap<String, Vec<PathBuf>> {
    let mut namespace_files_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    namespace_files_map.reserve(parsed_files.len() + total_module_names);

    let mut dependency_lookup_base_namespace_ns = 0_u64;
    let mut dependency_lookup_module_namespace_ns = 0_u64;
    let mut dependency_lookup_sort_dedup_ns = 0_u64;

    build_timings.measure_step("dependency lookup prep", || {
        for unit in parsed_files {
            let started_at = Instant::now();
            namespace_files_map
                .entry(unit.namespace.clone())
                .or_default()
                .push(unit.file.clone());
            dependency_lookup_base_namespace_ns += elapsed_nanos_u64(started_at);

            for module_name in &unit.module_names {
                let started_at = Instant::now();
                namespace_files_map
                    .entry(format!(
                        "{}.{}",
                        unit.namespace,
                        module_name.replace("__", ".")
                    ))
                    .or_default()
                    .push(unit.file.clone());
                dependency_lookup_module_namespace_ns += elapsed_nanos_u64(started_at);
            }
        }

        let sort_started_at = Instant::now();
        for files in namespace_files_map.values_mut() {
            files.sort();
            files.dedup();
        }
        dependency_lookup_sort_dedup_ns += elapsed_nanos_u64(sort_started_at);
    });

    build_timings.record_duration_ns(
        "dependency lookup/base namespace",
        dependency_lookup_base_namespace_ns,
    );
    build_timings.record_duration_ns(
        "dependency lookup/module namespace",
        dependency_lookup_module_namespace_ns,
    );
    build_timings.record_duration_ns("dependency lookup/function files", 0);
    build_timings.record_duration_ns("dependency lookup/class files", 0);
    build_timings.record_duration_ns("dependency lookup/interface files", 0);
    build_timings.record_duration_ns("dependency lookup/module files", 0);
    build_timings.record_duration_ns(
        "dependency lookup/sort dedup",
        dependency_lookup_sort_dedup_ns,
    );

    namespace_files_map
}
