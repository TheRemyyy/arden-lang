mod bindgen;
mod check;
mod debug;
mod info;
mod perf;
mod project_init;
mod quality;
mod run;
mod testing;

pub(crate) use bindgen::bindgen_header;
pub(crate) use check::check_command;
#[cfg(test)]
pub(crate) use check::check_file;
pub(crate) use debug::{lex_file, parse_file};
pub(crate) use info::show_project_info;
pub(crate) use perf::{bench_target, profile_target};
pub(crate) use project_init::create_new_project as new_project;
pub(crate) use quality::{fix_target, format_targets, lint_target};
pub(crate) use run::{run_project, run_single_file};
pub(crate) use testing::run_tests;
