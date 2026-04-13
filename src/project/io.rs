use crate::project::types::ProjectTomlRoot;
use crate::project::ProjectConfig;
use std::fs;
use std::path::Path;

impl ProjectConfig {
    /// Load project config from arden.toml
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read project file '{}': {}", path.display(), e))?;

        match toml::from_str::<ProjectConfig>(&content) {
            Ok(config) => Ok(config),
            Err(root_error) => match toml::from_str::<ProjectTomlRoot>(&content) {
                Ok(wrapper) => Ok(wrapper.project),
                Err(project_table_error) => Err(format!(
                    "Failed to parse project file '{}': root shape error: {}; [project] table shape error: {}",
                    path.display(),
                    root_error,
                    project_table_error
                )),
            },
        }
    }

    /// Save project config to arden.toml
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            format!(
                "Failed to serialize project for '{}': {}",
                path.display(),
                e
            )
        })?;

        fs::write(path, content)
            .map_err(|e| format!("Failed to write project file '{}': {}", path.display(), e))?;

        Ok(())
    }
}
