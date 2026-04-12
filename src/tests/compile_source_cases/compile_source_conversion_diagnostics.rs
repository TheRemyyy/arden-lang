use super::*;
use std::fs;

#[test]
fn compile_source_rejects_integer_payloads_for_float_option_and_result() {
    let temp_root = make_temp_project_root("reject-int-to-float-option-result");
    let source_path = temp_root.join("reject_int_to_float_option_result.arden");
    let output_path = temp_root.join("reject_int_to_float_option_result");
    let source = r#"
            function main(): Integer {
                maybe: Option<Float> = Option.some(1);
                okv: Result<Float, String> = Result.ok(2);
                errv: Result<String, Float> = Result.error(3);
                errValue: Float = match (errv) {
                    Result.Error(v) => v,
                    _ => 0.0,
                };

                if (!maybe.is_some() || maybe.unwrap() != 1.0) { return 1; }
                if (!okv.is_ok() || okv.unwrap() != 2.0) { return 2; }
                if (!errv.is_error() || errValue != 3.0) { return 3; }
                return 0;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("Option/Result payloads should stay invariant across Integer/Float");
    assert!(err.contains("Type mismatch"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn compile_source_rejects_invalid_to_int_and_to_float_argument_types() {
    let temp_root = make_temp_project_root("invalid-to-int-to-float-types");
    let source_path = temp_root.join("invalid_to_int_to_float_types.arden");
    let output_path = temp_root.join("invalid_to_int_to_float_types");
    let source = r#"
            function main(): Integer {
                a: Integer = to_int(true);
                b: Float = to_float("8");
                return a + to_int(b);
            }
        "#;

    fs::write(&source_path, source).must("write source");
    let err = compile_source(source, &source_path, &output_path, false, true, None, None)
        .must_err("invalid to_int/to_float argument types should fail");
    assert!(err.contains("to_int") || err.contains("to_float"), "{err}");

    let _ = fs::remove_dir_all(temp_root);
}
