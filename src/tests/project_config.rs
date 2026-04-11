use super::{TestExpectErrExt, TestExpectExt};
use crate::project::{find_project_root, OutputKind, ProjectConfig};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .must("system time should be after unix epoch")
        .as_nanos();
    let base_temp = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    base_temp.join(format!("{prefix}_{unique}"))
}

#[test]
fn defaults_include_linker_configuration_fields() {
    let config = ProjectConfig::new("demo");
    assert_eq!(config.output_kind, OutputKind::Bin);
    assert!(config.link_libs.is_empty());
    assert!(config.link_search.is_empty());
    assert!(config.link_args.is_empty());
}

#[test]
fn parses_linker_configuration_from_toml() {
    let config: ProjectConfig = toml::from_str(
        r#"
name = "demo"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
output = "demo"
output_kind = "shared"
link_libs = ["ssl", "crypto"]
link_search = ["native/lib", "/usr/local/lib"]
link_args = ["-Wl,--as-needed"]
"#,
    )
    .must("project config parses");

    assert_eq!(config.output_kind, OutputKind::Shared);
    assert_eq!(config.link_libs, vec!["ssl", "crypto"]);
    assert_eq!(config.link_search, vec!["native/lib", "/usr/local/lib"]);
    assert_eq!(config.link_args, vec!["-Wl,--as-needed"]);
}

#[test]
fn loads_project_table_toml_shape() {
    let dir = std::env::temp_dir();
    let path = dir.join("arden_project_table_shape_test.toml");
    let content = r#"
[project]
name = "demo"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
output = "demo"
"#;
    std::fs::write(&path, content).must("write temporary toml");
    let config = ProjectConfig::load(&path).must("project table shape should load");
    let _ = std::fs::remove_file(&path);
    assert_eq!(config.name, "demo");
    assert_eq!(config.entry, "src/main.arden");
}

#[test]
fn validate_rejects_entry_outside_project_root() {
    let project_root = unique_temp_dir("arden_project_validate_entry_escape");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("entry file should be written");

    let escaped_file = project_root
        .parent()
        .must("temp dir should have parent")
        .join("escaped_entry.arden");
    std::fs::write(&escaped_file, "function main(): None { return None; }\n")
        .must("escaped file should be written");

    let mut config = ProjectConfig::new("demo");
    config.entry = "../escaped_entry.arden".to_string();
    config.files = vec!["../escaped_entry.arden".to_string()];

    let error = config
        .validate(&project_root)
        .must_err("entry outside project root should be rejected");

    let _ = std::fs::remove_file(&escaped_file);
    let _ = std::fs::remove_dir_all(&project_root);

    assert!(error.contains("outside the project root"), "{error}");
}

#[test]
fn validate_rejects_source_file_outside_project_root() {
    let project_root = unique_temp_dir("arden_project_validate_file_escape");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("entry file should be written");

    let escaped_file = project_root
        .parent()
        .must("temp dir should have parent")
        .join("escaped_module.arden");
    std::fs::write(&escaped_file, "function helper(): None { return None; }\n")
        .must("escaped module should be written");

    let mut config = ProjectConfig::new("demo");
    config.files.push("../escaped_module.arden".to_string());

    let error = config
        .validate(&project_root)
        .must_err("source file outside project root should be rejected");

    let _ = std::fs::remove_file(&escaped_file);
    let _ = std::fs::remove_dir_all(&project_root);

    assert!(error.contains("outside the project root"), "{error}");
}

#[test]
fn validate_rejects_directory_entry_path() {
    let project_root = unique_temp_dir("arden_project_validate_entry_dir");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");

    let mut config = ProjectConfig::new("demo");
    config.entry = "src".to_string();
    config.files = vec!["src".to_string()];

    let error = config
        .validate(&project_root)
        .must_err("directory entry path should be rejected");

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(error.contains("must resolve to a file"), "{error}");
}

#[test]
fn validate_rejects_directory_source_path() {
    let project_root = unique_temp_dir("arden_project_validate_file_dir");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("entry file should be written");
    std::fs::create_dir_all(src_dir.join("nested")).must("nested source dir should exist");

    let mut config = ProjectConfig::new("demo");
    config.files.push("src/nested".to_string());

    let error = config
        .validate(&project_root)
        .must_err("directory source path should be rejected");

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(error.contains("must resolve to a file"), "{error}");
}

#[test]
fn validate_rejects_non_arden_entry_path() {
    let project_root = unique_temp_dir("arden_project_validate_entry_non_arden");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(src_dir.join("main.txt"), "not arden\n").must("entry file should be written");

    let mut config = ProjectConfig::new("demo");
    config.entry = "src/main.txt".to_string();
    config.files = vec!["src/main.txt".to_string()];

    let error = config
        .validate(&project_root)
        .must_err("non-arden entry path should be rejected");

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(
        error.contains("must resolve to an .arden source file"),
        "{error}"
    );
}

#[test]
fn validate_rejects_non_arden_source_path() {
    let project_root = unique_temp_dir("arden_project_validate_source_non_arden");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("entry file should be written");
    std::fs::write(src_dir.join("helper.txt"), "not arden\n").must("helper file should be written");

    let mut config = ProjectConfig::new("demo");
    config.files.push("src/helper.txt".to_string());

    let error = config
        .validate(&project_root)
        .must_err("non-arden source path should be rejected");

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(
        error.contains("must resolve to an .arden source file"),
        "{error}"
    );
}

