use super::*;
use std::fs;

#[test]
fn compile_source_runs_option_is_some_in_condition() {
    let temp_root = make_temp_project_root("option-is-some-condition-runtime");
    let source_path = temp_root.join("option_is_some_condition_runtime.arden");
    let output_path = temp_root.join("option_is_some_condition_runtime");
    let source = r#"
            function choose(): Option<Integer> {
                return Option.some(1);
            }

            function main(): Integer {
                return if (choose().is_some()) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("option is_some condition should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled option-is-some binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_result_is_ok_in_condition() {
    let temp_root = make_temp_project_root("result-is-ok-condition-runtime");
    let source_path = temp_root.join("result_is_ok_condition_runtime.arden");
    let output_path = temp_root.join("result_is_ok_condition_runtime");
    let source = r#"
            function choose(): Result<Integer, String> {
                return Result.ok(1);
            }

            function main(): Integer {
                return if (choose().is_ok()) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("result is_ok condition should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled result-is-ok binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_length_on_literal_receiver() {
    let temp_root = make_temp_project_root("string-length-literal-runtime");
    let source_path = temp_root.join("string_length_literal_runtime.arden");
    let output_path = temp_root.join("string_length_literal_runtime");
    let source = r#"
            function main(): Integer {
                return "abc".length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string length on literal receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string-length literal binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_length_on_concatenation_receiver() {
    let temp_root = make_temp_project_root("string-length-concat-runtime");
    let source_path = temp_root.join("string_length_concat_runtime.arden");
    let output_path = temp_root.join("string_length_concat_runtime");
    let source = r#"
            function main(): Integer {
                return ("a" + "bc").length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string length on concatenation receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string-length concat binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_string_length_on_interpolation_receiver() {
    let temp_root = make_temp_project_root("string-length-interp-runtime");
    let source_path = temp_root.join("string_length_interp_runtime.arden");
    let output_path = temp_root.join("string_length_interp_runtime");
    let source = r#"
            function main(): Integer {
                return ("a{1}c").length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string length on interpolation receiver should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string-length interpolation binary");
    assert_eq!(status.code(), Some(3));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_list_get_object_results() {
    let temp_root = make_temp_project_root("list-get-object-field-runtime");
    let source_path = temp_root.join("list_get_object_field_runtime.arden");
    let output_path = temp_root.join("list_get_object_field_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                xs: List<Boxed> = List<Boxed>();
                xs.push(Boxed(5));
                return xs.get(0).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("field access on list.get object result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled list-get object field binary");
    assert_eq!(status.code(), Some(5));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_map_get_object_results() {
    let temp_root = make_temp_project_root("map-get-object-field-runtime");
    let source_path = temp_root.join("map_get_object_field_runtime.arden");
    let output_path = temp_root.join("map_get_object_field_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(6));
                return m.get(1).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("field access on map.get object result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map-get object field binary");
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_fails_fast_on_missing_map_get_object_results() {
    let temp_root = make_temp_project_root("map-get-missing-object-runtime");
    let source_path = temp_root.join("map_get_missing_object_runtime.arden");
    let output_path = temp_root.join("map_get_missing_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                return m.get(1).value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("missing map.get object result should still codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled missing map.get object binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_access_on_map_index_object_results() {
    let temp_root = make_temp_project_root("map-index-object-field-runtime");
    let source_path = temp_root.join("map_index_object_field_runtime.arden");
    let output_path = temp_root.join("map_index_object_field_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                m: Map<Integer, Boxed> = Map<Integer, Boxed>();
                m.set(1, Boxed(8));
                return m[1].value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("field access on map index object result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map index object field binary");
    assert_eq!(status.code(), Some(8));

    let _ = fs::remove_dir_all(temp_root);
}
