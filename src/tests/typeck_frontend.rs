use super::*;
use crate::typeck::TypeChecker;
use std::fs;

#[test]
fn semantic_program_fingerprint_ignores_comments_and_whitespace() {
    let a = r#"
import std.io.*;

function main(): None {
    println("hi");
    return None;
}
"#;
    let b = r#"
// top comment
import std.io.*;

function main(): None {
    // inside body
    println("hi");
    return None;
}
"#;

    assert_eq!(fingerprint_for(a), fingerprint_for(b));
}

#[test]
fn semantic_program_fingerprint_changes_with_code_changes() {
    let a = r#"
function main(): None {
    println("hi");
    return None;
}
"#;
    let b = r#"
function main(): None {
    println("bye");
    return None;
}
"#;

    assert_ne!(fingerprint_for(a), fingerprint_for(b));
}

#[test]
fn api_program_fingerprint_ignores_body_only_changes() {
    let a = r#"
function add(x: Integer): Integer {
    return x + 1;
}
"#;
    let b = r#"
function add(x: Integer): Integer {
    return x + 999;
}
"#;

    let pa = parse_program(a);
    let pb = parse_program(b);
    assert_eq!(api_program_fingerprint(&pa), api_program_fingerprint(&pb));
    assert_ne!(
        semantic_program_fingerprint(&pa),
        semantic_program_fingerprint(&pb)
    );
}

#[test]
fn api_program_fingerprint_changes_with_signature_changes() {
    let a = r#"
function add(x: Integer): Integer {
    return x + 1;
}
"#;
    let b = r#"
function add(x: Float): Float {
    return x + 1.0;
}
"#;

    let pa = parse_program(a);
    let pb = parse_program(b);
    assert_ne!(api_program_fingerprint(&pa), api_program_fingerprint(&pb));
}

#[test]
fn api_program_fingerprint_changes_with_nested_interface_rename() {
    let a = r#"
package app;
module M {
    module Api {
        interface Named {
            function name(): Integer;
        }
    }
}
"#;
    let b = r#"
package app;
module M {
    module Api {
        interface Labelled {
            function name(): Integer;
        }
    }
}
"#;

    let pa = parse_program(a);
    let pb = parse_program(b);
    assert_ne!(api_program_fingerprint(&pa), api_program_fingerprint(&pb));
}

#[test]
fn frontend_pipeline_corpus_survives_parse_check_borrow_and_format() {
    let corpus = [
        r#"
package demo.core;
import std.io.*;
function main(): None {
    println("hello");
    return None;
}
"#,
        r#"
function apply(f: () -> Integer): Integer {
    return f();
}

function one(): Integer {
    return 1;
}
"#,
        r#"
class Counter {
    mut value: Integer;

    constructor(start: Integer) {
        this.value = start;
    }

    function next(): Integer {
        this.value = this.value + 1;
        return this.value;
    }
}

function main(): None {
    mut c: Counter = Counter(1);
    x: Integer = c.next();
    println("count {x}");
    return None;
}
"#,
        r#"
enum MaybeInt {
    Some(value: Integer),
    Empty
}

function unwrap_or_zero(v: MaybeInt): Integer {
    match (v) {
        Some(value) => { return value; },
        _ => { return 0; },
    }
}
"#,
        r#"
module Math {
    function id<T>(value: T): T {
        return value;
    }
}

function main(): None {
    x: Integer = Math.id<Integer>(1);
    y: Integer = if (x == 1) { 10; } else { 20; };
    println("value {y}");
    return None;
}
"#,
    ];

    for source in corpus {
        assert_frontend_pipeline_ok(source);
    }
}

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
            .any(|m| m.contains("Type mismatch") || m.contains("expected String")),
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
        messages.iter().any(|m| m.contains("Type mismatch")
            || m.contains("Match expression arm type mismatch")
            || m.contains("If expression branch type mismatch")
            || m.contains("expected String")),
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

#[test]
fn rest_style_alias_heavy_tagged_pipeline_survives_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Request {
    route: String;
    constructor(route: String) { this.route = route; }
}

class Response {
    code: Integer;
    body: String;
    constructor(code: Integer, body: String) {
        this.code = code;
        this.body = body;
    }
}

function decode(req: Request): Result<Option<Integer>, String> {
    if (req.route == "/users") {
        return Result.ok(Option.some(200));
    }
    return Result.error("missing");
}

function handle(req: Request, verbose: Boolean): Response {
    status: Integer = if (verbose) {
        match (decode(req)) {
            Success(inner) => match (inner) {
                Present(code) => code,
                Empty => 204,
            },
            Failure(err) => 500,
        }
    } else {
        400
    };
    return Response(status, "done");
}

