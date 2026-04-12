use super::*;
use std::fs;

#[test]
fn compile_source_runs_mixed_numeric_arithmetic_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-arithmetic-runtime");
    let source_path = temp_root.join("mixed_numeric_arithmetic_runtime.arden");
    let output_path = temp_root.join("mixed_numeric_arithmetic_runtime");
    let source = r#"
            function main(): Integer {
                sum: Float = 1 + 2.5;
                product: Float = 3.0 * 2;
                less: Boolean = 1 < 1.5;
                greater_or_equal: Boolean = 6.0 >= 6;
                return if (sum == 3.5 && product == 6.0 && less && greater_or_equal) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric arithmetic should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric arithmetic binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_equality_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-equality-runtime");
    let source_path = temp_root.join("mixed_numeric_equality_runtime.arden");
    let output_path = temp_root.join("mixed_numeric_equality_runtime");
    let source = r#"
            function main(): Integer {
                left_to_right: Boolean = 1 == 1.0;
                right_to_left: Boolean = 1.0 == 1;
                neq_left_to_right: Boolean = 1 != 2.0;
                neq_right_to_left: Boolean = 2.0 != 1;
                return if (left_to_right && right_to_left && neq_left_to_right && neq_right_to_left) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric equality should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric equality binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_branch_and_math_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-branch-math-runtime");
    let source_path = temp_root.join("mixed_numeric_branch_math_runtime.arden");
    let output_path = temp_root.join("mixed_numeric_branch_math_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                branch: Float = if (true) { 1 } else { 2.5 };
                min_value: Float = Math.min(1, 2.5);
                max_value: Float = Math.max(2, 1.5);
                return if (branch == 1.0 && min_value == 1.0 && max_value == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric branch and math should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric branch and math binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_match_expression_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-match-runtime");
    let source_path = temp_root.join("mixed_numeric_match_runtime.arden");
    let output_path = temp_root.join("mixed_numeric_match_runtime");
    let source = r#"
            enum Kind {
                IntCase,
                FloatCase
            }

            function main(): Integer {
                kind: Kind = Kind.IntCase;
                value: Float = match (kind) {
                    Kind.IntCase => 1,
                    Kind.FloatCase => 2.5,
                };
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric match expression should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric match expression binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_assert_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-assert-runtime");
    let source_path = temp_root.join("mixed_numeric_assert_runtime.arden");
    let output_path = temp_root.join("mixed_numeric_assert_runtime");
    let source = r#"
            function main(): Integer {
                assert_eq(1, 1.0);
                assert_eq(1.0, 1);
                assert_ne(1, 2.0);
                assert_ne(2.0, 1);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric assert helpers should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric assert binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mixed_numeric_match_literal_runtime() {
    let temp_root = make_temp_project_root("mixed-numeric-match-literal-runtime");
    let source_path = temp_root.join("mixed_numeric_match_literal_runtime.arden");
    let output_path = temp_root.join("mixed_numeric_match_literal_runtime");
    let source = r#"
            function main(): Integer {
                first: Integer = match (1.0) {
                    1 => 0,
                    _ => 1,
                };
                second: Integer = match (1) {
                    1.0 => 0,
                    _ => 2,
                };
                return if (first == 0 && second == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed numeric match literal should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mixed numeric match literal binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_parameter_runtime() {
    let temp_root = make_temp_project_root("int-to-float-param-runtime");
    let source_path = temp_root.join("int_to_float_param_runtime.arden");
    let output_path = temp_root.join("int_to_float_param_runtime");
    let source = r#"
            function echo(value: Float): Float {
                return value;
            }

            function main(): Integer {
                value: Float = echo(1);
                return if (value == 1.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer argument to float parameter should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float parameter binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_method_and_constructor_runtime() {
    let temp_root = make_temp_project_root("int-to-float-method-ctor-runtime");
    let source_path = temp_root.join("int_to_float_method_ctor_runtime.arden");
    let output_path = temp_root.join("int_to_float_method_ctor_runtime");
    let source = r#"
            class Boxed {
                value: Float;
                constructor(value: Float) {
                    this.value = value;
                }
                function scale(factor: Float): Float {
                    return this.value * factor;
                }
            }

            function main(): Integer {
                box: Boxed = Boxed(2);
                scaled: Float = box.scale(3);
                return if (box.value == 2.0 && scaled == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer argument to float method and constructor should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float method/constructor binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_function_value_runtime() {
    let temp_root = make_temp_project_root("int-to-float-function-value-runtime");
    let source_path = temp_root.join("int_to_float_function_value_runtime.arden");
    let output_path = temp_root.join("int_to_float_function_value_runtime");
    let source = r#"
            function main(): Integer {
                scale: (Float) -> Float = (value: Float) => value * 2.0;
                result: Float = scale(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer argument to float function value should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_module_local_function_call_runtime() {
    let temp_root = make_temp_project_root("int-to-float-module-local-function-call-runtime");
    let source_path = temp_root.join("int_to_float_module_local_function_call_runtime.arden");
    let output_path = temp_root.join("int_to_float_module_local_function_call_runtime");
    let source = r#"
            module Mathy {
                function scale(value: Float): Float {
                    return value * 2.0;
                }
            }

            function main(): Integer {
                result: Float = Mathy.scale(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer argument to Float module-local function call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float module-local function call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_nested_module_function_call_runtime() {
    let temp_root = make_temp_project_root("int-to-float-nested-module-function-call-runtime");
    let source_path = temp_root.join("int_to_float_nested_module_function_call_runtime.arden");
    let output_path = temp_root.join("int_to_float_nested_module_function_call_runtime");
    let source = r#"
            module Mathy {
                module Ops {
                    function scale(value: Float): Float {
                        return value * 2.0;
                    }
                }
            }

            function main(): Integer {
                result: Float = Mathy.Ops.scale(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer argument to Float nested-module function call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float nested-module function call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_exact_import_alias_call_runtime() {
    let temp_root = make_temp_project_root("int-to-float-exact-import-alias-call-runtime");
    let source_path = temp_root.join("int_to_float_exact_import_alias_call_runtime.arden");
    let output_path = temp_root.join("int_to_float_exact_import_alias_call_runtime");
    let source = r#"
            module Mathy {
                function scale(value: Float): Float {
                    return value * 2.0;
                }
            }

            import Mathy.scale as scalef;

            function main(): Integer {
                result: Float = scalef(3);
                return if (result == 6.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer argument to Float exact-import alias call should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float exact-import alias call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_enum_payload_variant_runtime() {
    let temp_root = make_temp_project_root("int-to-float-enum-payload-variant-runtime");
    let source_path = temp_root.join("int_to_float_enum_payload_variant_runtime.arden");
    let output_path = temp_root.join("int_to_float_enum_payload_variant_runtime");
    let source = r#"
            enum Metric {
                Value(Float)
            }

            function main(): Integer {
                metric: Metric = Metric.Value(3);
                return match (metric) {
                    Metric.Value(value) => {
                        if (value == 3.0) { 0 } else { 1 }
                    }
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer argument to Float enum payload variant should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float enum payload variant binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_nested_enum_payload_variant_runtime() {
    let temp_root = make_temp_project_root("int-to-float-nested-enum-payload-variant-runtime");
    let source_path = temp_root.join("int_to_float_nested_enum_payload_variant_runtime.arden");
    let output_path = temp_root.join("int_to_float_nested_enum_payload_variant_runtime");
    let source = r#"
            module Metrics {
                enum Metric {
                    Value(Float)
                }
            }

            function main(): Integer {
                metric: Metrics.Metric = Metrics.Metric.Value(3);
                return match (metric) {
                    Metrics.Metric.Value(value) => {
                        if (value == 3.0) { 0 } else { 1 }
                    }
                };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer argument to Float nested enum payload variant should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float nested enum payload variant binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_argument_to_float_container_methods_runtime() {
    let temp_root = make_temp_project_root("int-to-float-container-methods-runtime");
    let source_path = temp_root.join("int_to_float_container_methods_runtime.arden");
    let output_path = temp_root.join("int_to_float_container_methods_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Float> = List<Float>();
                xs.push(1);
                xs.set(0, 4);

                m: Map<String, Float> = Map<String, Float>();
                m.set("k", 2);

                s: Set<Float> = Set<Float>();
                s.add(3);

                return if (xs[0] == 4.0 && m["k"] == 2.0 && s.contains(3.0)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer argument to float container methods should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float container methods binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_assignment_into_float_containers_runtime() {
    let temp_root = make_temp_project_root("int-to-float-container-assign-runtime");
    let source_path = temp_root.join("int_to_float_container_assign_runtime.arden");
    let output_path = temp_root.join("int_to_float_container_assign_runtime");
    let source = r#"
            class Boxed {
                mut items: List<Float>;
                constructor() {
                    this.items = List<Float>();
                    this.items.push(1.0);
                }
            }

            function main(): Integer {
                mut xs: List<Float> = List<Float>();
                xs.push(1.0);
                xs[0] = 5;

                mut m: Map<String, Float> = Map<String, Float>();
                m["k"] = 6;

                mut box: Boxed = Boxed();
                box.items[0] = 7;

                return if (xs[0] == 5.0 && m["k"] == 6.0 && box.items[0] == 7.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer assignment into float containers should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float container assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_assignment_into_float_fields_runtime() {
    let temp_root = make_temp_project_root("int-to-float-field-assign-runtime");
    let source_path = temp_root.join("int_to_float_field_assign_runtime.arden");
    let output_path = temp_root.join("int_to_float_field_assign_runtime");
    let source = r#"
            class Boxed {
                mut value: Float;
                constructor() {
                    this.value = 1;
                }
            }

            function main(): Integer {
                mut box: Boxed = Boxed();
                box.value = 2;
                return if (box.value == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer assignment into float fields should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float field assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_arguments_to_float_math_unary_runtime() {
    let temp_root = make_temp_project_root("int-to-float-math-unary-runtime");
    let source_path = temp_root.join("int_to_float_math_unary_runtime.arden");
    let output_path = temp_root.join("int_to_float_math_unary_runtime");
    let source = r#"
            import std.math.*;

            function main(): Integer {
                floorValue: Float = Math.floor(2);
                ceilValue: Float = Math.ceil(2);
                roundValue: Float = Math.round(2);
                return if (floorValue == 2.0 && ceilValue == 2.0 && roundValue == 2.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer arguments to float math unary functions should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float math unary binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
