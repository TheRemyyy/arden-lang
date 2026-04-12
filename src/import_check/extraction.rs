use crate::ast::{Decl, Program};
use std::collections::{HashMap, HashSet};

/// Extract all function definitions from a program with their namespace.
pub fn extract_function_namespaces(program: &Program, namespace: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    fn walk_decl(
        out: &mut HashMap<String, String>,
        decl: &Decl,
        namespace: &str,
        module_prefix: Option<String>,
    ) {
        match decl {
            Decl::Function(func) => {
                if let Some(module) = module_prefix {
                    out.insert(format!("{}__{}", module, func.name), namespace.to_string());
                } else {
                    out.insert(func.name.clone(), namespace.to_string());
                }
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}__{}", prefix, module.name)
                } else {
                    module.name.clone()
                };
                for inner in &module.declarations {
                    walk_decl(out, &inner.node, namespace, Some(next_prefix.clone()));
                }
            }
            Decl::Class(_) | Decl::Enum(_) | Decl::Interface(_) | Decl::Import(_) => {}
        }
    }

    for decl in &program.declarations {
        walk_decl(&mut result, &decl.node, namespace, None);
    }

    result
}

pub fn extract_known_namespace_paths(program: &Program, namespace: &str) -> HashSet<String> {
    let mut result = HashSet::from([namespace.to_string()]);

    fn walk_decl(
        out: &mut HashSet<String>,
        decl: &Decl,
        namespace: &str,
        module_prefix: Option<String>,
    ) {
        match decl {
            Decl::Class(class) => {
                let path = if let Some(prefix) = module_prefix {
                    format!("{}.{}", prefix, class.name)
                } else {
                    format!("{}.{}", namespace, class.name)
                };
                out.insert(path);
            }
            Decl::Enum(en) => {
                let path = if let Some(prefix) = module_prefix.as_ref() {
                    format!("{}.{}", prefix, en.name)
                } else {
                    format!("{}.{}", namespace, en.name)
                };
                out.insert(path);
                for variant in &en.variants {
                    let variant_path = if let Some(prefix) = module_prefix.as_ref() {
                        format!("{}.{}.{}", prefix, en.name, variant.name)
                    } else {
                        format!("{}.{}.{}", namespace, en.name, variant.name)
                    };
                    out.insert(variant_path);
                }
            }
            Decl::Interface(interface) => {
                let path = if let Some(prefix) = module_prefix {
                    format!("{}.{}", prefix, interface.name)
                } else {
                    format!("{}.{}", namespace, interface.name)
                };
                out.insert(path);
            }
            Decl::Module(module) => {
                let next_prefix = if let Some(prefix) = module_prefix {
                    format!("{}.{}", prefix, module.name)
                } else {
                    format!("{}.{}", namespace, module.name)
                };
                out.insert(next_prefix.clone());
                for inner in &module.declarations {
                    walk_decl(out, &inner.node, namespace, Some(next_prefix.clone()));
                }
            }
            Decl::Function(_) | Decl::Import(_) => {}
        }
    }

    for decl in &program.declarations {
        walk_decl(&mut result, &decl.node, namespace, None);
    }

    result
}
