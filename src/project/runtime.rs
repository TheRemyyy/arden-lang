use super::{OutputKind, ProjectConfig};
use colored::Colorize;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum ProjectRuntimeError {
    NotRunnable(String),
}

impl fmt::Display for ProjectRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRunnable(message) => write!(f, "{message}"),
        }
    }
}

impl From<ProjectRuntimeError> for String {
    fn from(value: ProjectRuntimeError) -> Self {
        value.to_string()
    }
}

pub(crate) fn ensure_project_is_runnable(output_kind: &OutputKind) -> Result<(), String> {
    ensure_project_is_runnable_impl(output_kind).map_err(Into::into)
}

fn ensure_project_is_runnable_impl(output_kind: &OutputKind) -> Result<(), ProjectRuntimeError> {
    if *output_kind == OutputKind::Bin {
        return Ok(());
    }

    Err(ProjectRuntimeError::NotRunnable(format!(
        "{}: `arden run` requires `output_kind = \"bin\"`, found {:?}. Use `arden build` for library targets.",
        "error".red().bold(),
        output_kind
    )))
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
