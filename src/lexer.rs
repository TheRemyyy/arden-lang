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

    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(&s[1..s.len()-1])
    })]
    String(&'src str),

    #[regex(r"'([^'\\]|\\.)'", |lex| {
        let s = lex.slice();
        s.chars().nth(1)
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
    let lexer = Token::lexer(source);
    let mut tokens = Vec::new();

    for (token, span) in lexer.spanned() {
        match token {
            Ok(t) => tokens.push((t, span)),
            Err(_) => {
                let snippet: String = source[span.clone()].chars().take(20).collect();
                return Err(format!("Unknown token at {}: '{}'", span.start, snippet));
            }
        }
    }

    Ok(tokens)
}