function main(): Integer {
    return handle(Request("/users"), true).code;
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn data_pipeline_tagged_container_growth_chains_survive_codegen() {
    let temp_root = make_temp_project_root("data-pipeline-tagged-container-runtime");
    let source_path = temp_root.join("data_pipeline_tagged_container_runtime.arden");
    let output_path = temp_root.join("data_pipeline_tagged_container_runtime");
    let source = r#"
            class Row {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function bump(): Row { return Row(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, Integer>, Option<Row>> {
                m: Map<Result<Option<Integer>, Integer>, Option<Row>> = Map<Result<Option<Integer>, Integer>, Option<Row>>();
                mut i: Integer = 0;
                while (i < 9) {
                    m.set(Result.ok(Option.some(i)), Option.some(Row(i)));
                    i = i + 1;
                }
                if (flag) {
                    m.set(Result.error(3), Option.some(Row(40)));
                }
                return m;
            }

            function main(): Integer {
                pipeline: Map<Result<Option<Integer>, Integer>, Option<Row>> = build(true);
                hit: Option<Row> = pipeline.get(Result.error(3));
                return hit.unwrap().bump().value;
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("data pipeline tagged container runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled data pipeline tagged container binary");
    assert_eq!(status.code(), Some(41));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn batch_style_tagged_container_mutation_chain_survives_codegen() {
    let temp_root = make_temp_project_root("batch-style-tagged-mutation-runtime");
    let source_path = temp_root.join("batch_style_tagged_mutation_runtime.arden");
    let output_path = temp_root.join("batch_style_tagged_mutation_runtime");
    let source = r#"
            class Row {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function bump(): Row { return Row(this.value + 1); }
            }

            function main(): Integer {
                queue: Map<Result<Option<Integer>, Integer>, Option<Row>> = Map<Result<Option<Integer>, Integer>, Option<Row>>();
                mut i: Integer = 0;
                while (i < 9) {
                    queue.set(Result.ok(Option.some(i)), Option.some(Row(i)));
                    i = i + 1;
                }
                queue.set(Result.error(3), Option.some(Row(10)));
                queue.set(Result.error(3), Option.some(Row(20)));
                had_old: Boolean = queue.contains(Result.ok(Option.some(4)));
                queue.set(Result.ok(Option.some(4)), Option.some(Row(30)));
                has_new: Boolean = queue.contains(Result.ok(Option.some(4)));
                picked: Option<Row> = queue.get(Result.error(3));
                restored: Option<Row> = queue.get(Result.ok(Option.some(4)));
                return if (had_old && has_new && queue.length() == 10) { picked.unwrap().bump().value + restored.unwrap().value } else { 0 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("batch-style tagged mutation runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled batch-style tagged mutation binary");
    assert_eq!(status.code(), Some(51));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn forward_declared_generic_class_enum_payload_survives_codegen() {
    let temp_root = make_temp_project_root("forward-declared-generic-enum-payload-runtime");
    let source_path = temp_root.join("forward_declared_generic_enum_payload_runtime.arden");
    let output_path = temp_root.join("forward_declared_generic_enum_payload_runtime");
    let source = r#"
            enum Choice {
                Boxed(Box<String>),
                Empty
            }

            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
            }

            function main(): Integer {
                current: Choice = Choice.Boxed(Box<String>("hi"));
                picked: Box<String> = match (current) {
                    Boxed(inner) => inner,
                    Empty => Box<String>("no")
                };
                return if (picked.value == "hi") { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("forward-declared generic enum payload runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled forward-declared generic enum payload binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn forward_declared_generic_class_method_chain_survives_codegen() {
    let temp_root = make_temp_project_root("forward-declared-generic-method-chain-runtime");
    let source_path = temp_root.join("forward_declared_generic_method_chain_runtime.arden");
    let output_path = temp_root.join("forward_declared_generic_method_chain_runtime");
    let source = r#"
            enum Choice {
                Boxed(Box<String>),
                Empty
            }

            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                return if ({
                    current: Choice = Choice.Boxed(Box<String>("hi"));
                    match (current) {
                        Boxed(inner) => inner,
                        Empty => Box<String>("no")
                    }
                }.get().length() == 2) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("forward-declared generic method chain runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled forward-declared generic method chain binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn bound_generic_method_value_survives_codegen() {
    let temp_root = make_temp_project_root("bound-generic-method-value-runtime");
    let source_path = temp_root.join("bound_generic_method_value_runtime.arden");
    let output_path = temp_root.join("bound_generic_method_value_runtime");
    let source = r#"
            class Box<T> {
                value: T;
                constructor(value: T) { this.value = value; }
                function get(): T { return this.value; }
            }

            function main(): Integer {
                box: Box<String> = Box<String>("hello");
                getter: () -> String = box.get;
                return if (getter().length() == 5) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("bound generic method value runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled bound generic method value binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn alias_heavy_ultra_edge_tagged_container_method_chain_survives_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
    function inc(): Boxed { return Boxed(this.value + 1); }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.ok(Option.some(1)), Option.some(Boxed(50)));
    store.set(Result.ok(Option.none()), Option.none());
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(60)));
    }
    return store;
}

function main(): Integer {
    value: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
        Present(row) => Option.some(row.inc()),
        Empty => Option.some(Boxed(0)),
    };
    return value.unwrap().value;
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn alias_heavy_tagged_runtime_pipeline_with_updates_and_chains() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
    function inc(): Boxed { return Boxed(this.value + 1); }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    mut i: Integer = 0;
    while (i < 9) {
        store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
        i = i + 1;
    }
    store.set(Result.ok(Option.none()), Option.none());
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(70)));
        store.set(Result.error("missing"), Option.some(Boxed(80)));
    }
    return store;
}

function main(): Integer {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = build(true);
    fallback: Option<Boxed> = store.get(Result.ok(Option.none()));
    value: Option<Boxed> = match (store.get(Result.error("missing"))) {
        Present(row) => Option.some(row.inc()),
        Empty => fallback,
    };
    return value.unwrap().value;
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn unicode_tagged_key_pipeline_survives_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
    function inc(): Boxed { return Boxed(this.value + 1); }
}

function build(): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("σφάλμα🚀"), Option.some(Boxed(90)));
    store.set(Result.ok(Option.some(1)), Option.some(Boxed(5)));
    return store;
}

function main(): Integer {
    chosen: Option<Boxed> = match (build().get(Result.error("σφάλμα🚀"))) {
        Present(row) => Option.some(row.inc()),
        Empty => Option.some(Boxed(0)),
    };
    status: Boolean = build().contains(Result.error("σφάλμα🚀"));
    return if (status) { chosen.unwrap().value } else { 0 };
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn unicode_tagged_join_and_equality_survives_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function choose(flag: Boolean): Result<Option<Boxed>, String> {
    return if (flag) {
        Result.ok(Option.some(Boxed(91)))
    } else {
        Result.error("σφάλμα🚀")
    };
}

function main(): Integer {
    picked: Result<Option<Boxed>, String> = match (true) {
        true => choose(true),
        false => choose(false),
    };
    if (picked == Result.error("σφάλμα🚀")) {
        return 0;
    }
    value: Option<Boxed> = match (picked) {
        Success(row) => row,
        Failure(err) => Option.none(),
    };
    return match (value) {
        Present(row) => row.value,
        Empty => 0,
    };
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn unicode_string_keyed_tagged_container_updates_survive_runtime() {
    let temp_root = make_temp_project_root("unicode-tagged-container-update-runtime");
    let source_path = temp_root.join("unicode_tagged_container_update_runtime.arden");
    let output_path = temp_root.join("unicode_tagged_container_update_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Integer> = Map<Result<Option<Integer>, String>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), i);
                    i = i + 1;
                }
                store.set(Result.error("σφάλμα🚀"), 100);
                store.set(Result.error("σφάλμα🚀"), 110);
                present: Boolean = store.contains(Result.error("σφάλμα🚀"));
                value: Integer = store.get(Result.error("σφάλμα🚀"));
                return if (present && value == 110) { 0; } else { 1; };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("unicode tagged container update runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled unicode tagged container update binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn string_keyed_tagged_set_growth_remove_contains_survives_runtime() {
    let temp_root = make_temp_project_root("string-keyed-tagged-set-runtime");
    let source_path = temp_root.join("string_keyed_tagged_set_runtime.arden");
    let output_path = temp_root.join("string_keyed_tagged_set_runtime");
    let source = r#"
            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    seen.add(Result.ok(Option.some(i)));
                    i = i + 1;
                }
                seen.add(Result.error("missing"));
                seen.add(Result.error("missing"));
                removed: Boolean = seen.remove(Result.ok(Option.some(4)));
                return if (removed && !seen.contains(Result.ok(Option.some(4))) && seen.contains(Result.error("missing")) && seen.length() == 9) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("string-keyed tagged set runtime should codegen");

    let status = std::process::Command::new(&output_path)
        .status()
        .must("run compiled string-keyed tagged set binary");
    assert_eq!(status.code(), Some(0));

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn mixed_unicode_ascii_string_error_keys_survive_map_runtime() {
    let temp_root = make_temp_project_root("mixed-unicode-ascii-string-error-map-runtime");
    let source_path = temp_root.join("mixed_unicode_ascii_string_error_map_runtime.arden");
    let output_path = temp_root.join("mixed_unicode_ascii_string_error_map_runtime");
    let source = r#"
            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Integer> = Map<Result<Option<Integer>, String>, Integer>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), i);
                    i = i + 1;
                }
                store.set(Result.error("missing"), 40);
                store.set(Result.error("σφάλμα🚀"), 50);
                store.set(Result.error("missing"), 41);
                return if (
                    store.contains(Result.error("missing"))
                    && store.contains(Result.error("σφάλμα🚀"))
                    && store.get(Result.error("missing")) == 41
                    && store.get(Result.error("σφάλμα🚀")) == 50
                ) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed unicode/ascii string error map runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled mixed unicode/ascii string error map binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn mixed_ascii_unicode_string_error_object_values_survive_runtime() {
    let temp_root = make_temp_project_root("mixed-ascii-unicode-string-error-object-runtime");
    let source_path = temp_root.join("mixed_ascii_unicode_string_error_object_runtime.arden");
    let output_path = temp_root.join("mixed_ascii_unicode_string_error_object_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(100)));
                store.set(Result.error("σφάλμα🚀"), Option.some(Boxed(200)));
                store.set(Result.error("missing"), Option.some(Boxed(110)));
                first: Option<Boxed> = store.get(Result.error("missing"));
                second: Option<Boxed> = store.get(Result.error("σφάλμα🚀"));
                return if (first.unwrap().inc().value == 111 && second.unwrap().inc().value == 201) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("mixed ascii/unicode string error object runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled mixed ascii/unicode string error object binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn alias_patterns_with_explicit_tagged_constructors_survive_frontend_backend() {
    let source = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.ok(Option.some(1)), Option.some(Boxed(5)));
    store.set(Result.ok(Option.none()), Option.none());
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(95)));
    }
    return store;
}

function main(): Integer {
    fetched: Result<Option<Integer>, String> = Result.error("missing");
    value: Option<Boxed> = match (build(true).get(fetched)) {
        Present(row) => Option.some(Boxed(row.value + 1)),
        Empty => Option.some(Boxed(0)),
    };
    status: Integer = match (fetched) {
        Success(inner) => match (inner) {
            Present(code) => code,
            Empty => 204,
        },
        Failure(err) => 500,
    };
    return value.unwrap().value + status;
}
"#;

    assert_frontend_pipeline_ok(source);
}

#[test]
fn nested_tagged_updates_match_values_and_chains_survive_runtime() {
    let temp_root = make_temp_project_root("nested-tagged-updates-match-values-runtime");
    let source_path = temp_root.join("nested_tagged_updates_match_values_runtime.arden");
    let output_path = temp_root.join("nested_tagged_updates_match_values_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(120)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(130)));
                }
                return store;
            }

            function main(): Integer {
                chosen: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                fallback: Option<Boxed> = build(false).get(Result.error("missing"));
                return if (chosen.unwrap().value == 131 && fallback.unwrap().value == 120) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested tagged updates + match values runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled nested tagged updates + match values binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn alias_tagged_container_valid_and_invalid_pair_reports_cleanly() {
    let valid = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

function classify(value: Result<Option<Integer>, String>): Integer {
    return match (value) {
        Success(inner) => match (inner) {
            Present(code) => code,
            Empty => 204,
        },
        Failure(err) => 500,
    };
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
import app.Option.Some as Present;
import app.Option.None as Empty;
import app.Result.Ok as Success;
import app.Result.Error as Failure;

function classify(value: Result<Option<Integer>, String>, cond: Boolean): Integer {
    return if (cond) {
        match (value) {
            Success(inner) => match (inner) {
                Present(code) => code,
                Empty => 204,
            },
            Failure(err) => 500,
        }
    } else {
        "oops"
    };
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid paired source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn nested_tagged_match_get_chain_survives_runtime() {
    let temp_root = make_temp_project_root("nested-tagged-match-get-chain-runtime");
    let source_path = temp_root.join("nested_tagged_match_get_chain_runtime.arden");
    let output_path = temp_root.join("nested_tagged_match_get_chain_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(140)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(150)));
                }
                return store;
            }

            function main(): Integer {
                selected: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                fallback: Option<Boxed> = match (build(false).get(Result.error("missing"))) {
                    Some(row) => Option.some(row),
                    None => Option.none(),
                };
                return if (selected.unwrap().value == 151 && fallback.unwrap().value == 140) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("nested tagged match/get chain runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled nested tagged match/get chain binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn paired_nested_tagged_match_get_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function main(): Integer {
    chosen: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
        Some(row) => Option.some(Boxed(row.value + 1)),
        None => Option.some(Boxed(0)),
    };
    return chosen.unwrap().value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function run(flag: Boolean): Integer {
    return if (flag) {
        match (build(true).get(Result.error("missing"))) {
            Some(row) => row.value,
            None => 0,
        }
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid nested tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn repeated_update_tagged_pipeline_match_get_equality_and_chain_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-tagged-pipeline-runtime");
    let source_path = temp_root.join("repeated_update_tagged_pipeline_runtime.arden");
    let output_path = temp_root.join("repeated_update_tagged_pipeline_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(160)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(170)));
                }
                return store;
            }

            function main(): Integer {
                latest: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                earlier: Option<Boxed> = build(false).get(Result.error("missing"));
                latest_key_present: Boolean = build(true).contains(Result.error("missing"));
                same_value: Boolean = build(true).get(Result.error("missing")).unwrap().value == 170;
                return if (latest_key_present && same_value && latest.unwrap().value == 171 && earlier.unwrap().value == 160) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated update tagged pipeline runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated update tagged pipeline binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn repeated_update_tagged_match_value_chain_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-tagged-match-value-runtime");
    let source_path = temp_root.join("repeated_update_tagged_match_value_runtime.arden");
    let output_path = temp_root.join("repeated_update_tagged_match_value_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(180)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(190)));
                }
                return store;
            }

            function choose(flag: Boolean): Option<Boxed> {
                picked: Option<Boxed> = match (build(flag).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                return picked;
            }

            function main(): Integer {
                latest: Option<Boxed> = choose(true);
                earlier: Option<Boxed> = choose(false);
                return if (latest.unwrap().value == 191 && earlier.unwrap().value == 181) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated update tagged match value runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated update tagged match value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn repeated_update_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function main(): Integer {
    picked: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
        Some(row) => Option.some(Boxed(row.value + 1)),
        None => Option.some(Boxed(0)),
    };
    return picked.unwrap().value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function run(flag: Boolean): Integer {
    return if (flag) {
        match (build(true).get(Result.error("missing"))) {
            Some(row) => row.value,
            None => 0,
        }
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid repeated-update tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn repeated_update_match_receiver_equality_chain_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-match-receiver-equality-runtime");
    let source_path = temp_root.join("repeated_update_match_receiver_equality_runtime.arden");
    let output_path = temp_root.join("repeated_update_match_receiver_equality_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(200)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(210)));
                }
                return store;
            }

            function pick(flag: Boolean): Boxed {
                chosen: Option<Boxed> = match (build(flag).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                return chosen.unwrap();
            }

            function main(): Integer {
                latest: Boxed = pick(true);
                earlier: Boxed = pick(false);
                same: Boolean = build(true).get(Result.error("missing")).unwrap().value == 210;
                return if (same && latest.value == 211 && earlier.value == 201) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated-update match receiver equality runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated-update match receiver equality binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn repeated_update_boolean_join_match_receiver_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-boolean-join-runtime");
    let source_path = temp_root.join("repeated_update_boolean_join_runtime.arden");
    let output_path = temp_root.join("repeated_update_boolean_join_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(220)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(230)));
                }
                return store;
            }

            function select(flag: Boolean): Integer {
                chosen: Option<Boxed> = match (build(flag).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                current_ok: Boolean = build(flag).contains(Result.error("missing"));
                current_val_ok: Boolean = build(flag).get(Result.error("missing")).unwrap().value == if (flag) { 230 } else { 220 };
                return if (current_ok && current_val_ok && chosen.unwrap().value == if (flag) { 231 } else { 221 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (select(true) == 1 && select(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated update boolean join runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated update boolean join binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn repeated_update_tagged_set_boolean_join_chain_survives_runtime() {
    let temp_root = make_temp_project_root("repeated-update-tagged-set-boolean-join-runtime");
    let source_path = temp_root.join("repeated_update_tagged_set_boolean_join_runtime.arden");
    let output_path = temp_root.join("repeated_update_tagged_set_boolean_join_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Set<Result<Option<Integer>, String>> {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    seen.add(Result.ok(Option.some(i)));
                    i = i + 1;
                }
                seen.add(Result.error("missing"));
                if (flag) {
                    seen.remove(Result.ok(Option.some(4)));
                }
                return seen;
            }

            function select(flag: Boolean): Boxed {
                return if (build(flag).contains(Result.error("missing")) && !build(flag).contains(Result.ok(Option.some(4)))) {
                    Boxed(240).inc()
                } else {
                    Boxed(0)
                };
            }

            function main(): Integer {
                latest: Boxed = select(true);
                earlier: Boxed = select(false);
                return if (latest.value == 241 && earlier.value == 0) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("repeated-update tagged set boolean join runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled repeated-update tagged set boolean join binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn tagged_set_valid_invalid_boolean_join_pair_reports_single_primary_error() {
    let valid = r#"
function build(flag: Boolean): Set<Result<Option<Integer>, String>> {
    seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
    mut i: Integer = 0;
    while (i < 9) {
        seen.add(Result.ok(Option.some(i)));
        i = i + 1;
    }
    seen.add(Result.error("missing"));
    if (flag) {
        seen.remove(Result.ok(Option.some(4)));
    }
    return seen;
}

function run(flag: Boolean): Integer {
    return if (build(flag).contains(Result.error("missing")) && !build(flag).contains(Result.ok(Option.some(4)))) {
        1
    } else {
        0
    };
}

function main(): Integer {
    return if (run(true) == 1 && run(false) == 0) { 0 } else { 1 };
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
function build(flag: Boolean): Set<Result<Option<Integer>, String>> {
    seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
    mut i: Integer = 0;
    while (i < 9) {
        seen.add(Result.ok(Option.some(i)));
        i = i + 1;
    }
    seen.add(Result.error("missing"));
    if (flag) {
        seen.remove(Result.ok(Option.some(4)));
    }
    return seen;
}

function run(flag: Boolean): Integer {
    return if (build(flag).contains(Result.error("missing")) && !build(flag).contains(Result.ok(Option.some(4)))) {
        1
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid tagged set pair should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn combined_map_set_tagged_pipeline_survives_runtime() {
    let temp_root = make_temp_project_root("combined-map-set-tagged-runtime");
    let source_path = temp_root.join("combined_map_set_tagged_runtime.arden");
    let output_path = temp_root.join("combined_map_set_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(250)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(260)));
                }
                return store;
            }

            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                seen.add(Result.error("missing"));
                seen.add(Result.ok(Option.some(1)));
                picked: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                return if (
                    seen.contains(Result.error("missing"))
                    && seen.contains(Result.ok(Option.some(1)))
                    && picked.unwrap().value == 261
                    && build(false).get(Result.error("missing")).unwrap().value == 250
                ) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("combined map/set tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled combined map/set tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn combined_map_set_repeated_update_equality_chain_survives_runtime() {
    let temp_root = make_temp_project_root("combined-map-set-repeated-update-equality-runtime");
    let source_path = temp_root.join("combined_map_set_repeated_update_equality_runtime.arden");
    let output_path = temp_root.join("combined_map_set_repeated_update_equality_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(300)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(310)));
                }
                return store;
            }

            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                seen.add(Result.error("missing"));
                seen.add(Result.ok(Option.some(1)));
                chosen: Option<Boxed> = match (build(true).get(Result.error("missing"))) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                same_latest: Boolean = build(true).get(Result.error("missing")).unwrap().value == 310;
                same_earlier: Boolean = build(false).get(Result.error("missing")).unwrap().value == 300;
                return if (
                    seen.contains(Result.error("missing"))
                    && seen.contains(Result.ok(Option.some(1)))
                    && same_latest
                    && same_earlier
                    && chosen.unwrap().value == 311
                ) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("combined map/set repeated update equality runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled combined map/set repeated update equality binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn tagged_membership_branch_match_value_equality_survives_runtime() {
    let temp_root = make_temp_project_root("tagged-membership-branch-match-value-runtime");
    let source_path = temp_root.join("tagged_membership_branch_match_value_runtime.arden");
    let output_path = temp_root.join("tagged_membership_branch_match_value_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(320)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(330)));
                }
                return store;
            }

            function choose(flag: Boolean): Option<Boxed> {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                seen.add(Result.error("missing"));
                return if (seen.contains(Result.error("missing"))) {
                    match (build(flag).get(Result.error("missing"))) {
                        Some(row) => Option.some(row.inc()),
                        None => Option.some(Boxed(0)),
                    }
                } else {
                    Option.some(Boxed(1))
                };
            }

            function main(): Integer {
                latest: Option<Boxed> = choose(true);
                earlier: Option<Boxed> = choose(false);
                return if (latest.unwrap().value == 331 && earlier.unwrap().value == 321) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("tagged membership branch match value runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled tagged membership branch match value binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn multi_function_tagged_pipeline_returned_values_and_chains_survive_runtime() {
    let temp_root = make_temp_project_root("multi-function-tagged-pipeline-runtime");
    let source_path = temp_root.join("multi_function_tagged_pipeline_runtime.arden");
    let output_path = temp_root.join("multi_function_tagged_pipeline_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(340)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(350)));
                }
                return store;
            }

            function fetch(flag: Boolean): Option<Boxed> {
                return build(flag).get(Result.error("missing"));
            }

            function elevate(flag: Boolean): Boxed {
                chosen: Option<Boxed> = match (fetch(flag)) {
                    Some(row) => Option.some(row.inc()),
                    None => Option.some(Boxed(0)),
                };
                return chosen.unwrap();
            }

            function main(): Integer {
                latest: Boxed = elevate(true);
                earlier: Boxed = elevate(false);
                return if (latest.value == 351 && earlier.value == 341) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("multi-function tagged pipeline runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled multi-function tagged pipeline binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn multi_function_tagged_pipeline_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function fetch(flag: Boolean): Option<Boxed> {
    return build(flag).get(Result.error("missing"));
}

function elevate(flag: Boolean): Boxed {
    chosen: Option<Boxed> = match (fetch(flag)) {
        Some(row) => Option.some(Boxed(row.value + 1)),
        None => Option.some(Boxed(0)),
    };
    return chosen.unwrap();
}

function main(): Integer {
    return elevate(true).value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function fetch(flag: Boolean): Option<Boxed> {
    return build(flag).get(Result.error("missing"));
}

function run(flag: Boolean): Integer {
    return if (flag) {
        match (fetch(flag)) {
            Some(row) => row.value,
            None => 0,
        }
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid multi-function tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn multi_helper_tagged_runtime_pipeline_survives_codegen() {
    let temp_root = make_temp_project_root("multi-helper-tagged-runtime");
    let source_path = temp_root.join("multi_helper_tagged_runtime.arden");
    let output_path = temp_root.join("multi_helper_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
                store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Option.some(Boxed(i)));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Option.some(Boxed(400)));
                if (flag) {
                    store.set(Result.error("missing"), Option.some(Boxed(410)));
                }
                return store;
            }

            function fetch(flag: Boolean): Option<Boxed> {
                return build(flag).get(Result.error("missing"));
            }

            function elevate(flag: Boolean): Boxed {
                return match (fetch(flag)) {
                    Some(row) => row.inc(),
                    None => Boxed(0),
                };
            }

            function score(flag: Boolean): Integer {
                current: Boxed = elevate(flag);
                return if (build(flag).contains(Result.error("missing")) && current.value == if (flag) { 411 } else { 401 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (score(true) == 1 && score(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("multi-helper tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled multi-helper tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn multi_helper_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function fetch(flag: Boolean): Option<Boxed> {
    return build(flag).get(Result.error("missing"));
}

function elevate(flag: Boolean): Boxed {
    chosen: Option<Boxed> = match (fetch(flag)) {
        Some(row) => Option.some(Boxed(row.value + 1)),
        None => Option.some(Boxed(0)),
    };
    return chosen.unwrap();
}

function main(): Integer {
    return elevate(true).value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Option<Boxed>> {
    store: Map<Result<Option<Integer>, String>, Option<Boxed>> = Map<Result<Option<Integer>, String>, Option<Boxed>>();
    store.set(Result.error("missing"), Option.some(Boxed(1)));
    if (flag) {
        store.set(Result.error("missing"), Option.some(Boxed(2)));
    }
    return store;
}

function fetch(flag: Boolean): Option<Boxed> {
    return build(flag).get(Result.error("missing"));
}

function run(flag: Boolean): Integer {
    return if (fetch(flag).is_some()) {
        match (fetch(flag)) {
            Some(row) => row.value,
            None => 0,
        }
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid multi-helper tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_nested_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-nested-tagged-runtime");
    let source_path = temp_root.join("fresh_nested_tagged_runtime.arden");
    let output_path = temp_root.join("fresh_nested_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(500))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(510))));
                }
                return store;
            }

            function lift(flag: Boolean): Boxed {
                chosen: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
                payload: Option<Boxed> = match (chosen) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return payload.unwrap().inc();
            }

            function main(): Integer {
                latest: Boxed = lift(true);
                earlier: Boxed = lift(false);
                return if (latest.value == 511 && earlier.value == 501) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh nested tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh nested tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn nested_tagged_value_flow_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lift(flag: Boolean): Boxed {
    chosen: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    payload: Option<Boxed> = match (chosen) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return payload.unwrap();
}

function main(): Integer {
    return lift(true).value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function run(flag: Boolean): Integer {
    chosen: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    payload: Option<Boxed> = match (chosen) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        payload.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid nested tagged value-flow source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn deeper_multi_helper_nested_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("deeper-multi-helper-nested-tagged-runtime");
    let source_path = temp_root.join("deeper_multi_helper_nested_tagged_runtime.arden");
    let output_path = temp_root.join("deeper_multi_helper_nested_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(600))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(610))));
                }
                return store;
            }

            function fetch(flag: Boolean): Result<Option<Boxed>, String> {
                return build(flag).get(Result.error("missing"));
            }

            function lift(flag: Boolean): Option<Boxed> {
                selected: Result<Option<Boxed>, String> = fetch(flag);
                return match (selected) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
            }

            function score(flag: Boolean): Integer {
                latest: Option<Boxed> = lift(flag);
                current: Boxed = latest.unwrap().inc();
                return if (current.value == if (flag) { 611 } else { 601 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (score(true) == 1 && score(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("deeper multi-helper nested tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled deeper multi-helper nested tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn deeper_multi_helper_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function lift(flag: Boolean): Option<Boxed> {
    selected: Result<Option<Boxed>, String> = fetch(flag);
    return match (selected) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
}

function main(): Integer {
    return lift(true).unwrap().value;
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function lift(flag: Boolean): Option<Boxed> {
    selected: Result<Option<Boxed>, String> = fetch(flag);
    return match (selected) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
}

function run(flag: Boolean): Integer {
    return if (flag) {
        lift(flag).unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid deeper multi-helper tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_multi_stage_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-multi-stage-tagged-runtime");
    let source_path = temp_root.join("fresh_multi_stage_tagged_runtime.arden");
    let output_path = temp_root.join("fresh_multi_stage_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(700))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(710))));
                }
                return store;
            }

            function fetch(flag: Boolean): Result<Option<Boxed>, String> {
                return build(flag).get(Result.error("missing"));
            }

            function project(flag: Boolean): Boxed {
                staged: Option<Boxed> = match (fetch(flag)) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return staged.unwrap().inc();
            }

            function main(): Integer {
                latest: Boxed = project(true);
                earlier: Boxed = project(false);
                current: Result<Option<Boxed>, String> = build(true).get(Result.error("missing"));
                same: Boolean = match (current) {
                    Ok(inner) => match (inner) {
                        Some(row) => row.value == 710,
                        None => false,
                    },
                    Error(err) => false,
                };
                return if (same && latest.value == 711 && earlier.value == 701) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh multi-stage tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh multi-stage tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn scalar_observation_nested_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("scalar-observation-nested-tagged-runtime");
    let source_path = temp_root.join("scalar_observation_nested_tagged_runtime.arden");
    let output_path = temp_root.join("scalar_observation_nested_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
                function inc(): Boxed { return Boxed(this.value + 1); }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(800))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(810))));
                }
                return store;
            }

            function fetch_value(flag: Boolean): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().inc().value;
            }

            function main(): Integer {
                latest: Integer = fetch_value(true);
                earlier: Integer = fetch_value(false);
                still_present: Boolean = build(true).contains(Result.error("missing"));
                return if (still_present && latest == 811 && earlier == 801) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("scalar observation nested tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled scalar observation nested tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn scalar_only_nested_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch_value(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return fetch_value(true);
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function run(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid scalar-only tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_scalar_join_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-scalar-join-tagged-runtime");
    let source_path = temp_root.join("fresh_scalar_join_tagged_runtime.arden");
    let output_path = temp_root.join("fresh_scalar_join_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(900))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(910))));
                }
                return store;
            }

            function scalar(flag: Boolean): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                value: Integer = boxed.unwrap().value;
                present: Boolean = build(flag).contains(Result.error("missing"));
                return if (present && value == if (flag) { 910 } else { 900 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (scalar(true) == 1 && scalar(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh scalar join tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh scalar join tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn scalar_only_multi_helper_tagged_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch_value(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return fetch_value(true);
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function scalar(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error("missing"));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return scalar(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid scalar-only multi-helper tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn three_helper_scalar_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("three-helper-scalar-tagged-runtime");
    let source_path = temp_root.join("three_helper_scalar_tagged_runtime.arden");
    let output_path = temp_root.join("three_helper_scalar_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1000))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1010))));
                }
                return store;
            }

            function fetch(flag: Boolean): Result<Option<Boxed>, String> {
                return build(flag).get(Result.error("missing"));
            }

            function extract(flag: Boolean): Integer {
                current: Result<Option<Boxed>, String> = fetch(flag);
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function score(flag: Boolean): Integer {
                value: Integer = extract(flag);
                return if (value == if (flag) { 1010 } else { 1000 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (score(true) == 1 && score(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("three-helper scalar tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled three-helper scalar tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn three_helper_scalar_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function extract(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = fetch(flag);
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return extract(true);
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function run(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = fetch(flag);
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return run(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid three-helper scalar tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_three_helper_repeated_update_scalar_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-three-helper-repeated-update-scalar-runtime");
    let source_path = temp_root.join("fresh_three_helper_repeated_update_scalar_runtime.arden");
    let output_path = temp_root.join("fresh_three_helper_repeated_update_scalar_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1100))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1110))));
                }
                return store;
            }

            function fetch(flag: Boolean): Result<Option<Boxed>, String> {
                return build(flag).get(Result.error("missing"));
            }

            function extract(flag: Boolean): Integer {
                current: Result<Option<Boxed>, String> = fetch(flag);
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function score(flag: Boolean): Integer {
                latest: Integer = extract(flag);
                present: Boolean = build(flag).contains(Result.error("missing"));
                return if (present && latest == if (flag) { 1110 } else { 1100 }) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (score(true) == 1 && score(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh three-helper repeated-update scalar runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh three-helper repeated-update scalar runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn three_helper_repeated_update_scalar_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function extract(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = fetch(flag);
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return extract(true);
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch(flag: Boolean): Result<Option<Boxed>, String> {
    return build(flag).get(Result.error("missing"));
}

function extract(flag: Boolean): Integer {
    current: Result<Option<Boxed>, String> = fetch(flag);
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return extract(true);
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid three-helper repeated-update scalar source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn independent_lookup_nested_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("independent-lookup-nested-tagged-runtime");
    let source_path = temp_root.join("independent_lookup_nested_tagged_runtime.arden");
    let output_path = temp_root.join("independent_lookup_nested_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1200))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(1300))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1210))));
                }
                return store;
            }

            function lookup(flag: Boolean, key: String): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function main(): Integer {
                a: Integer = lookup(true, "missing");
                b: Integer = lookup(false, "missing");
                c: Integer = lookup(true, "other");
                d: Boolean = build(true).contains(Result.error("missing"));
                e: Boolean = build(true).contains(Result.error("other"));
                return if (a == 1210 && b == 1200 && c == 1300 && d && e) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("independent lookup nested tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled independent lookup nested tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn parameterized_lookup_tagged_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid parameterized lookup tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn parameterized_scalar_lookup_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("parameterized-scalar-lookup-runtime");
    let source_path = temp_root.join("parameterized_scalar_lookup_runtime.arden");
    let output_path = temp_root.join("parameterized_scalar_lookup_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1400))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(1500))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1410))));
                }
                return store;
            }

            function lookup(flag: Boolean, key: String): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function main(): Integer {
                a: Integer = lookup(true, "missing");
                b: Integer = lookup(false, "missing");
                c: Integer = lookup(true, "other");
                d: Boolean = build(true).contains(Result.error("missing"));
                e: Boolean = build(true).contains(Result.error("other"));
                return if (a == 1410 && b == 1400 && c == 1500 && d && e) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("parameterized scalar lookup runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled parameterized scalar lookup binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn parameterized_multi_key_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid parameterized multi-key tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn parameterized_multi_key_scalar_join_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("parameterized-multi-key-scalar-join-runtime");
    let source_path = temp_root.join("parameterized_multi_key_scalar_join_runtime.arden");
    let output_path = temp_root.join("parameterized_multi_key_scalar_join_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1600))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(1700))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1610))));
                }
                return store;
            }

            function lookup(flag: Boolean, key: String): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function main(): Integer {
                latest: Integer = lookup(true, "missing");
                earlier: Integer = lookup(false, "missing");
                other: Integer = lookup(true, "other");
                has_missing: Boolean = build(true).contains(Result.error("missing"));
                has_other: Boolean = build(true).contains(Result.error("other"));
                return if (latest == 1610 && earlier == 1600 && other == 1700 && has_missing && has_other) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("parameterized multi-key scalar join runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled parameterized multi-key scalar join binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn parameterized_multi_key_scalar_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function lookup(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return lookup(true, "missing");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid parameterized multi-key scalar source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn fresh_multi_key_repeated_update_scalar_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-multi-key-repeated-update-scalar-runtime");
    let source_path = temp_root.join("fresh_multi_key_repeated_update_scalar_runtime.arden");
    let output_path = temp_root.join("fresh_multi_key_repeated_update_scalar_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1800))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(1900))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1810))));
                }
                return store;
            }

            function fetch_value(flag: Boolean, key: String): Integer {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                boxed: Option<Boxed> = match (current) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return boxed.unwrap().value;
            }

            function main(): Integer {
                latest: Integer = fetch_value(true, "missing");
                earlier: Integer = fetch_value(false, "missing");
                other: Integer = fetch_value(true, "other");
                present_missing: Boolean = build(true).contains(Result.error("missing"));
                present_other: Boolean = build(true).contains(Result.error("other"));
                return if (latest == 1810 && earlier == 1800 && other == 1900 && present_missing && present_other) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh multi-key repeated-update scalar runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh multi-key repeated-update scalar runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn fresh_multi_key_scalar_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch_value(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return boxed.unwrap().value;
}

function main(): Integer {
    return fetch_value(true, "missing");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function fetch_value(flag: Boolean, key: String): Integer {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    boxed: Option<Boxed> = match (current) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        boxed.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return fetch_value(true, "missing");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid fresh multi-key scalar source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn new_multi_key_nested_join_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("new-multi-key-nested-join-runtime");
    let source_path = temp_root.join("new_multi_key_nested_join_runtime.arden");
    let output_path = temp_root.join("new_multi_key_nested_join_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2000))));
                store.set(Result.error("beta"), Result.ok(Option.some(Boxed(3000))));
                if (flag) {
                    store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2010))));
                }
                return store;
            }

            function value_for(flag: Boolean, key: String): Integer {
                entry: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                payload: Option<Boxed> = match (entry) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return payload.unwrap().value;
            }

            function joined(flag: Boolean): Integer {
                left: Integer = value_for(flag, "alpha");
                right: Integer = value_for(true, "beta");
                left_present: Boolean = build(flag).contains(Result.error("alpha"));
                right_present: Boolean = build(true).contains(Result.error("beta"));
                return if (left_present && right_present && left == if (flag) { 2010 } else { 2000 } && right == 3000) { 1 } else { 0 };
            }

            function main(): Integer {
                return if (joined(true) == 1 && joined(false) == 1) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("new multi-key nested join runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled new multi-key nested join binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn multi_key_joined_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("beta"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function value_for(flag: Boolean, key: String): Integer {
    entry: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    payload: Option<Boxed> = match (entry) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return payload.unwrap().value;
}

function main(): Integer {
    return value_for(true, "alpha");
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("beta"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function value_for(flag: Boolean, key: String): Integer {
    entry: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    payload: Option<Boxed> = match (entry) {
        Ok(inner) => inner,
        Error(err) => Option.some(Boxed(0)),
    };
    return if (flag) {
        payload.unwrap().value
    } else {
        "oops"
    };
}

function main(): Integer {
    return value_for(true, "alpha");
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid multi-key joined tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn static_constructor_comparison_multi_key_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("static-constructor-comparison-multi-key-runtime");
    let source_path = temp_root.join("static_constructor_comparison_multi_key_runtime.arden");
    let output_path = temp_root.join("static_constructor_comparison_multi_key_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(1))));
                store.set(Result.error("beta"), Result.ok(Option.some(Boxed(3))));
                if (flag) {
                    store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(2))));
                }
                return store;
            }

            function key_matches(flag: Boolean, key: String, expected: Integer): Boolean {
                entry: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                scalar_tag: Result<Option<Integer>, String> = match (entry) {
                    Ok(inner) => match (inner) {
                        Some(row) => Result.ok(Option.some(row.value)),
                        None => Result.ok(Option.none()),
                    },
                    Error(err) => Result.error(err),
                };
                same_tag: Boolean = scalar_tag == Result.ok(Option.some(expected));
                payload: Option<Boxed> = match (entry) {
                    Ok(inner) => inner,
                    Error(err) => Option.some(Boxed(0)),
                };
                return same_tag && payload.unwrap().value == expected;
            }

            function main(): Integer {
                return if (key_matches(true, "alpha", 2) && key_matches(false, "alpha", 1) && key_matches(true, "beta", 3)) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("static constructor comparison multi-key runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled static constructor comparison multi-key binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn fresh_scalar_only_tagged_runtime_stressor_survives_codegen() {
    let temp_root = make_temp_project_root("fresh-scalar-only-tagged-runtime");
    let source_path = temp_root.join("fresh_scalar_only_tagged_runtime.arden");
    let output_path = temp_root.join("fresh_scalar_only_tagged_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2100))));
                store.set(Result.error("other"), Result.ok(Option.some(Boxed(2200))));
                if (flag) {
                    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2110))));
                }
                return store;
            }

            function scalar(flag: Boolean, key: String): Result<Option<Integer>, String> {
                current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
                return match (current) {
                    Ok(inner) => match (inner) {
                        Some(row) => Result.ok(Option.some(row.value)),
                        None => Result.ok(Option.none()),
                    },
                    Error(err) => Result.error(err),
                };
            }

            function main(): Integer {
                a: Result<Option<Integer>, String> = scalar(true, "missing");
                b: Result<Option<Integer>, String> = scalar(false, "missing");
                c: Result<Option<Integer>, String> = scalar(true, "other");
                d: Result<Option<Integer>, String> = Result.error("missing");
                return if (
                    a == Result.ok(Option.some(2110))
                    && b == Result.ok(Option.some(2100))
                    && c == Result.ok(Option.some(2200))
                    && d == Result.error("missing")
                ) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("fresh scalar-only tagged runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled fresh scalar-only tagged runtime binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn scalarized_nested_tagged_valid_invalid_pair_reports_single_primary_error() {
    let valid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function scalar(flag: Boolean, key: String): Result<Option<Integer>, String> {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    return match (current) {
        Ok(inner) => match (inner) {
            Some(row) => Result.ok(Option.some(row.value)),
            None => Result.ok(Option.none()),
        },
        Error(err) => Result.error(err),
    };
}

function main(): Integer {
    return if (scalar(true, "missing") == Result.ok(Option.some(2))) { 0 } else { 1 };
}
"#;
    assert_frontend_pipeline_ok(valid);

    let invalid = r#"
class Boxed {
    value: Integer;
    constructor(value: Integer) { this.value = value; }
}

function build(flag: Boolean): Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> {
    store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
    store.set(Result.error("missing"), Result.ok(Option.some(Boxed(1))));
    store.set(Result.error("other"), Result.ok(Option.some(Boxed(3))));
    if (flag) {
        store.set(Result.error("missing"), Result.ok(Option.some(Boxed(2))));
    }
    return store;
}

function scalar(flag: Boolean, key: String): Result<Option<Integer>, String> {
    current: Result<Option<Boxed>, String> = build(flag).get(Result.error(key));
    return if (flag) {
        match (current) {
            Ok(inner) => match (inner) {
                Some(row) => Result.ok(Option.some(row.value)),
                None => Result.ok(Option.none()),
            },
            Error(err) => Result.error(err),
        }
    } else {
        "oops"
    };
}

function main(): Integer {
    return if (scalar(true, "missing") == Result.ok(Option.some(2))) { 0 } else { 1 };
}
"#;
    let program = parse_program(invalid);
    let mut type_checker = TypeChecker::new();
    let errors = type_checker
        .check(&program)
        .must_err("invalid scalarized nested tagged source should fail");
    let messages = errors.into_iter().map(|e| e.message).collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("If expression branch type mismatch")),
        "{}",
        messages.join("\n")
    );
    assert_eq!(messages.len(), 1, "{}", messages.join("\n"));
}

#[test]
fn source_driven_nested_tagged_storage_path_survives_runtime() {
    let temp_root = make_temp_project_root("source-driven-nested-tagged-storage-runtime");
    let source_path = temp_root.join("source_driven_nested_tagged_storage_runtime.arden");
    let output_path = temp_root.join("source_driven_nested_tagged_storage_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Boxed>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(Boxed(i))));
                    i = i + 1;
                }
                store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(10))));
                store.set(Result.error("beta"), Result.ok(Option.some(Boxed(20))));
                store.set(Result.error("alpha"), Result.ok(Option.some(Boxed(11))));

                has_alpha: Boolean = store.contains(Result.error("alpha"));
                has_beta: Boolean = store.contains(Result.error("beta"));

                alpha: Result<Option<Boxed>, String> = store.get(Result.error("alpha"));
                beta: Result<Option<Boxed>, String> = store.get(Result.error("beta"));

                alpha_value: Integer = match (alpha) {
                    Ok(inner) => match (inner) {
                        Some(row) => row.value,
                        None => -1,
                    },
                    Error(err) => -2,
                };
                beta_value: Integer = match (beta) {
                    Ok(inner) => match (inner) {
                        Some(row) => row.value,
                        None => -3,
                    },
                    Error(err) => -4,
                };

                return if (has_alpha && has_beta && alpha_value == 11 && beta_value == 20 && store.length() == 11) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("source-driven nested tagged storage runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled source-driven nested tagged storage binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn source_driven_set_remove_shift_scalar_observation_survives_runtime() {
    let temp_root = make_temp_project_root("source-driven-set-remove-shift-runtime");
    let source_path = temp_root.join("source_driven_set_remove_shift_runtime.arden");
    let output_path = temp_root.join("source_driven_set_remove_shift_runtime");
    let source = r#"
            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    seen.add(Result.ok(Option.some(i)));
                    i = i + 1;
                }
                seen.add(Result.error("alpha"));
                seen.add(Result.error("beta"));
                removed: Boolean = seen.remove(Result.ok(Option.some(4)));
                has_alpha: Boolean = seen.contains(Result.error("alpha"));
                has_beta: Boolean = seen.contains(Result.error("beta"));
                has_four: Boolean = seen.contains(Result.ok(Option.some(4)));
                len: Integer = seen.length();
                return if (removed && has_alpha && has_beta && !has_four && len == 10) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("source-driven set remove shift runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled source-driven set remove shift binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn source_driven_multi_overwrite_tagged_map_runtime_survives_codegen() {
    let temp_root = make_temp_project_root("source-driven-multi-overwrite-tagged-map-runtime");
    let source_path = temp_root.join("source_driven_multi_overwrite_tagged_map_runtime.arden");
    let output_path = temp_root.join("source_driven_multi_overwrite_tagged_map_runtime");
    let source = r#"
            class Boxed {
                value: Integer;
                constructor(value: Integer) { this.value = value; }
            }

            function main(): Integer {
                store: Map<Result<Option<Integer>, String>, Result<Option<Integer>, String>> = Map<Result<Option<Integer>, String>, Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    store.set(Result.ok(Option.some(i)), Result.ok(Option.some(i + 100)));
                    i = i + 1;
                }
                store.set(Result.error("alpha"), Result.ok(Option.some(10)));
                store.set(Result.error("beta"), Result.ok(Option.some(20)));
                store.set(Result.error("alpha"), Result.ok(Option.some(11)));
                store.set(Result.error("beta"), Result.ok(Option.some(21)));

                a: Result<Option<Integer>, String> = store.get(Result.error("alpha"));
                b: Result<Option<Integer>, String> = store.get(Result.error("beta"));

                a_value: Integer = match (a) {
                    Ok(inner) => match (inner) {
                        Some(v) => v,
                        None => -1,
                    },
                    Error(err) => -2,
                };
                b_value: Integer = match (b) {
                    Ok(inner) => match (inner) {
                        Some(v) => v,
                        None => -3,
                    },
                    Error(err) => -4,
                };

                has_alpha: Boolean = store.contains(Result.error("alpha"));
                has_beta: Boolean = store.contains(Result.error("beta"));
                return if (a_value == 11 && b_value == 21 && has_alpha && has_beta && store.length() == 11) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("source-driven multi-overwrite tagged map runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled source-driven multi-overwrite tagged map binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn source_driven_set_add_remove_readd_scalar_observation_survives_runtime() {
    let temp_root = make_temp_project_root("source-driven-set-readd-runtime");
    let source_path = temp_root.join("source_driven_set_readd_runtime.arden");
    let output_path = temp_root.join("source_driven_set_readd_runtime");
    let source = r#"
            function main(): Integer {
                seen: Set<Result<Option<Integer>, String>> = Set<Result<Option<Integer>, String>>();
                mut i: Integer = 0;
                while (i < 9) {
                    seen.add(Result.ok(Option.some(i)));
                    i = i + 1;
                }
                seen.add(Result.error("alpha"));
                removed: Boolean = seen.remove(Result.ok(Option.some(4)));
                seen.add(Result.ok(Option.some(4)));
                has_alpha: Boolean = seen.contains(Result.error("alpha"));
                has_four: Boolean = seen.contains(Result.ok(Option.some(4)));
                len: Integer = seen.length();
                return if (removed && has_alpha && has_four && len == 10) { 0 } else { 1 };
            }
        "#;

    fs::write(&source_path, source).must("write source");
    compile_source(source, &source_path, &output_path, false, true, None, None)
        .must("source-driven set readd runtime should codegen");

    let output = std::process::Command::new(&output_path)
        .output()
        .must("run compiled source-driven set readd binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(temp_root);
}
