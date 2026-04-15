use crate::bindgen;
use crate::cli::output::{cli_path, cli_soft, cli_success};
use std::fmt;
use std::path::Path;

#[derive(Debug)]
enum BindgenCommandError {
    Generate(String),
}

impl fmt::Display for BindgenCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generate(message) => write!(f, "{message}"),
        }
    }
}

impl From<BindgenCommandError> for String {
    fn from(value: BindgenCommandError) -> Self {
        value.to_string()
    }
}

impl From<String> for BindgenCommandError {
    fn from(value: String) -> Self {
        Self::Generate(value)
    }
}

pub(crate) fn bindgen_header(header: &Path, output: Option<&Path>) -> Result<(), String> {
    bindgen_header_impl(header, output).map_err(Into::into)
}

fn bindgen_header_impl(header: &Path, output: Option<&Path>) -> Result<(), BindgenCommandError> {
    let count =
        bindgen::generate_bindings(header, output).map_err(BindgenCommandError::Generate)?;
    if let Some(out) = output {
        println!(
            "{} {} binding(s) -> {}",
            cli_success("Generated"),
            count,
            cli_path(out)
        );
    } else {
        eprintln!(
            "{} {}",
            cli_success("Generated"),
            cli_soft(format!("{count} binding(s)"))
        );
    }
    Ok(())
}
