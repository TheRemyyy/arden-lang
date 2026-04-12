use super::*;
use crate::typeck::TypeChecker;

#[test]
fn type_checker_accepts_exhaustive_result_match_expression_with_error_pattern() {
    let source = r#"
function classify(result: Result<Integer, String>): Integer {
    return match (result) {
        Ok(value) => value,
        Error(err) => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    type_checker
        .check(&program)
        .must("Ok/Error Result match should be exhaustive");
}

#[test]
fn type_checker_rejects_err_pattern_for_result_match_expression() {
    let source = r#"
function classify(result: Result<Integer, String>): Integer {
    return match (result) {
        Ok(value) => value,
        Err(err) => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("Err pattern should be rejected");
    let joined = errors
        .into_iter()
        .map(|e| e.message)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Invalid Result pattern: Err")
            || joined.contains("Non-exhaustive match expression for type Result<Integer, String>"),
        "unexpected error: {joined}"
    );
}

#[test]
fn type_checker_accepts_exhaustive_result_match_expression_with_imported_variant_aliases() {
    let source = r#"
import app.Result.Ok as Success;
import app.Result.Error as Failure;

function classify(result: Result<Integer, String>): Integer {
    return match (result) {
        Success(value) => value,
        Failure(err) => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    type_checker
        .check(&program)
        .must("alias-based Ok/Error Result match should be exhaustive");
}

#[test]
fn type_checker_accepts_exhaustive_option_match_expression_with_imported_variant_aliases() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => inner,
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    type_checker
        .check(&program)
        .must("alias-based Some/None Option match should be exhaustive");
}

#[test]
fn ultra_edge_nested_generics_alias_match_builtins() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

function collapse_opt(value: Option<List<Map<String, Integer>>>): Integer {
    return match (value) {
        Present(items) => 1,
        Empty => 0,
    };
}

function collapse_result(value: Result<Option<List<Map<String, Integer>>>, String>): Integer {
    return match (value) {
        Success(v) => collapse_opt(v),
        Failure(err) => 0,
    };
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn alias_builtins_nested_match_if_expression_branch_typing() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

function unwrap_opt(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => inner,
        Empty => 0,
    };
}

function pick(flag: Boolean, value: Result<Option<Integer>, String>): Integer {
    return if (flag) {
        match (value) {
            Success(inner) => unwrap_opt(inner),
            Failure(err) => 0,
        }
    } else {
        0
    };
}
"#;

    assert_frontend_pipeline_ok(source);
}
