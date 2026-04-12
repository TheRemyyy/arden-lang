use crate::ast::Type;
use crate::lexer::Token;

use super::{ParseError, ParseResult, Parser};

impl<'src> Parser<'src> {
    pub(super) fn builtin_generic_arity(name: &str) -> Option<usize> {
        match name {
            "Option" | "List" | "Set" | "Box" | "Rc" | "Arc" | "Ptr" | "Task" | "Range" => Some(1),
            "Result" | "Map" => Some(2),
            _ => None,
        }
    }

    pub(super) fn parse_type_arg_list(&mut self) -> ParseResult<Vec<Type>> {
        let list_span = self.current_span();
        let mut type_args = Vec::new();
        if self.check(&Token::Gt) {
            return Err(ParseError::new(
                "Generic type argument list cannot be empty",
                list_span,
            ));
        }

        loop {
            type_args.push(self.parse_type()?);
            if self.check(&Token::Gt) {
                break;
            }
            self.eat(&Token::Comma)?;
            if self.check(&Token::Gt) {
                return Err(ParseError::new(
                    "Trailing comma is not allowed in generic type arguments",
                    self.current_span(),
                ));
            }
        }

        Ok(type_args)
    }

    pub(super) fn parse_type_list(
        &mut self,
        empty_message: &str,
        trailing_comma_message: &str,
        terminator: &Token,
    ) -> ParseResult<Vec<Type>> {
        let list_span = self.current_span();
        let mut items = Vec::new();
        if self.check(terminator) {
            return Err(ParseError::new(empty_message, list_span));
        }

        loop {
            items.push(self.parse_type()?);
            if self.check(terminator) {
                break;
            }
            self.eat(&Token::Comma)?;
            if self.check(terminator) {
                return Err(ParseError::new(trailing_comma_message, self.current_span()));
            }
        }

        Ok(items)
    }

    pub(super) fn finish_named_type(
        &self,
        name: String,
        type_args: Vec<Type>,
        span: std::ops::Range<usize>,
    ) -> ParseResult<Type> {
        if let Some(expected_arity) = Self::builtin_generic_arity(&name) {
            if type_args.len() != expected_arity {
                let plural = if expected_arity == 1 { "" } else { "s" };
                return Err(ParseError::new(
                    format!(
                        "Built-in type '{}' expects {} type argument{}, found {}",
                        name,
                        expected_arity,
                        plural,
                        type_args.len()
                    ),
                    span,
                ));
            }
        }

        Ok(match (name.as_str(), type_args.len()) {
            ("Option", 1) => {
                let mut iter = type_args.into_iter();
                let Some(inner) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Option' is missing its type argument",
                        span,
                    ));
                };
                Type::Option(Box::new(inner))
            }
            ("Result", 2) => {
                let mut iter = type_args.into_iter();
                let Some(ok) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Result' is missing its first type argument",
                        span.clone(),
                    ));
                };
                let Some(err) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Result' is missing its second type argument",
                        span,
                    ));
                };
                Type::Result(Box::new(ok), Box::new(err))
            }
            ("List", 1) => {
                let mut iter = type_args.into_iter();
                let Some(inner) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'List' is missing its type argument",
                        span,
                    ));
                };
                Type::List(Box::new(inner))
            }
            ("Map", 2) => {
                let mut iter = type_args.into_iter();
                let Some(key) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Map' is missing its first type argument",
                        span.clone(),
                    ));
                };
                let Some(value) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Map' is missing its second type argument",
                        span,
                    ));
                };
                Type::Map(Box::new(key), Box::new(value))
            }
            ("Set", 1) => {
                let mut iter = type_args.into_iter();
                let Some(inner) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Set' is missing its type argument",
                        span,
                    ));
                };
                Type::Set(Box::new(inner))
            }
            ("Box", 1) => {
                let mut iter = type_args.into_iter();
                let Some(inner) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Box' is missing its type argument",
                        span,
                    ));
                };
                Type::Box(Box::new(inner))
            }
            ("Rc", 1) => {
                let mut iter = type_args.into_iter();
                let Some(inner) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Rc' is missing its type argument",
                        span,
                    ));
                };
                Type::Rc(Box::new(inner))
            }
            ("Arc", 1) => {
                let mut iter = type_args.into_iter();
                let Some(inner) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Arc' is missing its type argument",
                        span,
                    ));
                };
                Type::Arc(Box::new(inner))
            }
            ("Ptr", 1) => {
                let mut iter = type_args.into_iter();
                let Some(inner) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Ptr' is missing its type argument",
                        span,
                    ));
                };
                Type::Ptr(Box::new(inner))
            }
            ("Task", 1) => {
                let mut iter = type_args.into_iter();
                let Some(inner) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Task' is missing its type argument",
                        span,
                    ));
                };
                Type::Task(Box::new(inner))
            }
            ("Range", 1) => {
                let mut iter = type_args.into_iter();
                let Some(inner) = iter.next() else {
                    return Err(ParseError::new(
                        "Built-in type 'Range' is missing its type argument",
                        span,
                    ));
                };
                Type::Range(Box::new(inner))
            }
            _ => Type::Generic(name, type_args),
        })
    }
}
