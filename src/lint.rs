use crate::ast::{Decl, Expr, ImportDecl, Parameter, Program, Span, Stmt, Type};
use crate::lexer;
use crate::parser::Parser;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintLevel {
    Warning,
}

#[derive(Debug, Clone)]
pub struct LintFinding {
    pub code: &'static str,
    pub level: LintLevel,
    pub message: String,
    pub suggestion: Option<String>,
    pub span: Option<Span>,
}

impl LintFinding {
    pub fn format(&self) -> String {
        let level = match self.level {
            LintLevel::Warning => "warning",
        };
        let mut out = format!("[{}] {}: {}", self.code, level, self.message);
        if let Some(span) = &self.span {
            out.push_str(&format!(" @{}..{}", span.start, span.end));
        }
        if let Some(suggestion) = &self.suggestion {
            out.push_str(&format!("\n  hint: {}", suggestion));
        }
        out
    }
}

pub struct LintResult {
    pub findings: Vec<LintFinding>,
    pub fixed_source: Option<String>,
}

pub fn lint_source(source: &str, apply_fixes: bool) -> Result<LintResult, String> {
    let tokens = lexer::tokenize(source).map_err(|e| format!("Lexer error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format!("Parse error: {}", e.message))?;

    let mut findings = Vec::new();
    findings.extend(check_duplicate_imports(&program));
    findings.extend(check_import_sorting(&program));
    findings.extend(check_unused_specific_imports(&program));
    findings.extend(check_unused_variables(&program));
    findings.extend(check_shadowed_variables(&program));

    let fixed_source = if apply_fixes {
        Some(apply_safe_import_fixes(source, &program))
    } else {
        None
    };

    Ok(LintResult {
        findings,
        fixed_source,
    })
}

fn check_duplicate_imports(program: &Program) -> Vec<LintFinding> {
    let mut seen = HashSet::new();
    let mut duplicates = BTreeSet::new();

    for decl in &program.declarations {
        if let Decl::Import(import) = &decl.node {
            let key = import_identity(import);
            if !seen.insert(key.clone()) {
                duplicates.insert(key);
            }
        }
    }

    duplicates
        .into_iter()
        .map(|path| LintFinding {
            code: "L001",
            level: LintLevel::Warning,
            message: format!("duplicate import '{}'", path),
            suggestion: Some("remove the redundant import".to_string()),
            span: None,
        })
        .collect()
}

fn check_import_sorting(program: &Program) -> Vec<LintFinding> {
    let imports: Vec<String> = program
        .declarations
        .iter()
        .filter_map(|decl| match &decl.node {
            Decl::Import(import) => Some(import_identity(import)),
            _ => None,
        })
        .collect();

    if imports.len() < 2 {
        return Vec::new();
    }

    let mut sorted = imports.clone();
    sorted.sort();
    sorted.dedup();

    if imports == sorted {
        Vec::new()
    } else {
        vec![LintFinding {
            code: "L002",
            level: LintLevel::Warning,
            message: "imports are not sorted and deduplicated".to_string(),
            suggestion: Some("run `apex fix` or sort imports lexicographically".to_string()),
            span: None,
        }]
    }
}

fn check_unused_specific_imports(program: &Program) -> Vec<LintFinding> {
    let mut used_names = HashSet::new();
    collect_used_names(program, &mut used_names);

    let mut findings = Vec::new();
    for decl in &program.declarations {
        let Decl::Import(import) = &decl.node else {
            continue;
        };
        if import.path.ends_with(".*") {
            continue;
        }

        let Some(imported_name) = import.path.rsplit('.').next() else {
            continue;
        };
        let binding_name = import.alias.as_deref().unwrap_or(imported_name);

        if !used_names.contains(binding_name) {
            findings.push(LintFinding {
                code: "L003",
                level: LintLevel::Warning,
                message: format!(
                    "specific import '{}' appears unused",
                    import_identity(import)
                ),
                suggestion: Some(
                    "remove it or switch to a wildcard import only if justified".to_string(),
                ),
                span: Some(decl.span.clone()),
            });
        }
    }

    findings
}

fn check_unused_variables(program: &Program) -> Vec<LintFinding> {
    let mut findings = Vec::new();

    for decl in &program.declarations {
        match &decl.node {
            Decl::Function(func) => {
                findings.extend(check_unused_variables_in_block(&func.body));
            }
            Decl::Class(class) => {
                if let Some(ctor) = &class.constructor {
                    findings.extend(check_unused_variables_in_block(&ctor.body));
                }
                if let Some(dtor) = &class.destructor {
                    findings.extend(check_unused_variables_in_block(&dtor.body));
                }
                for method in &class.methods {
                    findings.extend(check_unused_variables_in_block(&method.body));
                }
            }
            Decl::Module(module) => {
                let nested = Program {
                    package: None,
                    declarations: module.declarations.clone(),
                };
                findings.extend(check_unused_variables(&nested));
            }
            Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }

    findings
}

fn check_unused_variables_in_block(block: &[crate::ast::Spanned<Stmt>]) -> Vec<LintFinding> {
    let mut declared: Vec<(String, Span)> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();

    collect_declared_and_used_in_block(block, &mut declared, &mut used);

    declared
        .into_iter()
        .filter(|(name, _)| !name.starts_with('_') && !used.contains(name))
        .map(|(name, span)| LintFinding {
            code: "L004",
            level: LintLevel::Warning,
            message: format!("Variable '{}' is declared but never used", name),
            suggestion: Some("remove it or prefix it with '_' if intentional".to_string()),
            span: Some(span),
        })
        .collect()
}

fn collect_declared_and_used_in_block(
    block: &[crate::ast::Spanned<Stmt>],
    declared: &mut Vec<(String, Span)>,
    used: &mut HashSet<String>,
) {
    for stmt in block {
        collect_declared_and_used_in_stmt(stmt, declared, used);
    }
}

fn collect_declared_and_used_in_stmt(
    stmt: &crate::ast::Spanned<Stmt>,
    declared: &mut Vec<(String, Span)>,
    used: &mut HashSet<String>,
) {
    match &stmt.node {
        Stmt::Let { name, value, .. } => {
            declared.push((name.clone(), stmt.span.clone()));
            collect_expr_idents(&value.node, used);
        }
        Stmt::Assign { target, value } => {
            collect_expr_idents(&target.node, used);
            collect_expr_idents(&value.node, used);
        }
        Stmt::Expr(expr) => collect_expr_idents(&expr.node, used),
        Stmt::Return(expr) => {
            if let Some(expr) = expr {
                collect_expr_idents(&expr.node, used);
            }
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_expr_idents(&condition.node, used);
            collect_declared_and_used_in_block(then_block, declared, used);
            if let Some(else_block) = else_block {
                collect_declared_and_used_in_block(else_block, declared, used);
            }
        }
        Stmt::While { condition, body } => {
            collect_expr_idents(&condition.node, used);
            collect_declared_and_used_in_block(body, declared, used);
        }
        Stmt::For {
            var,
            iterable,
            body,
            ..
        } => {
            declared.push((var.clone(), stmt.span.clone()));
            collect_expr_idents(&iterable.node, used);
            collect_declared_and_used_in_block(body, declared, used);
        }
        Stmt::Match { expr, arms } => {
            collect_expr_idents(&expr.node, used);
            for arm in arms {
                collect_declared_and_used_in_block(&arm.body, declared, used);
            }
        }
        Stmt::Break | Stmt::Continue => {}
    }
}

fn collect_expr_idents(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Ident(name) => {
            used.insert(name.clone());
        }
        Expr::Call { callee, args, .. } => {
            collect_expr_idents(&callee.node, used);
            for arg in args {
                collect_expr_idents(&arg.node, used);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_idents(&left.node, used);
            collect_expr_idents(&right.node, used);
        }
        Expr::Unary { expr, .. }
        | Expr::Try(expr)
        | Expr::Borrow(expr)
        | Expr::MutBorrow(expr)
        | Expr::Deref(expr)
        | Expr::Await(expr) => collect_expr_idents(&expr.node, used),
        Expr::Field { object, .. } => collect_expr_idents(&object.node, used),
        Expr::Index { object, index } => {
            collect_expr_idents(&object.node, used);
            collect_expr_idents(&index.node, used);
        }
        Expr::Construct { args, .. } => {
            for arg in args {
                collect_expr_idents(&arg.node, used);
            }
        }
        Expr::Lambda { body, .. } => collect_expr_idents(&body.node, used),
        Expr::Match { expr, arms } => {
            collect_expr_idents(&expr.node, used);
            for arm in arms {
                for stmt in &arm.body {
                    collect_declared_and_used_in_stmt(stmt, &mut Vec::new(), used);
                }
            }
        }
        Expr::StringInterp(parts) => {
            for part in parts {
                if let crate::ast::StringPart::Expr(expr) = part {
                    collect_expr_idents(&expr.node, used);
                }
            }
        }
        Expr::AsyncBlock(block) | Expr::Block(block) => {
            for stmt in block {
                collect_declared_and_used_in_stmt(stmt, &mut Vec::new(), used);
            }
        }
        Expr::Require { condition, message } => {
            collect_expr_idents(&condition.node, used);
            if let Some(message) = message {
                collect_expr_idents(&message.node, used);
            }
        }
        Expr::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_expr_idents(&start.node, used);
            }
            if let Some(end) = end {
                collect_expr_idents(&end.node, used);
            }
        }
        Expr::IfExpr {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_idents(&condition.node, used);
            for stmt in then_branch {
                collect_declared_and_used_in_stmt(stmt, &mut Vec::new(), used);
            }
            if let Some(else_branch) = else_branch {
                for stmt in else_branch {
                    collect_declared_and_used_in_stmt(stmt, &mut Vec::new(), used);
                }
            }
        }
        Expr::Literal(_) | Expr::This => {}
    }
}

fn check_shadowed_variables(program: &Program) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for decl in &program.declarations {
        match &decl.node {
            Decl::Function(func) => {
                let mut scopes = vec![
                    scope_with_params(&func.params),
                    HashMap::<String, Span>::new(),
                ];
                check_shadowed_in_block(&func.body, &mut scopes, &mut findings);
            }
            Decl::Class(class) => {
                if let Some(ctor) = &class.constructor {
                    let mut scopes = vec![
                        scope_with_params(&ctor.params),
                        HashMap::<String, Span>::new(),
                    ];
                    check_shadowed_in_block(&ctor.body, &mut scopes, &mut findings);
                }
                if let Some(dtor) = &class.destructor {
                    let mut scopes = vec![HashMap::<String, Span>::new()];
                    check_shadowed_in_block(&dtor.body, &mut scopes, &mut findings);
                }
                for method in &class.methods {
                    let mut scopes = vec![
                        scope_with_params(&method.params),
                        HashMap::<String, Span>::new(),
                    ];
                    check_shadowed_in_block(&method.body, &mut scopes, &mut findings);
                }
            }
            Decl::Module(module) => {
                let nested = Program {
                    package: None,
                    declarations: module.declarations.clone(),
                };
                findings.extend(check_shadowed_variables(&nested));
            }
            Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }
    findings
}

fn scope_with_params(params: &[Parameter]) -> HashMap<String, Span> {
    let mut scope = HashMap::new();
    for param in params {
        // Parameter spans are not tracked separately in the AST.
        scope.insert(param.name.clone(), 0..0);
    }
    scope
}

fn check_shadowed_in_block(
    block: &[crate::ast::Spanned<Stmt>],
    scopes: &mut Vec<HashMap<String, Span>>,
    findings: &mut Vec<LintFinding>,
) {
    for stmt in block {
        match &stmt.node {
            Stmt::Let { name, .. } => {
                if let Some(parent_span) = scopes
                    .iter()
                    .rev()
                    .skip(1)
                    .find_map(|scope| scope.get(name))
                {
                    findings.push(LintFinding {
                        code: "L005",
                        level: LintLevel::Warning,
                        message: format!(
                            "Variable '{}' shadows an outer variable declared at offset {}",
                            name, parent_span.start
                        ),
                        suggestion: Some("rename inner variable for clarity".to_string()),
                        span: Some(stmt.span.clone()),
                    });
                }
                if let Some(current) = scopes.last_mut() {
                    current.insert(name.clone(), stmt.span.clone());
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                scopes.push(HashMap::new());
                check_shadowed_in_block(then_block, scopes, findings);
                scopes.pop();
                if let Some(block) = else_block {
                    scopes.push(HashMap::new());
                    check_shadowed_in_block(block, scopes, findings);
                    scopes.pop();
                }
            }
            Stmt::While { body, .. } => {
                scopes.push(HashMap::new());
                check_shadowed_in_block(body, scopes, findings);
                scopes.pop();
            }
            Stmt::For { var, body, .. } => {
                scopes.push(HashMap::new());
                if let Some(parent_span) =
                    scopes.iter().rev().skip(1).find_map(|scope| scope.get(var))
                {
                    findings.push(LintFinding {
                        code: "L005",
                        level: LintLevel::Warning,
                        message: format!(
                            "Variable '{}' shadows an outer variable declared at offset {}",
                            var, parent_span.start
                        ),
                        suggestion: Some("rename inner variable for clarity".to_string()),
                        span: Some(stmt.span.clone()),
                    });
                }
                if let Some(current) = scopes.last_mut() {
                    current.insert(var.clone(), stmt.span.clone());
                }
                check_shadowed_in_block(body, scopes, findings);
                scopes.pop();
            }
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    scopes.push(HashMap::new());
                    check_shadowed_in_block(&arm.body, scopes, findings);
                    scopes.pop();
                }
            }
            Stmt::Assign { .. }
            | Stmt::Expr(_)
            | Stmt::Return(_)
            | Stmt::Break
            | Stmt::Continue => {}
        }
    }
}

