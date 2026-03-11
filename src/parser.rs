//! Apex Parser - Recursive descent parser
//!
//! Production-ready parser with full language support

use crate::ast::*;
use crate::lexer::Token;
use std::collections::HashSet;

pub struct Parser<'src> {
    tokens: Vec<(Token<'src>, std::ops::Range<usize>)>,
    pos: usize,
    known_functions: HashSet<String>,
    known_types: HashSet<String>,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: std::ops::Range<usize>,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: std::ops::Range<usize>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

type ParseResult<T> = Result<T, ParseError>;

impl<'src> Parser<'src> {
    fn builtin_generic_arity(name: &str) -> Option<usize> {
        match name {
            "Option" | "List" | "Set" | "Box" | "Rc" | "Arc" | "Ptr" | "Task" | "Range" => Some(1),
            "Result" | "Map" => Some(2),
            _ => None,
        }
    }

    fn parse_type_arg_list(&mut self) -> ParseResult<Vec<Type>> {
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

    fn parse_type_list(
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

    fn finish_named_type(
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
            ("Option", 1) => Type::Option(Box::new(type_args.into_iter().next().unwrap())),
            ("Result", 2) => {
                let mut iter = type_args.into_iter();
                Type::Result(
                    Box::new(iter.next().unwrap()),
                    Box::new(iter.next().unwrap()),
                )
            }
            ("List", 1) => Type::List(Box::new(type_args.into_iter().next().unwrap())),
            ("Map", 2) => {
                let mut iter = type_args.into_iter();
                Type::Map(
                    Box::new(iter.next().unwrap()),
                    Box::new(iter.next().unwrap()),
                )
            }
            ("Set", 1) => Type::Set(Box::new(type_args.into_iter().next().unwrap())),
            ("Box", 1) => Type::Box(Box::new(type_args.into_iter().next().unwrap())),
            ("Rc", 1) => Type::Rc(Box::new(type_args.into_iter().next().unwrap())),
            ("Arc", 1) => Type::Arc(Box::new(type_args.into_iter().next().unwrap())),
            ("Ptr", 1) => Type::Ptr(Box::new(type_args.into_iter().next().unwrap())),
            ("Task", 1) => Type::Task(Box::new(type_args.into_iter().next().unwrap())),
            ("Range", 1) => Type::Range(Box::new(type_args.into_iter().next().unwrap())),
            _ => Type::Generic(name, type_args),
        })
    }

    pub fn new(tokens: Vec<(Token<'src>, std::ops::Range<usize>)>) -> Self {
        let (known_functions, known_types) = Self::scan_decl_names(&tokens);
        Self {
            tokens,
            pos: 0,
            known_functions,
            known_types,
        }
    }

    fn scan_decl_names(
        tokens: &[(Token<'src>, std::ops::Range<usize>)],
    ) -> (HashSet<String>, HashSet<String>) {
        let mut known_functions = HashSet::new();
        let mut known_types = HashSet::new();

        let mut i = 0usize;
        while i < tokens.len() {
            match &tokens[i].0 {
                Token::Function => {
                    if let Some((Token::Ident(name), _)) = tokens.get(i + 1) {
                        known_functions.insert((*name).to_string());
                    }
                }
                Token::Async => {
                    if let (Some((Token::Function, _)), Some((Token::Ident(name), _))) =
                        (tokens.get(i + 1), tokens.get(i + 2))
                    {
                        known_functions.insert((*name).to_string());
                    }
                }
                Token::Extern => {
                    // extern(...) function name ...
                    let mut j = i + 1;
                    while j + 1 < tokens.len() {
                        if matches!(tokens[j].0, Token::Function) {
                            if let Token::Ident(name) = &tokens[j + 1].0 {
                                known_functions.insert((*name).to_string());
                            }
                            break;
                        }
                        if matches!(tokens[j].0, Token::Semi | Token::LBrace) {
                            break;
                        }
                        j += 1;
                    }
                }
                Token::Class | Token::Enum | Token::Interface => {
                    if let Some((Token::Ident(name), _)) = tokens.get(i + 1) {
                        known_types.insert((*name).to_string());
                    }
                }
                _ => {}
            }
            i += 1;
        }

        (known_functions, known_types)
    }

    // === Utility Methods ===

    fn current(&self) -> Option<&Token<'src>> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn peek_token(&self, offset: usize) -> Option<&Token<'src>> {
        self.tokens.get(self.pos + offset).map(|(t, _)| t)
    }

    fn current_span(&self) -> std::ops::Range<usize> {
        self.tokens
            .get(self.pos)
            .map(|(_, s)| s.clone())
            .unwrap_or(0..0)
    }

    fn advance(&mut self) -> Option<Token<'src>> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].0.clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    fn check(&self, token: &Token) -> bool {
        self.current()
            .map(|t| std::mem::discriminant(t) == std::mem::discriminant(token))
            .unwrap_or(false)
    }

    fn eat(&mut self, expected: &Token) -> ParseResult<()> {
        if self.check(expected) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::new(
                format!("Expected {:?}, found {:?}", expected, self.current()),
                self.current_span(),
            ))
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Format a Type as a string for use in generic type names
    #[allow(clippy::only_used_in_recursion)]
    fn format_type(&self, ty: &Type) -> String {
        match ty {
            Type::Integer => "Integer".to_string(),
            Type::Float => "Float".to_string(),
            Type::Boolean => "Boolean".to_string(),
            Type::String => "String".to_string(),
            Type::Char => "Char".to_string(),
            Type::None => "None".to_string(),
            Type::Named(name) => name.clone(),
            Type::Option(inner) => format!("Option<{}>", self.format_type(inner)),
            Type::Result(ok, err) => format!(
                "Result<{}, {}>",
                self.format_type(ok),
                self.format_type(err)
            ),
            Type::List(inner) => format!("List<{}>", self.format_type(inner)),
            Type::Map(k, v) => format!("Map<{}, {}>", self.format_type(k), self.format_type(v)),
            Type::Set(inner) => format!("Set<{}>", self.format_type(inner)),
            Type::Ref(inner) => format!("&{}", self.format_type(inner)),
            Type::MutRef(inner) => format!("&mut {}", self.format_type(inner)),
            Type::Box(inner) => format!("Box<{}>", self.format_type(inner)),
            Type::Rc(inner) => format!("Rc<{}>", self.format_type(inner)),
            Type::Arc(inner) => format!("Arc<{}>", self.format_type(inner)),
            Type::Ptr(inner) => format!("Ptr<{}>", self.format_type(inner)),
            Type::Task(inner) => format!("Task<{}>", self.format_type(inner)),
            Type::Range(inner) => format!("Range<{}>", self.format_type(inner)),
            Type::Function(params, ret) => {
                let params_str = params
                    .iter()
                    .map(|p| self.format_type(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({}) -> {}", params_str, self.format_type(ret))
            }
            Type::Generic(name, args) => {
                let args_str = args
                    .iter()
                    .map(|a| self.format_type(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", name, args_str)
            }
        }
    }

    // === Parsing Methods ===

    pub fn parse_program(&mut self) -> ParseResult<Program> {
        let mut package = None;
        let mut declarations = Vec::new();

        // Parse optional package declaration at start
        if self.check(&Token::Package) {
            self.advance();

            // Parse qualified package name
            let mut pkg_parts = vec![self.parse_ident()?];
            while self.check(&Token::Dot) || self.check(&Token::DotDot) {
                if self.check(&Token::DotDot) {
                    return Err(ParseError::new(
                        "Package path cannot contain an empty segment",
                        self.current_span(),
                    ));
                }
                self.advance();
                if !matches!(self.current(), Some(Token::Ident(_))) {
                    return Err(ParseError::new(
                        "Package path cannot end with '.'",
                        self.current_span(),
                    ));
                }
                pkg_parts.push(self.parse_ident()?);
            }
            package = Some(pkg_parts.join("."));
            self.eat(&Token::Semi)?;
        }

        while !self.is_at_end() {
            let decl = self.parse_declaration()?;
            self.register_declaration_name(&decl.node);
            declarations.push(decl);
        }

        Ok(Program {
            package,
            declarations,
        })
    }

    fn parse_declaration(&mut self) -> ParseResult<Spanned<Decl>> {
        let start = self.current_span().start;

        // Parse attributes if present
        let attributes = self.parse_attributes()?;

        let decl = match self.current() {
            Some(Token::Function) | Some(Token::Async) => {
                Decl::Function(self.parse_function(attributes)?)
            }
            Some(Token::Extern) => Decl::Function(self.parse_extern_function(attributes)?),
            Some(Token::Public) | Some(Token::Private) | Some(Token::Protected) => {
                match self.peek_token(1) {
                    Some(Token::Function) | Some(Token::Async) => {
                        Decl::Function(self.parse_function(attributes)?)
                    }
                    Some(Token::Extern) => Decl::Function(self.parse_extern_function(attributes)?),
                    Some(Token::Class) => Decl::Class(self.parse_class(attributes)?),
                    Some(Token::Enum) => Decl::Enum(self.parse_enum(attributes)?),
                    Some(Token::Interface) => Decl::Interface(self.parse_interface(attributes)?),
                    Some(Token::Module) => {
                        return Err(ParseError::new(
                            "Visibility modifiers are not supported on modules",
                            self.current_span(),
                        ));
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!(
                                "Visibility modifier must be followed by a declaration, found {:?}",
                                self.peek_token(1)
                            ),
                            self.current_span(),
                        ));
                    }
                }
            }
            Some(Token::Class) => Decl::Class(self.parse_class(attributes)?),
            Some(Token::Enum) => Decl::Enum(self.parse_enum(attributes)?),
            Some(Token::Interface) => Decl::Interface(self.parse_interface(attributes)?),
            Some(Token::Module) => Decl::Module(self.parse_module(attributes)?),
            Some(Token::Import) => Decl::Import(self.parse_import()?),
            Some(Token::Package) => {
                // Package is handled at program level, skip here
                return Err(ParseError::new(
                    "Package declaration must be at the beginning of the file".to_string(),
                    self.current_span(),
                ));
            }
            _ => {
                return Err(ParseError::new(
                    format!("Expected declaration, found {:?}", self.current()),
                    self.current_span(),
                ));
            }
        };

        let end = self.current_span().start;
        Ok(Spanned::new(decl, start..end))
    }

    fn register_declaration_name(&mut self, decl: &Decl) {
        match decl {
            Decl::Function(func) => {
                self.known_functions.insert(func.name.clone());
            }
            Decl::Class(class) => {
                self.known_types.insert(class.name.clone());
            }
            Decl::Enum(enum_decl) => {
                self.known_types.insert(enum_decl.name.clone());
            }
            Decl::Interface(interface_decl) => {
                self.known_types.insert(interface_decl.name.clone());
            }
            _ => {}
        }
    }

    /// Parse attributes (e.g., @Test, @Ignore)
    fn parse_attributes(&mut self) -> ParseResult<Vec<Attribute>> {
        let mut attributes = Vec::new();

        while self.check(&Token::At) {
            self.advance(); // consume @
            let attr_name = self.parse_ident()?;

            let attr = match attr_name.as_str() {
                "Test" => Attribute::Test,
                "Ignore" => {
                    // Optional reason: @Ignore("reason")
                    let reason = if self.check(&Token::LParen) {
                        self.advance();
                        let reason_str = self.parse_string_literal()?;
                        self.eat(&Token::RParen)?;
                        Some(reason_str)
                    } else {
                        None
                    };
                    Attribute::Ignore(reason)
                }
                "Before" => Attribute::Before,
                "After" => Attribute::After,
                "BeforeAll" => Attribute::BeforeAll,
                "AfterAll" => Attribute::AfterAll,
                "Pure" => Attribute::Pure,
                "Io" => Attribute::EffectIo,
                "Net" => Attribute::EffectNet,
                "Alloc" => Attribute::EffectAlloc,
                "Unsafe" => Attribute::EffectUnsafe,
                "Thread" => Attribute::EffectThread,
                "Any" => Attribute::EffectAny,
                _ => {
                    return Err(ParseError::new(
                        format!("Unknown attribute: @{}", attr_name),
                        self.current_span(),
                    ));
                }
            };

            attributes.push(attr);
        }

        Ok(attributes)
    }

    /// Parse a string literal
    fn parse_string_literal(&mut self) -> ParseResult<String> {
        match self.current() {
            Some(Token::String(s)) => {
                let s = decode_escaped_string(s);
                self.advance();
                Ok(s)
            }
            _ => Err(ParseError::new(
                format!("Expected string literal, found {:?}", self.current()),
                self.current_span(),
            )),
        }
    }

    /// Parse visibility modifier if present
    fn parse_visibility(&mut self) -> Visibility {
        match self.current() {
            Some(Token::Public) => {
                self.advance();
                Visibility::Public
            }
            Some(Token::Private) => {
                self.advance();
                Visibility::Private
            }
            Some(Token::Protected) => {
                self.advance();
                Visibility::Protected
            }
            _ => Visibility::Public,
        }
    }

    /// Parse generic parameters: <T, U extends Comparable>
    fn parse_generic_params(&mut self) -> ParseResult<Vec<GenericParam>> {
        let mut params = Vec::new();
        if !self.check(&Token::Lt) {
            return Ok(params);
        }
        self.advance(); // eat '<'
        if self.check(&Token::Gt) {
            return Err(ParseError::new(
                "Generic parameter list cannot be empty",
                self.current_span(),
            ));
        }

        while !self.check(&Token::Gt) && !self.is_at_end() {
            let name = self.parse_ident()?;
            let mut bounds = Vec::new();

            if self.check(&Token::Extends) {
                self.advance();
                bounds.push(self.parse_ident()?);
                while self.check(&Token::Comma) && !self.check(&Token::Gt) {
                    // Check if next is another bound or next param
                    let saved_pos = self.pos;
                    self.advance();
                    if let Some(Token::Ident(_)) = self.current() {
                        let next_name = self.parse_ident()?;
                        // Check if this is a bound (next is comma or >) or a new param (next is extends or :)
                        if self.check(&Token::Extends) || self.check(&Token::Colon) {
                            // It's a new param, restore
                            self.pos = saved_pos;
                            break;
                        }
                        bounds.push(next_name);
                    } else {
                        self.pos = saved_pos;
                        break;
                    }
                }
            }

            params.push(GenericParam { name, bounds });

            if self.check(&Token::Comma) {
                self.advance();
                if self.check(&Token::Gt) {
                    return Err(ParseError::new(
                        "Trailing comma is not allowed in generic parameter lists",
                        self.current_span(),
                    ));
                }
            }
        }

        self.eat(&Token::Gt)?;
        Ok(params)
    }

    fn parse_function(&mut self, attributes: Vec<Attribute>) -> ParseResult<FunctionDecl> {
        let visibility = self.parse_visibility();

        let is_async = if self.check(&Token::Async) {
            self.advance();
            true
        } else {
            false
        };

        self.eat(&Token::Function)?;

        let name = self.parse_ident()?;
        let generic_params = self.parse_generic_params()?;
        self.eat(&Token::LParen)?;
        let params = self.parse_params()?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::Colon)?;
        let return_type = self.parse_type()?;
        self.eat(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.eat(&Token::RBrace)?;

        Ok(FunctionDecl {
            name,
            generic_params,
            params,
            is_variadic: false,
            extern_abi: None,
            extern_link_name: None,
            return_type,
            body,
            is_async,
            is_extern: false,
            visibility,
            attributes,
        })
    }

    fn parse_extern_params(&mut self) -> ParseResult<(Vec<Parameter>, bool)> {
        let mut params = Vec::new();
        let mut is_variadic = false;

        while !self.check(&Token::RParen) && !self.is_at_end() {
            if self.check(&Token::Ellipsis) {
                self.advance();
                is_variadic = true;
                break;
            }

            let mode = if self.check(&Token::Owned) {
                self.advance();
                ParamMode::Owned
            } else if self.check(&Token::Borrow) {
                self.advance();
                if self.check(&Token::Mut) {
                    self.advance();
                    ParamMode::BorrowMut
                } else {
                    ParamMode::Borrow
                }
            } else {
                ParamMode::Owned
            };

            let mutable = if self.check(&Token::Mut) {
                self.advance();
                true
            } else {
                false
            };

            let name = self.parse_ident()?;
            self.eat(&Token::Colon)?;
            let ty = self.parse_type()?;
            params.push(Parameter {
                name,
                ty,
                mutable,
                mode,
            });

            if self.check(&Token::Comma) {
                self.advance();
                if self.check(&Token::RParen) {
                    return Err(ParseError::new(
                        "Trailing comma is not allowed in extern parameter lists",
                        self.current_span(),
                    ));
                }
            } else {
                break;
            }
        }

        Ok((params, is_variadic))
    }

    fn parse_extern_function(&mut self, attributes: Vec<Attribute>) -> ParseResult<FunctionDecl> {
        let visibility = self.parse_visibility();
        self.eat(&Token::Extern)?;
        let (extern_abi, extern_link_name) = if self.check(&Token::LParen) {
            self.parse_extern_options()?
        } else {
            (Some("c".to_string()), None)
        };
        self.eat(&Token::Function)?;

        let name = self.parse_ident()?;
        let generic_params = self.parse_generic_params()?;
        if !generic_params.is_empty() {
            return Err(ParseError::new(
                "extern functions with generic parameters are not supported",
                self.current_span(),
            ));
        }

        self.eat(&Token::LParen)?;
        let (params, is_variadic) = self.parse_extern_params()?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::Colon)?;
        let return_type = self.parse_type()?;
        self.eat(&Token::Semi)?;

        Ok(FunctionDecl {
            name,
            generic_params,
            params,
            is_variadic,
            extern_abi,
            extern_link_name,
            return_type,
            body: vec![],
            is_async: false,
            is_extern: true,
            visibility,
            attributes,
        })
    }

    fn parse_extern_options(&mut self) -> ParseResult<(Option<String>, Option<String>)> {
        self.eat(&Token::LParen)?;
        if self.check(&Token::RParen) {
            return Err(ParseError::new(
                "extern(...) options cannot be empty",
                self.current_span(),
            ));
        }
        let abi_ident = self.parse_ident()?;
        let abi = match abi_ident.as_str() {
            "c" | "system" => abi_ident,
            _ => {
                return Err(ParseError::new(
                    format!(
                        "Unsupported extern ABI '{}'. Supported: c, system",
                        abi_ident
                    ),
                    self.current_span(),
                ));
            }
        };

        let mut link_name = None;
        if self.check(&Token::Comma) {
            self.advance();
            if self.check(&Token::RParen) {
                return Err(ParseError::new(
                    "Trailing comma is not allowed in extern options",
                    self.current_span(),
                ));
            }
            link_name = Some(self.parse_string_literal()?);
            if self.check(&Token::Comma) {
                return Err(ParseError::new(
                    "extern(...) accepts at most ABI and optional link name",
                    self.current_span(),
                ));
            }
        }
        self.eat(&Token::RParen)?;
        Ok((Some(abi), link_name))
    }

    fn parse_class(&mut self, _attributes: Vec<Attribute>) -> ParseResult<ClassDecl> {
        let visibility = self.parse_visibility();

        self.eat(&Token::Class)?;
        let name = self.parse_ident()?;
        let generic_params = self.parse_generic_params()?;

        // Parse extends clause
        let extends = if self.check(&Token::Extends) {
            self.advance();
            let parent = self.parse_ident()?;
            if self.check(&Token::Comma) {
                return Err(ParseError::new(
                    "Class extends clause accepts exactly one base class",
                    self.current_span(),
                ));
            }
            Some(parent)
        } else {
            None
        };

        // Parse implements clause
        let mut implements = Vec::new();
        if self.check(&Token::Implements) {
            self.advance();
            if self.check(&Token::LBrace) {
                return Err(ParseError::new(
                    "implements list cannot be empty",
                    self.current_span(),
                ));
            }
            implements.push(self.parse_ident()?);
            while self.check(&Token::Comma) {
                self.advance();
                if self.check(&Token::LBrace) {
                    return Err(ParseError::new(
                        "Trailing comma is not allowed in implements lists",
                        self.current_span(),
                    ));
                }
                implements.push(self.parse_ident()?);
            }
        }

        self.eat(&Token::LBrace)?;

        let mut fields = Vec::new();
        let mut constructor = None;
        let mut destructor = None;
        let mut methods = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // Check for visibility modifier first
            let member_visibility = self.parse_visibility();

            match self.current() {
                Some(Token::Constructor) => {
                    if member_visibility != Visibility::Public {
                        return Err(ParseError::new(
                            "Visibility modifiers are not supported on constructors",
                            self.current_span(),
                        ));
                    }
                    constructor = Some(self.parse_constructor()?);
                }
                Some(Token::Destructor) => {
                    if member_visibility != Visibility::Public {
                        return Err(ParseError::new(
                            "Visibility modifiers are not supported on destructors",
                            self.current_span(),
                        ));
                    }
                    destructor = Some(self.parse_destructor()?);
                }
                Some(Token::Function) | Some(Token::Async) => {
                    let method_attrs = self.parse_attributes()?;
                    let mut method = self.parse_function(method_attrs)?;
                    method.visibility = member_visibility;
                    methods.push(method);
                }
                Some(Token::Mut) | Some(Token::Ident(_)) => {
                    let mut field = self.parse_field()?;
                    field.visibility = member_visibility;
                    fields.push(field);
                }
                _ => {
                    return Err(ParseError::new(
                        format!("Unexpected token in class: {:?}", self.current()),
                        self.current_span(),
                    ));
                }
            }
        }

        self.eat(&Token::RBrace)?;

        Ok(ClassDecl {
            name,
            generic_params,
            extends,
            implements,
            fields,
            constructor,
            destructor,
            methods,
            visibility,
        })
    }

    fn parse_destructor(&mut self) -> ParseResult<Destructor> {
        self.eat(&Token::Destructor)?;
        self.eat(&Token::LParen)?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.eat(&Token::RBrace)?;

        Ok(Destructor { body })
    }

    fn parse_field(&mut self) -> ParseResult<Field> {
        let mutable = if self.check(&Token::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.parse_ident()?;
        self.eat(&Token::Colon)?;
        let ty = self.parse_type()?;
        self.eat(&Token::Semi)?;

        Ok(Field {
            name,
            ty,
            mutable,
            visibility: Visibility::Private,
        })
    }

    fn parse_constructor(&mut self) -> ParseResult<Constructor> {
        self.eat(&Token::Constructor)?;
        self.eat(&Token::LParen)?;
        let params = self.parse_params()?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.eat(&Token::RBrace)?;

        Ok(Constructor { params, body })
    }

    fn parse_enum(&mut self, _attributes: Vec<Attribute>) -> ParseResult<EnumDecl> {
        let visibility = self.parse_visibility();

        self.eat(&Token::Enum)?;
        let name = self.parse_ident()?;
        let generic_params = self.parse_generic_params()?;
        self.eat(&Token::LBrace)?;

        let mut variants = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let variant_name = self.parse_ident()?;
            let mut fields = Vec::new();

            if self.check(&Token::LParen) {
                self.advance();
                while !self.check(&Token::RParen) {
                    // Check for named field: name: Type
                    let field_name = if let Some(Token::Ident(_)) = self.current() {
                        let name = self.parse_ident()?;
                        if self.check(&Token::Colon) {
                            self.advance();
                            Some(name)
                        } else {
                            // Not a named field, parse as type
                            // Need to parse the type starting from this identifier
                            let ty = self.parse_type_from_ident(&name)?;
                            fields.push(EnumField { name: None, ty });
                            if !self.check(&Token::RParen) {
                                self.eat(&Token::Comma)?;
                                if self.check(&Token::RParen) {
                                    return Err(ParseError::new(
                                        "Trailing comma is not allowed in enum field lists",
                                        self.current_span(),
                                    ));
                                }
                            }
                            continue;
                        }
                    } else {
                        None
                    };

                    let ty = self.parse_type()?;
                    fields.push(EnumField {
                        name: field_name,
                        ty,
                    });

                    if !self.check(&Token::RParen) {
                        self.eat(&Token::Comma)?;
                        if self.check(&Token::RParen) {
                            return Err(ParseError::new(
                                "Trailing comma is not allowed in enum field lists",
                                self.current_span(),
                            ));
                        }
                    }
                }
                self.eat(&Token::RParen)?;
            }

            variants.push(EnumVariant {
                name: variant_name,
                fields,
            });

            if self.check(&Token::Comma) {
                self.advance();
                if self.check(&Token::RBrace) {
                    return Err(ParseError::new(
                        "Trailing comma is not allowed in enum variant lists",
                        self.current_span(),
                    ));
                }
            }
        }

        self.eat(&Token::RBrace)?;

        Ok(EnumDecl {
            name,
            generic_params,
            variants,
            visibility,
        })
    }

    /// Parse type from already parsed identifier
    fn parse_type_from_ident(&mut self, name: &str) -> ParseResult<Type> {
        // Check for generic params
        if self.check(&Token::Lt) {
            let span = self.current_span();
            self.advance();
            let type_args = self.parse_type_arg_list()?;
            self.eat(&Token::Gt)?;
            self.finish_named_type(name.to_string(), type_args, span)
        } else {
            Ok(Type::Named(name.to_string()))
        }
    }

    fn parse_interface(&mut self, _attributes: Vec<Attribute>) -> ParseResult<InterfaceDecl> {
        let visibility = self.parse_visibility();

        self.eat(&Token::Interface)?;
        let name = self.parse_ident()?;
        let generic_params = self.parse_generic_params()?;

        // Parse extends clause for interface inheritance
        let mut extends = Vec::new();
        if self.check(&Token::Extends) {
            self.advance();
            if self.check(&Token::LBrace) {
                return Err(ParseError::new(
                    "interface extends list cannot be empty",
                    self.current_span(),
                ));
            }
            extends.push(self.parse_ident()?);
            while self.check(&Token::Comma) {
                self.advance();
                if self.check(&Token::LBrace) {
                    return Err(ParseError::new(
                        "Trailing comma is not allowed in interface extends lists",
                        self.current_span(),
                    ));
                }
                extends.push(self.parse_ident()?);
            }
        }

        self.eat(&Token::LBrace)?;

        let mut methods = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // Parse method signature: function name(params): ReturnType; or with default impl
            self.eat(&Token::Function)?;
            let method_name = self.parse_ident()?;
            self.eat(&Token::LParen)?;
            let params = self.parse_params()?;
            self.eat(&Token::RParen)?;
            self.eat(&Token::Colon)?;
            let return_type = self.parse_type()?;

            // Check for default implementation or semicolon
            let default_impl = if self.check(&Token::LBrace) {
                self.advance();
                let body = self.parse_block()?;
                self.eat(&Token::RBrace)?;
                Some(body)
            } else {
                self.eat(&Token::Semi)?;
                None
            };

            methods.push(InterfaceMethod {
                name: method_name,
                params,
                return_type,
                default_impl,
            });
        }

        self.eat(&Token::RBrace)?;

        Ok(InterfaceDecl {
            name,
            generic_params,
            extends,
            methods,
            visibility,
        })
    }

    fn parse_module(&mut self, _attributes: Vec<Attribute>) -> ParseResult<ModuleDecl> {
        self.eat(&Token::Module)?;
        let name = self.parse_ident()?;
        self.eat(&Token::LBrace)?;

        let mut declarations = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            declarations.push(self.parse_declaration()?);
        }

        self.eat(&Token::RBrace)?;

        Ok(ModuleDecl { name, declarations })
    }

    fn parse_import(&mut self) -> ParseResult<ImportDecl> {
        self.eat(&Token::Import)?;

        // Parse qualified path: utils.math.* or utils.math.factorial
        let mut path_parts = vec![self.parse_ident()?];

        // Handle dots for qualified names
        while self.check(&Token::Dot) || self.check(&Token::DotDot) {
            if self.check(&Token::DotDot) {
                return Err(ParseError::new(
                    "Import path cannot contain an empty segment",
                    self.current_span(),
                ));
            }
            self.advance();

            // Check for wildcard
            if self.check(&Token::Star) {
                self.advance();
                path_parts.push("*".to_string());
                break;
            }

            if !matches!(self.current(), Some(Token::Ident(_))) {
                return Err(ParseError::new(
                    "Import path cannot end with '.'",
                    self.current_span(),
                ));
            }
            path_parts.push(self.parse_ident()?);
        }

        let alias = if self.check(&Token::As) {
            self.advance();
            Some(self.parse_ident()?)
        } else {
            None
        };

        if alias.is_some() && path_parts.last().is_some_and(|p| p == "*") {
            return Err(ParseError::new(
                "Cannot use alias with wildcard import",
                self.current_span(),
            ));
        }

        self.eat(&Token::Semi)?;

        Ok(ImportDecl {
            path: path_parts.join("."),
            alias,
        })
    }

    fn parse_params(&mut self) -> ParseResult<Vec<Parameter>> {
        let mut params = Vec::new();

        while !self.check(&Token::RParen) && !self.is_at_end() {
            // Parse parameter mode: owned, borrow, borrow mut
            let mode = if self.check(&Token::Owned) {
                self.advance();
                ParamMode::Owned
            } else if self.check(&Token::Borrow) {
                self.advance();
                if self.check(&Token::Mut) {
                    self.advance();
                    ParamMode::BorrowMut
                } else {
                    ParamMode::Borrow
                }
            } else {
                ParamMode::Owned // Default
            };

            let mutable = if self.check(&Token::Mut) {
                self.advance();
                true
            } else {
                false
            };

            let name = self.parse_ident()?;
            self.eat(&Token::Colon)?;
            let ty = self.parse_type()?;

            params.push(Parameter {
                name,
                ty,
                mutable,
                mode,
            });

            if !self.check(&Token::RParen) {
                self.eat(&Token::Comma)?;
                if self.check(&Token::RParen) {
                    return Err(ParseError::new(
                        "Trailing comma is not allowed in parameter lists",
                        self.current_span(),
                    ));
                }
            }
        }

        Ok(params)
    }

    fn parse_type(&mut self) -> ParseResult<Type> {
        // Check for reference types
        if self.check(&Token::Ampersand) {
            self.advance();
            // Check for &mut
            if self.check(&Token::Mut) {
                self.advance();
                let inner = self.parse_type()?;
                return Ok(Type::MutRef(Box::new(inner)));
            } else {
                let inner = self.parse_type()?;
                return Ok(Type::Ref(Box::new(inner)));
            }
        }

        // Check for function type: (Type, Type) -> Type
        if self.check(&Token::LParen) {
            self.advance();
            let params = if self.check(&Token::RParen) {
                Vec::new()
            } else {
                self.parse_type_list(
                    "Function type parameter list cannot be empty after '('",
                    "Trailing comma is not allowed in function type parameters",
                    &Token::RParen,
                )?
            };
            self.eat(&Token::RParen)?;

            // In Apex, function types MUST have -> ReturnType
            self.eat(&Token::Arrow)?;
            let ret = self.parse_type()?;
            return Ok(Type::Function(params, Box::new(ret)));
        }

        let ty = match self.current() {
            Some(Token::TyInteger) => {
                self.advance();
                Type::Integer
            }
            Some(Token::TyFloat) => {
                self.advance();
                Type::Float
            }
            Some(Token::TyBoolean) => {
                self.advance();
                Type::Boolean
            }
            Some(Token::TyString) => {
                self.advance();
                Type::String
            }
            Some(Token::TyChar) => {
                self.advance();
                Type::Char
            }
            Some(Token::None) => {
                self.advance();
                Type::None
            }
            Some(Token::Ident(name)) => {
                let name = name.to_string();
                self.advance();

                // Check for generic params
                if self.check(&Token::Lt) {
                    let span = self.current_span();
                    self.advance();
                    let type_args = self.parse_type_arg_list()?;
                    self.eat(&Token::Gt)?;
                    self.finish_named_type(name, type_args, span)?
                } else {
                    Type::Named(name)
                }
            }
            _ => {
                return Err(ParseError::new(
                    format!("Expected type, found {:?}", self.current()),
                    self.current_span(),
                ));
            }
        };

        Ok(ty)
    }

    fn parse_block(&mut self) -> ParseResult<Block> {
        let mut stmts = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }

        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> ParseResult<Spanned<Stmt>> {
        let start = self.current_span().start;

        let stmt = match self.current() {
            Some(Token::Return) => self.parse_return()?,
            Some(Token::If) => self.parse_if()?,
            Some(Token::While) => self.parse_while()?,
            Some(Token::For) => self.parse_for()?,
            Some(Token::Break) => {
                self.advance();
                self.eat(&Token::Semi)?;
                Stmt::Break
            }
            Some(Token::Continue) => {
                self.advance();
                self.eat(&Token::Semi)?;
                Stmt::Continue
            }
            Some(Token::Match) => {
                let stmt = self.parse_match_stmt()?;
                if self.check(&Token::Semi) {
                    self.advance();
                }
                stmt
            }
            Some(Token::Mut) => self.parse_let(true)?,
            Some(Token::Ident(_)) => {
                // Could be variable declaration or expression
                self.parse_ident_stmt()?
            }
            _ => {
                let expr = self.parse_expr()?;
                // Check if this is an assignment
                if self.check(&Token::Eq) {
                    self.advance();
                    let value = self.parse_expr()?;
                    self.eat(&Token::Semi)?;
                    Stmt::Assign {
                        target: expr,
                        value,
                    }
                } else if let Some(op) = self.current().and_then(Self::compound_assign_binop) {
                    self.advance();
                    let rhs = self.parse_expr()?;
                    self.eat(&Token::Semi)?;
                    let value = Spanned::new(
                        Expr::Binary {
                            op,
                            left: Box::new(expr.clone()),
                            right: Box::new(rhs),
                        },
                        expr.span.start..self.current_span().start,
                    );
                    Stmt::Assign {
                        target: expr,
                        value,
                    }
                } else {
                    self.eat(&Token::Semi)?;
                    Stmt::Expr(expr)
                }
            }
        };

        let end = self.current_span().start;
        Ok(Spanned::new(stmt, start..end))
    }

    fn parse_ident_stmt(&mut self) -> ParseResult<Stmt> {
        // Look ahead to determine if this is a declaration or expression
        let name = self.parse_ident()?;
        let start = self.current_span().start;

        if self.check(&Token::Colon) {
            // Variable declaration: name: Type = value;
            self.advance();
            let ty = self.parse_type()?;
            self.eat(&Token::Eq)?;
            let value = self.parse_expr()?;
            self.eat(&Token::Semi)?;
            Ok(Stmt::Let {
                name,
                ty,
                value,
                mutable: false,
            })
        } else if self.check(&Token::Eq) {
            // Assignment: name = value;
            self.advance();
            let value = self.parse_expr()?;
            self.eat(&Token::Semi)?;
            let target = Spanned::new(Expr::Ident(name), start..self.current_span().start);
            Ok(Stmt::Assign { target, value })
        } else if let Some(op) = self.current().and_then(Self::compound_assign_binop) {
            self.advance();
            let rhs = self.parse_expr()?;
            self.eat(&Token::Semi)?;
            let target = Spanned::new(Expr::Ident(name.clone()), start..self.current_span().start);
            let left = Spanned::new(Expr::Ident(name), start..self.current_span().start);
            let value = Spanned::new(
                Expr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(rhs),
                },
                start..self.current_span().start,
            );
            Ok(Stmt::Assign { target, value })
        } else {
            // Expression starting with identifier
            let ident_expr = Spanned::new(Expr::Ident(name), start..self.current_span().start);
            let expr = self.parse_expr_rest(ident_expr)?;

            if self.check(&Token::Eq) {
                self.advance();
                let value = self.parse_expr()?;
                self.eat(&Token::Semi)?;
                Ok(Stmt::Assign {
                    target: expr,
                    value,
                })
            } else if let Some(op) = self.current().and_then(Self::compound_assign_binop) {
                self.advance();
                let rhs = self.parse_expr()?;
                self.eat(&Token::Semi)?;
                let value = Spanned::new(
                    Expr::Binary {
                        op,
                        left: Box::new(expr.clone()),
                        right: Box::new(rhs),
                    },
                    expr.span.start..self.current_span().start,
                );
                Ok(Stmt::Assign {
                    target: expr,
                    value,
                })
            } else {
                self.eat(&Token::Semi)?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn compound_assign_binop(token: &Token<'src>) -> Option<BinOp> {
        match token {
            Token::PlusEq => Some(BinOp::Add),
            Token::MinusEq => Some(BinOp::Sub),
            Token::StarEq => Some(BinOp::Mul),
            Token::SlashEq => Some(BinOp::Div),
            _ => None,
        }
    }

    fn parse_let(&mut self, mutable: bool) -> ParseResult<Stmt> {
        if mutable {
            self.eat(&Token::Mut)?;
        }

        let name = self.parse_ident()?;
        self.eat(&Token::Colon)?;
        let ty = self.parse_type()?;
        self.eat(&Token::Eq)?;
        let value = self.parse_expr()?;
        self.eat(&Token::Semi)?;

        Ok(Stmt::Let {
            name,
            ty,
            value,
            mutable,
        })
    }

    fn parse_return(&mut self) -> ParseResult<Stmt> {
        self.eat(&Token::Return)?;

        let value = if self.check(&Token::Semi) {
            None
        } else {
            Some(self.parse_expr()?)
        };

        self.eat(&Token::Semi)?;
        Ok(Stmt::Return(value))
    }

    fn parse_if(&mut self) -> ParseResult<Stmt> {
        self.eat(&Token::If)?;
        self.eat(&Token::LParen)?;
        let condition = self.parse_expr()?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::LBrace)?;
        let then_block = self.parse_block()?;
        self.eat(&Token::RBrace)?;

        let else_block = if self.check(&Token::Else) {
            self.advance();
            if self.check(&Token::If) {
                let nested_if_start = self.current_span().start;
                let nested_if = self.parse_if()?;
                let nested_if_end = self.current_span().start;
                Some(vec![Spanned::new(
                    nested_if,
                    nested_if_start..nested_if_end,
                )])
            } else {
                self.eat(&Token::LBrace)?;
                let block = self.parse_block()?;
                self.eat(&Token::RBrace)?;
                Some(block)
            }
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_if_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.current_span().start;
        self.eat(&Token::If)?;
        self.eat(&Token::LParen)?;
        let condition = self.parse_expr()?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::LBrace)?;
        let then_branch = self.parse_block()?;
        self.eat(&Token::RBrace)?;

        let else_branch = if self.check(&Token::Else) {
            self.advance();
            if self.check(&Token::If) {
                let nested_if = self.parse_if_expr()?;
                Some(vec![Spanned::new(
                    Stmt::Expr(nested_if.clone()),
                    nested_if.span,
                )])
            } else {
                self.eat(&Token::LBrace)?;
                let block = self.parse_block()?;
                self.eat(&Token::RBrace)?;
                Some(block)
            }
        } else {
            None
        };

        let end = self.current_span().start;
        Ok(Spanned::new(
            Expr::IfExpr {
                condition: Box::new(condition),
                then_branch,
                else_branch,
            },
            start..end,
        ))
    }

    fn parse_while(&mut self) -> ParseResult<Stmt> {
        self.eat(&Token::While)?;
        self.eat(&Token::LParen)?;
        let condition = self.parse_expr()?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.eat(&Token::RBrace)?;

        Ok(Stmt::While { condition, body })
    }

    fn parse_for(&mut self) -> ParseResult<Stmt> {
        self.eat(&Token::For)?;
        self.eat(&Token::LParen)?;

        let var = self.parse_ident()?;
        let var_type = if self.check(&Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.eat(&Token::In)?;
        let mut iterable = self.parse_expr()?;

        // Desugar: for (i in n) -> for (i in 0..n)
        if let Expr::Literal(Literal::Integer(_)) = &iterable.node {
            let start = Spanned::new(
                Expr::Literal(Literal::Integer(0)),
                iterable.span.start..iterable.span.start,
            );
            let span = iterable.span.clone();
            iterable = Spanned::new(
                Expr::Range {
                    start: Some(Box::new(start)),
                    end: Some(Box::new(iterable)),
                    inclusive: false,
                },
                span,
            );
        }

        self.eat(&Token::RParen)?;
        self.eat(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.eat(&Token::RBrace)?;

        Ok(Stmt::For {
            var,
            var_type,
            iterable,
            body,
        })
    }

    fn parse_match_stmt(&mut self) -> ParseResult<Stmt> {
        self.eat(&Token::Match)?;
        self.eat(&Token::LParen)?;
        let expr = self.parse_expr()?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::LBrace)?;
        if self.check(&Token::RBrace) {
            return Err(ParseError::new(
                "match statements must contain at least one arm",
                self.current_span(),
            ));
        }

        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            arms.push(self.parse_match_arm()?);
        }

        self.eat(&Token::RBrace)?;

        Ok(Stmt::Match { expr, arms })
    }

    fn parse_match_arm(&mut self) -> ParseResult<MatchArm> {
        let pattern = self.parse_pattern()?;
        self.eat(&Token::FatArrow)?;

        let body = if self.check(&Token::LBrace) {
            self.advance();
            let block = self.parse_block()?;
            self.eat(&Token::RBrace)?;
            block
        } else {
            let expr = self.parse_expr()?;
            let span = expr.span.clone();
            vec![Spanned::new(Stmt::Expr(expr), span)]
        };

        if self.check(&Token::Comma) {
            self.advance();
        }

        Ok(MatchArm { pattern, body })
    }

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        match self.current() {
            Some(Token::Ident(name)) if *name == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Some(Token::Ident(name)) => {
                let name = name.to_string();
                self.advance();

                if self.check(&Token::LParen) {
                    self.advance();
                    let mut bindings = Vec::new();
                    while !self.check(&Token::RParen) {
                        bindings.push(self.parse_ident()?);
                        if !self.check(&Token::RParen) {
                            self.eat(&Token::Comma)?;
                            if self.check(&Token::RParen) {
                                return Err(ParseError::new(
                                    "Trailing comma is not allowed in pattern binding lists",
                                    self.current_span(),
                                ));
                            }
                        }
                    }
                    self.eat(&Token::RParen)?;
                    Ok(Pattern::Variant(name, bindings))
                } else {
                    Ok(Pattern::Ident(name))
                }
            }
            Some(Token::Integer(n)) => {
                let n = *n;
                self.advance();
                Ok(Pattern::Literal(Literal::Integer(n)))
            }
            Some(Token::Minus) => {
                self.advance();
                if let Some(Token::Integer(n)) = self.current() {
                    let n = -*n;
                    self.advance();
                    Ok(Pattern::Literal(Literal::Integer(n)))
                } else {
                    Err(ParseError::new(
                        format!("Expected integer after '-', found {:?}", self.current()),
                        self.current_span(),
                    ))
                }
            }
            Some(Token::Float(n)) => {
                let n = *n;
                self.advance();
                Ok(Pattern::Literal(Literal::Float(n)))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Boolean(true)))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Boolean(false)))
            }
            Some(Token::String(s)) => {
                let s = s.to_string();
                self.advance();
                Ok(Pattern::Literal(Literal::String(s)))
            }
            Some(Token::Char(c)) => {
                let c = *c;
                self.advance();
                Ok(Pattern::Literal(Literal::Char(c)))
            }
            // Handle None keyword as pattern (for Option::None)
            Some(Token::None) => {
                self.advance();
                Ok(Pattern::Variant("None".to_string(), vec![]))
            }
            _ => Err(ParseError::new(
                format!("Expected pattern, found {:?}", self.current()),
                self.current_span(),
            )),
        }
    }

    // === Expression Parsing ===

    fn parse_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        // Range can be prefix: ..10 or ..=10
        if self.check(&Token::DotDot) || self.check(&Token::DotDotEq) {
            let start_span = self.current_span();
            let inclusive = self.check(&Token::DotDotEq);
            self.advance();

            let end = if self.is_at_end_of_expr() {
                None
            } else {
                Some(Box::new(self.parse_binary(0)?))
            };

            let end_span = end.as_ref().map(|e| e.span.end).unwrap_or(start_span.end);
            return Ok(Spanned::new(
                Expr::Range {
                    start: None,
                    end,
                    inclusive,
                },
                start_span.start..end_span,
            ));
        }

        let mut expr = self.parse_binary(0)?;

        // Check for range operator (lowest precedence)
        if self.check(&Token::DotDot) || self.check(&Token::DotDotEq) {
            let start_span = expr.span.clone();
            let inclusive = self.check(&Token::DotDotEq);
            self.advance();

            // Peek to see if there is an end expression
            let end = if self.is_at_end_of_expr() {
                None
            } else {
                Some(Box::new(self.parse_binary(0)?))
            };

            let end_span = end
                .as_ref()
                .map(|e| e.span.end)
                .unwrap_or(self.current_span().start);
            expr = Spanned::new(
                Expr::Range {
                    start: Some(Box::new(expr)),
                    end,
                    inclusive,
                },
                start_span.start..end_span,
            );
        }

        Ok(expr)
    }

    fn is_at_end_of_expr(&self) -> bool {
        matches!(
            self.current(),
            Some(Token::RParen)
                | Some(Token::RBrace)
                | Some(Token::RBracket)
                | Some(Token::Comma)
                | Some(Token::Semi)
                | None
        )
    }

    fn parse_binary(&mut self, min_prec: u8) -> ParseResult<Spanned<Expr>> {
        let mut left = self.parse_unary()?;

        while let Some(op) = self.current_binop() {
            let prec = op.precedence();
            if prec < min_prec {
                break;
            }

            self.advance();
            let right = self.parse_binary(prec + 1)?;

            let span = left.span.start..right.span.end;
            left = Spanned::new(
                Expr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    fn current_binop(&self) -> Option<BinOp> {
        match self.current()? {
            Token::Plus => Some(BinOp::Add),
            Token::Minus => Some(BinOp::Sub),
            Token::Star => Some(BinOp::Mul),
            Token::Slash => Some(BinOp::Div),
            Token::Percent => Some(BinOp::Mod),
            Token::EqEq => Some(BinOp::Eq),
            Token::NotEq => Some(BinOp::NotEq),
            Token::Lt => Some(BinOp::Lt),
            Token::LtEq => Some(BinOp::LtEq),
            Token::Gt => Some(BinOp::Gt),
            Token::GtEq => Some(BinOp::GtEq),
            Token::And => Some(BinOp::And),
            Token::Or => Some(BinOp::Or),
            _ => None,
        }
    }

    fn parse_unary(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.current_span().start;

        match self.current() {
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_unary()?;
                let span = start..expr.span.end;
                Ok(Spanned::new(
                    Expr::Unary {
                        op: UnaryOp::Neg,
                        expr: Box::new(expr),
                    },
                    span,
                ))
            }
            Some(Token::Not) => {
                self.advance();
                let expr = self.parse_unary()?;
                let span = start..expr.span.end;
                Ok(Spanned::new(
                    Expr::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    },
                    span,
                ))
            }
            Some(Token::Ampersand) => {
                self.advance();
                // Check for &mut
                if self.check(&Token::Mut) {
                    self.advance();
                    let expr = self.parse_unary()?;
                    let span = start..expr.span.end;
                    Ok(Spanned::new(Expr::MutBorrow(Box::new(expr)), span))
                } else {
                    let expr = self.parse_unary()?;
                    let span = start..expr.span.end;
                    Ok(Spanned::new(Expr::Borrow(Box::new(expr)), span))
                }
            }
            Some(Token::Star) => {
                self.advance();
                let expr = self.parse_unary()?;
                let span = start..expr.span.end;
                Ok(Spanned::new(Expr::Deref(Box::new(expr)), span))
            }
            Some(Token::Await) => {
                self.advance();
                let expr = self.parse_unary()?;
                let span = start..expr.span.end;
                Ok(Spanned::new(Expr::Await(Box::new(expr)), span))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.parse_primary()?;
        expr = self.parse_expr_rest(expr)?;
        Ok(expr)
    }

    fn parse_call_type_args(&mut self) -> ParseResult<Vec<Type>> {
        if !self.check(&Token::Lt) {
            return Ok(Vec::new());
        }

        let saved = self.pos;
        self.advance();
        let type_args = match self.parse_type_list(
            "Generic call type argument list cannot be empty",
            "Trailing comma is not allowed in generic call type arguments",
            &Token::Gt,
        ) {
            Ok(type_args) => type_args,
            Err(_) => {
                self.pos = saved;
                return Ok(Vec::new());
            }
        };

        if self.check(&Token::Gt) {
            self.advance();
            if self.check(&Token::LParen) {
                Ok(type_args)
            } else {
                self.pos = saved;
                Ok(Vec::new())
            }
        } else {
            self.pos = saved;
            Ok(Vec::new())
        }
    }

    fn parse_expr_rest(&mut self, mut expr: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
        loop {
            let start = expr.span.start;

            match self.current() {
                Some(Token::Dot) => {
                    self.advance();
                    let field = self.parse_ident()?;
                    let type_args = self.parse_call_type_args()?;

                    if self.check(&Token::LParen) {
                        // Method call
                        self.advance();
                        let args = self.parse_args()?;
                        self.eat(&Token::RParen)?;

                        let method_expr = Spanned::new(
                            Expr::Field {
                                object: Box::new(expr),
                                field,
                            },
                            start..self.current_span().start,
                        );
                        expr = Spanned::new(
                            Expr::Call {
                                callee: Box::new(method_expr),
                                args,
                                type_args,
                            },
                            start..self.current_span().start,
                        );
                    } else {
                        expr = Spanned::new(
                            Expr::Field {
                                object: Box::new(expr),
                                field,
                            },
                            start..self.current_span().start,
                        );
                    }
                }
                Some(Token::LParen) => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.eat(&Token::RParen)?;
                    expr = Spanned::new(
                        Expr::Call {
                            callee: Box::new(expr),
                            args,
                            type_args: Vec::new(),
                        },
                        start..self.current_span().start,
                    );
                }
                Some(Token::Lt) => {
                    let type_args = self.parse_call_type_args()?;
                    if type_args.is_empty() || !self.check(&Token::LParen) {
                        break;
                    }
                    self.advance();
                    let args = self.parse_args()?;
                    self.eat(&Token::RParen)?;
                    expr = Spanned::new(
                        Expr::Call {
                            callee: Box::new(expr),
                            args,
                            type_args,
                        },
                        start..self.current_span().start,
                    );
                }
                Some(Token::LBracket) => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.eat(&Token::RBracket)?;
                    expr = Spanned::new(
                        Expr::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                        start..self.current_span().start,
                    );
                }
                Some(Token::Question) => {
                    self.advance();
                    expr =
                        Spanned::new(Expr::Try(Box::new(expr)), start..self.current_span().start);
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.current_span().start;

        let expr = match self.current() {
            Some(Token::Integer(n)) => {
                let n = *n;
                self.advance();
                Expr::Literal(Literal::Integer(n))
            }
            Some(Token::Float(n)) => {
                let n = *n;
                self.advance();
                Expr::Literal(Literal::Float(n))
            }
            Some(Token::True) => {
                self.advance();
                Expr::Literal(Literal::Boolean(true))
            }
            Some(Token::False) => {
                self.advance();
                Expr::Literal(Literal::Boolean(false))
            }
            Some(Token::None) => {
                self.advance();
                Expr::Literal(Literal::None)
            }
            Some(Token::String(s)) => {
                let s = s.to_string();
                self.advance();
                self.parse_string_interp(s)?
            }
            Some(Token::Char(c)) => {
                let c = *c;
                self.advance();
                Expr::Literal(Literal::Char(c))
            }
            Some(Token::This) => {
                self.advance();
                Expr::This
            }
            Some(Token::If) => return self.parse_if_expr(),
            Some(Token::Static)
            | Some(Token::Super)
            | Some(Token::SelfType)
            | Some(Token::As)
            | Some(Token::Is)
            | Some(Token::TypeOf) => {
                let token = format!("{:?}", self.current());
                return Err(ParseError::new(
                    format!(
                        "Reserved keyword {} is recognized by lexer but not implemented in parser yet",
                        token
                    ),
                    self.current_span(),
                ));
            }
            Some(Token::Pipe) => {
                // Lambda: |args| body
                self.advance();
                let mut params = Vec::new();
                while !self.check(&Token::Pipe) && !self.is_at_end() {
                    let name = self.parse_ident()?;
                    let ty = if self.check(&Token::Colon) {
                        self.advance();
                        self.parse_type()?
                    } else {
                        Type::None // Default to None or inferred?
                    };
                    params.push(Parameter {
                        name,
                        ty,
                        mutable: false,
                        mode: ParamMode::Owned,
                    });
                    if self.check(&Token::Comma) {
                        self.advance();
                        if self.check(&Token::Pipe) {
                            return Err(ParseError::new(
                                "Trailing comma is not allowed in lambda parameter lists",
                                self.current_span(),
                            ));
                        }
                    }
                }
                self.eat(&Token::Pipe)?;
                let body = self.parse_expr()?;
                Expr::Lambda {
                    params,
                    body: Box::new(body),
                }
            }
            Some(Token::Ident(name)) => {
                let name = name.to_string();
                self.advance();

                // Only check for generic type arguments if this looks like a type (starts with uppercase)
                let mut full_name = name.clone();
                let is_type_name = name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false);

                let explicit_type_args = self.parse_call_type_args()?;
                let has_explicit_type_args = !explicit_type_args.is_empty();
                if has_explicit_type_args {
                    let formatted = explicit_type_args
                        .iter()
                        .map(|t| self.format_type(t))
                        .collect::<Vec<_>>()
                        .join(", ");
                    full_name = format!("{}<{}>", name, formatted);
                }

                // Check if this is a constructor call
                if self.check(&Token::LParen) {
                    self.advance();
                    let args = self.parse_args()?;
                    self.eat(&Token::RParen)?;

                    // Constructor-call heuristic:
                    // - explicit generic type calls stay constructor-like (e.g. List<Integer>(...))
                    // - known functions always parse as calls, even if uppercased
                    // - known types parse as constructors
                    // - fallback preserves legacy uppercase constructor behavior
                    let is_builtin_generic_ctor = matches!(
                        name.as_str(),
                        "List"
                            | "Map"
                            | "Set"
                            | "Option"
                            | "Result"
                            | "Box"
                            | "Rc"
                            | "Arc"
                            | "Ptr"
                            | "Task"
                            | "Range"
                    );
                    let is_constructor = if has_explicit_type_args {
                        self.known_types.contains(&name) || is_builtin_generic_ctor
                    } else if self.known_functions.contains(&name) {
                        false
                    } else if self.known_types.contains(&name) {
                        true
                    } else {
                        is_type_name && !name.contains("__")
                    };

                    if is_constructor {
                        Expr::Construct {
                            ty: full_name,
                            args,
                        }
                    } else {
                        let callee =
                            Spanned::new(Expr::Ident(name), start..self.current_span().start);
                        Expr::Call {
                            callee: Box::new(callee),
                            args,
                            type_args: explicit_type_args,
                        }
                    }
                } else {
                    Expr::Ident(name)
                }
            }
            Some(Token::LParen) => {
                // Could be either (expr) or (params) => body (lambda)
                // Try to parse as lambda first by checking for => after params
                let saved_pos = self.pos;
                self.advance();

                // Try to parse as parameter list
                let mut params = Vec::new();
                let mut is_lambda = true;

                // Empty params: () => ...
                if self.check(&Token::RParen) {
                    self.advance();
                    if self.check(&Token::FatArrow) {
                        self.advance();
                        let body = self.parse_expr()?;
                        return Ok(Spanned::new(
                            Expr::Lambda {
                                params: vec![],
                                body: Box::new(body),
                            },
                            start..self.current_span().start,
                        ));
                    } else {
                        // Empty parens but no arrow - syntax error
                        return Err(ParseError::new(
                            "Expected expression or lambda body",
                            self.current_span(),
                        ));
                    }
                }

                // Try to parse params
                while !self.check(&Token::RParen) && is_lambda {
                    if let Some(Token::Ident(name)) = self.current() {
                        let name = name.to_string();
                        self.advance();

                        if self.check(&Token::Colon) {
                            // name: Type pattern - this is a parameter
                            self.advance();
                            let ty = self.parse_type()?;
                            params.push(Parameter {
                                name,
                                ty,
                                mutable: false,
                                mode: ParamMode::Owned,
                            });

                            if self.check(&Token::Comma) {
                                self.advance();
                                if self.check(&Token::RParen) {
                                    return Err(ParseError::new(
                                        "Trailing comma is not allowed in lambda parameter lists",
                                        self.current_span(),
                                    ));
                                }
                            } else if !self.check(&Token::RParen) {
                                is_lambda = false;
                            }
                        } else {
                            // No colon after ident - not a lambda param list
                            is_lambda = false;
                        }
                    } else {
                        is_lambda = false;
                    }
                }

                if is_lambda && self.check(&Token::RParen) {
                    self.advance();
                    if self.check(&Token::FatArrow) {
                        self.advance();
                        let body = self.parse_expr()?;
                        return Ok(Spanned::new(
                            Expr::Lambda {
                                params,
                                body: Box::new(body),
                            },
                            start..self.current_span().start,
                        ));
                    }
                }

                // Not a lambda - restore position and parse as parenthesized expression
                self.pos = saved_pos;
                self.advance(); // skip (
                let expr = self.parse_expr()?;
                self.eat(&Token::RParen)?;
                return Ok(expr);
            }
            Some(Token::Require) => {
                // require(condition) or require(condition, message)
                self.advance();
                self.eat(&Token::LParen)?;
                let condition = self.parse_expr()?;
                let message = if self.check(&Token::Comma) {
                    self.advance();
                    if self.check(&Token::RParen) {
                        return Err(ParseError::new(
                            "Trailing comma is not allowed in require(...)",
                            self.current_span(),
                        ));
                    }
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                self.eat(&Token::RParen)?;
                Expr::Require {
                    condition: Box::new(condition),
                    message,
                }
            }
            Some(Token::Async) => {
                // async { block } - async block expression
                self.advance();
                if self.check(&Token::LBrace) {
                    self.advance();
                    let body = self.parse_block()?;
                    self.eat(&Token::RBrace)?;
                    Expr::AsyncBlock(body)
                } else {
                    return Err(ParseError::new(
                        "Expected '{' after async",
                        self.current_span(),
                    ));
                }
            }
            Some(Token::Match) => {
                // Match expression
                self.advance();
                self.eat(&Token::LParen)?;
                let match_expr = self.parse_expr()?;
                self.eat(&Token::RParen)?;
                self.eat(&Token::LBrace)?;
                if self.check(&Token::RBrace) {
                    return Err(ParseError::new(
                        "match expressions must contain at least one arm",
                        self.current_span(),
                    ));
                }

                let mut arms = Vec::new();
                while !self.check(&Token::RBrace) && !self.is_at_end() {
                    let pattern = self.parse_pattern()?;
                    self.eat(&Token::FatArrow)?;

                    let body = if self.check(&Token::LBrace) {
                        self.advance();
                        let block = self.parse_block()?;
                        self.eat(&Token::RBrace)?;
                        block
                    } else {
                        let expr = self.parse_expr()?;
                        vec![Spanned::new(Stmt::Expr(expr.clone()), expr.span)]
                    };

                    arms.push(MatchArm { pattern, body });

                    if self.check(&Token::Comma) {
                        self.advance();
                    }
                }

                self.eat(&Token::RBrace)?;
                Expr::Match {
                    expr: Box::new(match_expr),
                    arms,
                }
            }
            _ => {
                return Err(ParseError::new(
                    format!("Expected expression, found {:?}", self.current()),
                    self.current_span(),
                ));
            }
        };

        let end = self.current_span().start;
        Ok(Spanned::new(expr, start..end))
    }

    fn parse_string_interp(&mut self, s: String) -> ParseResult<Expr> {
        // Parse string interpolation: "Hello, {name}!"
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(next) = chars.next() {
                    match next {
                        'n' => current.push('\n'),
                        't' => current.push('\t'),
                        'r' => current.push('\r'),
                        '\\' => current.push('\\'),
                        '"' => current.push('"'),
                        '\'' => current.push('\''),
                        '{' => current.push('{'),
                        '}' => current.push('}'),
                        other => {
                            current.push('\\');
                            current.push(other);
                        }
                    }
                } else {
                    current.push('\\');
                }
            } else if c == '{' {
                if !current.is_empty() {
                    parts.push(StringPart::Literal(std::mem::take(&mut current)));
                }

                let mut expr_str = String::new();
                let mut depth = 1;
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == '{' {
                        depth += 1;
                        expr_str.push(c);
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            closed = true;
                            break;
                        }
                        expr_str.push(c);
                    } else {
                        expr_str.push(c);
                    }
                }

                if !closed {
                    parts.push(StringPart::Literal(format!("{{{}", expr_str)));
                    continue;
                }

                // Parse the expression inside {}
                if expr_str.trim().is_empty() {
                    parts.push(StringPart::Literal("{}".to_string()));
                } else {
                    let tokens_result = crate::lexer::tokenize(&expr_str);
                    match tokens_result {
                        Ok(tokens) => {
                            let mut parser = Parser::new(tokens);
                            match parser.parse_expr() {
                                Ok(expr) => {
                                    // Ensure we consumed the entire content (no trailing garbage)
                                    if parser.is_at_end() {
                                        parts.push(StringPart::Expr(expr));
                                    } else {
                                        // Trailing content, treat as literal
                                        parts
                                            .push(StringPart::Literal(format!("{{{}}}", expr_str)));
                                    }
                                }
                                Err(_) => {
                                    // Parse error inside {}, treat as literal
                                    parts.push(StringPart::Literal(format!("{{{}}}", expr_str)));
                                }
                            }
                        }
                        Err(_) => {
                            // Tokenization error, treat as literal
                            parts.push(StringPart::Literal(format!("{{{}}}", expr_str)));
                        }
                    }
                }
            } else {
                current.push(c);
            }
        }

        if !current.is_empty() {
            parts.push(StringPart::Literal(current));
        }

        if parts.len() == 1 {
            if let StringPart::Literal(s) = &parts[0] {
                return Ok(Expr::Literal(Literal::String(s.clone())));
            }
        }

        if parts.iter().all(|p| matches!(p, StringPart::Literal(_))) {
            let merged = parts
                .into_iter()
                .map(|p| match p {
                    StringPart::Literal(s) => s,
                    StringPart::Expr(_) => unreachable!(),
                })
                .collect::<String>();
            return Ok(Expr::Literal(Literal::String(merged)));
        }

        if parts.is_empty() {
            return Ok(Expr::Literal(Literal::String(String::new())));
        }

        Ok(Expr::StringInterp(parts))
    }

    fn parse_args(&mut self) -> ParseResult<Vec<Spanned<Expr>>> {
        let mut args = Vec::new();

        while !self.check(&Token::RParen) && !self.is_at_end() {
            args.push(self.parse_expr()?);
            if !self.check(&Token::RParen) {
                self.eat(&Token::Comma)?;
                if self.check(&Token::RParen) {
                    return Err(ParseError::new(
                        "Trailing comma is not allowed in argument lists",
                        self.current_span(),
                    ));
                }
            }
        }

        Ok(args)
    }

    fn parse_ident(&mut self) -> ParseResult<String> {
        match self.current() {
            Some(Token::Ident(name)) => {
                let name = name.to_string();
                self.advance();
                Ok(name)
            }
            _ => Err(ParseError::new(
                format!("Expected identifier, found {:?}", self.current()),
                self.current_span(),
            )),
        }
    }
}

