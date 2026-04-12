use crate::project::types::ProjectTomlRoot;
use crate::project::ProjectConfig;
use std::fs;
use std::path::Path;

impl ProjectConfig {
    /// Load project config from arden.toml
    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read project file: {}", e))?;

        if let Ok(config) = toml::from_str::<ProjectConfig>(&content) {
            return Ok(config);
        }
        if let Ok(wrapper) = toml::from_str::<ProjectTomlRoot>(&content) {
            return Ok(wrapper.project);
        }

        toml::from_str::<ProjectConfig>(&content)
            .map_err(|e| format!("Failed to parse project file: {}", e))
    }

    /// Save project config to arden.toml
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize project: {}", e))?;

        fs::write(path, content).map_err(|e| format!("Failed to write project file: {}", e))?;

        Ok(())
    }
}
