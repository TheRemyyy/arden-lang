use super::{OutputKind, ProjectConfig};
use colored::Colorize;
use std::path::{Path, PathBuf};

pub(crate) fn ensure_project_is_runnable(output_kind: &OutputKind) -> Result<(), String> {
    if *output_kind == OutputKind::Bin {
        return Ok(());
    }

    Err(format!(
        "{}: `arden run` requires `output_kind = \"bin\"`, found {:?}. Use `arden build` for library targets.",
        "error".red().bold(),
        output_kind
    ))
}

pub(crate) fn resolve_project_output_path(project_root: &Path, config: &ProjectConfig) -> PathBuf {
    let output_path = project_root.join(&config.output);
    #[cfg(windows)]
    {
        if config.output_kind == OutputKind::Bin && output_path.extension().is_none() {
            return output_path.with_extension("exe");
        }
    }
    output_path
}
