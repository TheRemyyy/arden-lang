use super::*;
use std::fs;

#[test]
fn compile_source_runs_root_namespace_alias_builtin_option_constructor() {
    let temp_root = make_temp_project_root("root-namespace-alias-builtin-option-constructor");
    let source_path = temp_root.join("root_namespace_alias_builtin_option_constructor.arden");
    let output_path = temp_root.join("root_namespace_alias_builtin_option_constructor");
    let source = r#"
package app;

import app as root;

function main(): Integer {
    value: Option<Integer> = root.Option.Some(4);
    return value.unwrap();
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("root namespace alias builtin Option constructor should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled root alias Option constructor binary");
    assert_eq!(
        output.status.code(),
        Some(4),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_root_namespace_alias_builtin_option_none_constructor() {
    let temp_root = make_temp_project_root("root-namespace-alias-builtin-option-none-constructor");
    let source_path = temp_root.join("root_namespace_alias_builtin_option_none_constructor.arden");
    let output_path = temp_root.join("root_namespace_alias_builtin_option_none_constructor");
    let source = r#"
package app;

import app as root;

function main(): Integer {
    value: Option<Integer> = root.Option.None();
    return match (value) {
        None => 0,
        Some(_) => 1,
    };
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("root namespace alias builtin Option.None constructor should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled root alias Option.None constructor binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_package_qualified_builtin_option_none_pattern() {
    let temp_root = make_temp_project_root("package-qualified-builtin-option-none-pattern");
    let source_path = temp_root.join("package_qualified_builtin_option_none_pattern.arden");
    let output_path = temp_root.join("package_qualified_builtin_option_none_pattern");
    let source = r#"
package app;

function main(): Integer {
    return match (Option.None()) {
        Option.None => 0,
        Option.Some(_) => 1,
    };
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("package-qualified Option.None pattern should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled package-qualified Option.None pattern binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_root_namespace_alias_builtin_option_none_pattern() {
    let temp_root = make_temp_project_root("root-namespace-alias-builtin-option-none-pattern");
    let source_path = temp_root.join("root_namespace_alias_builtin_option_none_pattern.arden");
    let output_path = temp_root.join("root_namespace_alias_builtin_option_none_pattern");
    let source = r#"
package app;

import app as root;

function main(): Integer {
    return match (root.Option.None()) {
        root.Option.None => 0,
        root.Option.Some(_) => 1,
    };
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("root namespace alias Option.None pattern should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled root alias Option.None pattern binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_root_namespace_alias_inline_result_constructor_method_chain() {
    let temp_root =
        make_temp_project_root("root-namespace-alias-inline-result-constructor-method-chain");
    let source_path =
        temp_root.join("root_namespace_alias_inline_result_constructor_method_chain.arden");
    let output_path = temp_root.join("root_namespace_alias_inline_result_constructor_method_chain");
    let source = r#"
package app;

import app as root;

function main(): Integer {
    return root.Result.Ok(4).unwrap();
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("root namespace alias inline Result constructor chain should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled root alias Result constructor chain binary");
    assert_eq!(
        output.status.code(),
        Some(4),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
