//! Apex Parser - Recursive descent parser
//!
//! Production-ready parser with full language support

use crate::ast::*;
use crate::lexer::Token;

pub struct Parser<'src> {
    tokens: Vec<(Token<'src>, std::ops::Range<usize>)>,
    pos: usize,
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
    pub fn new(tokens: Vec<(Token<'src>, std::ops::Range<usize>)>) -> Self {
        Self { tokens, pos: 0 }
    }

    // === Utility Methods ===

    fn current(&self) -> Option<&Token<'src>> {
        self.tokens.get(self.pos).map(|(t, _)| t)
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
            Type::Task(inner) => format!("Task<{}>", self.format_type(inner)),
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
            while self.check(&Token::Dot) {
                self.advance();
                pkg_parts.push(self.parse_ident()?);
            }
            package = Some(pkg_parts.join("."));
            self.eat(&Token::Semi)?;
        }

        while !self.is_at_end() {
            declarations.push(self.parse_declaration()?);
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
                let s = s.to_string();
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
            _ => Visibility::Private,
        }
    }

    /// Parse generic parameters: <T, U extends Comparable>
    fn parse_generic_params(&mut self) -> ParseResult<Vec<GenericParam>> {
        let mut params = Vec::new();
        if !self.check(&Token::Lt) {
            return Ok(params);
        }
        self.advance(); // eat '<'

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
            return_type,
            body,
            is_async,
            visibility,
            attributes,
        })
    }

    fn parse_class(&mut self, _attributes: Vec<Attribute>) -> ParseResult<ClassDecl> {
        let visibility = self.parse_visibility();

        self.eat(&Token::Class)?;
        let name = self.parse_ident()?;
        let generic_params = self.parse_generic_params()?;

        // Parse extends clause
        let extends = if self.check(&Token::Extends) {
            self.advance();
            Some(self.parse_ident()?)
        } else {
            None
        };

        // Parse implements clause
        let mut implements = Vec::new();
        if self.check(&Token::Implements) {
            self.advance();
            implements.push(self.parse_ident()?);
            while self.check(&Token::Comma) {
                self.advance();
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
                    constructor = Some(self.parse_constructor()?);
                }
                Some(Token::Destructor) => {
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
            self.advance();
            let mut type_args = Vec::new();
            while !self.check(&Token::Gt) {
                type_args.push(self.parse_type()?);
                if !self.check(&Token::Gt) {
                    self.eat(&Token::Comma)?;
                }
            }
            self.eat(&Token::Gt)?;

            // Handle built-in generic types
            match name {
                "Option" if type_args.len() == 1 => Ok(Type::Option(Box::new(
                    type_args.into_iter().next().unwrap(),
                ))),
                "Result" if type_args.len() == 2 => {
                    let mut iter = type_args.into_iter();
                    Ok(Type::Result(
                        Box::new(iter.next().unwrap()),
                        Box::new(iter.next().unwrap()),
                    ))
                }
                "List" if type_args.len() == 1 => {
                    Ok(Type::List(Box::new(type_args.into_iter().next().unwrap())))
                }
                "Map" if type_args.len() == 2 => {
                    let mut iter = type_args.into_iter();
                    Ok(Type::Map(
                        Box::new(iter.next().unwrap()),
                        Box::new(iter.next().unwrap()),
                    ))
                }
                "Set" if type_args.len() == 1 => {
                    Ok(Type::Set(Box::new(type_args.into_iter().next().unwrap())))
                }
                "Box" if type_args.len() == 1 => {
                    Ok(Type::Box(Box::new(type_args.into_iter().next().unwrap())))
                }
                "Rc" if type_args.len() == 1 => {
                    Ok(Type::Rc(Box::new(type_args.into_iter().next().unwrap())))
                }
                "Arc" if type_args.len() == 1 => {
                    Ok(Type::Arc(Box::new(type_args.into_iter().next().unwrap())))
                }
                "Task" if type_args.len() == 1 => {
                    Ok(Type::Task(Box::new(type_args.into_iter().next().unwrap())))
                }
                _ => Ok(Type::Generic(name.to_string(), type_args)),
            }
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
            extends.push(self.parse_ident()?);
            while self.check(&Token::Comma) {
                self.advance();
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
        while self.check(&Token::Dot) {
            self.advance();

            // Check for wildcard
            if self.check(&Token::Star) {
                self.advance();
                path_parts.push("*".to_string());
                break;
            }

            path_parts.push(self.parse_ident()?);
        }

        self.eat(&Token::Semi)?;

        Ok(ImportDecl {
            path: path_parts.join("."),
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
            let mut params = Vec::new();
            while !self.check(&Token::RParen) && !self.is_at_end() {
                params.push(self.parse_type()?);
                if self.check(&Token::Comma) {
                    self.advance();
                } else if !self.check(&Token::RParen) {
                    break;
                }
            }
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
                    self.advance();
                    let mut type_args = Vec::new();
                    while !self.check(&Token::Gt) {
                        type_args.push(self.parse_type()?);
                        if !self.check(&Token::Gt) {
                            self.eat(&Token::Comma)?;
                        }
                    }
                    self.eat(&Token::Gt)?;

                    // Handle built-in generic types
                    match name.as_str() {
                        "Option" if type_args.len() == 1 => {
                            Type::Option(Box::new(type_args.into_iter().next().unwrap()))
                        }
                        "Result" if type_args.len() == 2 => {
                            let mut iter = type_args.into_iter();
                            Type::Result(
                                Box::new(iter.next().unwrap()),
                                Box::new(iter.next().unwrap()),
                            )
                        }
                        "List" if type_args.len() == 1 => {
                            Type::List(Box::new(type_args.into_iter().next().unwrap()))
                        }
                        "Map" if type_args.len() == 2 => {
                            let mut iter = type_args.into_iter();
                            Type::Map(
                                Box::new(iter.next().unwrap()),
                                Box::new(iter.next().unwrap()),
                            )
                        }
                        "Set" if type_args.len() == 1 => {
                            Type::Set(Box::new(type_args.into_iter().next().unwrap()))
                        }
                        "Box" if type_args.len() == 1 => {
                            Type::Box(Box::new(type_args.into_iter().next().unwrap()))
                        }
                        "Rc" if type_args.len() == 1 => {
                            Type::Rc(Box::new(type_args.into_iter().next().unwrap()))
                        }
                        "Arc" if type_args.len() == 1 => {
                            Type::Arc(Box::new(type_args.into_iter().next().unwrap()))
                        }
                        "Task" if type_args.len() == 1 => {
                            Type::Task(Box::new(type_args.into_iter().next().unwrap()))
                        }
                        _ => Type::Generic(name, type_args),
                    }
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
            Some(Token::Match) => self.parse_match_stmt()?,
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
        } else {
            // Expression starting with identifier
            let ident_expr = Spanned::new(Expr::Ident(name), start..self.current_span().start);
            let expr = self.parse_expr_rest(ident_expr)?;
            self.eat(&Token::Semi)?;
            Ok(Stmt::Expr(expr))
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
            self.eat(&Token::LBrace)?;
            let block = self.parse_block()?;
            self.eat(&Token::RBrace)?;
            Some(block)
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_block,
            else_block,
        })
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
            vec![Spanned::new(Stmt::Expr(expr), 0..0)]
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

    fn parse_expr_rest(&mut self, mut expr: Spanned<Expr>) -> ParseResult<Spanned<Expr>> {
        loop {
            let start = expr.span.start;

            match self.current() {
                Some(Token::Dot) => {
                    self.advance();
                    let field = self.parse_ident()?;

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

                if is_type_name && self.check(&Token::Lt) {
                    self.advance();
                    let mut type_args = Vec::new();
                    while !self.check(&Token::Gt) && !self.is_at_end() {
                        let parsed_type = self.parse_type()?;
                        type_args.push(self.format_type(&parsed_type));
                        if self.check(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.eat(&Token::Gt)?;
                    // Build full generic name with actual type args
                    full_name = format!("{}<{}>", name, type_args.join(", "));
                }

                // Check if this is a constructor call
                if self.check(&Token::LParen) {
                    self.advance();
                    let args = self.parse_args()?;
                    self.eat(&Token::RParen)?;

                    // Check if name starts with uppercase AND doesn't contain '__' (module prefix)
                    // Also check if it has generic args (full_name contains '<')
                    let is_constructor =
                        (is_type_name && !name.contains("__")) || full_name.contains("<");

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
                if let Some(next) = chars.peek() {
                    if *next == '{' || *next == '}' {
                        current.push(chars.next().unwrap());
                        continue;
                    }
                }
                current.push(c);
            } else if c == '{' {
                if !current.is_empty() {
                    parts.push(StringPart::Literal(std::mem::take(&mut current)));
                }

                let mut expr_str = String::new();
                let mut depth = 1;
                for c in chars.by_ref() {
                    if c == '{' {
                        depth += 1;
                        expr_str.push(c);
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        expr_str.push(c);
                    } else {
                        expr_str.push(c);
                    }
                }

                // Parse the expression inside {}
                if expr_str.trim().is_empty() {
                    // Treat empty {} as literal empty string to avoid crash
                    parts.push(StringPart::Literal(String::new()));
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
