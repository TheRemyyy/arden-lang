use crate::cache::{BuildTimings, RewrittenProjectUnit};
use crate::cli::output::print_cli_step;
use crate::linker::LinkConfig;
use crate::specialization::combined_program_for_files;
use std::path::Path;

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
    if inputs.emit_llvm {
        print_cli_step("Compiling program");
        let combined_program = combined_program_for_files(inputs.rewritten_files);
        build_timings.measure("full codegen", || {
            crate::compile_program_ast(
                &combined_program,
                inputs.entry_path,
                inputs.output_path,
                true,
                inputs.link,
            )
        })?;
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
        build_timings.measure("full codegen", || {
            crate::compile_program_ast(
                &combined_program,
                inputs.entry_path,
                inputs.output_path,
                false,
                inputs.link,
            )
        })?;
        build_timings.record_counts("full codegen", &[("files", inputs.rewritten_files.len())]);
        return Ok(FullCodegenRoute::FullProgramCompleted);
    }

    Ok(FullCodegenRoute::ObjectsRequired)
}
