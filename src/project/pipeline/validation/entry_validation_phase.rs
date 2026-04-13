use crate::cache::ParsedProjectUnit;
use std::fs;
use std::path::Path;

pub(crate) fn run_entry_validation_phase(
    do_check: bool,
    entry_path: &Path,
    parsed_files: &[ParsedProjectUnit],
) -> Result<(), String> {
    if do_check {
        return Ok(());
    }

    let entry_source = fs::read_to_string(entry_path).map_err(|error| {
        format!(
            "{}: Failed to read entry file '{}': {}",
            crate::cli_error("error"),
            entry_path.display(),
            error
        )
    })?;
    let entry_program = parsed_files
        .iter()
        .find(|unit| unit.file == entry_path)
        .map(|unit| &unit.program)
        .ok_or_else(|| {
            format!(
                "{}: Entry file '{}' was not parsed (parsed units: {})",
                crate::cli_error("error"),
                entry_path.display(),
                parsed_files.len()
            )
        })?;
    let entry_filename = entry_path.to_string_lossy();

    crate::validate_entry_main_signature(entry_program, &entry_source, &entry_filename)
}
