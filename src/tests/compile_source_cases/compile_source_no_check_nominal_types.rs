use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_extern_function_values_even_with_adapter_signature() {
    let temp_root = make_temp_project_root("no-check-extern-function-value-adapter");
    let source_path = temp_root.join("no_check_extern_function_value_adapter.arden");
    let output_path = temp_root.join("no_check_extern_function_value_adapter");
    let source = r#"
            extern(c, "puts") function puts(s: String): Integer;

            function main(): Integer {
                f: (String) -> Float = puts;
                value: Float = f("hi");
                return if (value > 0.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("extern function values with adapter signatures should fail in codegen");
    assert!(
        err.contains("extern function 'puts' cannot be used as a first-class value yet"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_user_defined_generic_enum_without_panicking() {
    let temp_root = make_temp_project_root("no-check-generic-enum-no-panic");
    let source_path = temp_root.join("no_check_generic_enum_no_panic.arden");
    let output_path = temp_root.join("no_check_generic_enum_no_panic");
    let source = r#"
            enum Boxed<T> {
                Item(value: T)
            }

            function main(): Integer {
                value: Boxed<Integer> = Boxed.Item(7);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("generic enum in no-check mode should fail with a diagnostic, not panic");
    assert!(
        err.contains("user-defined generic enums are not supported yet"),
        "{err}"
    );
    assert!(!err.contains("panicked at"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_function_value_adapter_between_unrelated_nominal_returns() {
    let temp_root = make_temp_project_root("no-check-fn-adapter-unrelated-nominal-return");
    let source_path = temp_root.join("no_check_fn_adapter_unrelated_nominal_return.arden");
    let output_path = temp_root.join("no_check_fn_adapter_unrelated_nominal_return");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function make_a(): A {
                return A();
            }

            function main(): Integer {
                f: () -> B = make_a;
                value: B = f();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated nominal return adapter should fail in codegen");
    assert!(
        err.contains("Cannot use function value () -> A as () -> B"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_for_loop_binding_between_unrelated_nominal_types() {
    let temp_root = make_temp_project_root("no-check-for-binding-unrelated-nominal");
    let source_path = temp_root.join("no_check_for_binding_unrelated_nominal.arden");
    let output_path = temp_root.join("no_check_for_binding_unrelated_nominal");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                xs: List<A> = List<A>();
                xs.push(A());

                for (item: B in xs) {
                    return 0;
                }

                return 1;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated nominal for-loop binding should fail in codegen");
    assert!(
        err.contains("unsupported for-loop binding conversion: Named(\"A\") -> Named(\"B\")"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_let_binding_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-let-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_let_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_let_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                value: B = A();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class let binding should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_assignment_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-assign-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_assign_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_assign_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                mut value: B = B();
                value = A();
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class assignment should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_list_push_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-list-push-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_list_push_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_list_push_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                xs: List<B> = List<B>();
                xs.push(A());
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class list push should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_option_some_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-option-some-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_option_some_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_option_some_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                value: Option<B> = Option.Some(A());
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class Option.some payload should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_result_ok_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-result-ok-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_result_ok_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_result_ok_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                value: Result<B, String> = Result.Ok(A());
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class Result.ok payload should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_map_set_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-map-set-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_map_set_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_map_set_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                values: Map<String, B> = Map<String, B>();
                values.set("x", A());
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class map set payload should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_map_key_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-map-key-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_map_key_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_map_key_unrelated_concrete_classes");
    let source = r#"
            class K1 {
                constructor() {}
            }

            class K2 {
                constructor() {}
            }

            function main(): Integer {
                values: Map<K1, Integer> = Map<K1, Integer>();
                values.set(K2(), 7);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class map key should fail in codegen");
    assert!(err.contains("Type mismatch: expected K1, got K2"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_map_get_key_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-map-get-key-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_map_get_key_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_map_get_key_unrelated_concrete_classes");
    let source = r#"
            class K1 {
                constructor() {}
            }

            class K2 {
                constructor() {}
            }

            function main(): Integer {
                values: Map<K1, Integer> = Map<K1, Integer>();
                values.set(K1(), 7);
                return values.get(K2());
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class map get key should fail in codegen");
    assert!(err.contains("Type mismatch: expected K1, got K2"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_list_set_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-list-set-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_list_set_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_list_set_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function main(): Integer {
                mut values: List<B> = List<B>();
                values.push(B());
                values.set(0, A());
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class list set payload should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_constructor_argument_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-ctor-arg-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_ctor_arg_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_ctor_arg_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            class Holder {
                value: B;

                constructor(value: B) {
                    this.value = value;
                }
            }

            function main(): Integer {
                box: Holder = Holder(A());
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class constructor argument should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_enum_variant_payload_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-enum-payload-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_enum_payload_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_enum_payload_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            enum Wrap {
                One(B)
            }

            function main(): Integer {
                value: Wrap = Wrap.One(A());
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class enum payload should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_function_argument_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-fn-arg-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_fn_arg_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_fn_arg_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            function take(value: B): Integer {
                return 0;
            }

            function main(): Integer {
                return take(A());
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class function argument should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_method_argument_between_unrelated_concrete_classes() {
    let temp_root = make_temp_project_root("no-check-method-arg-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_method_arg_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_method_arg_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            class Holder {
                constructor() {}

                function take(value: B): Integer {
                    return 0;
                }
            }

            function main(): Integer {
                h: Holder = Holder();
                return h.take(A());
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class method argument should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_field_function_value_argument_between_unrelated_concrete_classes(
) {
    let temp_root = make_temp_project_root("no-check-field-fn-arg-unrelated-concrete-classes");
    let source_path = temp_root.join("no_check_field_fn_arg_unrelated_concrete_classes.arden");
    let output_path = temp_root.join("no_check_field_fn_arg_unrelated_concrete_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            class Holder {
                callback: (B) -> Integer;

                constructor(callback: (B) -> Integer) {
                    this.callback = callback;
                }
            }

            function take(value: B): Integer {
                return 0;
            }

            function main(): Integer {
                h: Holder = Holder(take);
                return h.callback(A());
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class field function argument should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_nested_enum_variant_payload_between_unrelated_concrete_classes()
{
    let temp_root = make_temp_project_root("no-check-nested-enum-payload-unrelated-classes");
    let source_path = temp_root.join("no_check_nested_enum_payload_unrelated_classes.arden");
    let output_path = temp_root.join("no_check_nested_enum_payload_unrelated_classes");
    let source = r#"
            class A {
                constructor() {}
            }

            class B {
                constructor() {}
            }

            module Outer {
                module Inner {
                    enum Wrap {
                        One(B)
                    }
                }
            }

            function main(): Integer {
                value: Outer.Inner.Wrap = Outer.Inner.Wrap.One(A());
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("unrelated concrete class nested enum payload should fail in codegen");
    assert!(err.contains("Type mismatch: expected B, got A"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
