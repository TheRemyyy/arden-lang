use super::dependency_graph_metrics::record_dependency_graph_metrics;
use super::dependency_graph_types::DependencyGraphOutputs;
use crate::cache::{
    BuildTimings, DependencyGraphTimingTotals, ParsedProjectUnit, ProjectSymbolLookup,
};
use crate::dependency::{
    build_file_dependency_graph_incremental, build_namespace_files_lookup,
    build_reverse_dependency_graph, dependency_graph_cache_from_state, DependencyResolutionContext,
};
use crate::symbol_lookup::GlobalSymbolMaps;
use std::path::Path;
use std::sync::Arc;

pub(crate) struct DependencyGraphInputs<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) entry_path: &'a Path,
    pub(crate) parsed_files: &'a [ParsedProjectUnit],
    pub(crate) total_module_names: usize,
    pub(crate) global_maps: GlobalSymbolMaps<'a>,
    pub(crate) project_symbol_lookup: &'a ProjectSymbolLookup,
}

pub(crate) fn run_dependency_graph_phase(
    build_timings: &mut BuildTimings,
    inputs: DependencyGraphInputs<'_>,
) -> Result<DependencyGraphOutputs, String> {
    let previous_dependency_graph = build_timings.measure("dependency cache load", || {
        crate::load_dependency_graph_cache(inputs.project_root)
    })?;
    let namespace_files_map = build_namespace_files_lookup(
        build_timings,
        inputs.parsed_files,
        inputs.total_module_names,
    );
    let dependency_resolution_ctx = DependencyResolutionContext {
        namespace_files_map: &namespace_files_map,
        global_function_map: inputs.global_maps.function_map,
        global_function_file_map: inputs.global_maps.function_file_map,
        global_class_map: inputs.global_maps.class_map,
        global_class_file_map: inputs.global_maps.class_file_map,
        global_interface_map: inputs.global_maps.interface_map,
        global_interface_file_map: inputs.global_maps.interface_file_map,
        global_enum_map: inputs.global_maps.enum_map,
        global_enum_file_map: inputs.global_maps.enum_file_map,
        global_module_map: inputs.global_maps.module_map,
        global_module_file_map: inputs.global_maps.module_file_map,
        symbol_lookup: Arc::new(inputs.project_symbol_lookup.clone()),
    };

    let dependency_graph_timing_totals = Arc::new(DependencyGraphTimingTotals::default());
    let (file_dependency_graph, dependency_graph_cache_hits) =
        build_timings.measure_value("dependency graph", || {
            build_file_dependency_graph_incremental(
                inputs.parsed_files,
                &dependency_resolution_ctx,
                previous_dependency_graph.as_ref(),
                Some(dependency_graph_timing_totals.as_ref()),
            )
        });
    let reverse_file_dependency_graph = build_reverse_dependency_graph(&file_dependency_graph);
    let current_entry_namespace = inputs
        .parsed_files
        .iter()
        .find(|unit| unit.file == inputs.entry_path)
        .map(|unit| unit.namespace.clone())
        .unwrap_or_else(|| "global".to_string());
    let current_dependency_graph_cache = dependency_graph_cache_from_state(
        &current_entry_namespace,
        inputs.parsed_files,
        &file_dependency_graph,
    );

    record_dependency_graph_metrics(
        build_timings,
        dependency_graph_timing_totals.as_ref(),
        inputs.parsed_files.len(),
        dependency_graph_cache_hits,
    );

    Ok(DependencyGraphOutputs {
        previous_dependency_graph,
        file_dependency_graph,
        reverse_file_dependency_graph,
        current_dependency_graph_cache,
    })
}
