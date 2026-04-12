use super::*;
use std::fs;

#[test]
fn compile_source_no_check_runs_current_package_exact_import_class_alias_constructor() {
    let temp_root = make_temp_project_root("no-check-current-package-exact-alias-ctor");
    let source_path = temp_root.join("no_check_current_package_exact_alias_ctor.arden");
    let output_path = temp_root.join("no_check_current_package_exact_alias_ctor");
    let source = r#"
            package app;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                value: BoxType = BoxType(7);
                return value.value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("current-package exact imported class alias constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled current-package exact class alias constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_current_package_exact_import_generic_class_alias_constructor() {
    let temp_root = make_temp_project_root("no-check-current-package-exact-generic-alias-ctor");
    let source_path = temp_root.join("no_check_current_package_exact_generic_alias_ctor.arden");
    let output_path = temp_root.join("no_check_current_package_exact_generic_alias_ctor");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                value: BoxType<Integer> = BoxType<Integer>(7);
                return value.value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("current-package exact imported generic class alias constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled current-package exact generic class alias constructor binary");
    assert_eq!(status.code(), Some(7));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_generic_class_alias_non_function_call_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-generic-alias-non-function");
    let source_path =
        temp_root.join("no_check_current_package_exact_generic_alias_non_function.arden");
    let output_path = temp_root.join("no_check_current_package_exact_generic_alias_non_function");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Integer>(7)(1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic class alias non-function call should fail");
    assert!(
        err.contains("Cannot call non-function type M.Box<Integer>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root("no-check-current-package-exact-generic-alias-index");
    let source_path = temp_root.join("no_check_current_package_exact_generic_alias_index.arden");
    let output_path = temp_root.join("no_check_current_package_exact_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Integer>(7)[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic class alias indexing should fail");
    assert!(err.contains("Cannot index type M.Box<Integer>"), "{err}");
    assert!(!err.contains("M.Box.spec.I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_generic_class_alias_println_with_user_facing_type(
) {
    let temp_root = make_temp_project_root("no-check-current-package-exact-generic-alias-println");
    let source_path = temp_root.join("no_check_current_package_exact_generic_alias_println.arden");
    let output_path = temp_root.join("no_check_current_package_exact_generic_alias_println");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): None {
                println(BoxType<Integer>(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic class alias println should fail");
    assert!(err.contains("got M.Box<Integer>"), "{err}");
    assert!(!err.contains("M.Box.spec.I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_list_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-list-generic-alias-index");
    let source_path =
        temp_root.join("no_check_current_package_exact_list_generic_alias_index.arden");
    let output_path = temp_root.join("no_check_current_package_exact_list_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<List<Integer>>(List<Integer>())[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("list generic class alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<List<Integer>>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.ListI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_option_generic_class_alias_println_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-option-generic-alias-println");
    let source_path =
        temp_root.join("no_check_current_package_exact_option_generic_alias_println.arden");
    let output_path = temp_root.join("no_check_current_package_exact_option_generic_alias_println");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): None {
                println(BoxType<Option<Integer>>(Option.none<Integer>()));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("option generic class alias println should fail");
    assert!(err.contains("got M.Box<Option<Integer>>"), "{err}");
    assert!(!err.contains("M.Box.spec.OptI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_map_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-map-generic-alias-index");
    let source_path =
        temp_root.join("no_check_current_package_exact_map_generic_alias_index.arden");
    let output_path = temp_root.join("no_check_current_package_exact_map_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Map<String, Integer>>(Map<String, Integer>())[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("map generic class alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<Map<String, Integer>>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.MapStr_I64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_result_generic_class_alias_println_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-result-generic-alias-println");
    let source_path =
        temp_root.join("no_check_current_package_exact_result_generic_alias_println.arden");
    let output_path = temp_root.join("no_check_current_package_exact_result_generic_alias_println");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): None {
                println(BoxType<Result<Integer, String>>(Result.ok(7)));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("result generic class alias println should fail");
    assert!(err.contains("got M.Box<Result<Integer, String>>"), "{err}");
    assert!(!err.contains("M.Box.spec.ResI64_Str"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_function_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-function-generic-alias-index");
    let source_path =
        temp_root.join("no_check_current_package_exact_function_generic_alias_index.arden");
    let output_path = temp_root.join("no_check_current_package_exact_function_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function id(x: Integer): Integer {
                return x;
            }

            function main(): Integer {
                return BoxType<(Integer) -> Integer>(id)[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("function generic class alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<(Integer) -> Integer>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.FnI64ToI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_nested_map_result_generic_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-nested-map-result-generic-alias-index",
    );
    let source_path = temp_root
        .join("no_check_current_package_exact_nested_map_result_generic_alias_index.arden");
    let output_path =
        temp_root.join("no_check_current_package_exact_nested_map_result_generic_alias_index");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Map<Map<String, Integer>, Result<Integer, String>>>(
                    Map<Map<String, Integer>, Result<Integer, String>>()
                )[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("nested map/result generic class alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<Map<Map<String, Integer>, Result<Integer, String>>>"),
        "{err}"
    );
    assert!(
        !err.contains("M.Box.spec.MapMapStr_I64_ResI64_Str"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_list_function_generic_class_alias_println_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-list-function-generic-alias-println",
    );
    let source_path =
        temp_root.join("no_check_current_package_exact_list_function_generic_alias_println.arden");
    let output_path =
        temp_root.join("no_check_current_package_exact_list_function_generic_alias_println");
    let source = r#"
            package app;

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) {
                        this.value = value;
                    }
                }
            }

            import app.M.Box as BoxType;

            function main(): None {
                println(BoxType<List<(Integer) -> Integer>>(List<(Integer) -> Integer>()));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("list function generic class alias println should fail");
    assert!(
        err.contains("got M.Box<List<(Integer) -> Integer>>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.ListFnI64ToI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_nested_named_generic_map_result_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-nested-named-generic-map-result-class-alias-index",
    );
    let source_path = temp_root.join(
        "no_check_current_package_exact_nested_named_generic_map_result_class_alias_index.arden",
    );
    let output_path = temp_root
        .join("no_check_current_package_exact_nested_named_generic_map_result_class_alias_index");
    let source = r#"
            package app;

            module N {
                class Inner<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Map<N.Inner<Integer>, Result<N.Inner<String>, String>>>(
                    Map<N.Inner<Integer>, Result<N.Inner<String>, String>>()
                )[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("nested named generic map/result class alias indexing should fail");
    assert!(
        err.contains(
            "Cannot index type M.Box<Map<N.Inner<Integer>, Result<N.Inner<String>, String>>>"
        ),
        "{err}"
    );
    assert!(!err.contains("N.InnerI64.ResGN.InnerStr"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_named_generic_payload_class_alias_index_with_user_facing_type(
) {
    let temp_root =
        make_temp_project_root("no-check-current-package-exact-named-generic-payload-alias-index");
    let source_path =
        temp_root.join("no_check_current_package_exact_named_generic_payload_alias_index.arden");
    let output_path =
        temp_root.join("no_check_current_package_exact_named_generic_payload_alias_index");
    let source = r#"
            package app;

            module Payload {
                class Item<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<Payload.Item<Integer>>(Payload.Item<Integer>(7))[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("named generic payload alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<Payload.Item<Integer>>"),
        "{err}"
    );
    assert!(!err.contains("M.Box.spec.GPayload.ItemI64"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_underscored_named_generic_payload_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-underscored-named-generic-payload-alias-index",
    );
    let source_path = temp_root
        .join("no_check_current_package_exact_underscored_named_generic_payload_alias_index.arden");
    let output_path = temp_root
        .join("no_check_current_package_exact_underscored_named_generic_payload_alias_index");
    let source = r#"
            package app;

            module N {
                class Inner_Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<N.Inner_Box<Integer>>(N.Inner_Box<Integer>(7))[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("underscored named generic payload alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<N.Inner_Box<Integer>>"),
        "{err}"
    );
    assert!(!err.contains("N.Inner_<Box<Integer>>"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_current_package_exact_import_underscored_two_arg_named_generic_payload_class_alias_index_with_user_facing_type(
) {
    let temp_root = make_temp_project_root(
        "no-check-current-package-exact-underscored-two-arg-named-generic-payload-alias-index",
    );
    let source_path = temp_root.join(
        "no_check_current_package_exact_underscored_two_arg_named_generic_payload_alias_index.arden",
    );
    let output_path = temp_root.join(
        "no_check_current_package_exact_underscored_two_arg_named_generic_payload_alias_index",
    );
    let source = r#"
            package app;

            module N {
                class Inner_Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            module O {
                class Pair_Box<T, U> {
                    first: T;
                    second: U;
                    constructor(first: T, second: U) { this.first = first; this.second = second; }
                }
            }

            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import app.M.Box as BoxType;

            function main(): Integer {
                return BoxType<O.Pair_Box<N.Inner_Box<Integer>, String>>(O.Pair_Box<N.Inner_Box<Integer>, String>(N.Inner_Box<Integer>(7), "x"))[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("underscored two-arg named generic payload alias indexing should fail");
    assert!(
        err.contains("Cannot index type M.Box<O.Pair_Box<N.Inner_Box<Integer>, String>>"),
        "{err}"
    );
    assert!(
        !err.contains("O.Pair_Box<N.Inner_<Box<Integer>>, String>"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_field_non_function_call_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-field-call-non-function-type");
    let source_path = temp_root.join("no_check_field_call_non_function_type.arden");
    let output_path = temp_root.join("no_check_field_call_non_function_type");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) {
                    this.value = value;
                }
            }

            function main(): Integer {
                b: Box = Box(1);
                return b.value();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("field non-function call should fail in codegen without checks");
    assert!(
        err.contains("Cannot call non-function type Integer"),
        "{err}"
    );
    assert!(
        !err.contains("Unknown method 'value' for class 'Box'"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_indexing_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-integer-index-type");
    let source_path = temp_root.join("no_check_integer_index_type.arden");
    let output_path = temp_root.join("no_check_integer_index_type");
    let source = r#"
            function main(): Integer {
                value: Integer = 7;
                return value[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("integer indexing should fail in codegen without checks");
    assert!(err.contains("Cannot index type Integer"), "{err}");
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_class_indexing_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-class-index-type");
    let source_path = temp_root.join("no_check_class_index_type.arden");
    let output_path = temp_root.join("no_check_class_index_type");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) {
                    this.value = value;
                }
            }

            function main(): Integer {
                b: Box = Box(1);
                return b[0];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("class indexing should fail in codegen without checks");
    assert!(err.contains("Cannot index type Box"), "{err}");
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_class_indexing_with_user_facing_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-module-local-class-index-type");
    let source_path = temp_root.join("no_check_module_local_class_index_type.arden");
    let output_path = temp_root.join("no_check_module_local_class_index_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) {
                        this.value = value;
                    }
                }
            }

            function render(): Integer {
                return M.Box(1)[0];
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local class indexing should fail in codegen without checks");
    assert!(err.contains("Cannot index type M.Box"), "{err}");
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_index_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-integer-index-assign-type");
    let source_path = temp_root.join("no_check_integer_index_assign_type.arden");
    let output_path = temp_root.join("no_check_integer_index_assign_type");
    let source = r#"
            function main(): None {
                mut value: Integer = 7;
                value[0] = 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("integer index assignment should fail in codegen without checks");
    assert!(err.contains("Cannot index type Integer"), "{err}");
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_class_index_assignment_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-class-index-assign-type");
    let source_path = temp_root.join("no_check_class_index_assign_type.arden");
    let output_path = temp_root.join("no_check_class_index_assign_type");
    let source = r#"
            class Box {
                value: Integer;
                constructor(value: Integer) {
                    this.value = value;
                }
            }

            function main(): None {
                mut b: Box = Box(1);
                b[0] = 2;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("class index assignment should fail in codegen without checks");
    assert!(err.contains("Cannot index type Box"), "{err}");
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_class_index_assignment_with_user_facing_type_diagnostic(
) {
    let temp_root = make_temp_project_root("no-check-module-local-class-index-assign-type");
    let source_path = temp_root.join("no_check_module_local_class_index_assign_type.arden");
    let output_path = temp_root.join("no_check_module_local_class_index_assign_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                mut value: M.Box = M.Box(1);
                value[0] = 1;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local class index assignment should fail in codegen");
    assert!(err.contains("Cannot index type M.Box"), "{err}");
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_integer_deref_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-integer-deref-type");
    let source_path = temp_root.join("no_check_integer_deref_type.arden");
    let output_path = temp_root.join("no_check_integer_deref_type");
    let source = r#"
            function main(): Integer {
                value: Integer = 7;
                return *value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("integer deref should fail in codegen without checks");
    assert!(
        err.contains("Cannot dereference non-pointer type Integer"),
        "{err}"
    );
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_literal_deref_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-literal-deref-type");
    let source_path = temp_root.join("no_check_literal_deref_type.arden");
    let output_path = temp_root.join("no_check_literal_deref_type");
    let source = r#"
            function main(): Integer {
                return *1;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("literal deref should fail in codegen without checks");
    assert!(
        err.contains("Cannot dereference non-pointer type Integer"),
        "{err}"
    );
    assert!(!err.contains("expected PointerValue"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_exact_import_alias_deref_with_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-exact-import-alias-deref-type");
    let source_path = temp_root.join("no_check_exact_import_alias_deref_type.arden");
    let output_path = temp_root.join("no_check_exact_import_alias_deref_type");
    let source = r#"
            import std.system.cwd as CurrentDir;

            function main(): Integer {
                return *CurrentDir;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("exact import alias deref should fail in codegen without checks");
    assert!(
        err.contains("Cannot dereference non-pointer type String"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: CurrentDir"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_deref_with_user_facing_type_diagnostic() {
    let temp_root = make_temp_project_root("no-check-module-local-deref-type");
    let source_path = temp_root.join("no_check_module_local_deref_type.arden");
    let output_path = temp_root.join("no_check_module_local_deref_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): Integer {
                return *M.Box(1);
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("module-local deref should fail in codegen without checks");
    assert!(
        err.contains("Cannot dereference non-pointer type M.Box"),
        "{err}"
    );
    assert!(!err.contains("Undefined variable: M"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
