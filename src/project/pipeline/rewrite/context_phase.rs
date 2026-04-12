use super::{prepare_rewrite_inputs, RewritePreparation};
use crate::cache::{BuildTimings, DependencyGraphCache, ParsedProjectUnit, ProjectSymbolLookup};
use crate::cli::output::print_cli_step;
use crate::dependency::RewriteFingerprintContext;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) struct RewritePrepInputs<'a> {
    pub(crate) parsed_files: &'a [ParsedProjectUnit],
    pub(crate) entry_path: &'a Path,
    pub(crate) previous_dependency_graph: Option<&'a DependencyGraphCache>,
    pub(crate) body_only_changed: &'a HashSet<PathBuf>,
    pub(crate) api_changed: &'a HashSet<PathBuf>,
    pub(crate) dependent_api_impact: &'a HashSet<PathBuf>,
}

pub(crate) struct RewriteContextInputs<'a> {
    pub(crate) namespace_functions: &'a HashMap<String, HashSet<String>>,
    pub(crate) namespace_classes: &'a HashMap<String, HashSet<String>>,
    pub(crate) namespace_modules: &'a HashMap<String, HashSet<String>>,
    pub(crate) namespace_api_fingerprints: &'a HashMap<String, String>,
    pub(crate) file_api_fingerprints: &'a HashMap<PathBuf, String>,
    pub(crate) global_function_map: &'a HashMap<String, String>,
    pub(crate) global_function_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_class_map: &'a HashMap<String, String>,
    pub(crate) global_class_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_interface_map: &'a HashMap<String, String>,
    pub(crate) global_interface_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_enum_map: &'a HashMap<String, String>,
    pub(crate) global_enum_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) global_module_map: &'a HashMap<String, String>,
    pub(crate) global_module_file_map: &'a HashMap<String, PathBuf>,
    pub(crate) project_symbol_lookup: &'a ProjectSymbolLookup,
}

pub(crate) fn run_rewrite_prep_phase(
    build_timings: &mut BuildTimings,
    inputs: RewritePrepInputs<'_>,
) -> RewritePreparation {
    print_cli_step("Rewriting project graph");
    build_timings.measure_step("rewrite prep", || {
        prepare_rewrite_inputs(
            inputs.parsed_files,
            inputs.entry_path,
            inputs.previous_dependency_graph,
            inputs.body_only_changed,
            inputs.api_changed,
            inputs.dependent_api_impact,
        )
    })
}

pub(crate) fn build_rewrite_fingerprint_context<'a>(
    inputs: RewriteContextInputs<'a>,
) -> RewriteFingerprintContext<'a> {
    RewriteFingerprintContext {
        namespace_functions: inputs.namespace_functions,
        global_function_map: inputs.global_function_map,
        global_function_file_map: inputs.global_function_file_map,
        namespace_classes: inputs.namespace_classes,
        global_class_map: inputs.global_class_map,
        global_class_file_map: inputs.global_class_file_map,
        global_interface_map: inputs.global_interface_map,
        global_interface_file_map: inputs.global_interface_file_map,
        global_enum_map: inputs.global_enum_map,
        global_enum_file_map: inputs.global_enum_file_map,
        namespace_modules: inputs.namespace_modules,
        global_module_map: inputs.global_module_map,
        global_module_file_map: inputs.global_module_file_map,
        namespace_api_fingerprints: inputs.namespace_api_fingerprints,
        file_api_fingerprints: inputs.file_api_fingerprints,
        symbol_lookup: Arc::new(inputs.project_symbol_lookup.clone()),
    }
}