#[test]
fn validate_rejects_output_path_outside_project_root() {
    let project_root = unique_temp_dir("arden_project_validate_output_escape");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("entry file should be written");

    let mut config = ProjectConfig::new("demo");
    config.output = "../escaped-output/demo".to_string();

    let error = config
        .validate(&project_root)
        .must_err("output outside project root should be rejected");

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(error.contains("outside the project root"), "{error}");
}

#[test]
fn validate_rejects_output_path_matching_project_config() {
    let project_root = unique_temp_dir("arden_project_validate_output_config_collision");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("entry file should be written");

    let mut config = ProjectConfig::new("demo");
    config.output = "arden.toml".to_string();

    let error = config
        .validate(&project_root)
        .must_err("output matching arden.toml should be rejected");

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(error.contains("project config"), "{error}");
}

#[test]
fn validate_rejects_output_path_matching_entry_file() {
    let project_root = unique_temp_dir("arden_project_validate_output_entry_collision");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("entry file should be written");

    let mut config = ProjectConfig::new("demo");
    config.output = "src/main.arden".to_string();

    let error = config
        .validate(&project_root)
        .must_err("output matching entry should be rejected");

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(error.contains("overwrite source file"), "{error}");
}

#[test]
fn validate_rejects_output_path_matching_secondary_source_file() {
    let project_root = unique_temp_dir("arden_project_validate_output_source_collision");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("entry file should be written");
    std::fs::write(
        src_dir.join("helper.arden"),
        "function helper(): None { return None; }\n",
    )
    .must("helper file should be written");

    let mut config = ProjectConfig::new("demo");
    config.files.push("src/helper.arden".to_string());
    config.output = "src/helper.arden".to_string();

    let error = config
        .validate(&project_root)
        .must_err("output matching secondary source should be rejected");

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(error.contains("overwrite source file"), "{error}");
}

#[test]
fn validate_rejects_duplicate_source_files() {
    let project_root = unique_temp_dir("arden_project_validate_duplicate_files");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        src_dir.join("main.arden"),
        "function main(): None { return None; }\n",
    )
    .must("entry file should be written");

    let mut config = ProjectConfig::new("demo");
    config.files = vec!["src/main.arden".to_string(), "src/main.arden".to_string()];

    let error = config
        .validate(&project_root)
        .must_err("duplicate source file should be rejected");

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(error.contains("Duplicate source file"), "{error}");
}

#[test]
fn find_project_root_accepts_source_file_path() {
    let project_root = unique_temp_dir("arden_project_find_root_file");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(project_root.join("arden.toml"), "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n")
        .must("project config should be written");
    let source_file = src_dir.join("main.arden");
    std::fs::write(&source_file, "function main(): None { return None; }\n")
        .must("source file should be written");

    let discovered = find_project_root(&source_file);

    let _ = std::fs::remove_dir_all(&project_root);

    assert_eq!(discovered.as_deref(), Some(project_root.as_path()));
}

#[test]
fn is_in_project_accepts_source_file_path() {
    let project_root = unique_temp_dir("arden_project_is_in_project_file");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(project_root.join("arden.toml"), "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n")
        .must("project config should be written");
    let source_file = src_dir.join("main.arden");
    std::fs::write(&source_file, "function main(): None { return None; }\n")
        .must("source file should be written");

    let result = find_project_root(&source_file).is_some();

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(result);
}

#[test]
fn find_project_root_accepts_nonexistent_source_file_path() {
    let project_root = unique_temp_dir("arden_project_find_root_missing_file");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(project_root.join("arden.toml"), "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n")
        .must("project config should be written");
    let future_source_file = src_dir.join("new_file.arden");

    let discovered = find_project_root(&future_source_file);

    let _ = std::fs::remove_dir_all(&project_root);

    assert_eq!(discovered.as_deref(), Some(project_root.as_path()));
}

#[test]
fn find_project_root_accepts_existing_directory_with_dot_in_name() {
    let parent_root = unique_temp_dir("arden_project_find_root_dotted_dir_parent");
    let project_root = parent_root.join("demo.v1");
    std::fs::create_dir_all(project_root.join("src")).must("project src dir should exist");
    std::fs::write(
        project_root.join("arden.toml"),
        "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n",
    )
    .must("project config should be written");

    let discovered = find_project_root(&project_root);

    let _ = std::fs::remove_dir_all(&parent_root);

    assert_eq!(discovered.as_deref(), Some(project_root.as_path()));
}

#[test]
fn find_project_root_accepts_relative_existing_directory_inside_project() {
    let project_root = unique_temp_dir("arden_project_find_root_relative_dir");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");
    std::fs::write(
        project_root.join("arden.toml"),
        "name = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.arden\"\nfiles = [\"src/main.arden\"]\n",
    )
    .must("project config should be written");

    let previous_dir = std::env::current_dir().must("current dir");
    std::env::set_current_dir(&project_root).must("enter project root");
    let discovered = find_project_root(Path::new("src"));
    let _ = std::env::set_current_dir(previous_dir);

    let _ = std::fs::remove_dir_all(&project_root);

    assert_eq!(discovered.as_deref(), Some(project_root.as_path()));
}

#[test]
fn find_project_root_rejects_directory_named_arden_toml() {
    let project_root = unique_temp_dir("arden_project_find_root_fake_config_dir");
    let fake_config_dir = project_root.join("arden.toml");
    let src_dir = project_root.join("src");
    std::fs::create_dir_all(&fake_config_dir).must("fake arden.toml directory should exist");
    std::fs::create_dir_all(&src_dir).must("project src dir should be created");

    let discovered = find_project_root(&src_dir);

    let _ = std::fs::remove_dir_all(&project_root);

    assert!(discovered.is_none());
}
