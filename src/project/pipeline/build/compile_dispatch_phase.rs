use super::{
    finish_full_program_build, run_full_codegen_phase, run_object_pipeline, FullCodegenInputs,
    FullCodegenRoute, FullProgramFinishInputs, ObjectPipelineInputs,
};
use crate::cache::{BuildTimings, ParsedProjectUnit, ProjectSymbolLookup, RewrittenProjectUnit};
use crate::linker::LinkConfig;
use crate::symbol_lookup::GlobalSymbolMaps;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum CompileDispatchPhaseError {
    FullCodegen(String),
    FullProgramFinalize(String),
    ObjectPipeline(String),
}

impl fmt::Display for CompileDispatchPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FullCodegen(message)
            | Self::FullProgramFinalize(message)
            | Self::ObjectPipeline(message) => write!(f, "{message}"),
        }
    }
}

impl From<CompileDispatchPhaseError> for String {
    fn from(value: CompileDispatchPhaseError) -> Self {
        value.to_string()
    }
}

pub(crate) enum CompileDispatchOutcome {
    ContinueFinalize,
    Completed,
}

pub(crate) struct CompileDispatchInputs<'a, 'b> {
    pub(crate) rewritten_files: &'a [RewrittenProjectUnit],
    pub(crate) entry_path: &'a Path,
    pub(crate) output_path: &'a Path,
    pub(crate) emit_llvm: bool,
    pub(crate) link: &'a LinkConfig<'b>,
    pub(crate) project_root: &'a Path,
    pub(crate) config_name: &'a str,
    pub(crate) fingerprint: &'a str,
    pub(crate) parsed_files: &'a [ParsedProjectUnit],
    pub(crate) file_dependency_graph: &'a HashMap<PathBuf, HashSet<PathBuf>>,
    pub(crate) entry_namespace: &'a str,
    pub(crate) project_symbol_lookup: &'a ProjectSymbolLookup,
    pub(crate) global_maps: GlobalSymbolMaps<'a>,
}

pub(crate) fn run_compile_dispatch_phase(
    build_timings: &mut BuildTimings,
    inputs: CompileDispatchInputs<'_, '_>,
) -> Result<CompileDispatchOutcome, String> {
    run_compile_dispatch_phase_impl(build_timings, inputs).map_err(Into::into)
}

fn run_compile_dispatch_phase_impl(
    build_timings: &mut BuildTimings,
    inputs: CompileDispatchInputs<'_, '_>,
) -> Result<CompileDispatchOutcome, CompileDispatchPhaseError> {
    match run_full_codegen_phase(
        build_timings,
        FullCodegenInputs {
            rewritten_files: inputs.rewritten_files,
            entry_path: inputs.entry_path,
            output_path: inputs.output_path,
            emit_llvm: inputs.emit_llvm,
            link: inputs.link,
        },
    )
    .map_err(CompileDispatchPhaseError::FullCodegen)?
    {
        FullCodegenRoute::EmitLlvmCompleted => Ok(CompileDispatchOutcome::ContinueFinalize),
        FullCodegenRoute::FullProgramCompleted => {
            finish_full_program_build(
                build_timings,
                FullProgramFinishInputs {
                    project_root: inputs.project_root,
                    config_name: inputs.config_name,
                    output_path: inputs.output_path,
                    fingerprint: inputs.fingerprint,
                },
            )
            .map_err(CompileDispatchPhaseError::FullProgramFinalize)?;
            Ok(CompileDispatchOutcome::Completed)
        }
        FullCodegenRoute::ObjectsRequired => {
            run_object_pipeline(
                build_timings,
                ObjectPipelineInputs {
                    project_root: inputs.project_root,
                    output_path: inputs.output_path,
                    parsed_files: inputs.parsed_files,
                    rewritten_files: inputs.rewritten_files,
                    file_dependency_graph: inputs.file_dependency_graph,
                    link: inputs.link,
                    entry_namespace: inputs.entry_namespace,
                    project_symbol_lookup: inputs.project_symbol_lookup,
                    global_maps: inputs.global_maps,
                },
            )
            .map_err(CompileDispatchPhaseError::ObjectPipeline)?;
            Ok(CompileDispatchOutcome::ContinueFinalize)
        }
    }
}
