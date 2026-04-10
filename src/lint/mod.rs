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
    let mut duplicates = BTreeSet::new();
    fn collect_scope_duplicates(
        declarations: &[crate::ast::Spanned<Decl>],
        duplicates: &mut BTreeSet<String>,
    ) {
        let mut seen = HashSet::new();
        for decl in declarations {
            match &decl.node {
                Decl::Import(import) => {
                    let key = import_identity(import);
                    if !seen.insert(key.clone()) {
                        duplicates.insert(key);
                    }
                }
                Decl::Module(module) => {
                    collect_scope_duplicates(&module.declarations, duplicates);
                }
                Decl::Function(_) | Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) => {}
            }
        }
    }

    collect_scope_duplicates(&program.declarations, &mut duplicates);

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
            suggestion: Some("run `arden fix` or sort imports lexicographically".to_string()),
            span: None,
        }]
    }
}

fn check_unused_specific_imports(program: &Program) -> Vec<LintFinding> {
    fn collect_unused_specific_imports(program: &Program, findings: &mut Vec<LintFinding>) {
        let mut used_names = HashSet::new();
        collect_used_names(program, &mut used_names);

        for decl in &program.declarations {
            match &decl.node {
                Decl::Import(import) => {
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
                                "remove it or switch to a wildcard import only if justified"
                                    .to_string(),
                            ),
                            span: Some(decl.span.clone()),
                        });
                    }
                }
                Decl::Module(module) => {
                    let nested = Program {
                        package: None,
                        declarations: module.declarations.clone(),
                    };
                    collect_unused_specific_imports(&nested, findings);
                }
                Decl::Function(_) | Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) => {}
            }
        }
    }

    let mut findings = Vec::new();
    collect_unused_specific_imports(program, &mut findings);
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
            Decl::Interface(interface) => {
                for method in &interface.methods {
                    if let Some(default_impl) = &method.default_impl {
                        findings.extend(check_unused_variables_in_block(default_impl));
                    }
                }
            }
            Decl::Enum(_) | Decl::Import(_) => {}
        }
    }

    findings
}

