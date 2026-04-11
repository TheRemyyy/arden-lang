use super::*;

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
