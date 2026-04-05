use super::*;

impl TypeChecker {
    pub(crate) fn check(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        self.populate_import_aliases(program);
        // First pass: collect all declarations
        self.collect_declarations(program);
        self.normalize_inheritance_references();
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
        // Infer effects for non-annotated call graph nodes
        self.infer_effects(program);

        // Second pass: check all function bodies
        for decl in &program.declarations {
            self.check_decl_with_prefix(&decl.node, decl.span.clone(), None);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }
}
