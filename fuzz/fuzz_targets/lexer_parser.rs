#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/ast.rs"]
mod ast;
#[path = "../../src/lexer.rs"]
mod lexer;
#[path = "../../src/parser.rs"]
mod parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        if let Ok(tokens) = lexer::tokenize(source) {
            let mut parser = parser::Parser::new(tokens);
            let _ = parser.parse_program();
        }
    }
});
