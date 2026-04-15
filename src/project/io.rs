use crate::format_cli_path;
use crate::project::types::ProjectTomlRoot;
use crate::project::ProjectConfig;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
enum ProjectIoError {
    Read(String),
    Parse(String),
    Serialize(String),
    Write(String),
}

impl fmt::Display for ProjectIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(message)
            | Self::Parse(message)
            | Self::Serialize(message)
            | Self::Write(message) => write!(f, "{message}"),
        }
    }
}

impl From<ProjectIoError> for String {
    fn from(value: ProjectIoError) -> Self {
        value.to_string()
    }
}

impl ProjectConfig {
    /// Load project config from arden.toml
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::load_impl(path).map_err(Into::into)
    }

    fn load_impl(path: &Path) -> Result<Self, ProjectIoError> {
        let content = fs::read_to_string(path).map_err(|e| {
            ProjectIoError::Read(format!(
                "Failed to read project file '{}': {}",
                format_cli_path(path),
                e
            ))
        })?;

        match toml::from_str::<ProjectConfig>(&content) {
            Ok(config) => Ok(config),
            Err(root_error) => match toml::from_str::<ProjectTomlRoot>(&content) {
                Ok(wrapper) => Ok(wrapper.project),
                Err(project_table_error) => Err(ProjectIoError::Parse(format!(
                    "Failed to parse project file '{}': root shape error: {}; [project] table shape error: {}",
                    format_cli_path(path),
                    root_error,
                    project_table_error
                ))),
            },
        }
    }

    /// Save project config to arden.toml
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.save_impl(path).map_err(Into::into)
    }

    fn save_impl(&self, path: &Path) -> Result<(), ProjectIoError> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            ProjectIoError::Serialize(format!(
                "Failed to serialize project for '{}': {}",
                format_cli_path(path),
                e
            ))
        })?;

        fs::write(path, content).map_err(|e| {
            ProjectIoError::Write(format!(
                "Failed to write project file '{}': {}",
                format_cli_path(path),
                e
            ))
        })?;

        Ok(())
    }
}
