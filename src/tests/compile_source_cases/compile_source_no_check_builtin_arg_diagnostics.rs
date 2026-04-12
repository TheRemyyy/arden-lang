use super::*;
use std::fs;

#[test]
fn compile_source_no_check_rejects_non_integer_string_index_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-string-index-type");
    let source_path = temp_root.join("no_check_invalid_string_index_type.arden");
    let output_path = temp_root.join("no_check_invalid_string_index_type");
    let source = r#"
            function main(): Integer {
                ch: Char = "hi"[true];
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("string[Boolean] should fail in codegen");
    assert!(
        err.contains("Index must be Integer, found Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_list_index_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-index-type");
    let source_path = temp_root.join("no_check_invalid_list_index_type.arden");
    let output_path = temp_root.join("no_check_invalid_list_index_type");
    let source = r#"
            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs.push(20);
                return xs[true];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("list[Boolean] should fail in codegen");
    assert!(
        err.contains("Index must be Integer, found Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_integer_list_index_with_user_facing_type_name()
{
    let temp_root = make_temp_project_root("no-check-invalid-list-index-module-local-type");
    let source_path = temp_root.join("no_check_invalid_list_index_module_local_type.arden");
    let output_path = temp_root.join("no_check_invalid_list_index_module_local_type");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                xs: List<Integer> = List<Integer>();
                xs.push(10);
                return xs[M.Box(7)];
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("list index with module-local Box should fail in codegen");
    assert!(err.contains("Index must be Integer, found M.Box"), "{err}");
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_list_index_assignment_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-list-index-assignment-type");
    let source_path = temp_root.join("no_check_invalid_list_index_assignment_type.arden");
    let output_path = temp_root.join("no_check_invalid_list_index_assignment_type");
    let source = r#"
            function main(): None {
                mut xs: List<Integer> = List<Integer>();
                xs.push(10);
                xs[true] = 20;
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("list[Boolean] assignment should fail in codegen");
    assert!(
        err.contains("Index must be Integer, found Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_for_loop_sugar_iterable_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-for-loop-sugar-iterable");
    let source_path = temp_root.join("no_check_invalid_for_loop_sugar_iterable.arden");
    let output_path = temp_root.join("no_check_invalid_for_loop_sugar_iterable");
    let source = r#"
            function main(): Integer {
                mut total: Integer = 0;
                for (i in true) {
                    total = total + i;
                }
                return total;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("for-loop sugar over Boolean should fail in codegen");
    assert!(err.contains("Cannot iterate over Boolean"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_integer_for_loop_sugar_iterable_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-for-loop-sugar-module-local-iterable");
    let source_path = temp_root.join("no_check_invalid_for_loop_sugar_module_local_iterable.arden");
    let output_path = temp_root.join("no_check_invalid_for_loop_sugar_module_local_iterable");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                for (i in M.Box(7)) {
                }
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("for-loop sugar over module-local Box should fail in codegen");
    assert!(err.contains("Cannot iterate over M.Box"), "{err}");
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_invalid_range_argument_types_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-range-argument-types");
    let source_path = temp_root.join("no_check_invalid_range_argument_types.arden");
    let output_path = temp_root.join("no_check_invalid_range_argument_types");
    let source = r#"
            function main(): Integer {
                r: Range<Integer> = range(true, 3);
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("range(Boolean, Integer) should fail in codegen");
    assert!(
        err.contains("range() arguments must be all Integer or all Float"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_exit_code_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-exit-code-type");
    let source_path = temp_root.join("no_check_invalid_exit_code_type.arden");
    let output_path = temp_root.join("no_check_invalid_exit_code_type");
    let source = r#"
            function main(): None {
                exit(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("exit(Boolean) should fail in codegen");
    assert!(err.contains("exit() requires Integer code"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_time_sleep_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-time-sleep-type");
    let source_path = temp_root.join("no_check_invalid_time_sleep_type.arden");
    let output_path = temp_root.join("no_check_invalid_time_sleep_type");
    let source = r#"
            import std.time.*;

            function main(): None {
                Time.sleep(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Time.sleep(Boolean) should fail in codegen");
    assert!(
        err.contains("Time.sleep(ms) requires Integer milliseconds"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_integer_args_get_index_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-args-get-index-type");
    let source_path = temp_root.join("no_check_invalid_args_get_index_type.arden");
    let output_path = temp_root.join("no_check_invalid_args_get_index_type");
    let source = r#"
            import std.args.*;

            function main(): None {
                value: String = Args.get(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Args.get(Boolean) should fail in codegen");
    assert!(err.contains("Args.get() requires Integer index"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_system_shell_command_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-system-shell-command-type");
    let source_path = temp_root.join("no_check_invalid_system_shell_command_type.arden");
    let output_path = temp_root.join("no_check_invalid_system_shell_command_type");
    let source = r#"
            import std.system.*;

            function main(): Integer {
                return System.shell(true);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("System.shell(Boolean) should fail in codegen");
    assert!(
        err.contains("System.shell() requires String command"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_exists_path_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-exists-path-type");
    let source_path = temp_root.join("no_check_invalid_file_exists_path_type.arden");
    let output_path = temp_root.join("no_check_invalid_file_exists_path_type");
    let source = r#"
            import std.file.*;

            function main(): Integer {
                return if (File.exists(true)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("File.exists(Boolean) should fail in codegen");
    assert!(
        err.contains("File.exists() requires String path, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_file_exists_path_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-file-exists-module-local-path");
    let source_path = temp_root.join("no_check_invalid_file_exists_module_local_path.arden");
    let output_path = temp_root.join("no_check_invalid_file_exists_module_local_path");
    let source = r#"
            import std.file.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): Integer {
                return if (File.exists(M.Box(7))) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("File.exists(module-local Box) should fail in codegen");
    assert!(
        err.contains("File.exists() requires String path, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_read_path_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-read-path-type");
    let source_path = temp_root.join("no_check_invalid_file_read_path_type.arden");
    let output_path = temp_root.join("no_check_invalid_file_read_path_type");
    let source = r#"
            import std.file.*;

            function main(): None {
                value: String = File.read(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("File.read(Boolean) should fail in codegen");
    assert!(
        err.contains("File.read() requires String path, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_file_read_path_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-file-read-module-local-path");
    let source_path = temp_root.join("no_check_invalid_file_read_module_local_path.arden");
    let output_path = temp_root.join("no_check_invalid_file_read_module_local_path");
    let source = r#"
            import std.file.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function render(): String {
                return File.read(M.Box(7));
            }

            function main(): None {
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("File.read(module-local Box) should fail in codegen");
    assert!(
        err.contains("File.read() requires String path, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_delete_path_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-delete-path-type");
    let source_path = temp_root.join("no_check_invalid_file_delete_path_type.arden");
    let output_path = temp_root.join("no_check_invalid_file_delete_path_type");
    let source = r#"
            import std.file.*;

            function main(): None {
                File.delete(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("File.delete(Boolean) should fail in codegen");
    assert!(
        err.contains("File.delete() requires String path, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_file_delete_path_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-file-delete-module-local-path");
    let source_path = temp_root.join("no_check_invalid_file_delete_module_local_path.arden");
    let output_path = temp_root.join("no_check_invalid_file_delete_module_local_path");
    let source = r#"
            import std.file.*;

            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                File.delete(M.Box(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("File.delete(module-local Box) should fail in codegen");
    assert!(
        err.contains("File.delete() requires String path, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_write_path_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-write-path-type");
    let source_path = temp_root.join("no_check_invalid_file_write_path_type.arden");
    let output_path = temp_root.join("no_check_invalid_file_write_path_type");
    let source = r#"
            import std.file.*;

            function main(): None {
                File.write(true, "ok");
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("File.write(Boolean, String) should fail in codegen");
    assert!(err.contains("File.write() path must be String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_file_write_content_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-file-write-content-type");
    let source_path = temp_root.join("no_check_invalid_file_write_content_type.arden");
    let output_path = temp_root.join("no_check_invalid_file_write_content_type");
    let source = r#"
            import std.file.*;

            function main(): None {
                File.write("ok.txt", true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("File.write(String, Boolean) should fail in codegen");
    assert!(err.contains("File.write() content must be String"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_time_now_format_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-time-now-format-type");
    let source_path = temp_root.join("no_check_invalid_time_now_format_type.arden");
    let output_path = temp_root.join("no_check_invalid_time_now_format_type");
    let source = r#"
            import std.time.*;

            function main(): None {
                value: String = Time.now(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("Time.now(Boolean) should fail in codegen");
    assert!(err.contains("Time.now() requires String format"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_system_getenv_name_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-system-getenv-name-type");
    let source_path = temp_root.join("no_check_invalid_system_getenv_name_type.arden");
    let output_path = temp_root.join("no_check_invalid_system_getenv_name_type");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: String = System.getenv(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("System.getenv(Boolean) should fail in codegen");
    assert!(
        err.contains("System.getenv() requires String name"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_system_exec_command_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-system-exec-command-type");
    let source_path = temp_root.join("no_check_invalid_system_exec_command_type.arden");
    let output_path = temp_root.join("no_check_invalid_system_exec_command_type");
    let source = r#"
            import std.system.*;

            function main(): None {
                value: String = System.exec(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("System.exec(Boolean) should fail in codegen");
    assert!(
        err.contains("System.exec() requires String command"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_fail_message_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-fail-message-type");
    let source_path = temp_root.join("no_check_invalid_fail_message_type.arden");
    let output_path = temp_root.join("no_check_invalid_fail_message_type");
    let source = r#"
            function main(): None {
                fail(true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("fail(Boolean) should fail in codegen");
    assert!(err.contains("fail() requires String message"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_non_string_require_message_in_codegen() {
    let temp_root = make_temp_project_root("no-check-invalid-require-message-type");
    let source_path = temp_root.join("no_check_invalid_require_message_type.arden");
    let output_path = temp_root.join("no_check_invalid_require_message_type");
    let source = r#"
            function main(): None {
                require(false, true);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("require(Boolean, Boolean) should fail in codegen");
    assert!(
        err.contains("require() message must be String, got Boolean"),
        "{err}"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_no_check_rejects_module_local_non_string_require_message_with_user_facing_type_name(
) {
    let temp_root = make_temp_project_root("no-check-invalid-require-module-local-message");
    let source_path = temp_root.join("no_check_invalid_require_module_local_message.arden");
    let output_path = temp_root.join("no_check_invalid_require_module_local_message");
    let source = r#"
            module M {
                class Box {
                    value: Integer;
                    constructor(value: Integer) { this.value = value; }
                }
            }

            function main(): None {
                require(true, M.Box(7));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, false, None, None)
        .must_err("require(Boolean, module-local Box) should fail in codegen");
    assert!(
        err.contains("require() message must be String, got M.Box"),
        "{err}"
    );
    assert!(!err.contains("M__Box"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
