use super::*;
use std::fs;

#[test]
fn compile_source_runs_imported_generic_nested_type_alias_runtime() {
    let temp_root = make_temp_project_root("imported-generic-nested-type-alias-runtime");
    let source_path = temp_root.join("imported_generic_nested_type_alias_runtime.arden");
    let output_path = temp_root.join("imported_generic_nested_type_alias_runtime");
    let source = r#"
            module M {
                class Box<T> {
                    value: T;
                    constructor(value: T) { this.value = value; }
                    function get(): T { return this.value; }
                }
            }
            import M.Box as B;
            function main(): Integer {
                return B<Integer>(2).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported generic nested type alias should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported generic nested type alias binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("generic_class_ctor_fn_value_runtime.arden");
    let output_path = temp_root.join("generic_class_ctor_fn_value_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled generic class constructor function value binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("imported_generic_class_ctor_fn_value_runtime.arden");
    let output_path = temp_root.join("imported_generic_class_ctor_fn_value_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported generic class constructor function value binary");
    assert_eq!(status.code(), Some(4));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_nested_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-nested-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("imported_nested_generic_class_ctor_fn_value_runtime.arden");
    let output_path = temp_root.join("imported_nested_generic_class_ctor_fn_value_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported nested generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported nested generic class constructor function value binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_inferred_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("inferred-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("inferred_generic_class_ctor_fn_value_runtime.arden");
    let output_path = temp_root.join("inferred_generic_class_ctor_fn_value_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled inferred generic class constructor function value binary");
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_inferred_generic_class_constructor_function_value_runtime() {
    let temp_root = make_temp_project_root("imported-inferred-generic-class-ctor-fn-value-runtime");
    let source_path = temp_root.join("imported_inferred_generic_class_ctor_fn_value_runtime.arden");
    let output_path = temp_root.join("imported_inferred_generic_class_ctor_fn_value_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported inferred generic class constructor function value binary");
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_inferred_generic_class_constructor_function_value_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-inferred-generic-class-ctor-fn-value-runtime");
    let source_path =
        temp_root.join("namespace_alias_inferred_generic_class_ctor_fn_value_runtime.arden");
    let output_path =
        temp_root.join("namespace_alias_inferred_generic_class_ctor_fn_value_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("namespace alias inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path).status().must(
        "run compiled namespace alias inferred generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(9));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_inferred_generic_class_constructor_function_value_runtime()
{
    let temp_root =
        make_temp_project_root("wildcard-imported-inferred-generic-class-ctor-fn-value-runtime");
    let source_path =
        temp_root.join("wildcard_imported_inferred_generic_class_ctor_fn_value_runtime.arden");
    let output_path =
        temp_root.join("wildcard_imported_inferred_generic_class_ctor_fn_value_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("wildcard imported inferred generic class constructor function value should codegen");

    let status = std::process::Command::new(&output_path).status().must(
        "run compiled wildcard imported inferred generic class constructor function value binary",
    );
    assert_eq!(status.code(), Some(17));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_nested_generic_class_field_access_runtime() {
    let temp_root = make_temp_project_root("nested-generic-class-field-runtime");
    let source_path = temp_root.join("nested_generic_class_field_runtime.arden");
    let output_path = temp_root.join("nested_generic_class_field_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested generic class field access should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled nested generic class field access binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_nested_generic_class_method_runtime() {
    let temp_root = make_temp_project_root("nested-generic-class-method-runtime");
    let source_path = temp_root.join("nested_generic_class_method_runtime.arden");
    let output_path = temp_root.join("nested_generic_class_method_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested generic class method call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled nested generic class method call binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_nested_generic_class_field_access_runtime() {
    let temp_root = make_temp_project_root("wildcard-imported-nested-generic-class-field-runtime");
    let source_path = temp_root.join("wildcard_imported_nested_generic_class_field_runtime.arden");
    let output_path = temp_root.join("wildcard_imported_nested_generic_class_field_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("wildcard imported nested generic class field access should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled wildcard imported nested generic class field access binary");
    assert_eq!(status.code(), Some(13));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_wildcard_imported_nested_generic_class_method_runtime() {
    let temp_root = make_temp_project_root("wildcard-imported-nested-generic-class-method-runtime");
    let source_path = temp_root.join("wildcard_imported_nested_generic_class_method_runtime.arden");
    let output_path = temp_root.join("wildcard_imported_nested_generic_class_method_runtime");
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
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("wildcard imported nested generic class method should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled wildcard imported nested generic class method binary");
    assert_eq!(status.code(), Some(13));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_imported_enum_type_alias_variant_runtime() {
    let temp_root = make_temp_project_root("imported-enum-type-alias-variant-runtime");
    let source_path = temp_root.join("imported_enum_type_alias_variant_runtime.arden");
    let output_path = temp_root.join("imported_enum_type_alias_variant_runtime");
    let source = r#"
            enum E { A(Integer) }
            import E as Alias;
            function main(): Integer {
                value: Alias = Alias.A(2);
                return match (value) {
                    Alias.A(v) => { v }
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("imported enum type alias variant should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled imported enum type alias variant binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_nested_generic_class_constructor_runtime() {
    let temp_root =
        make_temp_project_root("namespace-alias-nested-generic-class-constructor-runtime");
    let source_path =
        temp_root.join("namespace_alias_nested_generic_class_constructor_runtime.arden");
    let output_path = temp_root.join("namespace_alias_nested_generic_class_constructor_runtime");
    let source = r#"
            module U {
                module M {
                    class Box<T> {
                        value: T;
                        constructor(value: T) { this.value = value; }
                        function get(): T { return this.value; }
                    }
                }
            }
            import U as u;
            function main(): Integer {
                return u.M.Box<Integer>(2).get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("namespace alias nested generic class constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled namespace alias nested generic class constructor binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_namespace_alias_enum_variant_constructor_runtime() {
    let temp_root = make_temp_project_root("namespace-alias-enum-variant-constructor-runtime");
    let source_path = temp_root.join("namespace_alias_enum_variant_constructor_runtime.arden");
    let output_path = temp_root.join("namespace_alias_enum_variant_constructor_runtime");
    let source = r#"
            module U {
                enum E { A(Integer), B }
            }
            import U as u;
            function main(): Integer {
                value: u.E = u.E.A(2);
                return match (value) { u.E.A(v) => { v } u.E.B => { 0 } };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("namespace alias enum variant constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled namespace alias enum variant constructor binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}
