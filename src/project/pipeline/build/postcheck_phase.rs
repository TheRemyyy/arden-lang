use super::{run_semantic_phase, SemanticPhaseInputs};
use crate::cache::BuildTimings;
use crate::cli::output::{cli_accent, cli_elapsed, cli_soft, cli_success};

pub(crate) enum PostcheckOutcome {
    ContinueBuild,
    Completed,
}

pub(crate) struct PostcheckInputs<'a> {
    pub(crate) do_check: bool,
    pub(crate) check_only: bool,
    pub(crate) config_name: &'a str,
    pub(crate) semantic_inputs: SemanticPhaseInputs<'a>,
}

pub(crate) fn run_postcheck_phase(
    build_timings: &mut BuildTimings,
    inputs: PostcheckInputs<'_>,
) -> Result<PostcheckOutcome, String> {
    if inputs.do_check {
        run_semantic_phase(build_timings, inputs.semantic_inputs)?;
    }

    if inputs.check_only {
        println!(
            "{} {} {}",
            cli_success("Check passed"),
            cli_accent(inputs.config_name),
            cli_soft(format!(
                "({})",
                cli_elapsed(build_timings.started_at.elapsed())
            ))
        );
        build_timings.print();
        return Ok(PostcheckOutcome::Completed);
    }

    Ok(PostcheckOutcome::ContinueBuild)
}
