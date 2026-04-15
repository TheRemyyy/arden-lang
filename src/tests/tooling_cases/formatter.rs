use super::TestExpectExt;
use crate::formatter::format_source;
use crate::lexer::tokenize;
use crate::parser::Parser;

#[test]
fn formats_basic_program() {
    let source = r#"package app;
import std.io.*;
function main(): None {mut value: Integer=1+2*3;println("hi {value}");return None;}"#;

    let formatted = format_source(source).must("format succeeds");

    assert_eq!(
        formatted,
        concat!(
            "package app;\n",
            "\n",
            "import std.io.*;\n",
            "\n",
            "public function main(): None {\n",
            "    mut value: Integer = 1 + 2 * 3;\n",
            "    println(\"hi {value}\");\n",
            "    return None;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_extern_and_generics() {
    let source = r#"extern(c,"puts") function c_puts(msg:String): Integer;function id<T>(value:T): T {return value;}"#;
    let formatted = format_source(source).must("format succeeds");

    assert!(formatted.contains("extern(c, \"puts\") function c_puts(msg: String): Integer;"));
    assert!(formatted.contains("public function id<T>(value: T): T {"));
}

#[test]
fn keeps_comments_in_output() {
    let source = "// note\nfunction main(): None { return None; }";
    let formatted = format_source(source).must("comments should be preserved");
    assert!(formatted.contains("// note"));
}

#[test]
fn preserves_leading_comments_before_package() {
    let source = r#"// banner
package demo;

function main(): None { return; }
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.starts_with("// banner\npackage demo;\n"),
        "{formatted}"
    );
}

#[test]
fn preserves_leading_comments_before_package_with_lone_cr_endings() {
    let source = "// banner\rpackage demo;\rfunction main(): None { return; }\r";
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.starts_with("// banner\npackage demo;\n"),
        "{formatted}"
    );
}

