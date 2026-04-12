use super::*;
use std::fs;

#[test]
fn compile_source_runs_entry_namespace_module_named_main_runtime() {
    let temp_root = make_temp_project_root("entry-namespace-module-main-runtime");
    let source_path = temp_root.join("entry_namespace_module_main_runtime.arden");
    let output_path = temp_root.join("entry_namespace_module_main_runtime");
    let source = r#"
package core;

module main {
    function ping(): Integer { return 22; }
}

function main(): Integer {
    return main.ping();
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("entry namespace module named main should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled entry namespace module named main binary");
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
fn compile_source_runs_entry_namespace_class_named_main_runtime() {
    let temp_root = make_temp_project_root("entry-namespace-class-main-runtime");
    let source_path = temp_root.join("entry_namespace_class_main_runtime.arden");
    let output_path = temp_root.join("entry_namespace_class_main_runtime");
    let source = r#"
package core;

class main {
    value: Integer;
    constructor(v: Integer) { this.value = v; }
    function get(): Integer { return this.value; }
}

function main(): Integer {
    value: main = main(22);
    return value.get();
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("entry namespace class named main should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled entry namespace class named main binary");
    assert_eq!(
        output.status.code(),
        Some(22),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
