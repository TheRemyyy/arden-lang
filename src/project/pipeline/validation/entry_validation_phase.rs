use crate::cache::ParsedProjectUnit;
use crate::cli::output::format_cli_path;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
enum EntryValidationPhaseError {
    ReadEntrySource(String),
    MissingParsedEntry(String),
    MainSignature(String),
}

impl fmt::Display for EntryValidationPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadEntrySource(message)
            | Self::MissingParsedEntry(message)
            | Self::MainSignature(message) => write!(f, "{message}"),
        }
    }
}

impl From<EntryValidationPhaseError> for String {
    fn from(value: EntryValidationPhaseError) -> Self {
        value.to_string()
    }
}

impl From<String> for EntryValidationPhaseError {
    fn from(value: String) -> Self {
        Self::MainSignature(value)
    }
}

pub(crate) fn run_entry_validation_phase(
    do_check: bool,
    entry_path: &Path,
    parsed_files: &[ParsedProjectUnit],
) -> Result<(), String> {
    run_entry_validation_phase_impl(do_check, entry_path, parsed_files).map_err(Into::into)
}

fn run_entry_validation_phase_impl(
    do_check: bool,
    entry_path: &Path,
    parsed_files: &[ParsedProjectUnit],
) -> Result<(), EntryValidationPhaseError> {
    if do_check {
        return Ok(());
    }

    let entry_source = fs::read_to_string(entry_path).map_err(|error| {
        EntryValidationPhaseError::ReadEntrySource(format!(
            "{}: Failed to read entry file '{}': {}",
            crate::cli_error("error"),
            format_cli_path(entry_path),
            error
        ))
    })?;
    let entry_program = parsed_files
        .iter()
        .find(|unit| unit.file == entry_path)
        .map(|unit| &unit.program)
        .ok_or_else(|| {
            EntryValidationPhaseError::MissingParsedEntry(format!(
                "{}: Entry file '{}' was not parsed (parsed units: {})",
                crate::cli_error("error"),
                format_cli_path(entry_path),
                parsed_files.len()
            ))
        })?;
    let entry_filename = format_cli_path(entry_path);

    crate::validate_entry_main_signature(entry_program, &entry_source, &entry_filename)
        .map_err(EntryValidationPhaseError::MainSignature)
}
