//! Apex Lexer - Tokenizes source code using Logos
//!
//! Production-ready lexer with full language support

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"/\*([^*]|\*[^/])*\*/")]
pub enum Token<'src> {
    // Keywords
    #[token("function")]
    Function,
    #[token("class")]
    Class,
    #[token("interface")]
    Interface,
    #[token("enum")]
    Enum,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("return")]
    Return,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("match")]
    Match,
    #[token("mut")]
    Mut,
    #[token("let")]
    Let,
    #[token("import")]
    Import,
    #[token("package")]
    Package,
    #[token("extern")]
    Extern,

    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("None")]
    None,
    #[token("this")]
    This,
    #[token("constructor")]
    Constructor,
    #[token("destructor")]
    Destructor,
    #[token("public")]
    Public,
    #[token("private")]
    Private,
    #[token("protected")]
    Protected,
    #[token("async")]
    Async,
    #[token("await")]
    Await,
    #[token("module")]
    Module,
    #[token("extends")]
    Extends,
    #[token("implements")]
    Implements,
    #[token("require")]
    Require,
    #[token("owned")]
    Owned,
    #[token("borrow")]
    Borrow,
    #[token("static")]
    Static,
    #[token("super")]
    Super,
    #[token("Self")]
    SelfType,
    #[token("as")]
    As,
    #[token("is")]
    Is,
    #[token("typeof")]
    TypeOf,

    // Types
    #[token("Integer")]
    TyInteger,
    #[token("Float")]
    TyFloat,
    #[token("Boolean")]
    TyBoolean,
    #[token("String")]
    TyString,
    #[token("Char")]
    TyChar,

    // Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("=")]
    Eq,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token("<=")]
    LtEq,
    #[token(">")]
    Gt,
    #[token(">=")]
    GtEq,
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Not,
    #[token("=>")]
    FatArrow,
    #[token("->")]
    Arrow,
    #[token("?")]
    Question,
    #[token("&")]
    Ampersand,
    #[token("|")]
    Pipe,
    #[token("@")]
    At,
    #[token("...")]
    Ellipsis,
    #[token("..=")]
    DotDotEq,
    #[token("..")]
    DotDot,
    #[token("::")]
    ColonColon,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token(".")]
    Dot,

    // Literals
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Integer(i64),

    #[regex(r"[0-9]+\.[0-9]+", |lex| {
        lex.slice()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
    })]
    Float(f64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(&s[1..s.len()-1])
    })]
    String(&'src str),

    #[regex(r"'([^'\\]|\\.)'", |lex| {
        let s = lex.slice();
        let inner = &s[1..s.len() - 1];
        if let Some(escaped) = inner.strip_prefix('\\') {
            let ch = escaped.chars().next()?;
            Some(match ch {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                _ => return None,
            })
        } else {
            inner.chars().next()
        }
    })]
    Char(char),

    // Identifier
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice())]
    Ident(&'src str),
}

impl<'src> std::fmt::Display for Token<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Function => write!(f, "function"),
            Token::Class => write!(f, "class"),
            Token::Ident(s) => write!(f, "{}", s),
            Token::Integer(n) => write!(f, "{}", n),
            Token::Float(n) => write!(f, "{}", n),
            Token::String(s) => write!(f, "\"{}\"", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

/// Tokenize source code
pub fn tokenize(source: &str) -> Result<Vec<(Token<'_>, std::ops::Range<usize>)>, String> {
    let (source, span_offset) = if source.starts_with("#!") {
        match source.find('\n') {
            Some(pos) => (&source[pos..], pos),
            None => ("", source.len()),
        }
    } else {
        (source, 0)
    };

    let lexer = Token::lexer(source);
    let mut tokens = Vec::new();

    for (token, span) in lexer.spanned() {
        let absolute_span = (span.start + span_offset)..(span.end + span_offset);
        match token {
            Ok(t) => tokens.push((t, absolute_span)),
            Err(_) => {
                let snippet = &source[span.clone()];
                if snippet.chars().all(|ch| ch.is_ascii_digit()) {
                    return Err(format!(
                        "Invalid integer literal at {}: '{}'",
                        absolute_span.start, snippet
                    ));
                }
                if snippet.contains('.')
                    && snippet.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
                {
                    return Err(format!(
                        "Invalid float literal at {}: '{}'",
                        absolute_span.start, snippet
                    ));
                }
                let display_snippet: String = snippet.chars().take(20).collect();
                return Err(format!(
                    "Unknown token at {}: '{}'",
                    absolute_span.start, display_snippet
                ));
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::{tokenize, Token};

    fn token_kinds(source: &str) -> Vec<Token<'_>> {
        tokenize(source)
            .expect("tokenization succeeds")
            .into_iter()
            .map(|(token, _)| token)
            .collect()
    }

    #[test]
    fn skips_unix_shebang_line() {
        let tokens = tokenize("#!/usr/bin/env apex\nfunction main(): None { return None; }")
            .expect("tokenization succeeds");
        assert!(matches!(tokens.first(), Some((Token::Function, _))));
    }

    #[test]
    fn preserves_absolute_spans_after_shebang() {
        let source = "#!/usr/bin/env apex\nfunction main(): None { return None; }";
        let tokens = tokenize(source).expect("tokenization succeeds");
        let (token, span) = tokens.first().expect("function token should exist");
        assert!(matches!(token, Token::Function));
        assert_eq!(&source[span.clone()], "function");
        assert_eq!(span.start, source.find("function").expect("function keyword"));
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
        let err = tokenize("/* unterminated comment")
            .expect_err("unterminated block comment should fail");
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
        let tokens = token_kinds("a += b -= c *= d /= e .. f ..= g ... h :: i -> j => k");
        assert!(matches!(tokens[0], Token::Ident("a")));
        assert!(matches!(tokens[1], Token::PlusEq));
        assert!(matches!(tokens[3], Token::MinusEq));
        assert!(matches!(tokens[5], Token::StarEq));
        assert!(matches!(tokens[7], Token::SlashEq));
        assert!(matches!(tokens[9], Token::DotDot));
        assert!(matches!(tokens[11], Token::DotDotEq));
        assert!(matches!(tokens[13], Token::Ellipsis));
        assert!(matches!(tokens[15], Token::ColonColon));
        assert!(matches!(tokens[17], Token::Arrow));
        assert!(matches!(tokens[19], Token::FatArrow));
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
}
