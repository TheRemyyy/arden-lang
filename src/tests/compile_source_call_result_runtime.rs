use super::*;
use std::fs;

#[test]
fn compile_source_runs_option_unwrap_method_chains_on_call_results() {
    let temp_root = make_temp_project_root("option-call-unwrap-method-runtime");
    let source_path = temp_root.join("option_call_unwrap_method_runtime.arden");
    let output_path = temp_root.join("option_call_unwrap_method_runtime");
    let source = r#"
            class Boxed<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function choose(): Option<Boxed<Integer>> {
                return Option.some(Boxed<Integer>(32));
            }

            function main(): Integer {
                return choose().unwrap().get();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("option unwrap method chain on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled option-unwrap method chain binary");
    assert_eq!(status.code(), Some(32));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_methods_on_call_results() {
    let temp_root = make_temp_project_root("list-call-method-runtime");
    let source_path = temp_root.join("list_call_method_runtime.arden");
    let output_path = temp_root.join("list_call_method_runtime");
    let source = r#"
            function make(): List<Integer> {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                xs.push(2);
                return xs;
            }

            function main(): Integer {
                return make().length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("list method on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled list-call method binary");
    assert_eq!(status.code(), Some(2));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_range_methods_on_call_results() {
    let temp_root = make_temp_project_root("range-call-method-runtime");
    let source_path = temp_root.join("range_call_method_runtime.arden");
    let output_path = temp_root.join("range_call_method_runtime");
    let source = r#"
            function mk(): Range<Integer> {
                return range(0, 10);
            }

            function main(): Integer {
                return if (mk().has_next()) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("range method on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled range-call method binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_methods_on_call_results() {
    let temp_root = make_temp_project_root("set-call-method-runtime");
    let source_path = temp_root.join("set_call_method_runtime.arden");
    let output_path = temp_root.join("set_call_method_runtime");
    let source = r#"
            function build(): Set<Integer> {
                s: Set<Integer> = Set<Integer>();
                s.add(7);
                return s;
            }

            function main(): Integer {
                return if (build().contains(7)) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("set method on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled set-call method binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_remove_on_call_results() {
    let temp_root = make_temp_project_root("set-remove-call-method-runtime");
    let source_path = temp_root.join("set_remove_call_method_runtime.arden");
    let output_path = temp_root.join("set_remove_call_method_runtime");
    let source = r#"
            function build(): Set<Integer> {
                s: Set<Integer> = Set<Integer>();
                s.add(7);
                return s;
            }

            function main(): Integer {
                return if (build().remove(7)) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("set remove on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled set-remove call binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_contains_on_option_values() {
    let temp_root = make_temp_project_root("set-option-contains-runtime");
    let source_path = temp_root.join("set_option_contains_runtime.arden");
    let output_path = temp_root.join("set_option_contains_runtime");
    let source = r#"
            function main(): Integer {
                s: Set<Option<Integer>> = Set<Option<Integer>>();
                s.add(Option.some(7));
                return if (s.contains(Option.some(7))) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("set option contains should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled set-option contains binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_set_contains_on_result_values() {
    let temp_root = make_temp_project_root("set-result-contains-runtime");
    let source_path = temp_root.join("set_result_contains_runtime.arden");
    let output_path = temp_root.join("set_result_contains_runtime");
    let source = r#"
            function main(): Integer {
                s: Set<Result<Integer, Integer>> = Set<Result<Integer, Integer>>();
                s.add(Result.ok(7));
                return if (s.contains(Result.ok(7))) { 1; } else { 2; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("set result contains should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled set-result contains binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_methods_on_call_results() {
    let temp_root = make_temp_project_root("map-call-method-runtime");
    let source_path = temp_root.join("map_call_method_runtime.arden");
    let output_path = temp_root.join("map_call_method_runtime");
    let source = r#"
            function build(): Map<Integer, Integer> {
                m: Map<Integer, Integer> = Map<Integer, Integer>();
                m.set(1, 7);
                return m;
            }

            function main(): Integer {
                return if (build().contains(1)) { build().length(); } else { 9; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map method on call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map-call method binary");
    assert_eq!(status.code(), Some(1));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_growth_past_initial_capacity() {
    let temp_root = make_temp_project_root("map-growth-runtime");
    let source_path = temp_root.join("map_growth_runtime.arden");
    let output_path = temp_root.join("map_growth_runtime");
    let source = r#"
            function build(): Map<Integer, Integer> {
                m: Map<Integer, Integer> = Map<Integer, Integer>();
                m.set(0, 10);
                m.set(1, 11);
                m.set(2, 12);
                m.set(3, 13);
                m.set(4, 14);
                m.set(5, 15);
                m.set(6, 16);
                m.set(7, 17);
                m.set(8, 18);
                return m;
            }

            function main(): Integer {
                m: Map<Integer, Integer> = build();
                return if (m.contains(8)) { m.get(8); } else { 99; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map growth should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map-growth binary");
    assert_eq!(status.code(), Some(18));

    let _ = fs::remove_dir_all(temp_root);
}
