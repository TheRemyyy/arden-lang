use super::*;
use crate::typeck::TypeChecker;

#[test]
fn flat_alias_builtins_helper_result_value_flow() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

function make(flag: Boolean): Result<Option<Integer>, String> {
    if (flag) {
        return Result<Option<Integer>, String>();
    }
    return Result<Option<Integer>, String>();
}

function unwrap_opt(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => inner,
        Empty => 0,
    };
}

function run(flag: Boolean): Integer {
    result: Result<Option<Integer>, String> = make(flag);
    value: Option<Integer> = match (result) {
        Success(inner) => inner,
        Failure(err) => Option<Integer>(),
    };
    return unwrap_opt(value);
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn parser_typeck_accepts_exact_import_keyword_alias_generic_chain() {
    let source = r#"
import app.Option.None as Empty;

class Box<T> {
    value: T;
    function get(): T { return this.value; }
}

function run(value: Option<Integer>): Integer {
    b: Box<Integer> = Box<Integer>(1);
    return match (value) {
        Empty => b.get(),
        _ => 0,
    };
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn alias_builtins_match_expression_branch_join_stays_typed() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

function classify(value: Option<Integer>): Integer {
    result: Integer = match (value) {
        Present(inner) => inner,
        Empty => 0,
    };
    return result;
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn alias_builtins_generic_helper_local_type_mismatch_reports_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

function make<T>(flag: Boolean, value: T): Result<Option<T>, String> {
    if (flag) {
        return Result<Option<T>, String>();
    }
    return Result<Option<T>, String>();
}

function classify(flag: Boolean): Integer {
    result: Result<Option<Integer>, String> = make<Integer>(flag, 1);
    wrong: String = match (result) {
        Success(inner) => match (inner) {
            Present(value) => value,
            Empty => 0,
        },
        Failure(err) => 0,
    };
    return 0;
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("local type mismatch should fail");
    let joined = errors
        .into_iter()
        .map(|e| e.message)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Type mismatch") || joined.contains("expected String"),
        "{joined}"
    );
}

#[test]
fn alias_builtins_generic_call_arity_mismatch_reports_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

class Box<T> {
    value: T;
    function map<U>(f: (T) -> U): Box<U> { return Box<U>(); }
}

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => Box<Integer>().map<Integer, String>(inner).value,
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("generic arity mismatch should fail");
    let joined = errors
        .into_iter()
        .map(|e| e.message)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("expects 1 type arguments, got 2")
            || joined.contains("Explicit type argument arity mismatch")
            || joined.contains("arity mismatch"),
        "{joined}"
    );
}

#[test]
fn alias_builtins_constructor_like_generic_misuse_reports_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => List<Integer, String>(),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("constructor-like generic misuse should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("Unknown type: List<Integer, String>"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_valid_type_generic_misuse_reports_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

class Box<T> {
    value: T;
    function map<U>(f: (T) -> U): Box<U> { return Box<U>(); }
}

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => Box<Integer>().map<Integer, String>(inner).value,
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("valid-type generic misuse should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("expects 1 type arguments, got 2"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_free_function_generic_misuse_reports_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

function id<T>(value: T): T { return value; }

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => id<Integer, String>(inner),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("free-function generic misuse should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("expects 1 type arguments, got 2"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_chained_generic_misuse_stays_primary() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

class Box<T> {
    value: T;
    function map<U>(f: (T) -> U): Box<U> { return Box<U>(); }
}

function id<T>(value: T): T { return value; }

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => id<Integer, String>(Box<Integer>().map<Integer, String>(inner).value),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("chained generic misuse should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("expects 1 type arguments, got 2"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_module_free_generic_misuse_stays_primary() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

module Math {
    function id<T>(value: T): T { return value; }
}

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => Math.id<Integer, String>(inner),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("module generic misuse should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("expects 1 type arguments, got 2"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_non_generic_method_type_args_report_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

class Box<T> {
    value: T;
    function get(): T { return this.value; }
}

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => Box<Integer>(inner).get<String>(),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("non-generic method type args should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("is not generic")
            || joined.contains("does not accept explicit type arguments"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_function_field_type_args_report_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

class Holder {
    func: (Integer) -> Integer;
}

function classify(value: Option<Integer>, holder: Holder): Integer {
    return match (value) {
        Present(inner) => holder.func<String>(inner),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("function-valued field type args should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("does not accept explicit type arguments"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_interface_method_type_args_report_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

interface Counter {
    function next(): Integer;
}

function classify(value: Option<Integer>, counter: Counter): Integer {
    return match (value) {
        Present(inner) => counter.next<String>(),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("interface method type args should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("is not generic")
            || joined.contains("does not accept explicit type arguments"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_interface_field_nested_type_args_report_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

interface Holder {
    function get(): (Integer) -> Integer;
}

function classify(value: Option<Integer>, holder: Holder): Integer {
    return match (value) {
        Present(inner) => holder.get()<String>(inner),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("interface field nested type args should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("Explicit type arguments are only supported on named function calls")
            || joined.contains("does not accept explicit type arguments")
            || joined.contains("not generic"),
        "{joined}"
    );
    assert!(!messages.is_empty(), "{joined}");
}

#[test]
fn alias_builtins_nested_module_free_generic_misuse_stays_primary() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

module Math {
    function id<T>(value: T): T { return value; }
}

module Util {
    function call(value: Integer): Integer {
        return Math.id<Integer, String>(value);
    }
}

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => Util.call(inner),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("nested module generic misuse should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("expects 1 type arguments, got 2"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_nested_module_method_generic_misuse_stays_primary() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

class Box<T> {
    value: T;
    function map<U>(f: (T) -> U): Box<U> { return Box<U>(); }
}

module Util {
    function build(value: Integer): Box<Integer> { return Box<Integer>(value); }
}

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => Util.build(inner).map<Integer, String>(inner).value,
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("nested module method generic misuse should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("expects 1 type arguments, got 2"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_static_path_type_args_report_cleanly() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

enum E {
    A(Integer)
}

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => E.A<String>(inner),
        Empty => 0,
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("static path type args should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    let joined = messages.join("\n");
    assert!(
        joined.contains("does not accept type arguments")
            || joined.contains("expects 1 argument")
            || joined.contains("Enum variant"),
        "{joined}"
    );
    assert_eq!(messages.len(), 1, "{joined}");
}

#[test]
fn alias_builtins_nested_match_local_type_mismatch_stays_single_error() {
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

function classify(result: Result<Option<Integer>, String>): None {
    wrong: String = match (result) {
        Success(inner) => unwrap_opt(inner),
        Failure(err) => 0,
    };
    return None;
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("local type mismatch should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("Type mismatch") || message.contains("expected String")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn alias_nested_if_match_expression_reports_single_branch_type_mismatch() {
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

function classify(result: Result<Option<Integer>, String>, cond: Boolean): Integer {
    return if (cond) {
        match (result) {
            Success(inner) => unwrap_opt(inner),
            Failure(err) => 0,
        }
    } else {
        "ok"
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("nested alias if-match branch type mismatch should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages.iter().any(|message| {
            message.contains("Type mismatch")
                || message.contains("Match expression arm type mismatch")
                || message.contains("If expression branch type mismatch")
                || message.contains("expected String")
        }),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn alias_match_expression_reports_single_branch_type_mismatch() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;

function classify(value: Option<Integer>): Integer {
    return match (value) {
        Present(inner) => inner,
        Empty => "oops",
    };
}
"#;

    let program = parse_program(source);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("branch type mismatch should fail");
    let joined = errors
        .into_iter()
        .map(|e| e.message)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("Match expression arm type mismatch")
            || joined.contains("Match expression branch type mismatch"),
        "{joined}"
    );
}
