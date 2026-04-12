use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_integer_deref_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-integer-deref-assign-type");
    let source_path = temp_root.join("no_check_integer_deref_assign_type.arden");
    let output_path = temp_root.join("no_check_integer_deref_assign_type");
    let source = r#"
            function main(): None {
                mut value: Integer = 7;
                *value = 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("integer deref assignment should fail in codegen without checks");
    assert!(
        err.contains("Cannot dereference non-pointer type Integer"),
        "{err}"
    );
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_try_on_non_option_result_type() {
    let temp_root = make_temp_project_root("no-check-invalid-try-non-result-type");
    let source_path = temp_root.join("no_check_invalid_try_non_result_type.arden");
    let output_path = temp_root.join("no_check_invalid_try_non_result_type");
    let source = r#"
            function main(): None {
                value: Integer = 7;
                out: Integer = value?;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("? on Integer should fail in codegen without checks");
    assert!(
        err.contains("'?' operator can only be used on Option or Result, got Integer"),
        "{err}"
    );
    assert!(!err.contains("expected the StructValue variant"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_try_on_module_local_non_option_result_type_with_user_facing_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-try-module-local-non-result");
    let source_path = temp_root.join("no_check_invalid_try_module_local_non_result.arden");
    let output_path = temp_root.join("no_check_invalid_try_module_local_non_result");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Integer {
                return M.Box(7)?;
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("? on module-local Box should fail in codegen without checks");
    assert!(
        err.contains("'?' operator can only be used on Option or Result, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_preserves_box_constructor_payload_in_codegen() {
    let temp_root = make_temp_project_root("no-check-box-payload-runtime");
    let source_path = temp_root.join("no_check_box_payload_runtime.arden");
    let output_path = temp_root.join("no_check_box_payload_runtime");
    let source = r#"
            function main(): Integer {
                value: Box<Integer> = Box<Integer>(41);
                return *value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("Box payload constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled no-check box payload binary");
    assert_eq!(status.code(), Some(41));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_preserves_rc_constructor_payload_in_codegen() {
    let temp_root = make_temp_project_root("no-check-rc-payload-runtime");
    let source_path = temp_root.join("no_check_rc_payload_runtime.arden");
    let output_path = temp_root.join("no_check_rc_payload_runtime");
    let source = r#"
            function main(): Integer {
                value: Rc<Integer> = Rc<Integer>(42);
                return *value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("Rc payload constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled no-check rc payload binary");
    assert_eq!(status.code(), Some(42));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_preserves_arc_constructor_payload_in_codegen() {
    let temp_root = make_temp_project_root("no-check-arc-payload-runtime");
    let source_path = temp_root.join("no_check_arc_payload_runtime.arden");
    let output_path = temp_root.join("no_check_arc_payload_runtime");
    let source = r#"
            function main(): Integer {
                value: Arc<Integer> = Arc<Integer>(43);
                return *value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("Arc payload constructor should codegen without checks");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled no-check arc payload binary");
    assert_eq!(status.code(), Some(43));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assert_eq_on_incompatible_types_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-eq-types");
    let source_path = temp_root.join("no_check_invalid_assert_eq_types.arden");
    let output_path = temp_root.join("no_check_invalid_assert_eq_types");
    let source = r#"
            function main(): None {
                assert_eq(1, true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("assert_eq on incompatible types should fail in codegen");
    assert!(err.contains("Cannot compare Integer and Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assert_ne_on_incompatible_types_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-assert-ne-types");
    let source_path = temp_root.join("no_check_invalid_assert_ne_types.arden");
    let output_path = temp_root.join("no_check_invalid_assert_ne_types");
    let source = r#"
            function main(): None {
                assert_ne(1, true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("assert_ne on incompatible types should fail in codegen");
    assert!(err.contains("Cannot compare Integer and Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