fn decode_escaped_string(raw: &str) -> String {
    let mut decoded = String::new();
    let mut chars = raw.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => decoded.push('\n'),
                    't' => decoded.push('\t'),
                    'r' => decoded.push('\r'),
                    '\\' => decoded.push('\\'),
                    '"' => decoded.push('"'),
                    '\'' => decoded.push('\''),
                    '{' => decoded.push('{'),
                    '}' => decoded.push('}'),
                    other => {
                        decoded.push('\\');
                        decoded.push(other);
                    }
                }
            } else {
                decoded.push('\\');
            }
        } else {
            decoded.push(ch);
        }
    }

    decoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatter::format_program_canonical;
    use crate::lexer::tokenize;

    fn parse_source(source: &str) -> Result<Program, ParseError> {
        let tokens = tokenize(source).map_err(|e| ParseError::new(e, 0..0))?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    #[test]
    fn test_parse_test_attribute() {
        let source = r#"
            package test;
            
            @Test
            function testAddition(): Integer {
                return 2 + 2;
            }
        "#;

        let program = parse_source(source).expect("Should parse successfully");
        assert_eq!(program.declarations.len(), 1);

        match &program.declarations[0].node {
            Decl::Function(func) => {
                assert_eq!(func.name, "testAddition");
                assert_eq!(func.attributes.len(), 1);
                assert_eq!(func.attributes[0], Attribute::Test);
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_parse_ignore_attribute_with_reason() {
        let source = r#"
            package test;
            
            @Test
            @Ignore("Not implemented yet")
            function testDivision(): Integer {
                return 10 / 2;
            }
        "#;

        let program = parse_source(source).expect("Should parse successfully");

        match &program.declarations[0].node {
            Decl::Function(func) => {
                assert_eq!(func.attributes.len(), 2);
                assert_eq!(func.attributes[0], Attribute::Test);
                assert_eq!(
                    func.attributes[1],
                    Attribute::Ignore(Some("Not implemented yet".to_string()))
                );
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_parse_function_without_attributes() {
        let source = r#"
            package test;
            
            function normalFunction(): Integer {
                return 42;
            }
        "#;

        let program = parse_source(source).expect("Should parse successfully");

        match &program.declarations[0].node {
            Decl::Function(func) => {
                assert_eq!(func.name, "normalFunction");
                assert!(func.attributes.is_empty());
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_parse_public_top_level_function() {
        let source = r#"
            public function exported(): Integer {
                return 1;
            }
        "#;

        let program = parse_source(source).expect("Should parse public top-level function");
        match &program.declarations[0].node {
            Decl::Function(func) => {
                assert_eq!(func.name, "exported");
                assert_eq!(func.visibility, Visibility::Public);
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_unknown_attribute_error() {
        let source = r#"
            package test;
            
            @Unknown
            function testFunc(): Integer {
                return 42;
            }
        "#;

        let result = parse_source(source);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unknown attribute"));
    }

    #[test]
    fn test_parse_import_with_alias() {
        let source = r#"
            import std.math as math;
            function main(): Integer {
                return 0;
            }
        "#;
        let program = parse_source(source).expect("Should parse import alias");
        match &program.declarations[0].node {
            Decl::Import(import) => {
                assert_eq!(import.path, "std.math");
                assert_eq!(import.alias.as_deref(), Some("math"));
            }
            _ => panic!("Expected import declaration"),
        }
    }

    #[test]
    fn test_reject_import_wildcard_with_alias() {
        let source = r#"
            import std.io.* as io;
            function main(): None { return None; }
        "#;
        let err = parse_source(source).expect_err("wildcard alias import should fail");
        assert!(
            err.message
                .contains("Cannot use alias with wildcard import"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_import_alias_without_identifier() {
        let source = r#"
            import std.math as ;
            function main(): None { return None; }
        "#;
        let err = parse_source(source).expect_err("import alias without identifier should fail");
        assert!(
            err.message.contains("Expected identifier"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_import_with_empty_path_segment() {
        let source = r#"
            import std..math;
            function main(): None { return None; }
        "#;
        let err = parse_source(source).expect_err("empty import path segment should fail");
        assert!(
            err.message
                .contains("Import path cannot contain an empty segment"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_package_with_empty_path_segment() {
        let source = r#"
            package app..core;
            function main(): None { return None; }
        "#;
        let err = parse_source(source).expect_err("empty package path segment should fail");
        assert!(
            err.message
                .contains("Package path cannot contain an empty segment"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_package_with_trailing_dot() {
        let source = r#"
            package app.;
            function main(): None { return None; }
        "#;
        let err = parse_source(source).expect_err("package trailing dot should fail");
        assert!(
            err.message.contains("Package path cannot end with '.'"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_parse_compound_assign_ident() {
        let source = r#"
            function main(): None {
                x: Integer = 1;
                x += 2;
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse compound assignment");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };

        let Stmt::Assign { target, value } = &func.body[1].node else {
            panic!("Expected assign statement");
        };
        let Expr::Ident(target_name) = &target.node else {
            panic!("Expected assign target ident");
        };
        assert_eq!(target_name, "x");

        let Expr::Binary { op, left, right } = &value.node else {
            panic!("Expected desugared binary expression");
        };
        assert_eq!(*op, BinOp::Add);
        let Expr::Ident(left_name) = &left.node else {
            panic!("Expected left ident");
        };
        let Expr::Literal(Literal::Integer(rhs)) = right.node else {
            panic!("Expected integer rhs");
        };
        assert_eq!(left_name, "x");
        assert_eq!(rhs, 2);
    }

    #[test]
    fn test_parse_compound_assign_index_target() {
        let source = r#"
            function main(): None {
                items: List<Integer> = range(0, 3);
                items[0] -= 1;
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse index compound assignment");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };

        let Stmt::Assign { target, value } = &func.body[1].node else {
            panic!("Expected assign statement");
        };
        let Expr::Index { object, index } = &target.node else {
            panic!("Expected index target");
        };
        let Expr::Ident(obj_name) = &object.node else {
            panic!("Expected indexed object ident");
        };
        let Expr::Literal(Literal::Integer(idx)) = index.node else {
            panic!("Expected integer index");
        };
        assert_eq!(obj_name, "items");
        assert_eq!(idx, 0);

        let Expr::Binary { op, left, right } = &value.node else {
            panic!("Expected desugared binary expression");
        };
        assert_eq!(*op, BinOp::Sub);
        let Expr::Index { .. } = &left.node else {
            panic!("Expected cloned index expression on lhs");
        };
        let Expr::Literal(Literal::Integer(rhs)) = right.node else {
            panic!("Expected integer rhs");
        };
        assert_eq!(rhs, 1);
    }

    #[test]
    fn test_reject_visibility_modifier_on_constructor() {
        let source = r#"
            class C {
                private constructor() { }
            }
        "#;
        let err = parse_source(source).expect_err("private constructor modifier should fail");
        assert!(
            err.message
                .contains("Visibility modifiers are not supported on constructors"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_match_arm_expression_keeps_expr_span() {
        let source = r#"
            function main(): None {
                match (1) {
                    1 => foo,
                    _ => bar,
                }
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse match statement");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Match { arms, .. } = &func.body[0].node else {
            panic!("Expected match statement");
        };
        let first_stmt = &arms[0].body[0];
        assert_ne!(first_stmt.span.start, 0);
        assert!(first_stmt.span.end > first_stmt.span.start);
    }

    #[test]
    fn test_parse_if_expression() {
        let source = r#"
            function main(): None {
                x: Integer = if (true) { 1; } else { 2; };
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse if-expression initializer");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::IfExpr {
            condition,
            then_branch,
            else_branch,
        } = &value.node
        else {
            panic!("Expected if expression");
        };
        assert!(matches!(
            condition.node,
            Expr::Literal(Literal::Boolean(true))
        ));
        assert_eq!(then_branch.len(), 1);
        assert!(else_branch.as_ref().is_some_and(|b| b.len() == 1));
    }

    #[test]
    fn test_parse_if_expression_without_else() {
        let source = r#"
            function main(): None {
                x: None = if (true) { println("x"); };
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse if-expression without else");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::IfExpr { else_branch, .. } = &value.node else {
            panic!("Expected if expression");
        };
        assert!(else_branch.is_none());
    }

    #[test]
    fn test_parse_if_statement_with_else_if() {
        let source = r#"
            function main(): None {
                if (true) {
                    return None;
                } else if (false) {
                    return None;
                } else {
                    return None;
                }
            }
        "#;
        let program = parse_source(source).expect("else-if statement should parse");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::If { else_block, .. } = &func.body[0].node else {
            panic!("Expected if statement");
        };
        let else_block = else_block
            .as_ref()
            .expect("else-if should build else block");
        assert!(matches!(else_block[0].node, Stmt::If { .. }));
    }

    #[test]
    fn test_parse_if_expression_with_else_if() {
        let source = r#"
            function main(): None {
                x: Integer = if (true) { 1; } else if (false) { 2; } else { 3; };
                return None;
            }
        "#;
        let program = parse_source(source).expect("else-if expression should parse");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::IfExpr { else_branch, .. } = &value.node else {
            panic!("Expected if expression");
        };
        let else_branch = else_branch
            .as_ref()
            .expect("else-if should build else branch");
        assert!(matches!(
            else_branch[0].node,
            Stmt::Expr(Spanned {
                node: Expr::IfExpr { .. },
                ..
            })
        ));
    }

    #[test]
    fn test_parse_match_statement_with_trailing_semicolon() {
        let source = r#"
            function main(): None {
                match (1) {
                    1 => { },
                    _ => { },
                };
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse match statement with semicolon");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        assert!(matches!(func.body[0].node, Stmt::Match { .. }));
    }

    #[test]
    fn test_reject_empty_match_statement() {
        let source = r#"
            function main(): None {
                match (1) {
                }
                return None;
            }
        "#;
        let err = parse_source(source).expect_err("empty match statement should fail");
        assert!(
            err.message
                .contains("match statements must contain at least one arm"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_empty_match_expression() {
        let source = r#"
            function main(): None {
                x: Integer = match (1) {
                };
                return None;
            }
        "#;
        let err = parse_source(source).expect_err("empty match expression should fail");
        assert!(
            err.message
                .contains("match expressions must contain at least one arm"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_parse_if_expression_branch_match_statement_with_semicolon() {
        let source = r#"
            function main(): None {
                x: None = if (true) {
                    match (1) {
                        1 => { },
                        _ => { },
                    };
                } else {
                    None;
                };
                return None;
            }
        "#;
        let program =
            parse_source(source).expect("Should parse if-expression with match statement");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::IfExpr { then_branch, .. } = &value.node else {
            panic!("Expected if expression");
        };
        assert!(matches!(then_branch[0].node, Stmt::Match { .. }));
    }

    #[test]
    fn test_uppercase_function_call_is_not_forced_constructor() {
        let source = r#"
            function Foo(): Integer { return 7; }
            function main(): None {
                x: Integer = Foo();
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse uppercase function call");
        let Decl::Function(func) = &program.declarations[1].node else {
            panic!("Expected main function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        match &value.node {
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                assert!(matches!(callee.node, Expr::Ident(ref n) if n == "Foo"));
                assert!(args.is_empty());
                assert!(type_args.is_empty());
            }
            other => panic!("Expected call expression, found {:?}", other),
        }
    }

    #[test]
    fn test_forward_uppercase_function_call_is_call() {
        let source = r#"
            function main(): None {
                x: Integer = Foo();
                return None;
            }
            function Foo(): Integer { return 7; }
        "#;
        let program = parse_source(source).expect("Should parse forward uppercase function call");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected main function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Call { type_args, .. } = &value.node else {
            panic!("Expected call");
        };
        assert_eq!(type_args.len(), 0);
    }

    #[test]
    fn test_parse_explicit_generic_method_call() {
        let source = r#"
            class C {
                function id<T>(x: T): T { return x; }
            }
            function main(): None {
                c: C = C();
                x: Integer = c.id<Integer>(1);
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse explicit generic method call");
        let Decl::Function(func) = &program.declarations[1].node else {
            panic!("Expected main function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[1].node else {
            panic!("Expected let statement");
        };
        let Expr::Call { type_args, .. } = &value.node else {
            panic!("Expected call");
        };
        assert_eq!(type_args.len(), 1);
    }

    #[test]
    fn test_parse_explicit_generic_module_call() {
        let source = r#"
            module M { function id<T>(x: T): T { return x; } }
            function main(): None {
                x: Integer = M.id<Integer>(1);
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse explicit generic module call");
        let Decl::Function(func) = &program.declarations[1].node else {
            panic!("Expected main function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Call { type_args, .. } = &value.node else {
            panic!("Expected call");
        };
        assert_eq!(type_args.len(), 1);
    }

    #[test]
    fn test_parse_explicit_generic_function_call() {
        let source = r#"
            function id<T>(x: T): T { return x; }
            function main(): None {
                x: Integer = id<Integer>(1);
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse explicit generic call");
        let Decl::Function(func) = &program.declarations[1].node else {
            panic!("Expected main function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Call { type_args, .. } = &value.node else {
            panic!("Expected call");
        };
        assert_eq!(type_args.len(), 1);
    }

    #[test]
    fn test_reject_function_type_trailing_comma() {
        let source = r#"
            function takes(f: (Integer,) -> Integer): None {
                return None;
            }
        "#;
        let err = parse_source(source).expect_err("function type trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in function type parameters"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_parse_zero_arg_function_type() {
        let source = r#"
            function takes(f: () -> Integer): None {
                return None;
            }
        "#;
        parse_source(source).expect("zero-arg function type should remain valid");
    }

    #[test]
    fn test_reject_explicit_generic_function_call_with_trailing_comma() {
        let source = r#"
            function id<T>(x: T): T { return x; }
            function main(): None {
                x: Integer = id<Integer,>(1);
                return None;
            }
        "#;
        let err = parse_source(source).expect_err("generic call trailing comma should fail");
        assert!(
            err.message.contains("Trailing comma") || err.message.contains("Expected"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_explicit_generic_module_call_with_trailing_comma() {
        let source = r#"
            module A {
                module B {
                    function id<T>(x: T): T { return x; }
                }
            }
            function main(): None {
                x: Integer = A.B.id<Integer,>(1);
                return None;
            }
        "#;
        let err = parse_source(source)
            .expect_err("nested module generic call trailing comma should fail");
        assert!(
            err.message.contains("Trailing comma") || err.message.contains("Expected"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_empty_generic_parameter_list() {
        let source = r#"
            function id<>(): Integer {
                return 1;
            }
        "#;
        let err = parse_source(source).expect_err("empty generic parameter list should fail");
        assert!(
            err.message
                .contains("Generic parameter list cannot be empty"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_generic_parameter_list() {
        let source = r#"
            function id<T,>(x: T): T {
                return x;
            }
        "#;
        let err = parse_source(source).expect_err("generic parameter trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in generic parameter lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_parameter_list() {
        let source = r#"
            function add(x: Integer,): Integer {
                return x;
            }
        "#;
        let err = parse_source(source).expect_err("parameter trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in parameter lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_extern_parameter_list() {
        let source = r#"
            extern(c) function puts(msg: String,): Integer;
        "#;
        let err = parse_source(source).expect_err("extern parameter trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in extern parameter lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_argument_list() {
        let source = r#"
            function add(x: Integer): Integer { return x; }
            function main(): None {
                value: Integer = add(1,);
                return None;
            }
        "#;
        let err = parse_source(source).expect_err("argument trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in argument lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_implements_list() {
        let source = r#"
            class C implements A, {
            }
        "#;
        let err = parse_source(source).expect_err("implements trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in implements lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_empty_implements_list() {
        let source = r#"
            class C implements {
            }
        "#;
        let err = parse_source(source).expect_err("empty implements list should fail");
        assert!(
            err.message.contains("implements list cannot be empty"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_interface_extends_list() {
        let source = r#"
            interface Child extends Parent, {
                function run(): None;
            }
        "#;
        let err = parse_source(source).expect_err("interface extends trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in interface extends lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_empty_interface_extends_list() {
        let source = r#"
            interface Child extends {
                function run(): None;
            }
        "#;
        let err = parse_source(source).expect_err("empty interface extends list should fail");
        assert!(
            err.message
                .contains("interface extends list cannot be empty"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_visibility_modifier_on_module() {
        let source = r#"
            public module Tools {
            }
        "#;
        let err = parse_source(source).expect_err("module visibility modifier should fail");
        assert!(
            err.message
                .contains("Visibility modifiers are not supported on modules"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_class_extends_trailing_comma() {
        let source = r#"
            class Child extends Base, {
            }
        "#;
        let err = parse_source(source).expect_err("class extends trailing comma should fail");
        assert!(
            err.message
                .contains("Class extends clause accepts exactly one base class"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_enum_field_list() {
        let source = r#"
            enum Value {
                One(Integer,),
            }
        "#;
        let err = parse_source(source).expect_err("enum field trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in enum field lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_enum_variant_list() {
        let source = r#"
            enum Value {
                One,
            }
        "#;
        let err = parse_source(source).expect_err("enum variant trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in enum variant lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_pattern_binding_list() {
        let source = r#"
            enum Value {
                One(Integer)
            }

            function main(): None {
                match (One(1)) {
                    One(x,) => { return None; },
                    _ => { return None; }
                }
            }
        "#;
        let err = parse_source(source).expect_err("pattern binding trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in pattern binding lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_empty_extern_options() {
        let source = r#"
            extern() function puts(msg: String): Integer;
        "#;
        let err = parse_source(source).expect_err("empty extern options should fail");
        assert!(
            err.message.contains("extern(...) options cannot be empty"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_extern_options() {
        let source = r#"
            extern(c,) function puts(msg: String): Integer;
        "#;
        let err = parse_source(source).expect_err("extern options trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in extern options"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_extra_extern_option_argument() {
        let source = r#"
            extern(c, "puts", "extra") function puts(msg: String): Integer;
        "#;
        let err = parse_source(source).expect_err("extra extern option should fail");
        assert!(
            err.message
                .contains("extern(...) accepts at most ABI and optional link name"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_lambda_parameter_list() {
        let source = r#"
            function main(): None {
                f: None = |x: Integer,| 1;
                return None;
            }
        "#;
        let err = parse_source(source).expect_err("lambda parameter trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in lambda parameter lists"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_reject_trailing_comma_in_require_call() {
        let source = r#"
            function main(): None {
                require(true,);
                return None;
            }
        "#;
        let err = parse_source(source).expect_err("require trailing comma should fail");
        assert!(
            err.message
                .contains("Trailing comma is not allowed in require(...)"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_parse_float_char_and_negative_match_patterns() {
        let source = r#"
            function main(): None {
                f: Float = 1.0;
                c: Char = 'a';
                i: Integer = -1;
                match (f) { 1.0 => { }, _ => { } }
                match (c) { 'a' => { }, _ => { } }
                match (i) { -1 => { }, _ => { } }
                return None;
            }
        "#;
        parse_source(source).expect("Should parse float/char/negative patterns");
    }

    #[test]
    fn test_parse_enum_named_field_with_ptr_type() {
        let source = r#"
            enum Handle {
                Raw(ptr: Ptr<Char>)
            }
        "#;
        let program = parse_source(source).expect("Should parse Ptr in named enum fields");
        let Decl::Enum(en) = &program.declarations[0].node else {
            panic!("Expected enum declaration");
        };
        let field = &en.variants[0].fields[0];
        assert!(matches!(field.ty, Type::Ptr(_)));
    }

    #[test]
    fn test_string_interp_empty_braces_stay_literal() {
        let source = r#"
            function main(): None {
                s: String = "before {} after";
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Literal(Literal::String(s)) = &value.node else {
            panic!("Expected string literal");
        };
        assert_eq!(s, "before {} after");
    }

    #[test]
    fn test_string_interp_unclosed_brace_stays_literal() {
        let source = r#"
            function main(): None {
                s: String = "value: {x";
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Literal(Literal::String(s)) = &value.node else {
            panic!("Expected string literal");
        };
        assert_eq!(s, "value: {x");
    }

    #[test]
    fn test_string_literal_decodes_common_escapes() {
        let source = r#"
            function main(): None {
                s: String = "line1\nline2\t\"ok\"\\";
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Literal(Literal::String(s)) = &value.node else {
            panic!("Expected string literal");
        };
        assert_eq!(s, "line1\nline2\t\"ok\"\\");
    }

    #[test]
    fn test_string_interp_escaped_braces_stay_literal() {
        let source = r#"
            function main(): None {
                s: String = "\{x\}";
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Literal(Literal::String(s)) = &value.node else {
            panic!("Expected string literal");
        };
        assert_eq!(s, "{x}");
    }

    #[test]
    fn test_string_interp_invalid_expression_stays_literal() {
        let source = r#"
            function main(): None {
                s: String = "value {1+}";
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Literal(Literal::String(s)) = &value.node else {
            panic!("Expected string literal");
        };
        assert_eq!(s, "value {1+}");
    }

    #[test]
    fn test_string_interp_nested_braces_invalid_expr_stays_literal() {
        let source = r#"
            function main(): None {
                s: String = "value {{1}}";
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Literal(Literal::String(s)) = &value.node else {
            panic!("Expected string literal");
        };
        assert_eq!(s, "value {{1}}");
    }

    #[test]
    fn test_string_interp_stray_closing_brace_stays_literal() {
        let source = r#"
            function main(): None {
                s: String = "abc }";
                return None;
            }
        "#;
        let program = parse_source(source).expect("Should parse");
        let Decl::Function(func) = &program.declarations[0].node else {
            panic!("Expected function declaration");
        };
        let Stmt::Let { value, .. } = &func.body[0].node else {
            panic!("Expected let statement");
        };
        let Expr::Literal(Literal::String(s)) = &value.node else {
            panic!("Expected string literal");
        };
        assert_eq!(s, "abc }");
    }

    #[test]
    fn test_builtin_generic_type_rejects_wrong_arity() {
        let source = r#"
            function main(): Map<Integer> {
                return 0;
            }
        "#;
        let err = parse_source(source).expect_err("Map with one type arg should fail to parse");
        assert!(err
            .message
            .contains("Built-in type 'Map' expects 2 type arguments"));
    }

    #[test]
    fn test_builtin_generic_type_rejects_empty_args() {
        let source = r#"
            function main(): Ptr<> {
                return 0;
            }
        "#;
        let err = parse_source(source).expect_err("Ptr<> should fail to parse");
        assert!(err
            .message
            .contains("Generic type argument list cannot be empty"));
    }

    #[test]
    fn test_builtin_generic_type_rejects_trailing_comma() {
        let source = r#"
            function main(): Result<Integer,> {
                return 0;
            }
        "#;
        let err = parse_source(source).expect_err("Trailing comma in type args should fail");
        assert!(err
            .message
            .contains("Trailing comma is not allowed in generic type arguments"));
    }

    #[test]
    fn test_malformed_syntax_corpus_never_panics() {
        let malformed_cases = [
            "function main(: None { return None; }",
            "function main(): Map<Integer> { return 0; }",
            "function main(): None { x: Integer = id<Integer,>(1); return None; }",
            "function takes(f: (Integer,) -> Integer): None { return None; }",
            "import std..math; function main(): None { return None; }",
            "package app.; function main(): None { return None; }",
            "module A { function f(): None { return None; }",
            "function main(): None { s: String = \"value {1+}\"; return None; }",
            "function main(): None { match (1) { 1 => { }, _ => } return None; }",
            "function main(): None { x: List<Integer = range(0, 1); return None; }",
        ];

        for source in malformed_cases {
            let result = std::panic::catch_unwind(|| parse_source(source));
            assert!(
                result.is_ok(),
                "parser panicked on malformed input: {source}"
            );
        }
    }

    #[test]
    fn test_valid_syntax_corpus_roundtrips_through_canonical_formatter() {
        let valid_cases = [
            r#"
                package app.core;
                import std.math as math;
                function main(): None {
                    value: Integer = math.abs<Integer>(1);
                    return None;
                }
            "#,
            r#"
                function takes(f: () -> Integer): Integer {
                    return f();
                }
            "#,
            r#"
                module A {
                    module B {
                        function id<T>(x: T): T { return x; }
                    }
                }
                function main(): None {
                    x: Integer = A.B.id<Integer>(1);
                    return None;
                }
            "#,
            r#"
                function main(): None {
                    msg: String = "hello {name}";
                    x: Integer = if (true) { 1; } else { 2; };
                    match (x) {
                        1 => { println(msg); },
                        _ => { println("fallback"); },
                    }
                    return None;
                }
            "#,
            r#"
                class Boxed<T> {
                    value: T;

                    function get(): T {
                        return self.value;
                    }
                }
            "#,
        ];

        for source in valid_cases {
            let program = parse_source(source).expect("valid corpus should parse");
            let formatted = format_program_canonical(&program);
            parse_source(&formatted).expect("canonical formatted corpus should still parse");
        }
    }

    #[test]
    fn test_generated_malformed_syntax_matrix_never_panics() {
        let prefixes = [
            "function main(): None { ",
            "module M { function main(): None { ",
            "class C { function main(): None { ",
        ];
        let fragments = [
            "x: Integer = (1 + );",
            "x: Integer = foo<Integer,>(1);",
            "x: Integer = if (true) { 1; } else ;",
            "x: String = \"value {1+}\";",
            "match (1) { 1 => { }, _ => };",
            "items: List<Integer = range(0, 1);",
            "x: Integer = foo(,);",
            "x: Integer = arr[);",
        ];
        let suffixes = [" return None; }", " } }", " return None; } }"];

        for prefix in prefixes {
            for fragment in fragments {
                for suffix in suffixes {
                    let source = format!("{prefix}{fragment}{suffix}");
                    let result = std::panic::catch_unwind(|| parse_source(&source));
                    assert!(
                        result.is_ok(),
                        "parser panicked on generated input: {source}"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "deterministic stress runner for manual hardening passes"]
    fn stress_deterministic_generated_noise_never_panics() {
        let seeds = [1u64, 7, 17, 29, 53, 97, 193, 389];
        let alphabet = [
            "function", "main", "(", ")", "{", "}", "<", ">", ",", ";", ":", "=", "+", "-", "*",
            "/", "if", "else", "match", "module", "class", "import", "package", "foo", "bar",
            "baz", "Integer", "None", "\"x\"", "1", "true", "\n", " ",
        ];

        for seed in seeds {
            let mut state = seed;
            for _case in 0..256 {
                let mut source = String::new();
                let len = 8 + (state as usize % 48);
                for _ in 0..len {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let idx = (state as usize) % alphabet.len();
                    source.push_str(alphabet[idx]);
                }
                let result = std::panic::catch_unwind(|| {
                    let tokens = tokenize(&source).ok();
                    if let Some(tokens) = tokens {
                        let mut parser = Parser::new(tokens);
                        let _ = parser.parse_program();
                    }
                });
                assert!(result.is_ok(), "generated stress input panicked: {source}");
            }
        }
    }
}
