use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_enum_variant_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-enum-variant-call-type-args");
    let source_path = temp_root.join("no_check_enum_variant_call_type_args.arden");
    let output_path = temp_root.join("no_check_enum_variant_call_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }

            function main(): Integer {
                return Boxed.Wrap<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("enum variant call type args should fail in codegen");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "{err}"
    );
    assert!(
        !err.contains("Explicit generic call code generation is not supported yet"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_enum_missing_method_with_user_facing_diagnostic() {
    let temp_root = make_temp_project_root("no-check-enum-missing-method-diagnostic");
    let source_path = temp_root.join("no_check_enum_missing_method_diagnostic.arden");
    let output_path = temp_root.join("no_check_enum_missing_method_diagnostic");
    let source = r#"
            enum Boxed { Wrap(Integer) }

            function main(): Integer {
                value: Boxed = Boxed.Wrap(1);
                return value.missing();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("missing enum method should fail in codegen");
    assert!(
        err.contains("Unknown method 'missing' for class 'Boxed'"),
        "{err}"
    );
    assert!(
        !err.contains("Unknown interface method implementation"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_imported_enum_variant_call_type_args_cleanly() {
    let temp_root = make_temp_project_root("no-check-imported-enum-variant-call-type-args");
    let source_path = temp_root.join("no_check_imported_enum_variant_call_type_args.arden");
    let output_path = temp_root.join("no_check_imported_enum_variant_call_type_args");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            import Boxed.Wrap as WrapCtor;

            function main(): Integer {
                return WrapCtor<Integer>(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("imported enum variant call type args should fail in codegen");
    assert!(
        err.contains("Enum variant 'Boxed.Wrap' does not accept type arguments"),
        "{err}"
    );
    assert!(!err.contains("Unknown type: WrapCtor<Integer>"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_enum_variant_alias_constructor_runtime() {
    let temp_root = make_temp_project_root("no-check-imported-enum-variant-alias-runtime");
    let source_path = temp_root.join("no_check_imported_enum_variant_alias_runtime.arden");
    let output_path = temp_root.join("no_check_imported_enum_variant_alias_runtime");
    let source = r#"
            enum Boxed { Wrap(Integer) }
            import Boxed.Wrap as WrapCtor;

            function main(): Integer {
                value: Boxed = WrapCtor(7);
                return match (value) { Boxed.Wrap(v) => { if (v == 7) { 0 } else { 1 } } };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked imported enum variant alias constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked imported enum variant alias constructor binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_option_some_alias_runtime() {
    let temp_root = make_temp_project_root("no-check-imported-option-some-alias-runtime");
    let source_path = temp_root.join("no_check_imported_option_some_alias_runtime.arden");
    let output_path = temp_root.join("no_check_imported_option_some_alias_runtime");
    let source = r#"
            import Option.Some as Present;

            function main(): Integer {
                value: Option<Integer> = Present(4);
                return if (value.unwrap() == 4) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked imported Option.Some alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked imported Option.Some alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_option_some_alias_function_value_runtime() {
    let temp_root = make_temp_project_root("no-check-imported-option-some-alias-fn-value-runtime");
    let source_path = temp_root.join("no_check_imported_option_some_alias_fn_value_runtime.arden");
    let output_path = temp_root.join("no_check_imported_option_some_alias_fn_value_runtime");
    let source = r#"
            import Option.Some as Present;

            function main(): Integer {
                wrap: (Integer) -> Option<Integer> = Present;
                value: Option<Integer> = wrap(6);
                return if (value.unwrap() == 6) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked imported Option.Some alias function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked imported Option.Some alias function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("no-check-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("no_check_generic_class_ctor_fn_value_runtime.arden");
    let output_path = temp_root.join("no_check_generic_class_ctor_fn_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = Box<Integer>;
                return ctor(3).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked generic class constructor function value binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("no-check-imported-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("no_check_imported_generic_class_ctor_fn_value_runtime.arden");
    let output_path = temp_root.join("no_check_imported_generic_class_ctor_fn_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            import Box as B;

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = B<Integer>;
                return ctor(4).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked imported generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked imported generic class constructor function value binary");
    assert_eq!(status.code(), Some(4));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_nested_generic_class_constructor_function_value_runtime() {
    let temp_root =
        make_temp_project_root("no-check-imported-nested-generic-class-ctor-fn-value-runtime");
    let source_path =
        temp_root.join("no_check_imported_nested_generic_class_ctor_fn_value_runtime.arden");
    let output_path =
        temp_root.join("no_check_imported_nested_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import M.Box as B;

            function main(): Integer {
                ctor: (Integer) -> M.Box<Integer> = B<Integer>;
                return ctor(6).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked imported nested generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path).status().must(
        "run compiled unchecked imported nested generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_inferred_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("no-check-inferred-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("no_check_inferred_generic_class_ctor_fn_value_runtime.arden");
    let output_path = temp_root.join("no_check_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = Box;
                return ctor(8).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked inferred generic class constructor function value binary");
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_exact_imported_nested_enum_variant_aliases_runtime() {
    let temp_root =
        make_temp_project_root("no-check-exact-imported-nested-enum-variant-aliases-runtime");
    let source_path =
        temp_root.join("no_check_exact_imported_nested_enum_variant_aliases_runtime.arden");
    let output_path = temp_root.join("no_check_exact_imported_nested_enum_variant_aliases_runtime");
    let source = r#"
            module util { enum Result { Ok(Integer), Error(String) } }
            import util.Result.Ok as Success;
            import util.Result.Error as Failure;

            function main(): Integer {
                value: util.Result = Success(2);
                return match (value) {
                    Success(v) => if (v == 2) { 0 } else { 1 },
                    Failure(err) => 2,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked exact imported nested enum variant aliases should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked exact imported nested enum variant alias binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
