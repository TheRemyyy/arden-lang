use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_negative_list_index_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-index-negative-constant");
    let source_path = temp_root.join("no_check_invalid_list_index_negative_constant.arden");
    let output_path = temp_root.join("no_check_invalid_list_index_negative_constant");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs.push(20);
                return xs[-1];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("xs[-1] should fail in codegen");
    assert!(err.contains("List index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_string_index_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-string-index-negative-constant");
    let source_path = temp_root.join("no_check_invalid_string_index_negative_constant.arden");
    let output_path = temp_root.join("no_check_invalid_string_index_negative_constant");
    let source = r#"
            function main(): Char {
                s: String = "abc";
                return s[-1];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("s[-1] should fail in codegen");
    assert!(err.contains("String index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_list_get_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-get-negative-constant");
    let source_path = temp_root.join("no_check_invalid_list_get_negative_constant.arden");
    let output_path = temp_root.join("no_check_invalid_list_get_negative_constant");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs.push(20);
                return xs.get(-1);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("List.get(-1) should fail in codegen");
    assert!(err.contains("List.get() index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_list_set_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-set-negative-constant");
    let source_path = temp_root.join("no_check_invalid_list_set_negative_constant.arden");
    let output_path = temp_root.join("no_check_invalid_list_set_negative_constant");
    let source = r#"
            function main(): None {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs.push(20);
                xs.set(-1, 99);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("List.set(-1, 99) should fail in codegen");
    assert!(err.contains("List.set() index cannot be negative"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_negative_list_constructor_capacity_constant_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-constructor-negative-capacity");
    let source_path = temp_root.join("no_check_invalid_list_constructor_negative_capacity.arden");
    let output_path = temp_root.join("no_check_invalid_list_constructor_negative_capacity");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>(-1);
                return xs.length();
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("List<Integer>(-1) should fail in codegen");
    assert!(
        err.contains("List constructor capacity cannot be negative"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_constant_ascii_string_index_out_of_bounds_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-ascii-string-index-oob");
    let source_path = temp_root.join("no_check_invalid_ascii_string_index_oob.arden");
    let output_path = temp_root.join("no_check_invalid_ascii_string_index_oob");
    let source = r#"
            function main(): Char {
                return "abc"[5];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("constant ASCII string index OOB should fail in codegen");
    assert!(err.contains("String index out of bounds"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_constant_unicode_string_index_out_of_bounds_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-unicode-string-index-oob");
    let source_path = temp_root.join("no_check_invalid_unicode_string_index_oob.arden");
    let output_path = temp_root.join("no_check_invalid_unicode_string_index_oob");
    let source = r#"
            function main(): Char {
                return "🚀"[1];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("constant Unicode string index OOB should fail in codegen");
    assert!(err.contains("String index out of bounds"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
