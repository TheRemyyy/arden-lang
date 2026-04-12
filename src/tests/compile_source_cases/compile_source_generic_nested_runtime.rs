use super::*;
use std::fs;

#[test]
fn compile_source_no_check_runs_imported_option_alias_match_runtime() {
    let temp_root = make_temp_project_root("no-check-imported-option-alias-match-runtime");
    let source_path = temp_root.join("no_check_imported_option_alias_match_runtime.arden");
    let output_path = temp_root.join("no_check_imported_option_alias_match_runtime");
    let source = r#"
            import Option.Some as Present;
            import Option.None as Empty;

            function main(): Integer {
                value: Option<Integer> = Present(7);
                return match (value) {
                    Present(inner) => if (inner == 7) { 0 } else { 1 },
                    Empty => 2,
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked imported Option alias match should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked imported Option alias match binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_imported_inferred_generic_class_constructor_function_value_runtime()
{
    let temp_root =
        make_temp_project_root("no-check-imported-inferred-generic-class-ctor-fn-value-runtime");
    let source_path =
        temp_root.join("no_check_imported_inferred_generic_class_ctor_fn_value_runtime.arden");
    let output_path =
        temp_root.join("no_check_imported_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import M.Box as B;

            function main(): Integer {
                ctor: (Integer) -> M.Box<Integer> = B;
                return ctor(8).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None).must(
        "unchecked imported inferred generic class constructor function value should codegen",
    );

    let status = std::process::Command::new(&output_path).status().must(
        "run compiled unchecked imported inferred generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_namespace_alias_inferred_generic_class_constructor_function_value_runtime(
) {
    let temp_root = make_temp_project_root(
        "no-check-namespace-alias-inferred-generic-class-ctor-fn-value-runtime",
    );
    let source_path = temp_root
        .join("no_check_namespace_alias_inferred_generic_class_ctor_fn_value_runtime.arden");
    let output_path =
        temp_root.join("no_check_namespace_alias_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module U {
                module M {
                    class Box<T> {
                        value: T;
                        constructor(value: T) { this.value = value; }
                    }
                }
            }

            import U as u;

            function main(): Integer {
                ctor: (Integer) -> u.M.Box<Integer> = u.M.Box;
                return ctor(9).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None).must(
        "unchecked namespace alias inferred generic class constructor function value should codegen",
    );

    let status = std::process::Command::new(&output_path).status().must(
        "run compiled unchecked namespace alias inferred generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(9));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_wildcard_imported_inferred_generic_class_constructor_function_value_runtime(
) {
    let temp_root = make_temp_project_root(
        "no-check-wildcard-imported-inferred-generic-class-ctor-fn-value-runtime",
    );
    let source_path = temp_root
        .join("no_check_wildcard_imported_inferred_generic_class_ctor_fn_value_runtime.arden");
    let output_path =
        temp_root.join("no_check_wildcard_imported_inferred_generic_class_ctor_fn_value_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }

            import M.*;

            function main(): Integer {
                ctor: (Integer) -> Box<Integer> = Box;
                return ctor(17).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None).must(
        "unchecked wildcard imported inferred generic class constructor function value should codegen",
    );

    let status = std::process::Command::new(&output_path).status().must(
        "run compiled unchecked wildcard imported inferred generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(17));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_nested_generic_class_field_access_runtime() {
    let temp_root = make_temp_project_root("no-check-nested-generic-class-field-runtime");
    let source_path = temp_root.join("no_check_nested_generic_class_field_runtime.arden");
    let output_path = temp_root.join("no_check_nested_generic_class_field_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            function main(): Integer {
                return M.Box<Integer>(6).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked nested generic class field access should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked nested generic class field access binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_nested_generic_class_method_runtime() {
    let temp_root = make_temp_project_root("no-check-nested-generic-class-method-runtime");
    let source_path = temp_root.join("no_check_nested_generic_class_method_runtime.arden");
    let output_path = temp_root.join("no_check_nested_generic_class_method_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }

            function main(): Integer {
                return M.Box<Integer>(6).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked nested generic class method call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked nested generic class method call binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_wildcard_imported_nested_generic_class_field_access_runtime() {
    let temp_root =
        make_temp_project_root("no-check-wildcard-imported-nested-generic-class-field-runtime");
    let source_path =
        temp_root.join("no_check_wildcard_imported_nested_generic_class_field_runtime.arden");
    let output_path =
        temp_root.join("no_check_wildcard_imported_nested_generic_class_field_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                }
            }

            import M.*;

            function main(): Integer {
                return Box<Integer>(13).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked wildcard imported nested generic class field access should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked wildcard imported nested generic class field access binary");
    assert_eq!(status.code(), Some(13));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_runs_wildcard_imported_nested_generic_class_method_runtime() {
    let temp_root =
        make_temp_project_root("no-check-wildcard-imported-nested-generic-class-method-runtime");
    let source_path =
        temp_root.join("no_check_wildcard_imported_nested_generic_class_method_runtime.arden");
    let output_path =
        temp_root.join("no_check_wildcard_imported_nested_generic_class_method_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }

            import M.*;

            function main(): Integer {
                return Box<Integer>(13).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, false, None, None)
        .must("unchecked wildcard imported nested generic class method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled unchecked wildcard imported nested generic class method binary");
    assert_eq!(status.code(), Some(13));

    let _ = fs::remove_dir_all(temp_root);
}
