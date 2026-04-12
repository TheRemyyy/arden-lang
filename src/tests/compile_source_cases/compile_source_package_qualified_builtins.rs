use super::*;
use std::fs;

#[test]
fn compile_source_runs_package_qualified_builtin_option_variant_alias_patterns() {
    let temp_root = make_temp_project_root("package-qualified-builtin-option-variant-aliases");
    let source_path = temp_root.join("package_qualified_builtin_option_variant_aliases.arden");
    let output_path = temp_root.join("package_qualified_builtin_option_variant_aliases");
    let source = r#"
package app;

import app.Option.Some as Present;
import app.Option.None as Empty;

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => inner,
        Empty => 0,
    };
}

function main(): Integer {
    return classify(Present(4));
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("package-qualified builtin Option variant aliases should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled package-qualified builtin Option alias binary");
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
fn compile_source_runs_package_qualified_builtin_result_variant_alias_patterns() {
    let temp_root = make_temp_project_root("package-qualified-builtin-result-variant-aliases");
    let source_path = temp_root.join("package_qualified_builtin_result_variant_aliases.arden");
    let output_path = temp_root.join("package_qualified_builtin_result_variant_aliases");
    let source = r#"
package app;

import app.Result.Ok as Success;
import app.Result.Error as Failure;

function classify(value: Result<Integer, String>): Integer {
    return match (value) {
        Success(inner) => inner,
        Failure(err) => 0,
    };
}

function main(): Integer {
    return classify(Success(4));
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("package-qualified builtin Result variant aliases should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled package-qualified builtin Result alias binary");
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
fn compile_source_runs_package_qualified_direct_builtin_option_constructor() {
    let temp_root = make_temp_project_root("package-qualified-direct-builtin-option-constructor");
    let source_path = temp_root.join("package_qualified_direct_builtin_option_constructor.arden");
    let output_path = temp_root.join("package_qualified_direct_builtin_option_constructor");
    let source = r#"
package app;

function main(): Integer {
    value: Option<Integer> = Option.Some(4);
    return value.unwrap();
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("package-qualified direct Option constructor should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled package-qualified direct Option constructor binary");
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
fn compile_source_runs_package_qualified_direct_builtin_option_none_constructor() {
    let temp_root =
        make_temp_project_root("package-qualified-direct-builtin-option-none-constructor");
    let source_path =
        temp_root.join("package_qualified_direct_builtin_option_none_constructor.arden");
    let output_path = temp_root.join("package_qualified_direct_builtin_option_none_constructor");
    let source = r#"
package app;

function main(): Integer {
    value: Option<Integer> = Option.None();
    return match (value) {
        None => 0,
        Some(_) => 1,
    };
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("package-qualified direct Option.None constructor should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled package-qualified direct Option.None constructor binary");
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
fn compile_source_runs_package_qualified_direct_builtin_result_constructor() {
    let temp_root = make_temp_project_root("package-qualified-direct-builtin-result-constructor");
    let source_path = temp_root.join("package_qualified_direct_builtin_result_constructor.arden");
    let output_path = temp_root.join("package_qualified_direct_builtin_result_constructor");
    let source = r#"
package app;

function main(): Integer {
    value: Result<Integer, String> = Result.Ok(4);
    return value.unwrap();
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("package-qualified direct Result constructor should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled package-qualified direct Result constructor binary");
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
fn compile_source_runs_package_qualified_inline_option_constructor_method_chain() {
    let temp_root =
        make_temp_project_root("package-qualified-inline-option-constructor-method-chain");
    let source_path =
        temp_root.join("package_qualified_inline_option_constructor_method_chain.arden");
    let output_path = temp_root.join("package_qualified_inline_option_constructor_method_chain");
    let source = r#"
package app;

function main(): Integer {
    return Option.Some(4).unwrap();
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("package-qualified inline Option constructor method chain should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled package-qualified inline Option constructor binary");
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
fn compile_source_runs_package_qualified_inline_result_constructor_method_chain() {
    let temp_root =
        make_temp_project_root("package-qualified-inline-result-constructor-method-chain");
    let source_path =
        temp_root.join("package_qualified_inline_result_constructor_method_chain.arden");
    let output_path = temp_root.join("package_qualified_inline_result_constructor_method_chain");
    let source = r#"
package app;

function main(): Integer {
    return Result.Ok(4).unwrap();
}
"#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("package-qualified inline Result constructor method chain should compile");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled package-qualified inline Result constructor binary");
    assert_eq!(
        output.status.code(),
        Some(4),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
