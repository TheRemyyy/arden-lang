use crate::lexer::{tokenize, Token};

fn token_kinds(source: &str) -> Vec<Token<'_>> {
    tokenize(source)
        .expect("tokenization succeeds")
        .into_iter()
        .map(|(token, _)| token)
        .collect()
}

#[test]
fn skips_unix_shebang_line() {
    let tokens = tokenize("#!/usr/bin/env arden\nfunction main(): None { return None; }")
        .expect("tokenization succeeds");
    assert!(matches!(tokens.first(), Some((Token::Function, _))));
}

#[test]
fn preserves_absolute_spans_after_shebang() {
    let source = "#!/usr/bin/env arden\nfunction main(): None { return None; }";
    let tokens = tokenize(source).expect("tokenization succeeds");
    let (token, span) = tokens.first().expect("function token should exist");
    assert!(matches!(token, Token::Function));
    assert_eq!(&source[span.clone()], "function");
    assert_eq!(
        span.start,
        source.find("function").expect("function keyword")
    );
}

#[test]
fn decodes_escaped_char_literals() {
    let tokens = tokenize("'\\n' '\\t' '\\\\' '\\''").expect("tokenization succeeds");
    assert!(matches!(tokens[0].0, Token::Char('\n')));
    assert!(matches!(tokens[1].0, Token::Char('\t')));
    assert!(matches!(tokens[2].0, Token::Char('\\')));
    assert!(matches!(tokens[3].0, Token::Char('\'')));
}

#[test]
fn malformed_lexer_corpus_never_panics() {
    let malformed_cases = [
        "\"unterminated",
        "'unterminated",
        "/* unterminated comment",
        "'ab'",
        "''",
        "\"bad \\q escape\"",
        "@@@ ### $$$",
        "function main(): None { s: String = \"{\"; }",
    ];

    for source in malformed_cases {
        let result = std::panic::catch_unwind(|| tokenize(source));
        assert!(
            result.is_ok(),
            "lexer panicked on malformed input: {source}"
        );
    }
}

#[test]
fn rejects_unterminated_string_literal() {
    let err = tokenize("\"unterminated").expect_err("unterminated string should fail");
    assert!(err.contains("Unknown token"), "{err}");
}

#[test]
fn rejects_invalid_char_escape_sequence() {
    let err = tokenize("'\\q'").expect_err("invalid char escape should fail");
    assert!(err.contains("Unknown token"), "{err}");
}

#[test]
fn rejects_unterminated_block_comment() {
    let err =
        tokenize("/* unterminated comment").expect_err("unterminated block comment should fail");
    assert!(err.contains("Unknown token"), "{err}");
}

#[test]
fn rejects_overflowing_integer_literal_with_specific_error() {
    let err = tokenize("9999999999999999999999999999999999999999")
        .expect_err("overflowing integer literal should fail");
    assert!(err.contains("Invalid integer literal"), "{err}");
}

#[test]
fn rejects_overflowing_float_literal_with_specific_error() {
    let source = format!("{}.0", "9".repeat(500));
    let err = tokenize(&source).expect_err("overflowing float literal should fail");
    assert!(err.contains("Invalid float literal"), "{err}");
}

#[test]
fn tokenizes_tricky_operator_corpus() {
    let tokens = token_kinds("a += b -= c *= d /= e %= f .. g ..= h ... i :: j -> k => l");
    assert!(matches!(tokens[0], Token::Ident("a")));
    assert!(matches!(tokens[1], Token::PlusEq));
    assert!(matches!(tokens[3], Token::MinusEq));
    assert!(matches!(tokens[5], Token::StarEq));
    assert!(matches!(tokens[7], Token::SlashEq));
    assert!(matches!(tokens[9], Token::PercentEq));
    assert!(matches!(tokens[11], Token::DotDot));
    assert!(matches!(tokens[13], Token::DotDotEq));
    assert!(matches!(tokens[15], Token::Ellipsis));
    assert!(matches!(tokens[17], Token::ColonColon));
    assert!(matches!(tokens[19], Token::Arrow));
    assert!(matches!(tokens[21], Token::FatArrow));
}

#[test]
fn tokenizes_string_and_char_escape_corpus() {
    let tokens = token_kinds(r#""line\n\t\"quote\"\\\{\}" '\n' '\t' '\\' '\''"#);
    assert!(matches!(tokens[0], Token::String(_)));
    assert!(matches!(tokens[1], Token::Char('\n')));
    assert!(matches!(tokens[2], Token::Char('\t')));
    assert!(matches!(tokens[3], Token::Char('\\')));
    assert!(matches!(tokens[4], Token::Char('\'')));
}

#[test]
fn tokenizes_string_interpolation_with_nested_string_literal() {
    let tokens = tokenize(r#"s: String = "{m["x"]}";"#).expect("tokenization succeeds");
    assert!(matches!(tokens[0].0, Token::Ident("s")));
    assert!(matches!(tokens[1].0, Token::Colon));
    assert!(matches!(tokens[2].0, Token::TyString));
    assert!(matches!(tokens[3].0, Token::Eq));
    assert!(matches!(tokens[4].0, Token::String("{m[\"x\"]}")));
    assert!(matches!(tokens[5].0, Token::Semi));
}
