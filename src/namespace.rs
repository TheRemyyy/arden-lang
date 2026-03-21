//! Namespace and module system for Apex
//!
//! Java-style: folder = namespace, file = module
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents a fully qualified name (e.g., "utils.math.factorial")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedName {
    pub parts: Vec<String>,
}

impl QualifiedName {
    pub fn new(parts: Vec<String>) -> Self {
        Self { parts }
    }

    pub fn from_string(s: &str) -> Self {
        let parts: Vec<String> = s.split('.').map(|s| s.to_string()).collect();
        Self { parts }
    }

    pub fn to_qualified_string(&self) -> String {
        self.parts.join(".")
    }
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.parts.join("."))
    }
}

impl QualifiedName {
    /// Get the namespace part (all except last)
    pub fn namespace(&self) -> Option<String> {
        if self.parts.len() > 1 {
            Some(self.parts[..self.parts.len() - 1].join("."))
        } else {
            None
        }
    }

    /// Get just the name (last part)
    pub fn name(&self) -> String {
        self.parts.last().cloned().unwrap_or_default()
    }
}

/// Module info from a source file
#[derive(Debug, Clone)]
pub struct Module {
    /// File path relative to src/
    pub path: PathBuf,
    /// Namespace (e.g., "utils.math")
    pub namespace: String,
    /// Exported functions
    pub exports: Vec<String>,
    /// Imported modules
    pub imports: Vec<Import>,
}

#[derive(Debug, Clone)]
pub struct Import {
    /// Full path (e.g., "utils.math.*" or "utils.math.factorial")
    pub path: QualifiedName,
    /// Is wildcard import?
    pub wildcard: bool,
    /// Alias (optional)
    pub alias: Option<String>,
}

/// Namespace resolver
pub struct NamespaceResolver {
    /// All known modules by namespace
    modules: HashMap<String, Module>,
    /// Root source directory
    src_root: PathBuf,
}

impl NamespaceResolver {
    pub fn new(src_root: PathBuf) -> Self {
        Self {
            modules: HashMap::new(),
            src_root,
        }
    }

    /// Register a module from file path
    pub fn register_file(&mut self, file_path: &Path) -> Result<Module, String> {
        if !file_path.exists() {
            return Err(format!("File {} does not exist", file_path.display()));
        }
        if !file_path.is_file() {
            return Err(format!("Path {} is not a file", file_path.display()));
        }
        if file_path.extension().and_then(|ext| ext.to_str()) != Some("apex") {
            return Err(format!(
                "File {} is not an .apex source file",
                file_path.display()
            ));
        }

        let canonical_root = self.src_root.canonicalize().map_err(|e| {
            format!(
                "Failed to resolve src root '{}': {}",
                self.src_root.display(),
                e
            )
        })?;
        let canonical_file = file_path
            .canonicalize()
            .map_err(|e| format!("Failed to resolve file '{}': {}", file_path.display(), e))?;
        if !canonical_file.starts_with(&canonical_root) {
            return Err(format!(
                "File {} resolves outside the src directory '{}'",
                file_path.display(),
                canonical_root.display()
            ));
        }

        // Build the namespace from the normalized on-disk path so inputs like
        // src/subdir/../main.apex cannot leak `..` into the derived namespace.
        let relative = canonical_file
            .strip_prefix(&canonical_root)
            .map_err(|_| format!("File {} is not in src directory", file_path.display()))?;

        // Convert path to namespace
        // src/utils/math.apex → utils.math
        let namespace = path_to_namespace(relative)?;

        let module = Module {
            path: relative.to_path_buf(),
            namespace: namespace.clone(),
            exports: vec![], // Will be populated during parsing
            imports: vec![], // Will be populated during parsing
        };

        self.modules.insert(namespace.clone(), module.clone());
        Ok(module)
    }

    /// Resolve a name in given namespace context
    pub fn resolve(&self, name: &str, current_namespace: &str) -> Option<QualifiedName> {
        // 1. Check if it's already fully qualified
        if name.contains('.') {
            return Some(QualifiedName::from_string(name));
        }

        // 2. Check current namespace
        let current = QualifiedName::from_string(current_namespace);
        let mut parts = current.parts.clone();
        parts.push(name.to_string());
        let qualified = QualifiedName::new(parts);

        // Check if exists
        if self
            .modules
            .contains_key(&qualified.namespace().unwrap_or_default())
        {
            return Some(qualified);
        }

        // 3. Check imports of current module
        if let Some(module) = self.modules.get(current_namespace) {
            for import in &module.imports {
                if import.wildcard {
                    // Check if name exists in imported namespace
                    let mut import_parts = import.path.parts.clone();
                    import_parts.pop(); // Remove *
                    import_parts.push(name.to_string());
                    let candidate = QualifiedName::new(import_parts);
                    if self.name_exists(&candidate) {
                        return Some(candidate);
                    }
                } else if import.path.name() == name {
                    return Some(import.path.clone());
                }
            }
        }

        // 4. Name not found
        None
    }

    fn name_exists(&self, qualified: &QualifiedName) -> bool {
        if let Some(ns) = qualified.namespace() {
            if let Some(module) = self.modules.get(&ns) {
                return module.exports.contains(&qualified.name());
            }
        }
        false
    }

    /// Get mangle name for codegen
    /// utils.math.factorial → utils__math__factorial
    pub fn mangle(&self, qualified: &QualifiedName) -> String {
        qualified.parts.join("__")
    }

    /// Get all modules
    pub fn get_modules(&self) -> &HashMap<String, Module> {
        &self.modules
    }
}