fn check_unused_variables_in_block(block: &[crate::ast::Spanned<Stmt>]) -> Vec<LintFinding> {
    let mut declared: Vec<(String, Span, usize)> = Vec::new();
    let mut used: HashSet<(String, usize)> = HashSet::new();

    let mut scope_stack = vec![0usize];
    let mut next_scope_id = 1usize;
    collect_declared_and_used_in_block(
        block,
        &mut declared,
        &mut used,
        &mut scope_stack,
        &mut next_scope_id,
    );

    declared
        .into_iter()
        .filter(|(name, _, scope_id)| {
            !name.starts_with('_') && !used.contains(&(name.clone(), *scope_id))
        })
        .map(|(name, span, _)| LintFinding {
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
    declared: &mut Vec<(String, Span, usize)>,
    used: &mut HashSet<(String, usize)>,
    scope_stack: &mut Vec<usize>,
    next_scope_id: &mut usize,
) {
    for stmt in block {
        collect_declared_and_used_in_stmt(stmt, declared, used, scope_stack, next_scope_id);
    }
}

fn collect_declared_and_used_in_stmt(
    stmt: &crate::ast::Spanned<Stmt>,
    declared: &mut Vec<(String, Span, usize)>,
    used: &mut HashSet<(String, usize)>,
    scope_stack: &mut Vec<usize>,
    next_scope_id: &mut usize,
) {
    match &stmt.node {
        Stmt::Let { name, value, .. } => {
            declared.push((
                name.clone(),
                stmt.span.clone(),
                *scope_stack.last().unwrap_or(&0),
            ));
            collect_expr_idents(&value.node, declared, used, scope_stack, next_scope_id);
        }
        Stmt::Assign { target, value } => {
            collect_expr_idents(&target.node, declared, used, scope_stack, next_scope_id);
            collect_expr_idents(&value.node, declared, used, scope_stack, next_scope_id);
        }
        Stmt::Expr(expr) => {
            collect_expr_idents(&expr.node, declared, used, scope_stack, next_scope_id)
        }
        Stmt::Return(expr) => {
            if let Some(expr) = expr {
                collect_expr_idents(&expr.node, declared, used, scope_stack, next_scope_id);
            }
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_expr_idents(&condition.node, declared, used, scope_stack, next_scope_id);
            let then_scope = *next_scope_id;
            *next_scope_id += 1;
            scope_stack.push(then_scope);
            collect_declared_and_used_in_block(
                then_block,
                declared,
                used,
                scope_stack,
                next_scope_id,
            );
            scope_stack.pop();
            if let Some(else_block) = else_block {
                let else_scope = *next_scope_id;
                *next_scope_id += 1;
                scope_stack.push(else_scope);
                collect_declared_and_used_in_block(
                    else_block,
                    declared,
                    used,
                    scope_stack,
                    next_scope_id,
                );
                scope_stack.pop();
            }
        }
        Stmt::While { condition, body } => {
            collect_expr_idents(&condition.node, declared, used, scope_stack, next_scope_id);
            let loop_scope = *next_scope_id;
            *next_scope_id += 1;
            scope_stack.push(loop_scope);
            collect_declared_and_used_in_block(body, declared, used, scope_stack, next_scope_id);
            scope_stack.pop();
        }
        Stmt::For {
            var,
            iterable,
            body,
            ..
        } => {
            let loop_scope = *next_scope_id;
            *next_scope_id += 1;
            scope_stack.push(loop_scope);
            declared.push((var.clone(), stmt.span.clone(), loop_scope));
            collect_expr_idents(&iterable.node, declared, used, scope_stack, next_scope_id);
            collect_declared_and_used_in_block(body, declared, used, scope_stack, next_scope_id);
            scope_stack.pop();
        }
        Stmt::Match { expr, arms } => {
            collect_expr_idents(&expr.node, declared, used, scope_stack, next_scope_id);
            for arm in arms {
                let arm_scope = *next_scope_id;
                *next_scope_id += 1;
                scope_stack.push(arm_scope);
                collect_pattern_bindings(&arm.pattern, &stmt.span, declared, arm_scope);
                collect_declared_and_used_in_block(
                    &arm.body,
                    declared,
                    used,
                    scope_stack,
                    next_scope_id,
                );
                scope_stack.pop();
            }
        }
        Stmt::Break | Stmt::Continue => {}
    }
}

fn collect_pattern_bindings(
    pattern: &crate::ast::Pattern,
    span: &Span,
    declared: &mut Vec<(String, Span, usize)>,
    scope_id: usize,
) {
    match pattern {
        crate::ast::Pattern::Ident(name) => declared.push((name.clone(), span.clone(), scope_id)),
        crate::ast::Pattern::Variant(_, bindings) => {
            for binding in bindings {
                declared.push((binding.clone(), span.clone(), scope_id));
            }
        }
        crate::ast::Pattern::Wildcard | crate::ast::Pattern::Literal(_) => {}
    }
}

fn collect_expr_idents(
    expr: &Expr,
    declared: &mut Vec<(String, Span, usize)>,
    used: &mut HashSet<(String, usize)>,
    scope_stack: &mut Vec<usize>,
    next_scope_id: &mut usize,
) {
    match expr {
        Expr::Ident(name) => {
            if let Some((_, _, scope_id)) = declared
                .iter()
                .rev()
                .find(|(declared_name, _, _)| declared_name == name)
            {
                used.insert((name.clone(), *scope_id));
            }
        }
        Expr::Call { callee, args, .. } => {
            collect_expr_idents(&callee.node, declared, used, scope_stack, next_scope_id);
            for arg in args {
                collect_expr_idents(&arg.node, declared, used, scope_stack, next_scope_id);
            }
        }
        Expr::GenericFunctionValue { callee, .. } => {
            collect_expr_idents(&callee.node, declared, used, scope_stack, next_scope_id)
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_idents(&left.node, declared, used, scope_stack, next_scope_id);
            collect_expr_idents(&right.node, declared, used, scope_stack, next_scope_id);
        }
        Expr::Unary { expr, .. }
        | Expr::Try(expr)
        | Expr::Borrow(expr)
        | Expr::MutBorrow(expr)
        | Expr::Deref(expr)
        | Expr::Await(expr) => {
            collect_expr_idents(&expr.node, declared, used, scope_stack, next_scope_id)
        }
        Expr::Field { object, .. } => {
            collect_expr_idents(&object.node, declared, used, scope_stack, next_scope_id)
        }
        Expr::Index { object, index } => {
            collect_expr_idents(&object.node, declared, used, scope_stack, next_scope_id);
            collect_expr_idents(&index.node, declared, used, scope_stack, next_scope_id);
        }
        Expr::Construct { args, .. } => {
            for arg in args {
                collect_expr_idents(&arg.node, declared, used, scope_stack, next_scope_id);
            }
        }
        Expr::Lambda { params, body } => {
            let lambda_scope = *next_scope_id;
            *next_scope_id += 1;
            scope_stack.push(lambda_scope);
            for param in params {
                declared.push((param.name.clone(), 0..0, lambda_scope));
            }
            collect_expr_idents(&body.node, declared, used, scope_stack, next_scope_id);
            scope_stack.pop();
        }
        Expr::Match { expr, arms } => {
            collect_expr_idents(&expr.node, declared, used, scope_stack, next_scope_id);
            for arm in arms {
                let arm_scope = *next_scope_id;
                *next_scope_id += 1;
                scope_stack.push(arm_scope);
                collect_pattern_bindings(&arm.pattern, &(0..0), declared, arm_scope);
                collect_declared_and_used_in_block(
                    &arm.body,
                    declared,
                    used,
                    scope_stack,
                    next_scope_id,
                );
                scope_stack.pop();
            }
        }
        Expr::StringInterp(parts) => {
            for part in parts {
                if let crate::ast::StringPart::Expr(expr) = part {
                    collect_expr_idents(&expr.node, declared, used, scope_stack, next_scope_id);
                }
            }
        }
        Expr::AsyncBlock(block) | Expr::Block(block) => {
            let block_scope = *next_scope_id;
            *next_scope_id += 1;
            scope_stack.push(block_scope);
            collect_declared_and_used_in_block(block, declared, used, scope_stack, next_scope_id);
            scope_stack.pop();
        }
        Expr::Require { condition, message } => {
            collect_expr_idents(&condition.node, declared, used, scope_stack, next_scope_id);
            if let Some(message) = message {
                collect_expr_idents(&message.node, declared, used, scope_stack, next_scope_id);
            }
        }
        Expr::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_expr_idents(&start.node, declared, used, scope_stack, next_scope_id);
            }
            if let Some(end) = end {
                collect_expr_idents(&end.node, declared, used, scope_stack, next_scope_id);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_idents(&condition.node, declared, used, scope_stack, next_scope_id);
            let then_scope = *next_scope_id;
            *next_scope_id += 1;
            scope_stack.push(then_scope);
            collect_declared_and_used_in_block(
                then_branch,
                declared,
                used,
                scope_stack,
                next_scope_id,
            );
            scope_stack.pop();
            if let Some(else_branch) = else_branch {
                let else_scope = *next_scope_id;
                *next_scope_id += 1;
                scope_stack.push(else_scope);
                collect_declared_and_used_in_block(
                    else_branch,
                    declared,
                    used,
                    scope_stack,
                    next_scope_id,
                );
                scope_stack.pop();
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
            Decl::Interface(interface) => {
                for method in &interface.methods {
                    if let Some(default_impl) = &method.default_impl {
                        let mut scopes = vec![
                            scope_with_params(&method.params),
                            HashMap::<String, Span>::new(),
                        ];
                        check_shadowed_in_block(default_impl, &mut scopes, &mut findings);
                    }
                }
            }
            Decl::Enum(_) | Decl::Import(_) => {}
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
                if let Stmt::Let { value, .. } = &stmt.node {
                    check_shadowed_in_expr(&value.node, scopes, findings);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                check_shadowed_in_expr(&condition.node, scopes, findings);
                scopes.push(HashMap::new());
                check_shadowed_in_block(then_block, scopes, findings);
                scopes.pop();
                if let Some(block) = else_block {
                    scopes.push(HashMap::new());
                    check_shadowed_in_block(block, scopes, findings);
                    scopes.pop();
                }
            }
            Stmt::While { condition, body } => {
                check_shadowed_in_expr(&condition.node, scopes, findings);
                scopes.push(HashMap::new());
                check_shadowed_in_block(body, scopes, findings);
                scopes.pop();
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                check_shadowed_in_expr(&iterable.node, scopes, findings);
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
            Stmt::Match { expr, arms } => {
                check_shadowed_in_expr(&expr.node, scopes, findings);
                for arm in arms {
                    scopes.push(HashMap::new());
                    declare_pattern_bindings_in_scope(&arm.pattern, scopes, findings, &stmt.span);
                    check_shadowed_in_block(&arm.body, scopes, findings);
                    scopes.pop();
                }
            }
            Stmt::Assign { target, value } => {
                check_shadowed_in_expr(&target.node, scopes, findings);
                check_shadowed_in_expr(&value.node, scopes, findings);
            }
            Stmt::Expr(expr) => check_shadowed_in_expr(&expr.node, scopes, findings),
            Stmt::Return(Some(expr)) => check_shadowed_in_expr(&expr.node, scopes, findings),
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
        }
    }
}

fn declare_name_in_scope(
    name: &str,
    span: &Span,
    scopes: &mut [HashMap<String, Span>],
    findings: &mut Vec<LintFinding>,
) {
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
            span: Some(span.clone()),
        });
    }
    if let Some(current) = scopes.last_mut() {
        current.insert(name.to_string(), span.clone());
    }
}