fn apply_safe_import_fixes(source: &str, program: &Program) -> String {
    let shebang = source
        .lines()
        .next()
        .filter(|line| line.starts_with("#!"))
        .map(ToString::to_string);

    let mut imports = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_block_comment = false;

    for line in source.lines() {
        if shebang.as_ref().is_some_and(|s| s == line) {
            continue;
        }
        let trimmed = line.trim();
        let starts_block_comment = trimmed.contains("/*");
        let ends_block_comment = trimmed.contains("*/");
        let can_extract_import =
            !in_block_comment && !trimmed.starts_with("//") && !trimmed.starts_with("/*");

        let is_package_line = can_extract_import
            && program
                .package
                .as_ref()
                .is_some_and(|package| trimmed == format!("package {};", package));

        if can_extract_import && trimmed.starts_with("import ") && trimmed.ends_with(';') {
            imports.push(trimmed.to_string());
        } else if is_package_line {
            continue;
        } else {
            body_lines.push(line);
        }

        if starts_block_comment && !ends_block_comment {
            in_block_comment = true;
        } else if in_block_comment && ends_block_comment {
            in_block_comment = false;
        }
    }

    if imports.is_empty() {
        return source.to_string();
    }

    let mut package_line = None;
    if let Some(package) = &program.package {
        package_line = Some(format!("package {};", package));
    }

    imports.sort();
    imports.dedup();

    let mut output = String::new();
    if let Some(shebang) = shebang {
        output.push_str(&shebang);
        output.push('\n');
        if !imports.is_empty() || package_line.is_some() {
            output.push('\n');
        }
    }
    if let Some(package_line) = package_line {
        output.push_str(&package_line);
        output.push_str("\n\n");
    }

    for import in &imports {
        output.push_str(import);
        output.push('\n');
    }
    output.push('\n');

    let body = body_lines.join("\n");
    output.push_str(body.trim_matches('\n'));
    if !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

fn import_identity(import: &ImportDecl) -> String {
    if let Some(alias) = &import.alias {
        format!("{} as {}", import.path, alias)
    } else {
        import.path.clone()
    }
}

fn collect_used_names(program: &Program, used: &mut HashSet<String>) {
    for decl in &program.declarations {
        match &decl.node {
            Decl::Function(func) => {
                for param in &func.params {
                    collect_type_names(&param.ty, used);
                }
                collect_type_names(&func.return_type, used);
                for stmt in &func.body {
                    collect_stmt_names(&stmt.node, used);
                }
            }
            Decl::Class(class) => {
                if let Some(base) = &class.extends {
                    used.insert(base.clone());
                }
                for name in &class.implements {
                    used.insert(name.clone());
                }
                for field in &class.fields {
                    collect_type_names(&field.ty, used);
                }
                if let Some(ctor) = &class.constructor {
                    for param in &ctor.params {
                        collect_type_names(&param.ty, used);
                    }
                    for stmt in &ctor.body {
                        collect_stmt_names(&stmt.node, used);
                    }
                }
                if let Some(dtor) = &class.destructor {
                    for stmt in &dtor.body {
                        collect_stmt_names(&stmt.node, used);
                    }
                }
                for method in &class.methods {
                    for param in &method.params {
                        collect_type_names(&param.ty, used);
                    }
                    collect_type_names(&method.return_type, used);
                    for stmt in &method.body {
                        collect_stmt_names(&stmt.node, used);
                    }
                }
            }
            Decl::Enum(en) => {
                for variant in &en.variants {
                    for field in &variant.fields {
                        collect_type_names(&field.ty, used);
                    }
                }
            }
            Decl::Interface(interface) => {
                for name in &interface.extends {
                    used.insert(name.clone());
                }
                for method in &interface.methods {
                    for param in &method.params {
                        collect_type_names(&param.ty, used);
                    }
                    collect_type_names(&method.return_type, used);
                }
            }
            Decl::Module(module) => {
                let nested = Program {
                    package: None,
                    declarations: module.declarations.clone(),
                };
                collect_used_names(&nested, used);
            }
            Decl::Import(_) => {}
        }
    }
}

fn collect_stmt_names(stmt: &Stmt, used: &mut HashSet<String>) {
    match stmt {
        Stmt::Let { ty, value, .. } => {
            collect_type_names(ty, used);
            collect_expr_names(&value.node, used);
        }
        Stmt::Assign { target, value } => {
            collect_expr_names(&target.node, used);
            collect_expr_names(&value.node, used);
        }
        Stmt::Expr(expr) => collect_expr_names(&expr.node, used),
        Stmt::Return(expr) => {
            if let Some(expr) = expr {
                collect_expr_names(&expr.node, used);
            }
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_expr_names(&condition.node, used);
            for stmt in then_block {
                collect_stmt_names(&stmt.node, used);
            }
            if let Some(block) = else_block {
                for stmt in block {
                    collect_stmt_names(&stmt.node, used);
                }
            }
        }
        Stmt::While { condition, body } => {
            collect_expr_names(&condition.node, used);
            for stmt in body {
                collect_stmt_names(&stmt.node, used);
            }
        }
        Stmt::For {
            var_type,
            iterable,
            body,
            ..
        } => {
            if let Some(ty) = var_type {
                collect_type_names(ty, used);
            }
            collect_expr_names(&iterable.node, used);
            for stmt in body {
                collect_stmt_names(&stmt.node, used);
            }
        }
        Stmt::Match { expr, arms } => {
            collect_expr_names(&expr.node, used);
            for arm in arms {
                for stmt in &arm.body {
                    collect_stmt_names(&stmt.node, used);
                }
            }
        }
        Stmt::Break | Stmt::Continue => {}
    }
}

fn collect_expr_names(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Ident(name) => {
            used.insert(name.clone());
        }
        Expr::Call { callee, args, .. } => {
            collect_expr_names(&callee.node, used);
            for arg in args {
                collect_expr_names(&arg.node, used);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_names(&left.node, used);
            collect_expr_names(&right.node, used);
        }
        Expr::Unary { expr, .. }
        | Expr::Try(expr)
        | Expr::Borrow(expr)
        | Expr::MutBorrow(expr)
        | Expr::Deref(expr)
        | Expr::Await(expr) => collect_expr_names(&expr.node, used),
        Expr::Field { object, field } => {
            collect_expr_names(&object.node, used);
            used.insert(field.clone());
        }
        Expr::Index { object, index } => {
            collect_expr_names(&object.node, used);
            collect_expr_names(&index.node, used);
        }
        Expr::Construct { ty, args } => {
            used.insert(ty.clone());
            for arg in args {
                collect_expr_names(&arg.node, used);
            }
        }
        Expr::Lambda { params, body } => {
            for param in params {
                collect_type_names(&param.ty, used);
            }
            collect_expr_names(&body.node, used);
        }
        Expr::Match { expr, arms } => {
            collect_expr_names(&expr.node, used);
            for arm in arms {
                for stmt in &arm.body {
                    collect_stmt_names(&stmt.node, used);
                }
            }
        }
        Expr::StringInterp(parts) => {
            for part in parts {
                if let crate::ast::StringPart::Expr(expr) = part {
                    collect_expr_names(&expr.node, used);
                }
            }
        }
        Expr::AsyncBlock(block) | Expr::Block(block) => {
            for stmt in block {
                collect_stmt_names(&stmt.node, used);
            }
        }
        Expr::Require { condition, message } => {
            collect_expr_names(&condition.node, used);
            if let Some(message) = message {
                collect_expr_names(&message.node, used);
            }
        }
        Expr::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_expr_names(&start.node, used);
            }
            if let Some(end) = end {
                collect_expr_names(&end.node, used);
            }
        }
        Expr::IfExpr {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_names(&condition.node, used);
            for stmt in then_branch {
                collect_stmt_names(&stmt.node, used);
            }
            if let Some(block) = else_branch {
                for stmt in block {
                    collect_stmt_names(&stmt.node, used);
                }
            }
        }
        Expr::Literal(_) | Expr::This => {}
    }
}

