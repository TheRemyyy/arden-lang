use super::*;

impl TypeChecker {
    pub(crate) fn apply_effect_seeds(
        &mut self,
        function_effects: &HashMap<String, Vec<String>>,
        class_method_effects: &HashMap<String, HashMap<String, Vec<String>>>,
    ) {
        for (name, effects) in function_effects {
            if let Some(sig) = self.functions.get_mut(name) {
                sig.effects = effects.clone();
            }
        }
        for (class_name, methods) in class_method_effects {
            if let Some(class) = self.classes.get_mut(class_name) {
                for (method_name, effects) in methods {
                    if let Some(sig) = class.methods.get_mut(method_name) {
                        sig.effects = effects.clone();
                    }
                }
            }
        }
    }

    pub(crate) fn export_effect_summary(
        &self,
    ) -> (FunctionEffectsSummary, ClassMethodEffectsSummary) {
        let function_effects = self
            .functions
            .iter()
            .map(|(name, sig)| (name.clone(), sig.effects.clone()))
            .collect();
        let class_method_effects = self
            .classes
            .iter()
            .map(|(class_name, class)| {
                (
                    class_name.clone(),
                    class
                        .methods
                        .iter()
                        .map(|(method_name, sig)| (method_name.clone(), sig.effects.clone()))
                        .collect(),
                )
            })
            .collect();
        (function_effects, class_method_effects)
    }

    pub(crate) fn check_with_effect_seeds(
        &mut self,
        program: &Program,
        function_effects: &FunctionEffectsSummary,
        class_method_effects: &ClassMethodEffectsSummary,
    ) -> Result<(), Vec<TypeError>> {
        self.populate_import_aliases(program);
        self.collect_declarations(program);
        self.normalize_inheritance_references();
        self.apply_effect_seeds(function_effects, class_method_effects);
        for (name, iface) in self.interfaces.clone() {
            for parent in iface.extends {
                if !self
                    .interfaces
                    .contains_key(self.interface_base_name(&parent))
                {
                    self.error(
                        format!(
                            "Interface '{}' extends unknown interface '{}'",
                            name, parent
                        ),
                        iface.span.clone(),
                    );
                }
            }
        }
        self.infer_effects(program);

        for decl in &program.declarations {
            self.check_decl_with_prefix(&decl.node, decl.span.clone(), None);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    pub(crate) fn validate_effect_attributes(
        &mut self,
        attrs: &[Attribute],
        span: Span,
        subject: &str,
    ) {
        let (effects, pure, any, _) = self.parse_effects_from_attributes(attrs);
        if pure && any {
            self.error(
                format!(
                    "{} cannot use both @Pure and @Any; pick one effect policy",
                    subject
                ),
                span.clone(),
            );
        }
        if pure && !effects.is_empty() {
            self.error(
                format!(
                    "{} cannot combine @Pure with explicit effects ({})",
                    subject,
                    effects.join(", ")
                ),
                span,
            );
        }
    }

    pub(crate) fn parse_effects_from_attributes(
        &self,
        attrs: &[Attribute],
    ) -> (Vec<String>, bool, bool, bool) {
        let mut effects = Vec::new();
        let mut pure = false;
        let mut any = false;
        let mut has_explicit_effects = false;

        for attr in attrs {
            match attr {
                Attribute::Pure => pure = true,
                Attribute::EffectIo => {
                    has_explicit_effects = true;
                    effects.push("io".to_string());
                }
                Attribute::EffectNet => {
                    has_explicit_effects = true;
                    effects.push("net".to_string());
                }
                Attribute::EffectAlloc => {
                    has_explicit_effects = true;
                    effects.push("alloc".to_string());
                }
                Attribute::EffectUnsafe => {
                    has_explicit_effects = true;
                    effects.push("unsafe".to_string());
                }
                Attribute::EffectThread => {
                    has_explicit_effects = true;
                    effects.push("thread".to_string());
                }
                Attribute::EffectAny => {
                    has_explicit_effects = true;
                    any = true;
                }
                _ => {}
            }
        }

        effects.sort();
        effects.dedup();
        (effects, pure, any, has_explicit_effects)
    }

    pub(crate) fn infer_effects(&mut self, program: &Program) {
        // Fixed-point inference over the call graph for declarations without explicit effects.
        let mut changed = true;
        let mut passes = 0usize;
        while changed && passes < 24 {
            changed = false;
            passes += 1;
            for decl in &program.declarations {
                changed |= self.infer_effects_decl(&decl.node, None);
            }
        }
    }

    pub(crate) fn infer_effects_decl(&mut self, decl: &Decl, module_prefix: Option<&str>) -> bool {
        match decl {
            Decl::Function(func) => {
                let key = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, func.name)
                } else {
                    func.name.clone()
                };
                self.infer_effects_for_function_key(&key, &func.body, None)
            }
            Decl::Class(class) => {
                let mut changed = false;
                for method in &class.methods {
                    changed |=
                        self.infer_effects_for_method(&class.name, &method.name, &method.body);
                }
                changed
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                let mut changed = false;
                for inner in &module.declarations {
                    changed |= self.infer_effects_decl(&inner.node, Some(&next_prefix));
                }
                changed
            }
            _ => false,
        }
    }

