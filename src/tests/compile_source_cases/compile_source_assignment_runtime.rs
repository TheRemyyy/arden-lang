use super::*;
use std::fs;

#[test]
fn compile_source_runs_map_index_assignment_with_string_keys() {
    let temp_root = make_temp_project_root("map-index-assign-runtime");
    let source_path = temp_root.join("map_index_assign_runtime.arden");
    let output_path = temp_root.join("map_index_assign_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["x"] = 21;
                return m["x"];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map index assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map index assignment binary");
    assert_eq!(status.code(), Some(21));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_index_assignment_on_function_returned_list() {
    let temp_root = make_temp_project_root("list-index-assign-call-runtime");
    let source_path = temp_root.join("list_index_assign_call_runtime.arden");
    let output_path = temp_root.join("list_index_assign_call_runtime");
    let source = r#"
            function make(): List<Integer> {
                xs: List<Integer> = List<Integer>();
                xs.push(1);
                return xs;
            }

            function main(): Integer {
                make()[0] = 7;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("list index assignment on function-returned list should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled list assignment call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_list_index_compound_assignment_without_double_evaluation() {
    let temp_root = make_temp_project_root("list-index-compound-assign-call-runtime");
    let source_path = temp_root.join("list_index_compound_assign_call_runtime.arden");
    let output_path = temp_root.join("list_index_compound_assign_call_runtime");
    let source = r#"
            class Factory {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function make(): List<Integer> {
                    this.calls += 1;
                    xs: List<Integer> = List<Integer>();
                    xs.push(1);
                    return xs;
                }
            }

            function main(): Integer {
                mut factory: Factory = Factory();
                factory.make()[0] += 2;
                return if (factory.calls == 1) { 0 } else { factory.calls };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("list index compound assignment on function-returned list should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled list compound assignment call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_compound_assignment_without_double_evaluation() {
    let temp_root = make_temp_project_root("field-compound-assign-call-runtime");
    let source_path = temp_root.join("field_compound_assign_call_runtime.arden");
    let output_path = temp_root.join("field_compound_assign_call_runtime");
    let source = r#"
            class Boxed {
                mut value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            class Factory {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function make_box(): Boxed {
                    this.calls += 1;
                    return Boxed(1);
                }
            }

            function main(): Integer {
                mut factory: Factory = Factory();
                factory.make_box().value += 2;
                return if (factory.calls == 1) { 0 } else { factory.calls };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("field compound assignment on function-returned object should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled field compound assignment call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_index_compound_assignment_without_double_evaluation() {
    let temp_root = make_temp_project_root("map-index-compound-assign-call-runtime");
    let source_path = temp_root.join("map_index_compound_assign_call_runtime.arden");
    let output_path = temp_root.join("map_index_compound_assign_call_runtime");
    let source = r#"
            class Factory {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function make_map(): Map<String, Integer> {
                    this.calls += 1;
                    mut m: Map<String, Integer> = Map<String, Integer>();
                    m["k"] = 1;
                    return m;
                }
            }

            function main(): Integer {
                mut factory: Factory = Factory();
                factory.make_map()["k"] += 2;
                return if (factory.calls == 1) { 0 } else { factory.calls };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map index compound assignment on function-returned map should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map compound assignment call binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_map_index_assignment_on_function_value_call_result() {
    let temp_root = make_temp_project_root("field-map-index-assign-function-value-runtime");
    let source_path = temp_root.join("field_map_index_assign_function_value_runtime.arden");
    let output_path = temp_root.join("field_map_index_assign_function_value_runtime");
    let source = r#"
            class Box {
                mut m: Map<String, Integer>;
                constructor() { this.m = Map<String, Integer>(); }
            }

            class Holder {
                make: (Integer) -> Box;
                constructor(make: (Integer) -> Box) { this.make = make; }
            }

            function build(x: Integer): Box { return Box(); }

            function main(): Integer {
                holder: Holder = Holder(build);
                holder.make(1).m["k"] = 7;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map index assignment on function-valued field call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled field map assignment function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_field_map_index_compound_assignment_on_function_value_call_result() {
    let temp_root =
        make_temp_project_root("field-map-index-compound-assign-function-value-runtime");
    let source_path =
        temp_root.join("field_map_index_compound_assign_function_value_runtime.arden");
    let output_path = temp_root.join("field_map_index_compound_assign_function_value_runtime");
    let source = r#"
            class Box {
                mut m: Map<String, Integer>;
                constructor() {
                    this.m = Map<String, Integer>();
                    this.m.set("k", 1);
                }
            }

            class Holder {
                make: () -> Box;
                constructor(make: () -> Box) { this.make = make; }
            }

            function build(): Box { return Box(); }

            function main(): Integer {
                holder: Holder = Holder(build);
                holder.make().m["k"] += 2;
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map index compound assignment on function-valued field call result should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled field map compound assignment function value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_map_index_compound_assignment_without_double_key_evaluation() {
    let temp_root = make_temp_project_root("map-index-compound-assign-key-runtime");
    let source_path = temp_root.join("map_index_compound_assign_key_runtime.arden");
    let output_path = temp_root.join("map_index_compound_assign_key_runtime");
    let source = r#"
            class Counter {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function key(): String {
                    this.calls += 1;
                    return "k";
                }
            }

            function main(): Integer {
                mut counter: Counter = Counter();
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["k"] = 1;
                m[counter.key()] += 2;
                return if (counter.calls == 1) { 0 } else { counter.calls };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("map index compound assignment with key side effects should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled map compound assignment key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_list_index_compound_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-list-index-compound-runtime");
    let source_path = temp_root.join("mutable_borrowed_list_index_compound_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_list_index_compound_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                rxs: &mut List<Integer> = &mut xs;
                rxs[0] += 2;
                return if (rxs[0] == 3 && xs[0] == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed list compound assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed list compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_map_index_compound_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-map-index-compound-runtime");
    let source_path = temp_root.join("mutable_borrowed_map_index_compound_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_map_index_compound_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["k"] = 1;
                rm: &mut Map<String, Integer> = &mut m;
                rm["k"] += 2;
                return if (rm["k"] == 3 && m["k"] == 3) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed map compound assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed map compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mod_compound_assignment_runtime() {
    let temp_root = make_temp_project_root("mod-compound-assign-runtime");
    let source_path = temp_root.join("mod_compound_assign_runtime.arden");
    let output_path = temp_root.join("mod_compound_assign_runtime");
    let source = r#"
            function main(): Integer {
                mut x: Integer = 17;
                x %= 5;
                return if (x == 2) { 0 } else { x };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mod compound assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mod compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_mod_runtime() {
    let temp_root = make_temp_project_root("float-mod-runtime");
    let source_path = temp_root.join("float_mod_runtime.arden");
    let output_path = temp_root.join("float_mod_runtime");
    let source = r#"
            function main(): Integer {
                value: Float = 5.5 % 2.0;
                return if (value == 1.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("float modulo should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled float modulo binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_float_mod_compound_assignment_runtime() {
    let temp_root = make_temp_project_root("float-mod-compound-assign-runtime");
    let source_path = temp_root.join("float_mod_compound_assign_runtime.arden");
    let output_path = temp_root.join("float_mod_compound_assign_runtime");
    let source = r#"
            function main(): Integer {
                mut value: Float = 5.5;
                value %= 2.0;
                return if (value == 1.5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("float modulo compound assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled float modulo compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_integer_rhs_to_float_compound_assignment_runtime() {
    let temp_root = make_temp_project_root("int-to-float-compound-assign-runtime");
    let source_path = temp_root.join("int_to_float_compound_assign_runtime.arden");
    let output_path = temp_root.join("int_to_float_compound_assign_runtime");
    let source = r#"
            function main(): Integer {
                mut value: Float = 1.5;
                value += 2;
                value *= 2;
                return if (value == 7.0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("integer RHS to Float compound assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled int-to-float compound assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mod_compound_assignment_without_double_key_evaluation() {
    let temp_root = make_temp_project_root("mod-compound-assign-key-runtime");
    let source_path = temp_root.join("mod_compound_assign_key_runtime.arden");
    let output_path = temp_root.join("mod_compound_assign_key_runtime");
    let source = r#"
            class Counter {
                mut calls: Integer;
                constructor() { this.calls = 0; }
                function key(): String {
                    this.calls += 1;
                    return "k";
                }
            }

            function main(): Integer {
                mut counter: Counter = Counter();
                mut m: Map<String, Integer> = Map<String, Integer>();
                m["k"] = 9;
                m[counter.key()] %= 4;
                return if (counter.calls == 1 && m["k"] == 1) { 0 } else { counter.calls };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mod compound assignment with key side effects should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mod compound assignment key binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
