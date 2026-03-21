use crate::ast::{
    Attribute, BinOp, Block, Decl, EnumField, EnumVariant, Expr, FunctionDecl, GenericParam,
    InterfaceMethod, Literal, MatchArm, Parameter, Pattern, Program, Spanned, Stmt, StringPart,
    Type, UnaryOp, Visibility,
};
use crate::lexer;
use crate::parser::Parser;

pub fn format_source(source: &str) -> Result<String, String> {
    let shebang = source
        .lines()
        .next()
        .filter(|line| line.starts_with("#!"))
        .map(ToString::to_string);

    let tokens = lexer::tokenize(source).map_err(|e| format!("Lexer error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format!("Parse error: {}", e.message))?;

    let mut formatter = Formatter::with_comments(collect_comments(source));
    formatter.format_program(&program);
    let formatted = formatter.finish();

    if let Some(shebang) = shebang {
        Ok(format!("{}\n{}", shebang, formatted))
    } else {
        Ok(formatted)
    }
}

pub fn format_program_canonical(program: &Program) -> String {
    let mut formatter = Formatter::new();
    formatter.format_program(program);
    formatter.finish()
}

#[derive(Clone)]
struct SourceComment {
    start: usize,
    text: String,
}

struct Formatter {
    output: String,
    indent: usize,
    comments: Vec<SourceComment>,
    next_comment: usize,
}

impl Formatter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            comments: Vec::new(),
            next_comment: 0,
        }
    }

    fn with_comments(comments: Vec<SourceComment>) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            comments,
            next_comment: 0,
        }
    }

    fn finish(mut self) -> String {
        while self.output.ends_with('\n') {
            self.output.pop();
        }
        self.output.push('\n');
        self.output
    }

    fn format_program(&mut self, program: &Program) {
        let mut first = true;
        if let Some(package) = &program.package {
            self.push_line(&format!("package {};", package));
            first = false;
        }

        for decl in &program.declarations {
            self.emit_comments_before(decl.span.start);
            if !first {
                self.blank_line();
            }
            self.format_decl(decl);
            first = false;
        }
        self.emit_comments_before(usize::MAX);
    }

    fn format_decl(&mut self, decl: &Spanned<Decl>) {
        match &decl.node {
            Decl::Import(import) => {
                if let Some(alias) = &import.alias {
                    self.push_line(&format!("import {} as {};", import.path, alias))
                } else {
                    self.push_line(&format!("import {};", import.path))
                }
            }
            Decl::Function(function) => self.format_function(function),
            Decl::Class(class) => {
                self.push_line(&format!(
                    "{}class {}{}{}{} {{",
                    visibility_prefix(class.visibility),
                    class.name,
                    format_generic_params(&class.generic_params),
                    class
                        .extends
                        .as_ref()
                        .map(|name| format!(" extends {}", name))
                        .unwrap_or_default(),
                    if class.implements.is_empty() {
                        String::new()
                    } else {
                        format!(" implements {}", class.implements.join(", "))
                    }
                ));
                self.indent += 1;

                let mut first = true;
                for field in &class.fields {
                    if !first {
                        self.blank_line();
                    }
                    let mut line = String::new();
                    line.push_str(visibility_prefix(field.visibility));
                    if field.mutable {
                        line.push_str("mut ");
                    }
                    line.push_str(&field.name);
                    line.push_str(": ");
                    line.push_str(&self.format_type(&field.ty));
                    line.push(';');
                    self.push_line(&line);
                    first = false;
                }

                if let Some(constructor) = &class.constructor {
                    if !first {
                        self.blank_line();
                    }
                    self.push_line(&format!(
                        "constructor({}) {{",
                        format_params(&constructor.params)
                    ));
                    self.indent += 1;
                    self.format_block_contents(&constructor.body);
                    self.indent -= 1;
                    self.push_line("}");
                    first = false;
                }

                if let Some(destructor) = &class.destructor {
                    if !first {
                        self.blank_line();
                    }
                    self.push_line("destructor() {");
                    self.indent += 1;
                    self.format_block_contents(&destructor.body);
                    self.indent -= 1;
                    self.push_line("}");
                    first = false;
                }

                for method in &class.methods {
                    if !first {
                        self.blank_line();
                    }
                    self.format_function(method);
                    first = false;
                }

                self.indent -= 1;
                self.emit_comments_before(decl.span.end);
                self.push_line("}");
            }
            Decl::Enum(en) => {
                self.push_line(&format!(
                    "{}enum {}{} {{",
                    visibility_prefix(en.visibility),
                    en.name,
                    format_generic_params(&en.generic_params)
                ));
                self.indent += 1;
                for (index, variant) in en.variants.iter().enumerate() {
                    let mut line = self.format_enum_variant(variant);
                    if index + 1 != en.variants.len() {
                        line.push(',');
                    }
                    self.push_line(&line);
                }
                self.emit_comments_before(decl.span.end);
                self.indent -= 1;
                self.push_line("}");
            }
            Decl::Interface(interface) => {
                self.push_line(&format!(
                    "{}interface {}{}{} {{",
                    visibility_prefix(interface.visibility),
                    interface.name,
                    format_generic_params(&interface.generic_params),
                    if interface.extends.is_empty() {
                        String::new()
                    } else {
                        format!(" extends {}", interface.extends.join(", "))
                    }
                ));
                self.indent += 1;
                for (index, method) in interface.methods.iter().enumerate() {
                    if index > 0 {
                        self.blank_line();
                    }
                    self.format_interface_method(method);
                }
                self.emit_comments_before(decl.span.end);
                self.indent -= 1;
                self.push_line("}");
            }
            Decl::Module(module) => {
                self.push_line(&format!("module {} {{", module.name));
                self.indent += 1;
                for (index, decl) in module.declarations.iter().enumerate() {
                    if index > 0 {
                        self.blank_line();
                    }
                    self.emit_comments_before(decl.span.start);
                    self.format_decl(decl);
                }
                self.indent -= 1;
                self.emit_comments_before(decl.span.end);
                self.push_line("}");
            }
        }
    }

    fn format_interface_method(&mut self, method: &InterfaceMethod) {
        let signature = format!(
            "function {}({}): {}",
            method.name,
            format_params(&method.params),
            self.format_type(&method.return_type)
        );
        if let Some(default_impl) = &method.default_impl {
            self.push_line(&format!("{} {{", signature));
            self.indent += 1;
            self.format_block_contents(default_impl);
            self.indent -= 1;
            self.push_line("}");
        } else {
            self.push_line(&format!("{};", signature));
        }
    }

    fn format_enum_variant(&self, variant: &EnumVariant) -> String {
        if variant.fields.is_empty() {
            return variant.name.clone();
        }

        let fields = variant
            .fields
            .iter()
            .map(format_enum_field)
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({})", variant.name, fields)
    }

    fn format_function(&mut self, function: &FunctionDecl) {
        for attribute in &function.attributes {
            self.push_line(&format_attribute(attribute));
        }

        let mut signature = String::new();
        signature.push_str(visibility_prefix(function.visibility));

        if function.is_async {
            signature.push_str("async ");
        }

        if function.is_extern {
            signature.push_str("extern");
            if let Some(abi) = &function.extern_abi {
                signature.push('(');
                signature.push_str(abi);
                if let Some(link_name) = &function.extern_link_name {
                    signature.push_str(", ");
                    signature.push_str(&self.format_string_literal(link_name));
                }
                signature.push(')');
            }
            signature.push(' ');
        }

        signature.push_str("function ");
        signature.push_str(&function.name);
        signature.push_str(&format_generic_params(&function.generic_params));
        signature.push('(');
        signature.push_str(&format_params(&function.params));
        if function.is_variadic {
            if !function.params.is_empty() {
                signature.push_str(", ");
            }
            signature.push_str("...");
        }
        signature.push(')');
        signature.push_str(": ");
        signature.push_str(&self.format_type(&function.return_type));

        if function.is_extern {
            signature.push(';');
            self.push_line(&signature);
            return;
        }

        self.push_line(&format!("{} {{", signature));
        self.indent += 1;
        self.format_block_contents(&function.body);
        self.indent -= 1;
        self.push_line("}");
    }

    fn format_block_contents(&mut self, block: &Block) {
        for stmt in block {
            self.emit_comments_before(stmt.span.start);
            self.format_stmt(stmt);
        }
    }

    fn format_stmt(&mut self, stmt: &Spanned<Stmt>) {
        match &stmt.node {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
            } => {
                let prefix = if *mutable { "mut " } else { "" };
                self.push_line(&format!(
                    "{}{}: {} = {};",
                    prefix,
                    name,
                    self.format_type(ty),
                    self.format_expr(&value.node)
                ));
            }
            Stmt::Assign { target, value } => self.push_line(&format!(
                "{} = {};",
                self.format_expr(&target.node),
                self.format_expr(&value.node)
            )),
            Stmt::Expr(expr) => {
                let formatted = self.format_expr(&expr.node);
                if matches!(expr.node, Expr::Match { .. } | Expr::IfExpr { .. }) {
                    self.push_line(&format!("({});", formatted));
                } else {
                    self.push_line(&format!("{};", formatted));
                }
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.push_line(&format!("return {};", self.format_expr(&expr.node)));
                } else {
                    self.push_line("return;");
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.push_line(&format!("if ({}) {{", self.format_expr(&condition.node)));
                self.indent += 1;
                self.format_block_contents(then_block);
                self.indent -= 1;
                if let Some(else_block) = else_block {
                    self.format_else_tail(else_block);
                } else {
                    self.push_line("}");
                }
            }
            Stmt::While { condition, body } => {
                self.push_line(&format!("while ({}) {{", self.format_expr(&condition.node)));
                self.indent += 1;
                self.format_block_contents(body);
                self.indent -= 1;
                self.push_line("}");
            }
            Stmt::For {
                var,
                var_type,
                iterable,
                body,
            } => {
                let mut header = format!("for ({}", var);
                if let Some(var_type) = var_type {
                    header.push_str(": ");
                    header.push_str(&self.format_type(var_type));
                }
                header.push_str(" in ");
                header.push_str(&self.format_expr(&iterable.node));
                header.push_str(") {");
                self.push_line(&header);
                self.indent += 1;
                self.format_block_contents(body);
                self.indent -= 1;
                self.push_line("}");
            }
            Stmt::Break => self.push_line("break;"),
            Stmt::Continue => self.push_line("continue;"),
            Stmt::Match { expr, arms } => {
                self.push_line(&format!("match ({}) {{", self.format_expr(&expr.node)));
                self.indent += 1;
                for arm in arms {
                    self.push_line(&format!("{} => {{", self.format_pattern(&arm.pattern)));
                    self.indent += 1;
                    self.format_block_contents(&arm.body);
                    self.indent -= 1;
                    self.push_line("}");
                }
                self.indent -= 1;
                self.push_line("}");
            }
        }
        self.emit_comments_before(stmt.span.end);
    }

    fn format_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Literal(literal) => self.format_literal(literal),
            Pattern::Ident(name) => name.clone(),
            Pattern::Variant(name, bindings) => {
                if bindings.is_empty() {
                    name.clone()
                } else {
                    format!("{}({})", name, bindings.join(", "))
                }
            }
        }
    }

    fn format_expr(&self, expr: &Expr) -> String {
        self.format_expr_with_prec(expr, 0)
    }

    fn format_expr_with_prec(&self, expr: &Expr, parent_prec: u8) -> String {
        match expr {
            Expr::Literal(literal) => self.format_literal(literal),
            Expr::Ident(name) => name.clone(),
            Expr::Binary { op, left, right } => {
                let prec = op.precedence();
                let left_str = self.format_expr_with_prec(&left.node, prec);
                let right_prec = if matches!(op, BinOp::Sub | BinOp::Div | BinOp::Mod) {
                    prec + 1
                } else {
                    prec
                };
                let right_str = self.format_expr_with_prec(&right.node, right_prec);
                let formatted = format!("{} {} {}", left_str, format_binop(*op), right_str);
                if prec < parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::Unary { op, expr } => {
                let inner = self.format_expr_with_prec(&expr.node, 8);
                let formatted = format!("{}{}", format_unary_op(*op), inner);
                if 8 < parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::Call {
                callee,
                args,
                type_args,
            } => format!(
                "{}{}({})",
                self.format_expr_with_prec(&callee.node, 9),
                if type_args.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<{}>",
                        type_args
                            .iter()
                            .map(|ty| self.format_type(ty))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                },
                args.iter()
                    .map(|arg| self.format_expr(&arg.node))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expr::Field { object, field } => {
                format!("{}.{}", self.format_expr_with_prec(&object.node, 9), field)
            }
            Expr::Index { object, index } => format!(
                "{}[{}]",
                self.format_expr_with_prec(&object.node, 9),
                self.format_expr(&index.node)
            ),
            Expr::Construct { ty, args } => format!(
                "{}({})",
                ty,
                args.iter()
                    .map(|arg| self.format_expr(&arg.node))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expr::Lambda { params, body } => {
                let body = self.format_expr(&body.node);
                let formatted = format!("({}) => {}", format_params(params), body);
                if 9 <= parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::This => "this".to_string(),
            Expr::Match { expr, arms } => {
                let arms = arms
                    .iter()
                    .map(|arm| self.format_match_arm_inline(arm))
                    .collect::<Vec<_>>()
                    .join(", ");
                let formatted = format!("match ({}) {{ {} }}", self.format_expr(&expr.node), arms);
                if 9 <= parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::StringInterp(parts) => self.format_string_interp(parts),
            Expr::Try(expr) => {
                let formatted = format!("{}?", self.format_expr_with_prec(&expr.node, 8));
                if 8 < parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::Borrow(expr) => {
                let formatted = format!("&{}", self.format_expr_with_prec(&expr.node, 8));
                if 8 < parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::MutBorrow(expr) => {
                let formatted = format!("&mut {}", self.format_expr_with_prec(&expr.node, 8));
                if 8 < parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::Deref(expr) => {
                let formatted = format!("*{}", self.format_expr_with_prec(&expr.node, 8));
                if 8 < parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::Await(expr) => {
                let formatted = format!("await {}", self.format_expr_with_prec(&expr.node, 8));
                if 8 < parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::AsyncBlock(block) => {
                let mut nested = Formatter::new();
                nested.indent = 1;
                nested.format_block_contents(block);
                let body = nested.finish();
                let body = body.trim_end_matches('\n');
                let formatted = format!("async {{\n{}\n}}", body);
                if 9 <= parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::Require { condition, message } => {
                let mut parts = vec![self.format_expr(&condition.node)];
                if let Some(message) = message {
                    parts.push(self.format_expr(&message.node));
                }
                let formatted = format!("require({})", parts.join(", "));
                if 8 < parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::Range {
                start,
                end,
                inclusive,
            } => {
                let sep = if *inclusive { "..=" } else { ".." };
                let formatted = format!(
                    "{}{}{}",
                    start
                        .as_ref()
                        .map(|expr| self.format_expr(&expr.node))
                        .unwrap_or_default(),
                    sep,
                    end.as_ref()
                        .map(|expr| self.format_expr(&expr.node))
                        .unwrap_or_default()
                );
                if 8 < parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                let mut formatted = format!(
                    "if ({}) {}",
                    self.format_expr(&condition.node),
                    self.format_inline_block(then_branch)
                );
                if let Some(else_branch) = else_branch {
                    formatted.push_str(" else ");
                    if let [nested] = else_branch.as_slice() {
                        if let Stmt::Expr(expr) = &nested.node {
                            if let Expr::IfExpr { .. } = &expr.node {
                                formatted.push_str(&self.format_expr(&expr.node));
                            } else {
                                formatted.push_str(&self.format_inline_block(else_branch));
                            }
                        } else {
                            formatted.push_str(&self.format_inline_block(else_branch));
                        }
                    } else {
                        formatted.push_str(&self.format_inline_block(else_branch));
                    }
                }
                if 9 <= parent_prec {
                    format!("({})", formatted)
                } else {
                    formatted
                }
            }
            Expr::Block(block) => self.format_inline_block(block),
        }
    }

    fn format_match_arm_inline(&self, arm: &MatchArm) -> String {
        format!(
            "{} => {}",
            self.format_pattern(&arm.pattern),
            self.format_inline_block(&arm.body)
        )
    }

    fn format_else_tail(&mut self, else_block: &Block) {
        if let [nested] = else_block.as_slice() {
            if let Stmt::If {
                condition,
                then_block,
                else_block,
            } = &nested.node
            {
                self.push_line(&format!(
                    "}} else if ({}) {{",
                    self.format_expr(&condition.node)
                ));
                self.indent += 1;
                self.format_block_contents(then_block);
                self.indent -= 1;
                if let Some(else_block) = else_block {
                    self.format_else_tail(else_block);
                } else {
                    self.push_line("}");
                }
                return;
            }
        }

        self.push_line("} else {");
        self.indent += 1;
        self.format_block_contents(else_block);
        self.indent -= 1;
        self.push_line("}");
    }

    fn format_inline_block(&self, block: &Block) -> String {
        if block.is_empty() {
            return "{ }".to_string();
        }

        let mut nested = Formatter::new();
        nested.indent = 1;
        nested.format_block_contents(block);
        let body = nested.finish();
        let lines = body.trim_end_matches('\n');
        format!("{{\n{}\n}}", lines)
    }

    fn format_string_interp(&self, parts: &[StringPart]) -> String {
        let mut result = String::from("\"");
        for part in parts {
            match part {
                StringPart::Literal(value) => result.push_str(&escape_string_contents(value)),
                StringPart::Expr(expr) => {
                    result.push('{');
                    result.push_str(&self.format_expr(&expr.node));
                    result.push('}');
                }
            }
        }
        result.push('"');
        result
    }

    fn format_literal(&self, literal: &Literal) -> String {
        match literal {
            Literal::Integer(value) => value.to_string(),
            Literal::Float(value) => {
                let mut formatted = value.to_string();
                if !formatted.contains('.') {
                    formatted.push_str(".0");
                }
                formatted
            }
            Literal::Boolean(value) => value.to_string(),
            Literal::String(value) => self.format_string_literal(value),
            Literal::Char(value) => format!("'{}'", escape_char(*value)),
            Literal::None => "None".to_string(),
        }
    }

    fn format_string_literal(&self, value: &str) -> String {
        format!("\"{}\"", escape_string_contents(value))
    }

    fn format_type(&self, ty: &Type) -> String {
        match ty {
            Type::Integer => "Integer".to_string(),
            Type::Float => "Float".to_string(),
            Type::Boolean => "Boolean".to_string(),
            Type::String => "String".to_string(),
            Type::Char => "Char".to_string(),
            Type::None => "None".to_string(),
            Type::Named(name) => name.clone(),
            Type::Generic(name, args) => format!(
                "{}<{}>",
                name,
                args.iter()
                    .map(|arg| self.format_type(arg))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::Function(params, ret) => format!(
                "({}) -> {}",
                params
                    .iter()
                    .map(|param| self.format_type(param))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.format_type(ret)
            ),
            Type::Option(inner) => format!("Option<{}>", self.format_type(inner)),
            Type::Result(ok, err) => {
                format!(
                    "Result<{}, {}>",
                    self.format_type(ok),
                    self.format_type(err)
                )
            }
            Type::List(inner) => format!("List<{}>", self.format_type(inner)),
            Type::Map(key, value) => {
                format!(
                    "Map<{}, {}>",
                    self.format_type(key),
                    self.format_type(value)
                )
            }
            Type::Set(inner) => format!("Set<{}>", self.format_type(inner)),
            Type::Ref(inner) => format!("&{}", self.format_type(inner)),
            Type::MutRef(inner) => format!("&mut {}", self.format_type(inner)),
            Type::Box(inner) => format!("Box<{}>", self.format_type(inner)),
            Type::Rc(inner) => format!("Rc<{}>", self.format_type(inner)),
            Type::Arc(inner) => format!("Arc<{}>", self.format_type(inner)),
            Type::Ptr(inner) => format!("Ptr<{}>", self.format_type(inner)),
            Type::Task(inner) => format!("Task<{}>", self.format_type(inner)),
            Type::Range(inner) => format!("Range<{}>", self.format_type(inner)),
        }
    }

    fn push_line(&mut self, line: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn blank_line(&mut self) {
        if !self.output.ends_with("\n\n") {
            self.output.push('\n');
        }
    }

    fn emit_comments_before(&mut self, offset: usize) {
        while let Some(comment) = self.comments.get(self.next_comment) {
            if comment.start >= offset {
                break;
            }
            let comment = comment.clone();
            self.emit_comment(&comment);
            self.next_comment += 1;
        }
    }

    fn emit_comment(&mut self, comment: &SourceComment) {
        let trimmed = comment.text.trim();
        if trimmed.is_empty() {
            return;
        }
        for line in trimmed.lines() {
            self.push_line(line.trim_end());
        }
    }
}

fn format_enum_field(field: &EnumField) -> String {
    match &field.name {
        Some(name) => format!("{}: {}", name, format_type(field.ty.clone())),
        None => format_type(field.ty.clone()),
    }
}

fn format_type(ty: Type) -> String {
    Formatter::new().format_type(&ty)
}

fn format_params(params: &[Parameter]) -> String {
    params
        .iter()
        .map(format_param)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_param(param: &Parameter) -> String {
    let mut text = String::new();
    match param.mode {
        crate::ast::ParamMode::Owned => {
            if param.mutable {
                text.push_str("mut ");
            }
        }
        crate::ast::ParamMode::Borrow => text.push_str("borrow "),
        crate::ast::ParamMode::BorrowMut => text.push_str("borrow mut "),
    }
    text.push_str(&param.name);
    text.push_str(": ");
    text.push_str(&format_type(param.ty.clone()));
    text
}

fn format_generic_params(params: &[GenericParam]) -> String {
    if params.is_empty() {
        return String::new();
    }

    let inner = params
        .iter()
        .map(|param| {
            if param.bounds.is_empty() {
                param.name.clone()
            } else {
                format!("{} extends {}", param.name, param.bounds.join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{}>", inner)
}

fn format_attribute(attribute: &Attribute) -> String {
    match attribute {
        Attribute::Test => "@Test".to_string(),
        Attribute::Ignore(None) => "@Ignore".to_string(),
        Attribute::Ignore(Some(reason)) => {
            format!("@Ignore(\"{}\")", escape_string_contents(reason))
        }
        Attribute::Before => "@Before".to_string(),
        Attribute::After => "@After".to_string(),
        Attribute::BeforeAll => "@BeforeAll".to_string(),
        Attribute::AfterAll => "@AfterAll".to_string(),
        Attribute::Pure => "@Pure".to_string(),
        Attribute::EffectIo => "@Io".to_string(),
        Attribute::EffectNet => "@Net".to_string(),
        Attribute::EffectAlloc => "@Alloc".to_string(),
        Attribute::EffectUnsafe => "@Unsafe".to_string(),
        Attribute::EffectThread => "@Thread".to_string(),
        Attribute::EffectAny => "@Any".to_string(),
    }
}

fn visibility_prefix(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Private => "",
        Visibility::Protected => "protected ",
        Visibility::Public => "public ",
    }
}

fn format_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::NotEq => "!=",
        BinOp::Lt => "<",
        BinOp::LtEq => "<=",
        BinOp::Gt => ">",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

fn format_unary_op(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}

fn escape_string_contents(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            other => vec![other],
        })
        .collect()
}

fn escape_char(value: char) -> String {
    match value {
        '\\' => "\\\\".to_string(),
        '\'' => "\\'".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        other => other.to_string(),
    }
}

fn collect_comments(source: &str) -> Vec<SourceComment> {
    let bytes = source.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_char = false;
    let mut comments = Vec::new();

    while i + 1 < bytes.len() {
        let current = bytes[i];
        let next = bytes[i + 1];

        if in_string {
            if current == b'\\' {
                i += 2;
                continue;
            }
            if current == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if in_char {
            if current == b'\\' {
                i += 2;
                continue;
            }
            if current == b'\'' {
                in_char = false;
            }
            i += 1;
            continue;
        }

        if current == b'"' {
            in_string = true;
            i += 1;
            continue;
        }

        if current == b'\'' {
            in_char = true;
            i += 1;
            continue;
        }

        if current == b'/' && next == b'/' {
            let start = i;
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            comments.push(SourceComment {
                start,
                text: source[start..i].to_string(),
            });
            continue;
        }

        if current == b'/' && next == b'*' {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            comments.push(SourceComment {
                start,
                text: source[start..i].to_string(),
            });
            continue;
        }

        i += 1;
    }

    comments
}

#[cfg(test)]
mod tests {
    use super::format_source;
    use crate::lexer::tokenize;
    use crate::parser::Parser;

    #[test]
    fn formats_basic_program() {
        let source = r#"package app;
import std.io.*;
function main(): None {mut value: Integer=1+2*3;println("hi {value}");return None;}"#;

        let formatted = format_source(source).expect("format succeeds");

        assert_eq!(
            formatted,
            concat!(
                "package app;\n",
                "\n",
                "import std.io.*;\n",
                "\n",
                "public function main(): None {\n",
                "    mut value: Integer = 1 + 2 * 3;\n",
                "    println(\"hi {value}\");\n",
                "    return None;\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_extern_and_generics() {
        let source = r#"extern(c,"puts") function c_puts(msg:String): Integer;function id<T>(value:T): T {return value;}"#;
        let formatted = format_source(source).expect("format succeeds");

        assert!(formatted.contains("extern(c, \"puts\") function c_puts(msg: String): Integer;"));
        assert!(formatted.contains("public function id<T>(value: T): T {"));
    }

    #[test]
    fn keeps_comments_in_output() {
        let source = "// note\nfunction main(): None { return None; }";
        let formatted = format_source(source).expect("comments should be preserved");
        assert!(formatted.contains("// note"));
    }

    #[test]
    fn wraps_match_expression_statements_for_roundtrip() {
        let source = r#"
function main(): None {
    x: Integer = match (1) {
        1 => { (match (2) { 2 => { 3; }, _ => { 4; } }); },
        _ => { 0; }
    };
    return None;
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.contains("(match (2)"));
        let tokens = tokenize(&formatted).expect("formatted output should lex");
        let mut parser = Parser::new(tokens);
        parser
            .parse_program()
            .expect("formatted output should parse");
    }

    #[test]
    fn wraps_if_expression_statements_for_roundtrip() {
        let source = r#"
function main(): None {
    x: Integer = 0;
    (if (true) { 1; } else { 2; });
    return None;
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.contains("(if (true)"));
        let tokens = tokenize(&formatted).expect("formatted output should lex");
        let mut parser = Parser::new(tokens);
        parser
            .parse_program()
            .expect("formatted output should parse");
    }

    #[test]
    fn preserves_shebang_line() {
        let source = r#"#!/usr/bin/env apex
function main(): None { return None; }
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.starts_with("#!/usr/bin/env apex\n"));
    }

    #[test]
    fn formats_else_if_statement_chain() {
        let source = r#"
function main(): None {
    if (true) { return None; } else { if (false) { return None; } else { return None; } }
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.contains("} else if (false) {"), "{formatted}");
    }

    #[test]
    fn formats_else_if_expression_chain() {
        let source = r#"
function main(): None {
    x: Integer = if (true) { 1; } else if (false) { 2; } else { 3; };
    return None;
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.contains("else if (false)"), "{formatted}");
    }

    #[test]
    fn wraps_lambda_callee_for_roundtrip() {
        let source = r#"
function main(): None {
    y: Integer = ((x: Integer) => x + 1)(2);
    return None;
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(
            formatted.contains("((x: Integer) => x + 1)(2)"),
            "{formatted}"
        );
        let tokens = tokenize(&formatted).expect("formatted output should lex");
        let mut parser = Parser::new(tokens);
        parser
            .parse_program()
            .expect("formatted output should parse");
    }

    #[test]
    fn wraps_if_expression_callee_for_roundtrip() {
        let source = r#"
function main(): None {
    y: Integer = (if (true) { foo; } else { bar; })(2);
    return None;
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.contains("(if (true)"), "{formatted}");
        let tokens = tokenize(&formatted).expect("formatted output should lex");
        let mut parser = Parser::new(tokens);
        parser
            .parse_program()
            .expect("formatted output should parse");
    }

    #[test]
    fn wraps_match_expression_callee_for_roundtrip() {
        let source = r#"
function main(): None {
    y: Integer = (match (1) { 1 => { foo; }, _ => { bar; } })(2);
    return None;
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.contains("(match (1)"), "{formatted}");
        let tokens = tokenize(&formatted).expect("formatted output should lex");
        let mut parser = Parser::new(tokens);
        parser
            .parse_program()
            .expect("formatted output should parse");
    }

    #[test]
    fn wraps_deref_callee_for_roundtrip() {
        let source = r#"
function main(): None {
    y: Integer = (*f)(2);
    return None;
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.contains("(*f)(2)"), "{formatted}");
        let tokens = tokenize(&formatted).expect("formatted output should lex");
        let mut parser = Parser::new(tokens);
        parser
            .parse_program()
            .expect("formatted output should parse");
    }

    #[test]
    fn wraps_try_callee_for_roundtrip() {
        let source = r#"
function main(): Result<None, String> {
    y: Integer = (choose()?)(2);
    return Result.ok(None);
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.contains("(choose()?)(2)"), "{formatted}");
        let tokens = tokenize(&formatted).expect("formatted output should lex");
        let mut parser = Parser::new(tokens);
        parser
            .parse_program()
            .expect("formatted output should parse");
    }

    #[test]
    fn formats_borrow_mut_params_in_parser_order() {
        let source = r#"
function f(borrow mut value: String): None {
    return None;
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(
            formatted.contains("borrow mut value: String"),
            "{formatted}"
        );
        let tokens = tokenize(&formatted).expect("formatted output should lex");
        let mut parser = Parser::new(tokens);
        parser
            .parse_program()
            .expect("formatted borrow-mut params should parse");
    }

    #[test]
    fn formats_multiple_generic_bounds_with_commas() {
        let source = r#"
function f<T extends A, B>(value: T): None {
    return None;
}
"#;
        let formatted = format_source(source).expect("format succeeds");
        assert!(formatted.contains("T extends A, B"), "{formatted}");
        let tokens = tokenize(&formatted).expect("formatted output should lex");
        let mut parser = Parser::new(tokens);
        parser
            .parse_program()
            .expect("formatted generic bounds should parse");
    }
}
