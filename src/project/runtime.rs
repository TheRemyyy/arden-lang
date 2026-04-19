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

pub(crate) fn resolve_binary_output_path(output_path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        if output_path.extension().is_none() {
            return output_path.with_extension("exe");
        }
    }

    output_path.to_path_buf()
}

pub(crate) fn resolve_project_output_path(project_root: &Path, config: &ProjectConfig) -> PathBuf {
    let output_path = project_root.join(&config.output);
    if config.output_kind == OutputKind::Bin {
        return resolve_binary_output_path(&output_path);
    }
    output_path
}

#[cfg(test)]
mod tests {
    use super::resolve_binary_output_path;
    use std::path::Path;

    #[test]
    fn resolve_binary_output_path_keeps_existing_extension() {
        let path = resolve_binary_output_path(Path::new("build/output.bin"));
        assert_eq!(path, Path::new("build/output.bin"));
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_binary_output_path_keeps_extensionless_unix_path() {
        let path = resolve_binary_output_path(Path::new("build/output"));
        assert_eq!(path, Path::new("build/output"));
    }

    #[cfg(windows)]
    #[test]
    fn resolve_binary_output_path_appends_exe_for_extensionless_windows_binary() {
        let path = resolve_binary_output_path(Path::new("build/output"));
        assert_eq!(path, Path::new("build/output.exe"));
    }
}
