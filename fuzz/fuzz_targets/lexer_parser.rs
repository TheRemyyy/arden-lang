#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/ast/mod.rs"]
mod ast;
#[cfg(test)]
#[path = "../../src/formatter/mod.rs"]
mod formatter;
#[path = "../../src/lexer/mod.rs"]
mod lexer;
#[path = "../../src/parser/mod.rs"]
mod parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        if let Ok(tokens) = lexer::tokenize(source) {
            let mut parser = parser::Parser::new(tokens);
            if let Err(error) = parser.parse_program() {
                let _ = error.message.len();
                let _ = error.span.start;
            }
        }

        let _ = parser::parse_type_source(source);
    }
});