fn declare_pattern_bindings_in_scope(
    pattern: &crate::ast::Pattern,
    scopes: &mut [HashMap<String, Span>],
    findings: &mut Vec<LintFinding>,
    span: &Span,
) {
    match pattern {
        crate::ast::Pattern::Ident(name) => declare_name_in_scope(name, span, scopes, findings),
        crate::ast::Pattern::Variant(_, bindings) => {
            for binding in bindings {
                declare_name_in_scope(binding, span, scopes, findings);
            }
        }
        crate::ast::Pattern::Wildcard | crate::ast::Pattern::Literal(_) => {}
    }
}

fn check_shadowed_in_expr(
    expr: &Expr,
    scopes: &mut Vec<HashMap<String, Span>>,
    findings: &mut Vec<LintFinding>,
) {
    match expr {
        Expr::Call { callee, args, .. } => {
            check_shadowed_in_expr(&callee.node, scopes, findings);
            for arg in args {
                check_shadowed_in_expr(&arg.node, scopes, findings);
            }
        }
        Expr::GenericFunctionValue { callee, .. } => {
            check_shadowed_in_expr(&callee.node, scopes, findings)
        }
        Expr::Binary { left, right, .. } => {
            check_shadowed_in_expr(&left.node, scopes, findings);
            check_shadowed_in_expr(&right.node, scopes, findings);
        }
        Expr::Unary { expr, .. }
        | Expr::Try(expr)
        | Expr::Borrow(expr)
        | Expr::MutBorrow(expr)
        | Expr::Deref(expr)
        | Expr::Await(expr) => check_shadowed_in_expr(&expr.node, scopes, findings),
        Expr::Field { object, .. } => check_shadowed_in_expr(&object.node, scopes, findings),
        Expr::Index { object, index } => {
            check_shadowed_in_expr(&object.node, scopes, findings);
            check_shadowed_in_expr(&index.node, scopes, findings);
        }
        Expr::Construct { args, .. } => {
            for arg in args {
                check_shadowed_in_expr(&arg.node, scopes, findings);
            }
        }
        Expr::Lambda { params, body } => {
            scopes.push(HashMap::new());
            for param in params {
                declare_name_in_scope(&param.name, &(0..0), scopes, findings);
            }
            check_shadowed_in_expr(&body.node, scopes, findings);
            scopes.pop();
        }
        Expr::Match { expr, arms } => {
            check_shadowed_in_expr(&expr.node, scopes, findings);
            for arm in arms {
                scopes.push(HashMap::new());
                declare_pattern_bindings_in_scope(&arm.pattern, scopes, findings, &(0..0));
                check_shadowed_in_block(&arm.body, scopes, findings);
                scopes.pop();
            }
        }
        Expr::StringInterp(parts) => {
            for part in parts {
                if let crate::ast::StringPart::Expr(expr) = part {
                    check_shadowed_in_expr(&expr.node, scopes, findings);
                }
            }
        }
        Expr::AsyncBlock(block) | Expr::Block(block) => {
            scopes.push(HashMap::new());
            check_shadowed_in_block(block, scopes, findings);
            scopes.pop();
        }
        Expr::Require { condition, message } => {
            check_shadowed_in_expr(&condition.node, scopes, findings);
            if let Some(message) = message {
                check_shadowed_in_expr(&message.node, scopes, findings);
            }
        }
        Expr::Range { start, end, .. } => {
            if let Some(start) = start {
                check_shadowed_in_expr(&start.node, scopes, findings);
            }
            if let Some(end) = end {
                check_shadowed_in_expr(&end.node, scopes, findings);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            check_shadowed_in_expr(&condition.node, scopes, findings);
            scopes.push(HashMap::new());
            check_shadowed_in_block(then_branch, scopes, findings);
            scopes.pop();
            if let Some(else_branch) = else_branch {
                scopes.push(HashMap::new());
                check_shadowed_in_block(else_branch, scopes, findings);
                scopes.pop();
            }
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::This => {}
    }
}

fn apply_safe_import_fixes(source: &str, program: &Program) -> String {
    let shebang = source
        .lines()
        .next()
        .filter(|line| line.starts_with("#!"))
        .map(ToString::to_string);

    let mut imports = Vec::new();
    let mut header_lines = Vec::new();
    let mut package_prelude_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_block_comment = false;
    let mut package_seen = false;
    let mut import_seen = false;
    let mut body_started = false;

    for line in source.lines() {
        if shebang.as_ref().is_some_and(|s| s == line) {
            continue;
        }
        let trimmed = line.trim();
        let was_in_block_comment = in_block_comment;
        let starts_block_comment = trimmed.contains("/*");
        let ends_block_comment = trimmed.contains("*/");
        let can_extract_import =
            !in_block_comment && !trimmed.starts_with("//") && !trimmed.starts_with("/*");

        let is_package_line = can_extract_import
            && program
                .package
                .as_ref()
                .is_some_and(|package| trimmed == format!("package {};", package));

        if body_started {
            body_lines.push(line);
        } else if can_extract_import && trimmed.starts_with("import ") && trimmed.ends_with(';') {
            imports.push(trimmed.to_string());
            import_seen = true;
        } else if is_package_line {
            package_seen = true;
        } else {
            let is_trivia = trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*')
                || trimmed.starts_with("*/")
                || was_in_block_comment;
            if !package_seen && !import_seen && is_trivia {
                header_lines.push(line);
            } else if package_seen && !import_seen && is_trivia {
                package_prelude_lines.push(line);
            } else {
                body_started = true;
                body_lines.push(line);
            }
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
    }
    if !header_lines.is_empty() {
        output.push_str(header_lines.join("\n").trim_end_matches('\n'));
        output.push('\n');
    }
    if let Some(package_line) = package_line {
        output.push_str(&package_line);
        output.push_str("\n\n");
    }
    if !package_prelude_lines.is_empty() {
        output.push_str(package_prelude_lines.join("\n").trim_matches('\n'));
        output.push('\n');
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
                collect_generic_param_bound_names(&func.generic_params, used);
                for param in &func.params {
                    collect_type_names(&param.ty, used);
                }
                collect_type_names(&func.return_type, used);
                for stmt in &func.body {
                    collect_stmt_names(&stmt.node, used);
                }
            }
            Decl::Class(class) => {
                collect_generic_param_bound_names(&class.generic_params, used);
                if let Some(base) = &class.extends {
                    collect_qualified_name(base, used);
                }
                for name in &class.implements {
                    collect_qualified_name(name, used);
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
                    collect_generic_param_bound_names(&method.generic_params, used);
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
                collect_generic_param_bound_names(&en.generic_params, used);
                for variant in &en.variants {
                    for field in &variant.fields {
                        collect_type_names(&field.ty, used);
                    }
                }
            }
            Decl::Interface(interface) => {
                collect_generic_param_bound_names(&interface.generic_params, used);
                for name in &interface.extends {
                    collect_qualified_name(name, used);
                }
                for method in &interface.methods {
                    for param in &method.params {
                        collect_type_names(&param.ty, used);
                    }
                    collect_type_names(&method.return_type, used);
                    if let Some(default_impl) = &method.default_impl {
                        for stmt in default_impl {
                            collect_stmt_names(&stmt.node, used);
                        }
                    }
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

fn collect_generic_param_bound_names(
    generic_params: &[crate::ast::GenericParam],
    used: &mut HashSet<String>,
) {
    for param in generic_params {
        for bound in &param.bounds {
            collect_qualified_name(bound, used);
        }
    }
}

fn collect_qualified_name(name: &str, used: &mut HashSet<String>) {
    used.insert(name.to_string());
    if let Some((prefix, _)) = name.split_once('.') {
        used.insert(prefix.to_string());
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
                collect_pattern_names(&arm.pattern, used);
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
        Expr::Call {
            callee,
            args,
            type_args,
        } => {
            collect_expr_names(&callee.node, used);
            for arg in args {
                collect_expr_names(&arg.node, used);
            }
            for ty in type_args {
                collect_type_names(ty, used);
            }
        }
        Expr::GenericFunctionValue { callee, type_args } => {
            collect_expr_names(&callee.node, used);
            for ty in type_args {
                collect_type_names(ty, used);
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
            collect_construct_type_names(ty, used);
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
                collect_pattern_names(&arm.pattern, used);
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
        Expr::If {
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

fn collect_construct_type_names(ty: &str, used: &mut HashSet<String>) {
    let mut token = String::new();
    for ch in ty.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
            token.push(ch);
            continue;
        }
        flush_construct_type_token(&mut token, used);
    }
    flush_construct_type_token(&mut token, used);
}

fn flush_construct_type_token(token: &mut String, used: &mut HashSet<String>) {
    if token.is_empty() {
        return;
    }
    used.insert(token.clone());
    if let Some((prefix, _)) = token.split_once('.') {
        used.insert(prefix.to_string());
    }
    token.clear();
}

fn collect_pattern_names(pattern: &crate::ast::Pattern, used: &mut HashSet<String>) {
    match pattern {
        crate::ast::Pattern::Ident(name) => {
            used.insert(name.clone());
        }
        crate::ast::Pattern::Variant(name, _) => {
            used.insert(name.clone());
            if let Some((prefix, _)) = name.split_once('.') {
                used.insert(prefix.to_string());
            }
        }
        crate::ast::Pattern::Wildcard | crate::ast::Pattern::Literal(_) => {}
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
