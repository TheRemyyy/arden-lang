use super::*;
use std::fs;

#[test]
fn compile_source_runs_borrowed_float_list_index_arithmetic_runtime() {
    let temp_root = make_temp_project_root("borrowed-float-list-index-arithmetic-runtime");
    let source_path = temp_root.join("borrowed_float_list_index_arithmetic_runtime.arden");
    let output_path = temp_root.join("borrowed_float_list_index_arithmetic_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Float> = List<Float>();
                xs.push(1.5);
                rxs: &List<Float> = &xs;
                sum: Float = rxs[0] + 1.25;
                return if (sum == 2.75) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed float list index arithmetic should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed float list arithmetic binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_borrowed_float_list_index_interpolation_runtime() {
    let temp_root = make_temp_project_root("borrowed-float-list-index-interp-runtime");
    let source_path = temp_root.join("borrowed_float_list_index_interp_runtime.arden");
    let output_path = temp_root.join("borrowed_float_list_index_interp_runtime");
    let source = r#"
            function main(): Integer {
                xs: List<Float> = List<Float>();
                xs.push(1.5);
                rxs: &List<Float> = &xs;
                text: String = "{rxs[0]}";
                return if (text == "1.500000") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("borrowed float list index interpolation should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled borrowed float list interpolation binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_list_index_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-list-index-assignment-runtime");
    let source_path = temp_root.join("mutable_borrowed_list_index_assignment_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_list_index_assignment_runtime");
    let source = r#"
            function main(): Integer {
                mut xs: List<Integer> = List<Integer>();
                xs.push(1);
                rxs: &mut List<Integer> = &mut xs;
                rxs[0] = 2;
                return if (rxs[0] == 2 && xs[0] == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed list index assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed list index assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_map_index_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-map-index-assignment-runtime");
    let source_path = temp_root.join("mutable_borrowed_map_index_assignment_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_map_index_assignment_runtime");
    let source = r#"
            function main(): Integer {
                mut m: Map<String, Integer> = Map<String, Integer>();
                rm: &mut Map<String, Integer> = &mut m;
                rm["k"] = 7;
                return if (rm["k"] == 7 && m["k"] == 7) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed map index assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed map index assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_mutable_borrowed_nested_index_assignment_runtime() {
    let temp_root = make_temp_project_root("mutable-borrowed-nested-index-assignment-runtime");
    let source_path = temp_root.join("mutable_borrowed_nested_index_assignment_runtime.arden");
    let output_path = temp_root.join("mutable_borrowed_nested_index_assignment_runtime");
    let source = r#"
            class Bag {
                mut xs: List<Integer>;
                mut m: Map<String, Integer>;

                constructor() {
                    this.xs = List<Integer>();
                    this.xs.push(1);
                    this.m = Map<String, Integer>();
                }
            }

            function main(): Integer {
                mut bag: Bag = Bag();
                rb: &mut Bag = &mut bag;
                rb.xs[0] = 3;
                rb.m["k"] = 4;
                if (rb.xs[0] != 3) { return 1; }
                if (rb.m["k"] != 4) { return 2; }
                if (bag.xs[0] != 3) { return 3; }
                if (bag.m["k"] != 4) { return 4; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mutable borrowed nested index assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutable borrowed nested index assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_method_with_mutating_builtin_field_runtime() {
    let temp_root = make_temp_project_root("method-with-mutating-builtin-field-runtime");
    let source_path = temp_root.join("method_with_mutating_builtin_field_runtime.arden");
    let output_path = temp_root.join("method_with_mutating_builtin_field_runtime");
    let source = r#"
            class Bag {
                mut xs: List<Integer>;
                constructor() { this.xs = List<Integer>(); }
                function add_one(): None {
                    this.xs.push(1);
                    return None;
                }
            }

            function main(): Integer {
                mut bag: Bag = Bag();
                bag.add_one();
                return if (bag.xs[0] == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("method with mutating builtin field should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled mutating builtin field method binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_local_deref_assignment_runtime() {
    let temp_root = make_temp_project_root("local-deref-assignment-runtime");
    let source_path = temp_root.join("local_deref_assignment_runtime.arden");
    let output_path = temp_root.join("local_deref_assignment_runtime");
    let source = r#"
            function main(): Integer {
                mut x: Integer = 5;
                rx: &mut Integer = &mut x;
                *rx = 19;
                return if (*rx == 19) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("local deref assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled local deref assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_direct_mutable_reference_assignment_runtime() {
    let temp_root = make_temp_project_root("direct-mutable-reference-assignment-runtime");
    let source_path = temp_root.join("direct_mutable_reference_assignment_runtime.arden");
    let output_path = temp_root.join("direct_mutable_reference_assignment_runtime");
    let source = r#"
            function write_ref(r: &mut Integer): None {
                *r = 13;
                return None;
            }

            function main(): Integer {
                mut x: Integer = 5;
                rx: &mut Integer = &mut x;
                write_ref(rx);
                return if (*rx == 13) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("direct mutable reference assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled direct mutable reference assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_inline_mutable_reference_assignment_runtime() {
    let temp_root = make_temp_project_root("inline-mutable-reference-assignment-runtime");
    let source_path = temp_root.join("inline_mutable_reference_assignment_runtime.arden");
    let output_path = temp_root.join("inline_mutable_reference_assignment_runtime");
    let source = r#"
            function write_ref(r: &mut Integer): None {
                *r = 17;
                return None;
            }

            function main(): Integer {
                mut x: Integer = 5;
                write_ref(&mut x);
                rx: &mut Integer = &mut x;
                return if (*rx == 17) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("inline mutable reference assignment should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled inline mutable reference assignment binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}