#[test]
fn preserves_literal_braces_in_plain_strings() {
    let source = r#"
function main(): None {
s: String = "\{literal\}";
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("\"\\{literal\\}\""), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser
        .parse_program()
        .must("formatted output should preserve literal braces");
}

#[test]
fn preserves_literal_braces_in_ignore_reasons() {
    let source = r#"
@Test
@Ignore("\{skip\}")
function skipped(): None { return None; }
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("@Ignore(\"\\{skip\\}\")"), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser
        .parse_program()
        .must("formatted output should preserve ignore reason braces");
}

#[test]
fn wraps_match_expression_statements_for_roundtrip() {
    let source = r#"
function main(): None {
x: Integer = match (1) {
    1 => { (match (2) { 2 => { 3; }, _ => { 4; } }); },
    _ => { 0; }
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("match (2) { 2 => 3, _ => 4 }"),
        "{formatted}"
    );
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn wraps_if_expression_statements_for_roundtrip() {
    let source = r#"
function main(): None {
x: Integer = 0;
(if (true) { 1; } else { 2; });
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("(if (true)"));
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn preserves_shebang_line() {
    let source = r#"#!/usr/bin/env arden
function main(): None { return None; }
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.starts_with("#!/usr/bin/env arden\n"));
}

#[test]
fn preserves_shebang_line_with_lone_cr_ending() {
    let source = "#!/usr/bin/env arden\rfunction main(): None { return None; }\r";
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.starts_with("#!/usr/bin/env arden\n"),
        "{formatted}"
    );
}

#[test]
fn formats_else_if_statement_chain() {
    let source = r#"
function main(): None {
if (true) { return None; } else { if (false) { return None; } else { return None; } }
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("} else if (false) {"), "{formatted}");
}

#[test]
fn formats_else_if_expression_chain() {
    let source = r#"
function main(): None {
x: Integer = if (true) { 1; } else if (false) { 2; } else { 3; };
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("else if (false)"), "{formatted}");
}

#[test]
fn wraps_lambda_callee_for_roundtrip() {
    let source = r#"
function main(): None {
y: Integer = ((x: Integer) => x + 1)(2);
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("((x: Integer) => x + 1)(2)"),
        "{formatted}"
    );
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn wraps_if_expression_callee_for_roundtrip() {
    let source = r#"
function main(): None {
y: Integer = (if (true) { foo; } else { bar; })(2);
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("(if (true)"), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn wraps_match_expression_callee_for_roundtrip() {
    let source = r#"
function main(): None {
y: Integer = (match (1) { 1 => { foo; }, _ => { bar; } })(2);
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("(match (1)"), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn wraps_deref_callee_for_roundtrip() {
    let source = r#"
function main(): None {
y: Integer = (*f)(2);
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("(*f)(2)"), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn wraps_try_callee_for_roundtrip() {
    let source = r#"
function main(): Result<None, String> {
y: Integer = (choose()?)(2);
return Result.ok(None);
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("(choose()?)(2)"), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn formats_borrow_mut_params_in_parser_order() {
    let source = r#"
function f(borrow mut value: String): None {
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("borrow mut value: String"),
        "{formatted}"
    );
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser
        .parse_program()
        .must("formatted borrow-mut params should parse");
}

#[test]
fn formats_multiple_generic_bounds_with_commas() {
    let source = r#"
function f<T extends A, B>(value: T): None {
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("T extends A, B"), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser
        .parse_program()
        .must("formatted generic bounds should parse");
}

#[test]
fn preserves_comments_inside_async_blocks() {
    let source = r#"
function main(): None {
task: Task<Integer> = async {
    // keep me
    return 1;
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("async {\n        // keep me\n        return 1;\n    }"),
        "{formatted}"
    );
}

#[test]
fn preserves_async_block_tail_expression_without_semicolon() {
    let source = r#"
function main(): None {
task: Task<Integer> = async {
    1
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("async {\n        1\n    }"),
        "{formatted}"
    );
    assert!(
        !formatted.contains("async {\n        1;\n    }"),
        "{formatted}"
    );
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser
        .parse_program()
        .must("formatted async tail expression should parse");
}

#[test]
fn preserves_comments_inside_if_expression_blocks() {
    let source = r#"
function main(): None {
value: Integer = if (true) {
    // keep me
    1;
} else {
    2;
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("if (true) {\n        // keep me\n        1\n    }"),
        "{formatted}"
    );
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn preserves_if_expression_tail_without_semicolon() {
    let source = r#"
function main(): None {
value: Integer = if (true) {
    1
} else {
    2
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("if (true) {\n        1\n    } else {\n        2\n    }"),
        "{formatted}"
    );
    assert!(
        !formatted.contains("if (true) {\n        1;\n    } else {\n        2;\n    }"),
        "{formatted}"
    );
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser
        .parse_program()
        .must("formatted if-expression tail should parse");
}

#[test]
fn preserves_comments_inside_match_expression_blocks() {
    let source = r#"
function main(): None {
value: Integer = match (1) {
    1 => {
        // keep me
        1;
    },
    _ => {
        2;
    }
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("// keep me"), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn preserves_trailing_comments_inside_async_blocks() {
    let source = r#"
function main(): None {
task: Task<Integer> = async {
    return 1;
    // trailing keep me
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("return 1;\n        // trailing keep me\n    }"),
        "{formatted}"
    );
}

#[test]
fn preserves_trailing_comments_inside_if_expression_blocks() {
    let source = r#"
function main(): None {
value: Integer = if (true) {
    1;
    // trailing keep me
} else {
    2;
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("1\n        // trailing keep me\n    } else {"),
        "{formatted}"
    );
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn preserves_trailing_comments_inside_match_expression_blocks() {
    let source = r#"
function main(): None {
value: Integer = match (1) {
    1 => {
        1;
        // trailing keep me
    },
    _ => {
        2;
    }
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("// trailing keep me"), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn preserves_block_comments_inside_async_blocks() {
    let source = r#"
function main(): None {
task: Task<Integer> = async {
    /* keep me */
    return 1;
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("async {\n        /* keep me */\n        return 1;\n    }"),
        "{formatted}"
    );
}

#[test]
fn preserves_trailing_block_comments_inside_if_expression_blocks() {
    let source = r#"
function main(): None {
value: Integer = if (true) {
    1;
    /* trailing keep me */
} else {
    2;
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("1\n        /* trailing keep me */\n    } else {"),
        "{formatted}"
    );
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn preserves_comments_inside_empty_async_blocks() {
    let source = r#"
function main(): None {
task: Task<None> = async {
    // keep me
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("async {\n        // keep me\n    }"),
        "{formatted}"
    );
}

#[test]
fn preserves_comments_inside_empty_if_expression_blocks() {
    let source = r#"
function main(): None {
value: Integer = if (true) {
    // keep me
} else {
    2;
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(
        formatted.contains("if (true) {\n        // keep me\n    } else {"),
        "{formatted}"
    );
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn preserves_comments_inside_empty_match_expression_blocks() {
    let source = r#"
function main(): None {
value: Integer = match (1) {
    1 => {
        // keep me
    },
    _ => {
        2;
    }
};
return None;
}
"#;
    let formatted = format_source(source).must("format succeeds");
    assert!(formatted.contains("// keep me"), "{formatted}");
    let tokens = tokenize(&formatted).must("formatted output should lex");
    let mut parser = Parser::new(tokens);
    parser.parse_program().must("formatted output should parse");
}

#[test]
fn normalizes_lone_cr_inside_block_comments() {
    let source = "/*\r * banner\r */\rpackage demo;\rfunction main(): None { return None; }\r";
    let formatted = format_source(source).must("format succeeds");
    assert!(!formatted.contains('\r'), "{formatted:?}");
    assert!(formatted.contains("/*\n * banner\n */"), "{formatted}");
}
