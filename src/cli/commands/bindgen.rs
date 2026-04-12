use crate::bindgen;
use crate::cli::output::{cli_path, cli_soft, cli_success};
use std::path::Path;

pub(crate) fn bindgen_header(header: &Path, output: Option<&Path>) -> Result<(), String> {
    let count = bindgen::generate_bindings(header, output)?;
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
