use super::{run_import_check_phase, run_rewrite_phase, ImportCheckInputs, RewritePhaseInputs};
use crate::cache::{BuildTimings, ParsedProjectUnit, RewrittenProjectUnit};
use crate::cli::output::print_cli_step;
use crate::dependency::RewriteFingerprintContext;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(crate) struct RewritePipelineInputs<'a> {
    pub(crate) do_check: bool,
    pub(crate) project_root: &'a Path,
    pub(crate) parsed_files: &'a [ParsedProjectUnit],
    pub(crate) global_function_map: &'a HashMap<String, String>,
    pub(crate) entry_namespace: &'a str,
    pub(crate) rewrite_fingerprint_ctx: &'a RewriteFingerprintContext<'a>,
    pub(crate) safe_rewrite_cache_files: &'a HashSet<PathBuf>,
    pub(crate) namespace_functions: &'a HashMap<String, HashSet<String>>,
    pub(crate) namespace_class_map: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_class_map: &'a HashMap<String, String>,
    pub(crate) namespace_interface_map: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_interface_map: &'a HashMap<String, String>,
    pub(crate) namespace_enum_map: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_enum_map: &'a HashMap<String, String>,
    pub(crate) namespace_module_map: &'a HashMap<String, HashSet<String>>,
    pub(crate) global_module_map: &'a HashMap<String, String>,
}

pub(crate) fn run_rewrite_pipeline(
    build_timings: &mut BuildTimings,
    inputs: RewritePipelineInputs<'_>,
) -> Result<Vec<RewrittenProjectUnit>, String> {
    if inputs.do_check {
        print_cli_step("Checking imports");
        run_import_check_phase(
            build_timings,
            ImportCheckInputs {
                project_root: inputs.project_root,
                parsed_files: inputs.parsed_files,
                global_function_map: inputs.global_function_map,
                entry_namespace: inputs.entry_namespace,
                rewrite_fingerprint_ctx: inputs.rewrite_fingerprint_ctx,
            },
        )?;
    }

    run_rewrite_phase(
        build_timings,
        RewritePhaseInputs {
            project_root: inputs.project_root,
            parsed_files: inputs.parsed_files,
            safe_rewrite_cache_files: inputs.safe_rewrite_cache_files,
            entry_namespace: inputs.entry_namespace,
            rewrite_fingerprint_ctx: inputs.rewrite_fingerprint_ctx,
            namespace_functions: inputs.namespace_functions,
            global_function_map: inputs.global_function_map,
            namespace_class_map: inputs.namespace_class_map,
            global_class_map: inputs.global_class_map,
            namespace_interface_map: inputs.namespace_interface_map,
            global_interface_map: inputs.global_interface_map,
            namespace_enum_map: inputs.namespace_enum_map,
            global_enum_map: inputs.global_enum_map,
            namespace_module_map: inputs.namespace_module_map,
            global_module_map: inputs.global_module_map,
        },
    )
}
