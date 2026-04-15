use crate::ast::Expr;

/// Calculate Levenshtein distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut previous_row: Vec<usize> = (0..=len_b).collect();
    let mut current_row: Vec<usize> = vec![0; len_b + 1];

    for (i, left_char) in a_chars.iter().enumerate() {
        current_row[0] = i + 1;
        for (j, right_char) in b_chars.iter().enumerate() {
            let replacement_cost = if left_char == right_char { 0 } else { 1 };
            current_row[j + 1] = (previous_row[j + 1] + 1)
                .min(current_row[j] + 1)
                .min(previous_row[j] + replacement_cost);
        }
        std::mem::swap(&mut previous_row, &mut current_row);
    }

    previous_row[len_b]
}

/// Find the closest matching string from candidates.
pub(super) fn did_you_mean(name: &str, candidates: &[String]) -> Option<String> {
    let mut best_match: Option<(String, usize)> = None;

    for candidate in candidates {
        let distance = levenshtein_distance(name, candidate);
        // Only suggest if distance is reasonable (<= 3 and less than half the length).
        let threshold = (name.chars().count() / 2).max(3);
        if distance <= threshold {
            if let Some((_, best_distance)) = &best_match {
                if distance < *best_distance {
                    best_match = Some((candidate.clone(), distance));
                }
            } else {
                best_match = Some((candidate.clone(), distance));
            }
        }
    }

    best_match.map(|(symbol, _)| symbol)
}

pub(super) fn flatten_field_chain(expr: &Expr) -> Option<Vec<String>> {
    match expr {
        Expr::Ident(name) => Some(vec![name.clone()]),
        Expr::Field { object, field } => {
            let mut parts = flatten_field_chain(&object.node)?;
            parts.push(field.clone());
            Some(parts)
        }
        _ => None,
    }
}

pub(super) fn looks_like_function_symbol(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
}

pub(super) fn direct_wildcard_member_name(
    import_path: &str,
    owner_ns: &str,
    symbol_name: &str,
) -> Option<String> {
    if owner_ns == import_path {
        return (!symbol_name.contains("__")).then(|| symbol_name.to_string());
    }

    let module_path = import_path.strip_prefix(owner_ns)?.strip_prefix('.')?;
    if module_path.is_empty() {
        return None;
    }
    let module_prefix = module_path.replace('.', "__");
    let remainder = symbol_name.strip_prefix(&format!("{}__", module_prefix))?;
    (!remainder.is_empty() && !remainder.contains("__")).then(|| remainder.to_string())
}

pub(super) fn direct_stdlib_wildcard_member_name(
    import_path: &str,
    owner_ns: &str,
    symbol_name: &str,
) -> Option<String> {
    if owner_ns != import_path {
        return None;
    }
    let member = symbol_name
        .split_once("__")
        .map(|(_, value)| value)
        .unwrap_or(symbol_name);
    (!member.is_empty() && !member.contains("__")).then(|| member.to_string())
}

pub(super) fn parse_alias_member_path(name: &str) -> Option<Vec<String>> {
    let mut parts = name
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    if let Some(last) = parts.last_mut() {
        if let Some((base, _)) = last.split_once('<') {
            *last = base.trim().to_string();
        }
    }
    if parts.last().is_some_and(String::is_empty) {
        return None;
    }
    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::{direct_stdlib_wildcard_member_name, parse_alias_member_path};

    #[test]
    fn stdlib_wildcard_member_ignores_nested_members() {
        assert_eq!(
            direct_stdlib_wildcard_member_name("std.net", "std.net", "Net__http__get"),
            None
        );
        assert_eq!(
            direct_stdlib_wildcard_member_name("std.math", "std.math", "Math__abs"),
            Some("abs".to_string())
        );
        assert_eq!(
            direct_stdlib_wildcard_member_name("std.io", "std.io", "println"),
            Some("println".to_string())
        );
    }

    #[test]
    fn parse_alias_member_path_trims_member_before_generics() {
        assert_eq!(
            parse_alias_member_path("alias.Box <Integer>"),
            Some(vec!["alias".to_string(), "Box".to_string()])
        );
    }

    #[test]
    fn parse_alias_member_path_rejects_empty_member_after_generic_strip() {
        assert_eq!(parse_alias_member_path("alias.<Integer>"), None);
    }
}
