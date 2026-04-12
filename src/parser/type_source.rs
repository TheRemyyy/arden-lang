use crate::ast::Type;

use super::{ParseError, Parser};

pub fn parse_type_source(source: &str) -> Result<Type, ParseError> {
    let tokens = crate::lexer::tokenize(source).map_err(|e| ParseError::new(e, 0..0))?;
    let mut parser = Parser::new(tokens);
    let ty = parser.parse_type()?;
    if !parser.is_at_end() {
        return Err(ParseError::new(
            format!("Unexpected trailing tokens in type: {:?}", parser.current()),
            parser.current_span(),
        ));
    }
    Ok(ty)
}
