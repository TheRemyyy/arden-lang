use super::*;
use std::fs;

#[test]
fn compile_source_runs_print_on_option_runtime() {
    let temp_root = make_temp_project_root("print-option-runtime");
    let source_path = temp_root.join("print_option_runtime.arden");
    let output_path = temp_root.join("print_option_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(Option.some(1));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("print on Option should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled print Option binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "Some(1)");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_print_on_direct_option_none_runtime() {
    let temp_root = make_temp_project_root("print-direct-option-none-runtime");
    let source_path = temp_root.join("print_direct_option_none_runtime.arden");
    let output_path = temp_root.join("print_direct_option_none_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(Option.none());
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("print on direct Option.none should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled print direct Option.none binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "None");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_print_on_direct_result_error_with_option_none_runtime() {
    let temp_root = make_temp_project_root("print-direct-result-error-option-none-runtime");
    let source_path = temp_root.join("print_direct_result_error_option_none_runtime.arden");
    let output_path = temp_root.join("print_direct_result_error_option_none_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(Result.error(Option.none()));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("print on direct Result.error(Option.none()) should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled print direct Result.error(Option.none()) binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "Error(None)");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_print_on_result_runtime() {
    let temp_root = make_temp_project_root("print-result-runtime");
    let source_path = temp_root.join("print_result_runtime.arden");
    let output_path = temp_root.join("print_result_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                result: Result<Integer, String> = Result.error("boom");
                print(result);
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("print on Result should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled print Result binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "Error(boom)");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_runs_print_on_direct_result_error_runtime() {
    let temp_root = make_temp_project_root("print-direct-result-error-runtime");
    let source_path = temp_root.join("print_direct_result_error_runtime.arden");
    let output_path = temp_root.join("print_direct_result_error_runtime");
    let source = r#"
            import std.io.*;

            function main(): None {
                print(Result.error("boom"));
                return None;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("print on direct Result.error should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled print direct Result.error binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "Error(boom)");

    let _ = fs::remove_dir_all(temp_root);
}
