use super::*;
use std::fs;

#[test]
fn compile_source_rejects_enum_variant_function_value_type_args_cleanly() {
    let temp_root = make_temp_project_root("enum-variant-function-value-type-args");
    let source_path = temp_root.join("enum_variant_function_value_type_args.arden");
    let output_path = temp_root.join("enum_variant_function_value_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            function main(): Integer {
                wrap: (Integer) -> Boxed = Boxed.Wrap<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("enum variant function value type args should fail");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_constructor_function_value_type_args_cleanly() {
    let temp_root = make_temp_project_root("builtin-constructor-function-value-type-args");
    let source_path = temp_root.join("builtin_constructor_function_value_type_args.arden");
    let output_path = temp_root.join("builtin_constructor_function_value_type_args");
    let source = r#"
            function main(): Integer {
                wrap: (Integer) -> Option<Integer> = Option.some<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("builtin constructor function value type args should fail");
    assert!(
        err.contains("Built-in function 'Option.some' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_imported_enum_variant_function_value_type_args_cleanly() {
    let temp_root = make_temp_project_root("imported-enum-variant-function-value-type-args");
    let source_path = temp_root.join("imported_enum_variant_function_value_type_args.arden");
    let output_path = temp_root.join("imported_enum_variant_function_value_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            import Boxed.Wrap as WrapCtor;
            function main(): Integer {
                wrap: (Integer) -> Boxed = WrapCtor<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("imported enum variant function value type args should fail");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_imported_enum_variant_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("imported-enum-variant-call-type-args");
    let source_path = temp_root.join("imported_enum_variant_call_type_args.arden");
    let output_path = temp_root.join("imported_enum_variant_call_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            import Boxed.Wrap as WrapCtor;
            function main(): Integer {
                return WrapCtor<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("imported enum variant call type args should fail");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );
    assert!(!err.contains("Unknown type: WrapCtor<Integer>"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_nested_imported_enum_variant_function_value_type_args_cleanly() {
    let temp_root = make_temp_project_root("nested-imported-enum-variant-function-value-type-args");
    let source_path = temp_root.join("nested_imported_enum_variant_function_value_type_args.arden");
    let output_path = temp_root.join("nested_imported_enum_variant_function_value_type_args");
    let source = r#"
            module U {
                module V {
                    enum E { Wrap(Integer) }
                }
            }
            import U.V.E.Wrap as WrapCtor;
            function main(): Integer {
                wrap: (Integer) -> U.V.E = WrapCtor<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("nested imported enum variant function value type args should fail");
    assert!(
        err.contains("Enum variant 'U.V.E.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_nested_imported_enum_variant_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("nested-imported-enum-variant-call-type-args");
    let source_path = temp_root.join("nested_imported_enum_variant_call_type_args.arden");
    let output_path = temp_root.join("nested_imported_enum_variant_call_type_args");
    let source = r#"
            module U {
                module V {
                    enum E { Wrap(Integer) }
                }
            }
            import U.V.E.Wrap as WrapCtor;
            function main(): Integer {
                return WrapCtor<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("nested imported enum variant call type args should fail");
    assert!(
        err.contains("Enum variant 'U.V.E.Wrap' does not accept type arguments"),
        "unexpected error: {err}"
    );
    assert!(!err.contains("Unknown type: WrapCtor<Integer>"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_builtin_constructor_function_value_type_args_nocheck() {
    let temp_root = make_temp_project_root("builtin-constructor-function-value-type-args-nocheck");
    let source_path = temp_root.join("builtin_constructor_function_value_type_args_nocheck.arden");
    let output_path = temp_root.join("builtin_constructor_function_value_type_args_nocheck");
    let source = r#"
            function main(): Integer {
                wrap: (Integer) -> Result<Integer, String> = Result.ok<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("builtin constructor function value type args should fail in codegen");
    assert!(
        err.contains("Built-in function 'Result.ok' does not accept type arguments"),
        "unexpected error: {err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_named_function_value_signature_mismatch_with_function_type() {
    let temp_root = make_temp_project_root("no-check-named-function-value-signature-mismatch");
    let source_path = temp_root.join("no_check_named_function_value_signature_mismatch.arden");
    let output_path = temp_root.join("no_check_named_function_value_signature_mismatch");
    let source = r#"
            function get(): Integer {
                return 1;
            }

            function main(): Integer {
                f: (Integer) -> Integer = get;
                return f(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("named function value signature mismatch should fail in codegen");
    assert!(
        err.contains("Cannot use function value () -> Integer as (Integer) -> Integer"),
        "{err}"
    );
    assert!(!err.contains("builtin function"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_constructor_function_value_signature_mismatch() {
    let temp_root =
        make_temp_project_root("no-check-constructor-function-value-signature-mismatch");
    let source_path =
        temp_root.join("no_check_constructor_function_value_signature_mismatch.arden");
    let output_path = temp_root.join("no_check_constructor_function_value_signature_mismatch");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                f: () -> Box = Box;
                return f().value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("constructor function value signature mismatch should fail in codegen");
    assert!(
        err.contains("Cannot use function value (Integer) -> Box as () -> Box"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_formats_specialized_constructor_function_value_signature_mismatch() {
    let temp_root =
        make_temp_project_root("no-check-specialized-constructor-function-signature-mismatch");
    let source_path =
        temp_root.join("no_check_specialized_constructor_function_signature_mismatch.arden");
    let output_path =
        temp_root.join("no_check_specialized_constructor_function_signature_mismatch");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                f: () -> Box<Integer> = Box<Integer>;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err(
            "specialized constructor function value signature mismatch should fail in codegen",
        );
    assert!(
        err.contains("Cannot use function value (Integer) -> Box<Integer> as () -> Box<Integer>"),
        "{err}"
    );
    assert!(!err.contains("Box.spec.I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_formats_specialized_builtin_function_value_signature_mismatch() {
    let temp_root =
        make_temp_project_root("no-check-specialized-builtin-function-signature-mismatch");
    let source_path =
        temp_root.join("no_check_specialized_builtin_function_signature_mismatch.arden");
    let output_path = temp_root.join("no_check_specialized_builtin_function_signature_mismatch");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                f: () -> Option<Box<Integer>> = Option.some;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("specialized builtin function value signature mismatch should fail in codegen");
    assert!(
        err.contains(
            "Type mismatch: expected () -> Option<Box<Integer>>, got (unknown) -> Option<unknown>"
        ),
        "{err}"
    );
    assert!(!err.contains("Box__spec__I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_formats_specialized_constructor_builtin_diagnostics() {
    let temp_root = make_temp_project_root("no-check-specialized-constructor-builtin-diagnostics");
    let source_path = temp_root.join("no_check_specialized_constructor_builtin_diagnostics.arden");
    let output_path = temp_root.join("no_check_specialized_constructor_builtin_diagnostics");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                value: Option<Box<Integer>> = Option<Box<Integer>>(1);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("specialized constructor builtin diagnostic should fail in codegen");
    assert!(
        err.contains("Constructor Option<Box<Integer>> expects 0 arguments, got 1"),
        "{err}"
    );
    assert!(!err.contains("Box__spec__I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_formats_specialized_unknown_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-specialized-unknown-type-diagnostic");
    let source_path = temp_root.join("no_check_specialized_unknown_type_diagnostic.arden");
    let output_path = temp_root.join("no_check_specialized_unknown_type_diagnostic");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                value: Missing<Box<Integer>> = Missing<Box<Integer>>();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("specialized unknown type diagnostic should fail in codegen");
    assert!(err.contains("Unknown type: Missing<Box<Integer>>"), "{err}");
    assert!(!err.contains("Box__spec__I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
