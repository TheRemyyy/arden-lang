pub(crate) fn split_generic_args_static(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;

    for ch in input.chars() {
        match ch {
            '<' => {
                angle_depth += 1;
                current.push(ch);
            }
            '>' => {
                angle_depth = angle_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if angle_depth == 0 && paren_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }

    parts
}

pub(crate) fn format_diagnostic_class_name(name: &str) -> String {
    if let Some(open_bracket) = name.find('<') {
        if name.ends_with('>') {
            let base = &name[..open_bracket];
            let inner = &name[open_bracket + 1..name.len() - 1];
            let formatted_args = split_generic_args_static(inner)
                .into_iter()
                .map(|arg| format_diagnostic_class_name(&arg))
                .collect::<Vec<_>>()
                .join(", ");
            return format!("{}<{}>", base.replace("__", "."), formatted_args);
        }
    }

    name.replace("__", ".")
}
