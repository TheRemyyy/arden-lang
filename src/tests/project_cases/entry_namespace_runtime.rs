use super::*;
use std::fs;

#[test]
fn project_build_supports_shadowed_alias_in_helper_return_path_survives_runtime() {
    let temp_root = make_temp_project_root("shadowed-alias-helper-return-project");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nmodule M { function add1(x: Integer): Integer { return x + 1; } }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        r#"
package app;
import util as u;

class Holder {
    value: Integer;
    constructor(v: Integer) { this.value = v; }
    function get(): Integer { return this.value + 10; }
}

function fetch(v: Integer): Holder {
    u: Holder = Holder(v);
    return u;
}

function main(): Integer {
    h: Holder = fetch(2);
    return h.get();
}
"#,
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support shadowed alias in helper return path");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled shadowed-alias-helper-return binary");
    assert_eq!(
        output.status.code(),
        Some(12),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_incorrectly_filters_class_dependency_under_shadowed_alias_in_dependency_closure() {
    let temp_root = make_temp_project_root("shadowed-alias-dependency-filtering");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/lib.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("lib.arden"),
        "package util;\nclass Box { value: Integer; constructor(v: Integer) { this.value = v; } }\n",
    )
    .must("write lib");
    fs::write(
        src_dir.join("main.arden"),
        r#"
package app;
import util as u;

class Local {
    value: Integer;
    constructor(v: Integer) { this.value = v; }
    function get(): Integer { return this.value + 10; }
}

function main(): Integer {
    u: Local = Local(2);
    return u.get();
}
"#,
    )
    .must("write main");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should handle shadowed alias in dependency closure");
    });

    let output_path = temp_root.join("smoke");
    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled shadowed-alias-dependency-filtering binary");
    assert_eq!(
        output.status.code(),
        Some(12),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_split_file_module_named_main_in_entry_namespace_runtime() {
    let temp_root = make_temp_project_root("project-module-main-entry-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/module.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        "package core;\nfunction main(): Integer { return main.ping(); }\n",
    )
    .must("write main");
    fs::write(
        src_dir.join("module.arden"),
        "package core;\nmodule main { function ping(): Integer { return 22; } }\n",
    )
    .must("write module");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support split-file module named main");
    });

    let output = std::process::Command::new(temp_root.join("smoke"))
        .output()
        .must("run compiled split-file module named main binary");
    assert_eq!(
        output.status.code(),
        Some(22),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_split_file_class_named_main_in_entry_namespace_runtime() {
    let temp_root = make_temp_project_root("project-class-main-entry-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/model.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        r#"
package core;
function main(): Integer {
    value: main = main(22);
    return value.get();
}
"#,
    )
    .must("write main");
    fs::write(
        src_dir.join("model.arden"),
        r#"
package core;
class main {
    value: Integer;
    constructor(v: Integer) { this.value = v; }
    function get(): Integer { return this.value; }
}
"#,
    )
    .must("write model");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support split-file class named main");
    });

    let output = std::process::Command::new(temp_root.join("smoke"))
        .output()
        .must("run compiled split-file class named main binary");
    assert_eq!(
        output.status.code(),
        Some(22),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn project_build_runs_split_file_enum_named_main_in_entry_namespace_runtime() {
    let temp_root = make_temp_project_root("project-enum-main-entry-runtime");
    let src_dir = temp_root.join("src");
    write_test_project_config(
        &temp_root,
        &["src/main.arden", "src/enum.arden"],
        "src/main.arden",
        "smoke",
    );
    fs::write(
        src_dir.join("main.arden"),
        r#"
package core;
function main(): Integer {
    return match (main.Ok(22)) {
        Ok(value) => value,
    };
}
"#,
    )
    .must("write main");
    fs::write(
        src_dir.join("enum.arden"),
        r#"
package core;
enum main {
    Ok(Integer)
}
"#,
    )
    .must("write enum");

    with_current_dir(&temp_root, || {
        build_project(false, false, true, false, false)
            .must("project build should support split-file enum named main");
    });

    let output = std::process::Command::new(temp_root.join("smoke"))
        .output()
        .must("run compiled split-file enum named main binary");
    assert_eq!(
        output.status.code(),
        Some(22),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
