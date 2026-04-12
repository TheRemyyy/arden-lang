use super::*;
use std::fs;

#[test]
fn compile_source_runs_unit_enum_match_expression_runtime() {
    let temp_root = make_temp_project_root("unit-enum-match-expression-runtime");
    let source_path = temp_root.join("unit_enum_match_expression_runtime.arden");
    let output_path = temp_root.join("unit_enum_match_expression_runtime");
    let source = r#"
            enum Kind { A, B }
            function main(): Integer {
                value: Integer = match (Kind.A) { Kind.A => { 1 } Kind.B => { 2 } };
                require(value == 1);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unit enum match expression should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled unit enum match expression binary");
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
fn compile_source_runs_imported_unit_enum_variant_alias_match_expression_runtime() {
    let temp_root = make_temp_project_root("imported-unit-enum-variant-alias-match-runtime");
    let source_path = temp_root.join("imported_unit_enum_variant_alias_match_runtime.arden");
    let output_path = temp_root.join("imported_unit_enum_variant_alias_match_runtime");
    let source = r#"
            import std.io.*;
            enum E { A, B }
            import E.A as A;
            function main(): Integer {
                println("value={match (A) { A => { 1 } E.B => { 2.5 } }}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported unit enum variant alias match interpolation should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled imported unit enum variant alias match binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=1.000000\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_unit_enum_variant_alias_patterns_runtime() {
    let temp_root = make_temp_project_root("imported-unit-enum-variant-alias-pattern-runtime");
    let source_path = temp_root.join("imported_unit_enum_variant_alias_pattern_runtime.arden");
    let output_path = temp_root.join("imported_unit_enum_variant_alias_pattern_runtime");
    let source = r#"
            import std.io.*;
            enum E { A, B }
            import E.A as A;
            function main(): Integer {
                println("value={match (E.B) { A => { 1 } E.B => { 2 } }}");
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported unit enum variant alias pattern should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled imported unit enum variant alias pattern binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalize_output(&output.stdout), "value=2\n");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn check_source_rejects_non_exhaustive_imported_unit_enum_variant_alias_pattern() {
    let temp_root =
        make_temp_project_root("imported-unit-enum-variant-alias-pattern-non-exhaustive");
    let source_path =
        temp_root.join("imported_unit_enum_variant_alias_pattern_non_exhaustive.arden");
    let output_path = temp_root.join("imported_unit_enum_variant_alias_pattern_non_exhaustive");
    let source = r#"
            enum E { A, B }
            import E.A as A;
            function main(): Integer {
                return match (E.B) { A => { 1 } };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("alias unit variant pattern should not act as catch-all");
    assert!(
        err.contains("Non-exhaustive match expression"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_payload_enum_variant_alias_patterns_runtime() {
    let temp_root = make_temp_project_root("imported-payload-enum-variant-alias-pattern-runtime");
    let source_path = temp_root.join("imported_payload_enum_variant_alias_pattern_runtime.arden");
    let output_path = temp_root.join("imported_payload_enum_variant_alias_pattern_runtime");
    let source = r#"
            enum E { A(Integer), B(Integer) }
            import E.A as First;
            function main(): Integer {
                value: Integer = match (E.B(2)) { First(v) => { v } E.B(v) => { v + 1 } };
                require(value == 3);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported payload enum variant alias patterns should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled imported payload enum variant alias pattern binary");
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
fn compile_source_runs_namespace_alias_nested_enum_variant_constructor_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-nested-enum-variant-constructor-runtime");
    let source_path =
        temp_root.join("namespace_alias_nested_enum_variant_constructor_runtime.arden");
    let output_path = temp_root.join("namespace_alias_nested_enum_variant_constructor_runtime");
    let source = r#"
            module U {
                module M {
                    enum E { A(Integer), B }
                }
            }
            import U as u;
            function main(): Integer {
                value: u.M.E = u.M.E.A(2);
                return match (value) { u.M.E.A(v) => { v } u.M.E.B => { 0 } };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("namespace alias nested enum variant constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled namespace alias nested enum variant constructor binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}