fn collect_type_names(ty: &Type, used: &mut HashSet<String>) {
    match ty {
        Type::Named(name) => {
            used.insert(name.clone());
            if let Some((prefix, _)) = name.split_once('.') {
                used.insert(prefix.to_string());
            }
        }
        Type::Generic(name, args) => {
            used.insert(name.clone());
            if let Some((prefix, _)) = name.split_once('.') {
                used.insert(prefix.to_string());
            }
            for arg in args {
                collect_type_names(arg, used);
            }
        }
        Type::Function(params, ret) => {
            for param in params {
                collect_type_names(param, used);
            }
            collect_type_names(ret, used);
        }
        Type::Option(inner)
        | Type::List(inner)
        | Type::Set(inner)
        | Type::Ref(inner)
        | Type::MutRef(inner)
        | Type::Box(inner)
        | Type::Rc(inner)
        | Type::Arc(inner)
        | Type::Ptr(inner)
        | Type::Task(inner)
        | Type::Range(inner) => collect_type_names(inner, used),
        Type::Result(ok, err) | Type::Map(ok, err) => {
            collect_type_names(ok, used);
            collect_type_names(err, used);
        }
        Type::Integer | Type::Float | Type::Boolean | Type::String | Type::Char | Type::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::lint_source;

    #[test]
    fn detects_duplicate_and_unsorted_imports() {
        let source = r#"import std.string.*;
import std.io.*;
import std.io.*;

function main(): None {
    println("ok");
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert_eq!(result.findings.len(), 2);
        assert!(result.findings.iter().any(|f| f.code == "L001"));
        assert!(result.findings.iter().any(|f| f.code == "L002"));
    }

    #[test]
    fn fixes_import_order_and_dedupes() {
        let source = r#"import std.string.*;
import std.io.*;
import std.io.*;

function main(): None {
    println("ok");
    return None;
}
"#;
        let result = lint_source(source, true).expect("lint succeeds");
        let fixed = result.fixed_source.expect("fixed source");
        assert!(fixed.starts_with("import std.io.*;\nimport std.string.*;"));
        assert_eq!(fixed.matches("import std.io.*;").count(), 1);
    }

    #[test]
    fn flags_unused_specific_imports() {
        let source = r#"import project.helper;
import std.io.*;

function main(): None {
    println("ok");
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(result.findings.iter().any(|f| f.code == "L003"));
    }

    #[test]
    fn flags_unused_variables() {
        let source = r#"function main(): None {
    used: Integer = 1;
    unused: Integer = 2;
    _ignored: Integer = 3;
    used = used + 1;
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        let unused_findings: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.code == "L004")
            .collect();
        assert_eq!(unused_findings.len(), 1);
        assert!(unused_findings[0]
            .message
            .contains("Variable 'unused' is declared but never used"));
    }

    #[test]
    fn flags_shadowed_variables() {
        let source = r#"function main(): None {
    x: Integer = 1;
    if (true) {
        x: Integer = 2;
        x = x + 1;
    }
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(result
            .findings
            .iter()
            .any(|f| f.code == "L005"
                && f.message.contains("Variable 'x' shadows an outer variable")));
    }

    #[test]
    fn alias_imports_are_not_false_duplicate() {
        let source = r#"import std.io as io;
import std.io as io2;

function main(): None {
    io.println("a");
    io2.println("b");
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(!result.findings.iter().any(|f| f.code == "L001"));
    }

    #[test]
    fn fix_keeps_import_with_trailing_comment() {
        let source = r#"import std.string.*; // needed for Str.len
import std.io.*;

function main(): None {
    println(to_string(Str.len("abc")));
    return None;
}
"#;
        let result = lint_source(source, true).expect("lint succeeds");
        let fixed = result.fixed_source.expect("fixed source");
        assert!(fixed.contains("import std.string.*;"));
        assert!(fixed.contains("import std.io.*;"));
    }

    #[test]
    fn fix_keeps_import_with_trailing_block_comment() {
        let source = r#"import std.string.*; /* needed for Str.len */
import std.io.*;

function main(): None {
    println(to_string(Str.len("abc")));
    return None;
}
"#;
        let result = lint_source(source, true).expect("lint succeeds");
        let fixed = result.fixed_source.expect("fixed source");
        assert!(fixed.contains("import std.string.*;"));
        assert!(fixed.contains("import std.io.*;"));
    }

    #[test]
    fn fix_preserves_shebang_line() {
        let source = r#"#!/usr/bin/env apex
import std.string.*;
import std.io.*;
function main(): None { return None; }
"#;
        let result = lint_source(source, true).expect("lint succeeds");
        let fixed = result.fixed_source.expect("fixed source");
        assert!(fixed.starts_with("#!/usr/bin/env apex\n"));
    }

    #[test]
    fn flags_unused_stdlib_specific_imports() {
        let source = r#"import std.math.abs;
import std.io.*;

function main(): None {
    println("ok");
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(result.findings.iter().any(|f| {
            f.code == "L003"
                && f.message
                    .contains("specific import 'std.math.abs' appears unused")
        }));
    }

    #[test]
    fn flags_shadowing_function_parameter() {
        let source = r#"function main(x: Integer): None {
    x: Integer = 1;
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(result
            .findings
            .iter()
            .any(|f| f.code == "L005"
                && f.message.contains("Variable 'x' shadows an outer variable")));
    }

    #[test]
    fn flags_shadowing_for_loop_variable() {
        let source = r#"function main(): None {
    i: Integer = 10;
    for (i in range(0, 3)) {
        println(to_string(i));
    }
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(result
            .findings
            .iter()
            .any(|f| f.code == "L005"
                && f.message.contains("Variable 'i' shadows an outer variable")));
    }

    #[test]
    fn flags_unused_for_loop_variable() {
        let source = r#"function main(): None {
    for (i in range(0, 3)) {
        println("x");
    }
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(result.findings.iter().any(|f| f.code == "L004"
            && f.message
                .contains("Variable 'i' is declared but never used")));
    }

    #[test]
    fn does_not_flag_used_aliased_specific_import() {
        let source = r#"import std.math.Math__abs as abs_fn;

function main(): None {
    x: Float = abs_fn(-1.0);
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(!result.findings.iter().any(|f| {
            f.code == "L003"
                && f.message
                    .contains("specific import 'std.math.Math__abs as abs_fn' appears unused")
        }));
    }

    #[test]
    fn flags_unused_aliased_specific_import() {
        let source = r#"import std.math.Math__abs as abs_fn;

function main(): None {
    println("ok");
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(result.findings.iter().any(|f| {
            f.code == "L003"
                && f.message
                    .contains("specific import 'std.math.Math__abs as abs_fn' appears unused")
        }));
    }

    #[test]
    fn fix_does_not_hoist_imports_out_of_block_comments() {
        let source = r#"/*
import evil.pkg;
*/
import std.io.*;

function main(): None {
    return None;
}
"#;
        let result = lint_source(source, true).expect("lint succeeds");
        let fixed = result.fixed_source.expect("fixed source");
        assert!(fixed.starts_with("import std.io.*;\n\n"), "{fixed}");
        assert!(fixed.contains("/*\nimport evil.pkg;\n*/"), "{fixed}");
    }

    #[test]
    fn type_position_alias_usage_marks_import_as_used() {
        let source = r#"import util as u;

function main(value: u.Box): None {
    return None;
}
"#;
        let result = lint_source(source, false).expect("lint succeeds");
        assert!(!result.findings.iter().any(|f| {
            f.code == "L003" && f.message.contains("import 'util as u' appears unused")
        }));
    }
}