    pub(crate) fn infer_effects_for_function_key(
        &mut self,
        key: &str,
        body: &Block,
        current_class: Option<&str>,
    ) -> bool {
        let Some(sig) = self.functions.get(key).cloned() else {
            return false;
        };
        if sig.is_pure || sig.allow_any || sig.has_explicit_effects {
            return false;
        }

        let mut inferred: Vec<String> = self
            .infer_effects_in_block(body, current_class)
            .into_iter()
            .collect();
        inferred.sort();
        inferred.dedup();

        if inferred != sig.effects {
            if let Some(edit_sig) = self.functions.get_mut(key) {
                edit_sig.effects = inferred;
                return true;
            }
        }
        false
    }

    pub(crate) fn infer_effects_for_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        body: &Block,
    ) -> bool {
        let Some(class_info) = self.classes.get(class_name).cloned() else {
            return false;
        };
        let Some(method_sig) = class_info.methods.get(method_name).cloned() else {
            return false;
        };
        if method_sig.is_pure || method_sig.allow_any || method_sig.has_explicit_effects {
            return false;
        }

        let mut inferred: Vec<String> = self
            .infer_effects_in_block(body, Some(class_name))
            .into_iter()
            .collect();
        inferred.sort();
        inferred.dedup();

        if inferred != method_sig.effects {
            if let Some(class_edit) = self.classes.get_mut(class_name) {
                if let Some(sig_edit) = class_edit.methods.get_mut(method_name) {
                    sig_edit.effects = inferred;
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn infer_effects_in_block(
        &self,
        block: &Block,
        current_class: Option<&str>,
    ) -> std::collections::BTreeSet<String> {
        let mut out = std::collections::BTreeSet::new();
        for stmt in block {
            self.collect_effects_stmt(&stmt.node, current_class, &mut out);
        }
        out
    }

    pub(crate) fn collect_effects_stmt(
        &self,
        stmt: &Stmt,
        current_class: Option<&str>,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        match stmt {
            Stmt::Let { value, .. } => self.collect_effects_expr(&value.node, current_class, out),
            Stmt::Assign { target, value } => {
                self.collect_effects_expr(&target.node, current_class, out);
                self.collect_effects_expr(&value.node, current_class, out);
            }
            Stmt::Expr(expr) => self.collect_effects_expr(&expr.node, current_class, out),
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.collect_effects_expr(&expr.node, current_class, out);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.collect_effects_expr(&condition.node, current_class, out);
                for s in then_block {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
                if let Some(else_block) = else_block {
                    for s in else_block {
                        self.collect_effects_stmt(&s.node, current_class, out);
                    }
                }
            }
            Stmt::While { condition, body } => {
                self.collect_effects_expr(&condition.node, current_class, out);
                for s in body {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
            }
            Stmt::For { iterable, body, .. } => {
                self.collect_effects_expr(&iterable.node, current_class, out);
                for s in body {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
            }
            Stmt::Match { expr, arms } => {
                self.collect_effects_expr(&expr.node, current_class, out);
                for arm in arms {
                    for s in &arm.body {
                        self.collect_effects_stmt(&s.node, current_class, out);
                    }
                }
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    pub(crate) fn collect_effects_expr(
        &self,
        expr: &Expr,
        current_class: Option<&str>,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        match expr {
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident(name) = &callee.node {
                    let canonical = self
                        .resolve_import_alias_symbol(name)
                        .unwrap_or_else(|| name.clone());
                    if let Some(required) = Self::builtin_required_effect(&canonical) {
                        out.insert(required.to_string());
                    }
                    if let Some(sig) = self.functions.get(&canonical) {
                        for eff in &sig.effects {
                            out.insert(eff.clone());
                        }
                    }
                } else if let Expr::Field { object, field } = &callee.node {
                    if let Expr::Ident(name) = &object.node {
                        let builtin_name = self
                            .resolve_stdlib_alias_call_name(name, field)
                            .unwrap_or_else(|| format!("{}__{}", name, field));
                        if let Some(required) = Self::builtin_required_effect(&builtin_name) {
                            out.insert(required.to_string());
                        }
                        if let Some(sig) = self.functions.get(&builtin_name) {
                            for eff in &sig.effects {
                                out.insert(eff.clone());
                            }
                        }
                        // Instance-style method call; infer conservatively by method name across classes.
                        self.collect_class_method_name_effects(field, out);
                    } else if matches!(object.node, Expr::This) {
                        if let Some(class_name) = current_class {
                            if let Some(class_info) = self.classes.get(class_name) {
                                if let Some(sig) = class_info.methods.get(field) {
                                    for eff in &sig.effects {
                                        out.insert(eff.clone());
                                    }
                                }
                            }
                        }
                    } else {
                        self.collect_class_method_name_effects(field, out);
                    }
                }

                self.collect_effects_expr(&callee.node, current_class, out);
                for arg in args {
                    self.collect_effects_expr(&arg.node, current_class, out);
                }
            }
            Expr::GenericFunctionValue { callee, .. } => {
                self.collect_effects_expr(&callee.node, current_class, out);
            }
            Expr::Binary { left, right, .. } => {
                self.collect_effects_expr(&left.node, current_class, out);
                self.collect_effects_expr(&right.node, current_class, out);
            }
            Expr::Unary { expr, .. }
            | Expr::Try(expr)
            | Expr::Borrow(expr)
            | Expr::MutBorrow(expr)
            | Expr::Deref(expr)
            | Expr::Await(expr) => {
                self.collect_effects_expr(&expr.node, current_class, out);
            }
            Expr::Field { object, .. } => {
                self.collect_effects_expr(&object.node, current_class, out)
            }
            Expr::Index { object, index } => {
                self.collect_effects_expr(&object.node, current_class, out);
                self.collect_effects_expr(&index.node, current_class, out);
            }
            Expr::Construct { args, .. } => {
                for arg in args {
                    self.collect_effects_expr(&arg.node, current_class, out);
                }
            }
            Expr::Lambda { body, .. } => self.collect_effects_expr(&body.node, current_class, out),
            Expr::Match { expr, arms } => {
                self.collect_effects_expr(&expr.node, current_class, out);
                for arm in arms {
                    for s in &arm.body {
                        self.collect_effects_stmt(&s.node, current_class, out);
                    }
                }
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.collect_effects_expr(&e.node, current_class, out);
                    }
                }
            }
            Expr::AsyncBlock(body) | Expr::Block(body) => {
                for s in body {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
            }
            Expr::Require { condition, message } => {
                self.collect_effects_expr(&condition.node, current_class, out);
                if let Some(msg) = message {
                    self.collect_effects_expr(&msg.node, current_class, out);
                }
            }
            Expr::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.collect_effects_expr(&s.node, current_class, out);
                }
                if let Some(e) = end {
                    self.collect_effects_expr(&e.node, current_class, out);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_effects_expr(&condition.node, current_class, out);
                for s in then_branch {
                    self.collect_effects_stmt(&s.node, current_class, out);
                }
                if let Some(else_branch) = else_branch {
                    for s in else_branch {
                        self.collect_effects_stmt(&s.node, current_class, out);
                    }
                }
            }
            Expr::Literal(_) | Expr::Ident(_) | Expr::This => {}
        }
    }

    pub(crate) fn collect_class_method_name_effects(
        &self,
        method_name: &str,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        for class_info in self.classes.values() {
            if let Some(sig) = class_info.methods.get(method_name) {
                for eff in &sig.effects {
                    out.insert(eff.clone());
                }
            }
        }
    }

    pub(crate) fn collect_interface_methods(
        &self,
        interface_name: &str,
        out: &mut HashMap<String, FuncSig>,
        visited: &mut std::collections::HashSet<String>,
    ) {
        if !visited.insert(interface_name.to_string()) {
            return;
        }
        let (base_name, _, name_substitutions) =
            self.instantiated_interface_substitutions(interface_name);
        let Some(info) = self.interfaces.get(&base_name) else {
            return;
        };
        for parent in &info.extends {
            let instantiated_parent =
                self.substitute_interface_reference(parent, &name_substitutions);
            self.collect_interface_methods(&instantiated_parent, out, visited);
        }
        for (name, sig) in &info.methods {
            out.insert(
                name.clone(),
                self.instantiate_interface_signature(interface_name, sig),
            );
        }
    }
}