/// Convert file path to namespace
/// src/utils/math.apex → utils.math
fn path_to_namespace(path: &Path) -> Result<String, String> {
    let mut parts: Vec<String> = vec![];

    for component in path.parent().unwrap_or(Path::new("")).components() {
        let name = component
            .as_os_str()
            .to_str()
            .ok_or("Invalid path component")?;
        if name != "." && name != "src" {
            parts.push(name.to_string());
        }
    }

    // Add filename without extension
    if let Some(stem) = path.file_stem() {
        let name = stem.to_str().ok_or("Invalid filename")?;
        parts.push(name.to_string());
    }

    Ok(parts.join("."))
}

/// Parse import statement from source
pub fn parse_import(line: &str) -> Option<Import> {
    let line = line.trim();

    // import utils.math.*;
    // import utils.math.factorial;
    // import utils.math as um;

    if !line.starts_with("import ") {
        return None;
    }

    let rest = &line[7..]; // Skip "import "
    let rest = rest.trim_end_matches(';').trim();

    let is_valid_import_path = |path: &str| {
        !path.is_empty()
            && !path.starts_with('.')
            && !path.ends_with('.')
            && path.split('.').all(|segment| {
                !segment.is_empty()
                    && {
                        let mut chars = segment.chars();
                        matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
                            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
                    }
            })
    };

    // Check for wildcard
    if let Some(path_str) = rest.strip_suffix(".*") {
        if !is_valid_import_path(path_str) {
            return None;
        }
        let path = QualifiedName::from_string(path_str);
        return Some(Import {
            path,
            wildcard: true,
            alias: None,
        });
    }

    // Check for alias: import utils.math as um
    if let Some(pos) = rest.find(" as ") {
        let path_str = &rest[..pos];
        let alias_str = &rest[pos + 4..];
        if !is_valid_import_path(path_str) || alias_str.is_empty() || {
            let mut chars = alias_str.chars();
            !matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
                || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        } {
            return None;
        }
        let path = QualifiedName::from_string(path_str);
        let alias = Some(alias_str.to_string());
        return Some(Import {
            path,
            wildcard: false,
            alias,
        });
    }

    // Regular import
    if !is_valid_import_path(rest) {
        return None;
    }
    let path = QualifiedName::from_string(rest);
    Some(Import {
        path,
        wildcard: false,
        alias: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_namespace() {
        let path = Path::new("utils/math.apex");
        assert_eq!(path_to_namespace(path).unwrap(), "utils.math");
    }

    #[test]
    fn test_parse_import_wildcard() {
        let import = parse_import("import utils.math.*;").unwrap();
        assert!(import.wildcard);
        assert_eq!(import.path.to_string(), "utils.math");
    }

    #[test]
    fn test_parse_import_single() {
        let import = parse_import("import utils.math.factorial;").unwrap();
        assert!(!import.wildcard);
        assert_eq!(import.path.to_string(), "utils.math.factorial");
    }

    #[test]
    fn test_rejects_wildcard_alias_import() {
        assert!(parse_import("import utils.math.* as math;").is_none());
    }

    #[test]
    fn test_rejects_invalid_import_paths() {
        assert!(parse_import("import .utils.math;").is_none());
        assert!(parse_import("import utils..math;").is_none());
        assert!(parse_import("import utils.math.;").is_none());
        assert!(parse_import("import 9utils.math;").is_none());
        assert!(parse_import("import utils.9math;").is_none());
        assert!(parse_import("import utils.math as 9alias;").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn register_file_rejects_symlinked_files_outside_src_root() {
        use std::fs;
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let temp_root = std::env::temp_dir().join(format!(
            "apex-namespace-symlink-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let src_root = temp_root.join("src");
        fs::create_dir_all(&src_root).expect("create src root");

        let outside_file = temp_root.parent().expect("temp parent").join(format!(
            "apex-namespace-outside-{}-{}.apex",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::write(&outside_file, "function stray(): None { return None; }\n")
            .expect("write outside file");

        let linked = src_root.join("linked.apex");
        symlink(&outside_file, &linked).expect("create file symlink");

        let mut resolver = NamespaceResolver::new(src_root.clone());
        let err = resolver
            .register_file(&linked)
            .expect_err("symlink outside src root should be rejected");
        assert!(
            err.contains("resolves outside the src directory"),
            "expected boundary error, got: {err}"
        );

        let _ = fs::remove_dir_all(temp_root);
        let _ = fs::remove_file(outside_file);
    }

    #[test]
    fn register_file_rejects_non_apex_files() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let temp_root = std::env::temp_dir().join(format!(
            "apex-namespace-non-apex-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let src_root = temp_root.join("src");
        fs::create_dir_all(&src_root).expect("create src root");
        let note_path = src_root.join("notes.txt");
        fs::write(&note_path, "not apex\n").expect("write note file");

        let mut resolver = NamespaceResolver::new(src_root.clone());
        let err = resolver
            .register_file(&note_path)
            .expect_err("non-apex file should be rejected");
        assert!(
            err.contains("is not an .apex source file"),
            "expected non-apex validation error, got: {err}"
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn register_file_normalizes_dotdot_segments_inside_src_root() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let temp_root = std::env::temp_dir().join(format!(
            "apex-namespace-dotdot-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let src_root = temp_root.join("src");
        let nested_dir = src_root.join("nested");
        fs::create_dir_all(&nested_dir).expect("create nested dir");
        let main_path = src_root.join("main.apex");
        fs::write(&main_path, "function main(): None { return None; }\n").expect("write main");

        let odd_path = nested_dir.join("..").join("main.apex");
        let mut resolver = NamespaceResolver::new(src_root.clone());
        let module = resolver
            .register_file(&odd_path)
            .expect("normalized path inside src root should register");

        assert_eq!(module.namespace, "main");
        assert_eq!(module.path, PathBuf::from("main.apex"));

        let _ = fs::remove_dir_all(temp_root);
    }
}
