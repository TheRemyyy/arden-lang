use crate::cache::{BuildTimings, RewrittenProjectUnit};
use crate::cli::output::print_cli_step;
use crate::linker::LinkConfig;
use crate::specialization::combined_program_for_files;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
enum FullCodegenPhaseError {
    EmitLlvmCodegen(String),
    FullProgramCodegen(String),
}

impl fmt::Display for FullCodegenPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmitLlvmCodegen(message) | Self::FullProgramCodegen(message) => {
                write!(f, "{message}")
            }
        }
    }
}

impl From<FullCodegenPhaseError> for String {
    fn from(value: FullCodegenPhaseError) -> Self {
        value.to_string()
    }
}

pub(crate) enum FullCodegenRoute {
    EmitLlvmCompleted,
    FullProgramCompleted,
    ObjectsRequired,
}

pub(crate) struct FullCodegenInputs<'a, 'b> {
    pub(crate) rewritten_files: &'a [RewrittenProjectUnit],
    pub(crate) entry_path: &'a Path,
    pub(crate) output_path: &'a Path,
    pub(crate) emit_llvm: bool,
    pub(crate) link: &'a LinkConfig<'b>,
}

pub(crate) fn run_full_codegen_phase(
    build_timings: &mut BuildTimings,
    inputs: FullCodegenInputs<'_, '_>,
) -> Result<FullCodegenRoute, String> {
    run_full_codegen_phase_impl(build_timings, inputs).map_err(Into::into)
}

fn run_full_codegen_phase_impl(
    build_timings: &mut BuildTimings,
    inputs: FullCodegenInputs<'_, '_>,
) -> Result<FullCodegenRoute, FullCodegenPhaseError> {
    if inputs.emit_llvm {
        print_cli_step("Compiling program");
        let combined_program = combined_program_for_files(inputs.rewritten_files);
        build_timings
            .measure("full codegen", || {
                crate::compile_program_ast(
                    &combined_program,
                    inputs.entry_path,
                    inputs.output_path,
                    true,
                    inputs.link,
                )
            })
            .map_err(FullCodegenPhaseError::EmitLlvmCodegen)?;
        build_timings.record_counts("full codegen", &[("files", inputs.rewritten_files.len())]);
        return Ok(FullCodegenRoute::EmitLlvmCompleted);
    }

    if inputs
        .rewritten_files
        .iter()
        .any(|unit| unit.has_specialization_demand)
    {
        print_cli_step("Compiling program");
        let combined_program = combined_program_for_files(inputs.rewritten_files);
        build_timings
            .measure("full codegen", || {
                crate::compile_program_ast(
                    &combined_program,
                    inputs.entry_path,
                    inputs.output_path,
                    false,
                    inputs.link,
                )
            })
            .map_err(FullCodegenPhaseError::FullProgramCodegen)?;
        build_timings.record_counts("full codegen", &[("files", inputs.rewritten_files.len())]);
        return Ok(FullCodegenRoute::FullProgramCompleted);
    }

    Ok(FullCodegenRoute::ObjectsRequired)
}
