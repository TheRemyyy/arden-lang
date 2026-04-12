use crate::cache::{
    save_cached_fingerprint, save_dependency_graph_cache, save_semantic_cached_fingerprint,
};
use crate::cache::{BuildTimings, DependencyGraphCache};
use crate::cli::output::print_cli_artifact_result;
use std::path::Path;

pub(crate) struct FinalizeBuildInputs<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) config_name: &'a str,
    pub(crate) output_path: &'a Path,
    pub(crate) fingerprint: &'a str,
    pub(crate) semantic_fingerprint: &'a str,
    pub(crate) current_dependency_graph_cache: &'a DependencyGraphCache,
}

pub(crate) struct FullProgramFinishInputs<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) config_name: &'a str,
    pub(crate) output_path: &'a Path,
    pub(crate) fingerprint: &'a str,
}

pub(crate) fn finish_full_program_build(
    build_timings: &mut BuildTimings,
    inputs: FullProgramFinishInputs<'_>,
) -> Result<(), String> {
    save_cached_fingerprint(inputs.project_root, inputs.fingerprint)?;
    print_cli_artifact_result(
        "Built",
        inputs.config_name,
        inputs.output_path,
        build_timings.started_at.elapsed(),
    );
    build_timings.print();
    Ok(())
}

pub(crate) fn finalize_completed_build(
    build_timings: &mut BuildTimings,
    inputs: FinalizeBuildInputs<'_>,
) -> Result<(), String> {
    build_timings.measure("build cache save", || {
        save_cached_fingerprint(inputs.project_root, inputs.fingerprint)?;
        save_semantic_cached_fingerprint(inputs.project_root, inputs.semantic_fingerprint)?;
        save_dependency_graph_cache(inputs.project_root, inputs.current_dependency_graph_cache)
    })?;

    print_cli_artifact_result(
        "Built",
        inputs.config_name,
        inputs.output_path,
        build_timings.started_at.elapsed(),
    );
    build_timings.print();
    Ok(())
}
