use crate::cache::{
    save_cached_fingerprint, save_dependency_graph_cache, save_semantic_cached_fingerprint,
};
use crate::cache::{BuildTimings, DependencyGraphCache};
use crate::cli::output::print_cli_artifact_result;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
enum FinishPhaseError {
    FullProgramCacheSave(String),
    BuildCacheSave(String),
}

impl fmt::Display for FinishPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FullProgramCacheSave(message) | Self::BuildCacheSave(message) => {
                write!(f, "{message}")
            }
        }
    }
}

impl From<FinishPhaseError> for String {
    fn from(value: FinishPhaseError) -> Self {
        value.to_string()
    }
}

impl From<String> for FinishPhaseError {
    fn from(value: String) -> Self {
        Self::BuildCacheSave(value)
    }
}

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
    finish_full_program_build_impl(build_timings, inputs).map_err(Into::into)
}

fn finish_full_program_build_impl(
    build_timings: &mut BuildTimings,
    inputs: FullProgramFinishInputs<'_>,
) -> Result<(), FinishPhaseError> {
    save_cached_fingerprint(inputs.project_root, inputs.fingerprint)
        .map_err(FinishPhaseError::FullProgramCacheSave)?;
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
    finalize_completed_build_impl(build_timings, inputs).map_err(Into::into)
}

fn finalize_completed_build_impl(
    build_timings: &mut BuildTimings,
    inputs: FinalizeBuildInputs<'_>,
) -> Result<(), FinishPhaseError> {
    build_timings
        .measure("build cache save", || {
            save_cached_fingerprint(inputs.project_root, inputs.fingerprint)?;
            save_semantic_cached_fingerprint(inputs.project_root, inputs.semantic_fingerprint)?;
            save_dependency_graph_cache(inputs.project_root, inputs.current_dependency_graph_cache)
        })
        .map_err(FinishPhaseError::BuildCacheSave)?;

    print_cli_artifact_result(
        "Built",
        inputs.config_name,
        inputs.output_path,
        build_timings.started_at.elapsed(),
    );
    build_timings.print();
    Ok(())
}
