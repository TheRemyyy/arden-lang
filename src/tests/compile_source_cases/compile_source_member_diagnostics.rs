use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_boolean_method_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-boolean-method-call");
    let source_path = temp_root.join("no_check_invalid_boolean_method_call.arden");
    let output_path = temp_root.join("no_check_invalid_boolean_method_call");
    let source = r#"
            function main(): Integer {
                flag: Boolean = true;
                return flag.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Boolean.length() should fail in codegen");
    assert!(err.contains("Cannot call method on type Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_method_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-integer-method-call");
    let source_path = temp_root.join("no_check_invalid_integer_method_call.arden");
    let output_path = temp_root.join("no_check_invalid_integer_method_call");
    let source = r#"
            function main(): Integer {
                value: Integer = 1;
                return value.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Integer.length() should fail in codegen");
    assert!(err.contains("Cannot call method on type Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_boolean_field_access_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-boolean-field-access");
    let source_path = temp_root.join("no_check_invalid_boolean_field_access.arden");
    let output_path = temp_root.join("no_check_invalid_boolean_field_access");
    let source = r#"
            function main(): Integer {
                flag: Boolean = true;
                return flag.value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Boolean field access should fail in codegen");
    assert!(err.contains("Cannot access field on type Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_field_access_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-integer-field-access");
    let source_path = temp_root.join("no_check_invalid_integer_field_access.arden");
    let output_path = temp_root.join("no_check_invalid_integer_field_access");
    let source = r#"
            function main(): Integer {
                value: Integer = 1;
                return value.value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Integer field access should fail in codegen");
    assert!(err.contains("Cannot access field on type Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_boolean_field_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-boolean-field-assignment");
    let source_path = temp_root.join("no_check_invalid_boolean_field_assignment.arden");
    let output_path = temp_root.join("no_check_invalid_boolean_field_assignment");
    let source = r#"
            function main(): Integer {
                mut flag: Boolean = true;
                flag.value = false;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Boolean field assignment should fail in codegen");
    assert!(err.contains("Cannot access field on type Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_field_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-invalid-integer-field-assignment");
    let source_path = temp_root.join("no_check_invalid_integer_field_assignment.arden");
    let output_path = temp_root.join("no_check_invalid_integer_field_assignment");
    let source = r#"
            function main(): Integer {
                mut value: Integer = 1;
                value.value = 2;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Integer field assignment should fail in codegen");
    assert!(err.contains("Cannot access field on type Integer"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_unknown_class_field_access_with_class_diagnostic() {
    let temp_root = make_temp_project_root("no-check-unknown-class-field-access");
    let source_path = temp_root.join("no_check_unknown_class_field_access.arden");
    let output_path = temp_root.join("no_check_unknown_class_field_access");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Box = Box(7);
                return b.missing;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("missing Box field access should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'Box'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_unknown_class_field_assignment_with_class_diagnostic() {
    let temp_root = make_temp_project_root("no-check-unknown-class-field-assignment");
    let source_path = temp_root.join("no_check_unknown_class_field_assignment.arden");
    let output_path = temp_root.join("no_check_unknown_class_field_assignment");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): None {
                mut b: Box = Box(7);
                b.missing = 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("missing Box field assignment should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'Box'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_nested_unknown_field_before_index_assignment_diagnostic() {
    let temp_root = make_temp_project_root("no-check-nested-unknown-field-index-assignment");
    let source_path = temp_root.join("no_check_nested_unknown_field_index_assignment.arden");
    let output_path = temp_root.join("no_check_nested_unknown_field_index_assignment");
    let source = r#"
            class Inner {
                mut items: List<Integer>;
                constructor() { this.items = List<Integer>(); }
            }

            class Box {
                mut inner: Inner;
                constructor() { this.inner = Inner(); }
            }

            class Holder {
                make: () -> Box;
                constructor(make: () -> Box) { this.make = make; }
            }

            function build(): Box {
                return Box();
            }

            function main(): None {
                holder: Holder = Holder(build);
                holder.make().inner.missing[0] = 9;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("nested missing field index assignment should fail in codegen");
    assert!(
        err.contains("Unknown field 'missing' on class 'Inner'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_nested_deref_root_cause_diagnostic() {
    let temp_root = make_temp_project_root("no-check-nested-deref-root-cause");
    let undef_source_path = temp_root.join("no_check_nested_deref_undefined_root.arden");
    let undef_output_path = temp_root.join("no_check_nested_deref_undefined_root");
    let undef_source = r#"
            function main(): None {
                println(*missing.inner.ptr);
                return None;
            }
        "#;

    fs::write(&undef_source_path, undef_source).must("write undefined-root source");
    let undef_err = compile_source(
        undef_source,
        &undef_source_path,
        &undef_output_path,
        false,
        false,
        None,
        None,
    )
    .must_err("nested undefined-root deref should fail in codegen");
    assert!(
        undef_err.contains("Undefined variable: missing"),
        "{undef_err}"
    );

    let missing_source_path = temp_root.join("no_check_nested_deref_missing_field.arden");
    let missing_output_path = temp_root.join("no_check_nested_deref_missing_field");
    let missing_source = r#"
            class Inner {
                value: Integer;
                constructor() { this.value = 1; }
            }

            class Box {
                inner: Inner;
                constructor() { this.inner = Inner(); }
            }

            class Holder {
                make: () -> Box;
                constructor(make: () -> Box) { this.make = make; }
            }

            function build(): Box { return Box(); }

            function main(): None {
                holder: Holder = Holder(build);
                println(*holder.make().inner.missing);
                return None;
            }
        "#;

    fs::write(&missing_source_path, missing_source).must("write missing-field source");
    let missing_err = compile_source(
        missing_source,
        &missing_source_path,
        &missing_output_path,
        false,
        false,
        None,
        None,
    )
    .must_err("nested missing-field deref should fail in codegen");
    assert!(
        missing_err.contains("Unknown field 'missing' on class 'Inner'"),
        "{missing_err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_reports_unknown_class_method_with_class_diagnostic() {
    let temp_root = make_temp_project_root("unknown-class-method-diagnostic");
    let source_path = temp_root.join("unknown_class_method_diagnostic.arden");
    let output_path = temp_root.join("unknown_class_method_diagnostic");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                b: Box = Box(7);
                return b.missing();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("missing Box method should fail");
    assert!(
        err.contains("Unknown method 'missing' for class 'Box'"),
        "{err}"
    );
    assert!(!err.contains("Unknown class: Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_generic_missing_class_root_without_internal_codegen_error() {
    let temp_root = make_temp_project_root("no-check-generic-missing-class-root");
    let source_path = temp_root.join("no_check_generic_missing_class_root.arden");
    let output_path = temp_root.join("no_check_generic_missing_class_root");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): None {
                b: Box<Integer> = Box<Integer>.missing(1);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic missing class root should fail in codegen");
    assert!(err.contains("Undefined variable: Box"), "{err}");
    assert!(
        !err.contains(
            "Explicit generic function value should be specialized before code generation"
        ),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_generic_missing_function_call_without_internal_codegen_error() {
    let temp_root = make_temp_project_root("no-check-generic-missing-function-call");
    let source_path = temp_root.join("no_check_generic_missing_function_call.arden");
    let output_path = temp_root.join("no_check_generic_missing_function_call");
    let source = r#"
            function main(): Integer {
                return missing<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic missing function call should fail in codegen");
    assert!(err.contains("Undefined function: missing"), "{err}");
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_generic_missing_method_call_without_internal_codegen_error() {
    let temp_root = make_temp_project_root("no-check-generic-missing-method-call");
    let source_path = temp_root.join("no_check_generic_missing_method_call.arden");
    let output_path = temp_root.join("no_check_generic_missing_method_call");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                return Box(1).missing<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic missing method call should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for class 'Box'"),
        "{err}"
    );
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}
