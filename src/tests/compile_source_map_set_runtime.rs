use super::*;
use std::fs;

#[test]
fn compile_source_runs_map_option_growth_for_earlier_keys() {
    let temp_root = make_temp_project_root("map-option-growth-earlier-runtime");
    let source_path = temp_root.join("map_option_growth_earlier_runtime.arden");
    let output_path = temp_root.join("map_option_growth_earlier_runtime");
    let source = r#"
            function main(): Integer {
                m: Map<Option<Integer>, Integer> = Map<Option<Integer>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Option.some(i), i + 10);
                    i = i + 1;
                }
                return if (m.contains(Option.some(0)) && m.get(Option.some(0)) == 10 && m.contains(Option.some(8)) && m.get(Option.some(8)) == 18) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map option growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map-option growth binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_option_updates_after_growth() {
    let temp_root = make_temp_project_root("map-option-update-runtime");
    let source_path = temp_root.join("map_option_update_runtime.arden");
    let output_path = temp_root.join("map_option_update_runtime");
    let source = r#"
            function main(): Integer {
                m: Map<Option<Integer>, Integer> = Map<Option<Integer>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Option.some(i), i + 10);
                    i = i + 1;
                }
                m.set(Option.some(4), 99);
                return if (m.length() == 9 && m.get(Option.some(4)) == 99 && m.get(Option.some(8)) == 18) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map option update should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map-option update binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_option_remove_after_growth() {
    let temp_root = make_temp_project_root("set-option-remove-runtime");
    let source_path = temp_root.join("set_option_remove_runtime.arden");
    let output_path = temp_root.join("set_option_remove_runtime");
    let source = r#"
            function main(): Integer {
                s: Set<Option<Integer>> = Set<Option<Integer>>();
                mut i: Integer = 0;
                while (i < 9) {
                    s.add(Option.some(i));
                    i = i + 1;
                }
                removed: Boolean = s.remove(Option.some(4));
                return if (removed && !s.contains(Option.some(4)) && s.contains(Option.some(8)) && s.length() == 8) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("set option remove should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled set-option remove binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_result_growth_with_integer_error_keys() {
    let temp_root = make_temp_project_root("map-result-growth-runtime");
    let source_path = temp_root.join("map_result_growth_runtime.arden");
    let output_path = temp_root.join("map_result_growth_runtime");
    let source = r#"
            function main(): Integer {
                m: Map<Result<Integer, Integer>, Integer> = Map<Result<Integer, Integer>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Result.error(i), i + 10);
                    i = i + 1;
                }
                return if (m.contains(Result.error(0)) && m.get(Result.error(0)) == 10 && m.contains(Result.error(8)) && m.get(Result.error(8)) == 18) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map result growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map-result growth binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_result_error_contains_after_growth() {
    let temp_root = make_temp_project_root("set-result-error-growth-runtime");
    let source_path = temp_root.join("set_result_error_growth_runtime.arden");
    let output_path = temp_root.join("set_result_error_growth_runtime");
    let source = r#"
            function main(): Integer {
                s: Set<Result<Integer, Integer>> = Set<Result<Integer, Integer>>();
                mut i: Integer = 0;
                while (i < 9) {
                    s.add(Result.error(i));
                    i = i + 1;
                }
                return if (s.contains(Result.error(0)) && s.contains(Result.error(8)) && !s.contains(Result.error(9))) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("set result growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled set-result growth binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_nested_result_option_growth_and_updates() {
    let temp_root = make_temp_project_root("map-nested-result-option-growth-runtime");
    let source_path = temp_root.join("map_nested_result_option_growth_runtime.arden");
    let output_path = temp_root.join("map_nested_result_option_growth_runtime");
    let source = r#"
            function key(i: Integer): Result<Option<Integer>, Integer> {
                if (i % 2 == 0) {
                    return Result.ok(Option.some(i));
                }
                return Result.error(i);
            }

            function main(): Integer {
                m: Map<Result<Option<Integer>, Integer>, Result<Integer, Integer>> = Map<Result<Option<Integer>, Integer>, Result<Integer, Integer>>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(key(i), Result.ok(i + 10));
                    i = i + 1;
                }
                m.set(key(4), Result.error(99));
                a: Result<Integer, Integer> = m.get(key(0));
                b: Result<Integer, Integer> = m.get(key(4));
                c: Result<Integer, Integer> = m.get(key(7));
                return if (a == Result.ok(10) && b == Result.error(99) && c == Result.ok(17) && m.length() == 9) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested result-option map growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled nested result-option map binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_result_error_with_non_integer_ok_type() {
    let temp_root = make_temp_project_root("result-error-layout-runtime");
    let source_path = temp_root.join("result_error_layout_runtime.arden");
    let output_path = temp_root.join("result_error_layout_runtime");
    let source = r#"
            function bad(): Result<Float, String> {
                return Result.error("x");
            }

            function main(): Integer {
                r: Result<Float, String> = bad();
                return if (r.is_ok()) { 1; } else { 0; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("result error layout should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled result-error layout binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_with_class_pointer_keys() {
    let temp_root = make_temp_project_root("map-class-key-runtime");
    let source_path = temp_root.join("map_class_key_runtime.arden");
    let output_path = temp_root.join("map_class_key_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                a: Boxed = Boxed(1);
                b: Boxed = Boxed(2);
                m: Map<Boxed, Integer> = Map<Boxed, Integer>();
                m.set(a, 11);
                m.set(b, 12);
                return if (m.contains(a) && m.get(a) == 11 && m.get(b) == 12) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map class key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map-class-key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_with_class_pointer_keys() {
    let temp_root = make_temp_project_root("set-class-key-runtime");
    let source_path = temp_root.join("set_class_key_runtime.arden");
    let output_path = temp_root.join("set_class_key_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                a: Boxed = Boxed(1);
                b: Boxed = Boxed(2);
                s: Set<Boxed> = Set<Boxed>();
                s.add(a);
                s.add(b);
                return if (s.contains(a) && s.contains(b)) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("set class key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled set-class-key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_with_nested_option_class_keys() {
    let temp_root = make_temp_project_root("map-option-class-key-runtime");
    let source_path = temp_root.join("map_option_class_key_runtime.arden");
    let output_path = temp_root.join("map_option_class_key_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                a: Boxed = Boxed(1);
                b: Boxed = Boxed(2);
                m: Map<Option<Boxed>, Integer> = Map<Option<Boxed>, Integer>();
                m.set(Option.some(a), 11);
                m.set(Option.some(b), 12);
                return if (m.contains(Option.some(a)) && m.get(Option.some(b)) == 12) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested option class key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled nested option class key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_with_multi_variant_enum_keys() {
    let temp_root = make_temp_project_root("map-enum-key-runtime");
    let source_path = temp_root.join("map_enum_key_runtime.arden");
    let output_path = temp_root.join("map_enum_key_runtime");
    let source = r#"
            enum E {
                A(Integer)
                B(Integer)
            }

            function main(): Integer {
                m: Map<E, Integer> = Map<E, Integer>();
                m.set(E.A(1), 11);
                m.set(E.B(2), 12);
                return if (m.contains(E.A(1)) && m.get(E.A(1)) == 11 && m.get(E.B(2)) == 12) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map enum key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map-enum-key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_with_multi_variant_enum_keys() {
    let temp_root = make_temp_project_root("set-enum-key-runtime");
    let source_path = temp_root.join("set_enum_key_runtime.arden");
    let output_path = temp_root.join("set_enum_key_runtime");
    let source = r#"
            enum E {
                A(Integer)
                B(Integer)
            }

            function main(): Integer {
                s: Set<E> = Set<E>();
                s.add(E.A(1));
                s.add(E.B(2));
                return if (s.contains(E.A(1)) && s.contains(E.B(2))) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("set enum key should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled set-enum-key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
