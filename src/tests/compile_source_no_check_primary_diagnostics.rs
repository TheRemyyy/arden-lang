use super::*;
use std::fs;

#[test]
fn compile_source_no_check_reports_undefined_variable_for_unknown_method_receiver() {
    let temp_root = make_temp_project_root("no-check-unknown-method-receiver-primary-error");
    let source_path = temp_root.join("no_check_unknown_method_receiver_primary_error.arden");
    let output_path = temp_root.join("no_check_unknown_method_receiver_primary_error");
    let source = r#"
            function main(): Integer {
                return nope.missing();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unknown method receiver should fail in codegen without checks");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Unknown variable: nope"), "{err}");
    assert!(
        !err.contains("Cannot determine object type for method call"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_import_alias_leaking_to_top_level() {
    let temp_root = make_temp_project_root("no-check-module-local-import-alias-leak");
    let source_path = temp_root.join("no_check_module_local_import_alias_leak.arden");
    let output_path = temp_root.join("no_check_module_local_import_alias_leak");
    let source = r#"
            module Inner {
                import std.math as math;
                function keep(): Float {
                    return math.abs(-1.0);
                }
            }

            function main(): Float {
                return math.abs(-1.0);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local alias should not resolve at top level in no-check mode");
    assert!(
        err.contains("Undefined variable: math") || err.contains("Unknown type: math"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_wildcard_import_leaking_to_top_level() {
    let temp_root = make_temp_project_root("no-check-module-local-wildcard-import-leak");
    let source_path = temp_root.join("no_check_module_local_wildcard_import_leak.arden");
    let output_path = temp_root.join("no_check_module_local_wildcard_import_leak");
    let source = r#"
            module Inner {
                import std.math.*;
                function keep(): Float {
                    return abs(-1.0);
                }
            }

            function main(): Float {
                return abs(-1.0);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local wildcard import should not resolve at top level in no-check mode");
    assert!(err.contains("Undefined function: abs"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_variable_for_unknown_field_root() {
    let temp_root = make_temp_project_root("no-check-unknown-field-root-primary-error");
    let source_path = temp_root.join("no_check_unknown_field_root_primary_error.arden");
    let output_path = temp_root.join("no_check_unknown_field_root_primary_error");
    let source = r#"
            function main(): Integer {
                return nope.value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unknown field root should fail in codegen without checks");
    assert!(err.contains("Undefined variable: nope"), "{err}");
    assert!(!err.contains("Unknown variable: nope"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_nested_undefined_root_before_read_or_method_diagnostic() {
    let temp_root = make_temp_project_root("no-check-nested-undefined-root-read-method");
    let read_source_path = temp_root.join("no_check_nested_undefined_root_read.arden");
    let read_output_path = temp_root.join("no_check_nested_undefined_root_read");
    let read_source = r#"
            function main(): None {
                println(missing.inner.items[0]);
                return None;
            }
        "#;

    fs::write(&read_source_path, read_source).must("write read source");
    let read_err = compile_source(
        read_source,
        &read_source_path,
        &read_output_path,
        false,
        false,
        None,
        None,
    )
    .must_err("nested undefined-root read should fail in codegen");
    assert!(
        read_err.contains("Undefined variable: missing"),
        "{read_err}"
    );

    let method_source_path = temp_root.join("no_check_nested_undefined_root_method.arden");
    let method_output_path = temp_root.join("no_check_nested_undefined_root_method");
    let method_source = r#"
            function main(): None {
                missing.inner.items.push(1);
                return None;
            }
        "#;

    fs::write(&method_source_path, method_source).must("write method source");
    let method_err = compile_source(
        method_source,
        &method_source_path,
        &method_output_path,
        false,
        false,
        None,
        None,
    )
    .must_err("nested undefined-root method should fail in codegen");
    assert!(
        method_err.contains("Undefined variable: missing"),
        "{method_err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_function_for_unknown_direct_call() {
    let temp_root = make_temp_project_root("no-check-unknown-direct-call-primary-error");
    let source_path = temp_root.join("no_check_unknown_direct_call_primary_error.arden");
    let output_path = temp_root.join("no_check_unknown_direct_call_primary_error");
    let source = r#"
            function main(): Integer {
                return missing();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unknown direct call should fail in codegen without checks");
    assert!(err.contains("Undefined function: missing"), "{err}");
    assert!(!err.contains("Unknown function: missing"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_reports_undefined_variable_for_unknown_function_value() {
    let temp_root = make_temp_project_root("no-check-unknown-function-value-primary-error");
    let source_path = temp_root.join("no_check_unknown_function_value_primary_error.arden");
    let output_path = temp_root.join("no_check_unknown_function_value_primary_error");
    let source = r#"
            function main(): None {
                callback: (Integer) -> Integer = missing;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unknown function value should fail in codegen without checks");
    assert!(err.contains("Undefined variable: missing"), "{err}");
    assert!(!err.contains("Unknown variable: missing"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_literal_call_with_non_function_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-literal-call-non-function-type");
    let source_path = temp_root.join("no_check_literal_call_non_function_type.arden");
    let output_path = temp_root.join("no_check_literal_call_non_function_type");
    let source = r#"
            function main(): Integer {
                return 1(2);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("literal call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type Integer"),
        "{err}"
    );
    assert!(!err.contains("Invalid callee"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_exact_import_alias_non_function_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("checked-exact-import-alias-call-non-function-type");
    let source_path = temp_root.join("checked_exact_import_alias_call_non_function_type.arden");
    let output_path = temp_root.join("checked_exact_import_alias_call_non_function_type");
    let source = r#"
            import std.system.cwd as CurrentDir;

            function main(): Integer {
                return CurrentDir();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("exact import alias non-function call should fail in checked build");
    assert!(
        err.contains("Cannot call non-function type String"),
        "{err}"
    );
    assert!(
        !err.contains("Return type mismatch: expected Integer, found String"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_exact_import_integer_alias_non_function_call_with_type_diagnostic() {
    let temp_root =
        make_temp_project_root("checked-exact-import-integer-alias-call-non-function-type");
    let source_path =
        temp_root.join("checked_exact_import_integer_alias_call_non_function_type.arden");
    let output_path = temp_root.join("checked_exact_import_integer_alias_call_non_function_type");
    let source = r#"
            import std.args.count as ArgCount;

            function main(): Integer {
                return ArgCount();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("integer exact import alias non-function call should fail in checked build");
    assert!(
        err.contains("Cannot call non-function type Integer"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_local_non_function_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-local-call-non-function-type");
    let source_path = temp_root.join("no_check_local_call_non_function_type.arden");
    let output_path = temp_root.join("no_check_local_call_non_function_type");
    let source = r#"
            function main(): Integer {
                s: String = "hi";
                return s(2);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("local non-function call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type String"),
        "{err}"
    );
    assert!(!err.contains("Undefined function: s"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_exact_import_alias_non_function_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-exact-import-alias-call-non-function-type");
    let source_path = temp_root.join("no_check_exact_import_alias_call_non_function_type.arden");
    let output_path = temp_root.join("no_check_exact_import_alias_call_non_function_type");
    let source = r#"
            import std.system.cwd as CurrentDir;

            function main(): Integer {
                return CurrentDir();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("exact import alias non-function call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type String"),
        "{err}"
    );
    assert!(!err.contains("Unknown type: CurrentDir"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_function_call_with_user_facing_type_diagnostic()
{
    let temp_root = make_temp_project_root("no-check-module-local-call-non-function-type");
    let source_path = temp_root.join("no_check_module_local_call_non_function_type.arden");
    let output_path = temp_root.join("no_check_module_local_call_non_function_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Integer {
                return M.Box(1)(2);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local non-function call should fail in codegen without checks");
    assert!(err.contains("Cannot call non-function type M.Box"), "{err}");
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
