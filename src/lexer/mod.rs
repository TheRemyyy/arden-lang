//! Arden Lexer - Tokenizes source code using Logos
//!
//! Production-ready lexer with full language support

use logos::Logos;

fn remainder_has_interpolation_close(remainder: &str) -> bool {
    let mut depth = 1usize;
    let mut escape_active = false;
    let mut nested_quote: Option<char> = None;

    for ch in remainder.chars() {
        if escape_active {
            escape_active = false;
            continue;
        }

        if let Some(active_quote) = nested_quote {
            match ch {
                '\\' => escape_active = true,
                current if current == active_quote => nested_quote = None,
                _ => {}
            }
            continue;
        }

        match ch {
            '\\' => escape_active = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return true;
                }
            }
            '"' | '\'' => nested_quote = Some(ch),
            _ => {}
        }
    }

    false
}

fn can_start_nested_interpolation_string(remainder: &str, quote: char) -> bool {
    let mut escape_active = false;

    for (idx, ch) in remainder.char_indices() {
        if escape_active {
            escape_active = false;
            continue;
        }

        match ch {
            '\\' => escape_active = true,
            current if current == quote => {
                return remainder_has_interpolation_close(&remainder[idx + ch.len_utf8()..]);
            }
            _ => {}
        }
    }

    false
}

fn lex_string_literal<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> Option<&'src str> {
    let mut interpolation_depth = 0usize;
    let mut escape_active = false;
    let mut nested_quote: Option<char> = None;

    for (idx, ch) in lex.remainder().char_indices() {
        if escape_active {
            escape_active = false;
            continue;
        }

        if let Some(active_quote) = nested_quote {
            match ch {
                '\\' => escape_active = true,
                current if current == active_quote => nested_quote = None,
                _ => {}
            }
            continue;
        }

        match ch {
            '\\' => escape_active = true,
            '{' => interpolation_depth += 1,
            '}' if interpolation_depth > 0 => interpolation_depth -= 1,
            '"' if interpolation_depth == 0 => {
                lex.bump(idx + ch.len_utf8());
                let slice = lex.slice();
                return Some(&slice[1..slice.len() - 1]);
            }
            '"' | '\'' if interpolation_depth > 0 => {
                if can_start_nested_interpolation_string(
                    &lex.remainder()[idx + ch.len_utf8()..],
                    ch,
                ) {
                    nested_quote = Some(ch);
                } else {
                    lex.bump(idx + ch.len_utf8());
                    let slice = lex.slice();
                    return Some(&slice[1..slice.len() - 1]);
                }
            }
            _ => {}
        }
    }

    None
}

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//[^\r\n]*")]
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
    #[token("%=")]
    PercentEq,

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

    #[token("\"", lex_string_literal)]
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
        match source.find(['\n', '\r']) {
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
