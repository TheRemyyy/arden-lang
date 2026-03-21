use std::fs;
use std::path::Path;

fn strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            if !out.ends_with([' ', '\n', '\t']) {
                out.push(' ');
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            if !out.ends_with([' ', '\n', '\t']) {
                out.push(' ');
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn normalize_c_type(raw: &str) -> String {
    raw.trim()
        .replace('\t', " ")
        .split_whitespace()
        .filter(|token| {
            !matches!(
                *token,
                "const" | "volatile" | "register" | "signed" | "extern" | "static"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn map_c_type_to_apex(c_type: &str) -> Option<String> {
    let t = normalize_c_type(c_type);
    let compact = t.replace(' ', "");
    if compact.is_empty() {
        return None;
    }

    if compact.ends_with('*') {
        let base = compact.trim_end_matches('*');
        if base == "char" {
            return Some("String".to_string());
        }
        return Some("Ptr<None>".to_string());
    }

    match compact.as_str() {
        "void" => Some("None".to_string()),
        "char" => Some("Char".to_string()),
        "float" | "double" => Some("Float".to_string()),
        "bool" | "_Bool" => Some("Boolean".to_string()),
        "int" | "short" | "long" | "longlong" | "size_t" | "ssize_t" | "intptr_t" | "uintptr_t"
        | "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t" | "int8_t" | "int16_t" | "int32_t"
        | "int64_t" | "unsigned" | "unsignedint" | "unsignedshort" | "unsignedlong"
        | "unsignedlonglong" => Some("Integer".to_string()),
        _ => None,
    }
}

fn parse_param(param: &str, index: usize) -> Option<(String, String)> {
    let p = param.trim();
    if p.is_empty() || p == "void" {
        return None;
    }
    if p.contains('(') || p.contains(')') {
        // Skip function-pointer params in this simple generator.
        return None;
    }

    let tokens: Vec<&str> = p.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let mut name = tokens[tokens.len() - 1].to_string();
    let mut type_part = tokens[..tokens.len() - 1].join(" ");
    if name.chars().all(|c| c == '*') {
        type_part = p.to_string();
        name = format!("arg{}", index);
    } else if name.starts_with('*') {
        let stars = name.chars().take_while(|c| *c == '*').count();
        name = name[stars..].to_string();
        type_part.push_str(&"*".repeat(stars));
    } else if tokens.len() == 1 {
        // No explicit parameter name in prototype.
        type_part = p.to_string();
        name = format!("arg{}", index);
    }

    let apex_ty = map_c_type_to_apex(&type_part)?;
    Some((name, apex_ty))
}

fn generate_from_prototype(proto: &str) -> Option<String> {
    let s = proto.trim();
    if s.is_empty()
        || s.starts_with('#')
        || s.contains('{')
        || s.starts_with("typedef")
        || s.starts_with("struct ")
        || s.starts_with("enum ")
        || s.starts_with("union ")
    {
        return None;
    }

    let open = s.find('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }

    let head = s[..open].trim();
    let params_raw = s[open + 1..close].trim();
    let head_tokens: Vec<&str> = head.split_whitespace().collect();
    if head_tokens.len() < 2 {
        return None;
    }

    let mut name = head_tokens[head_tokens.len() - 1].to_string();
    let pointer_prefix_len = name.chars().take_while(|c| *c == '*').count();
    let pointer_prefix = "*".repeat(pointer_prefix_len);
    if pointer_prefix_len > 0 {
        name = name[pointer_prefix_len..].to_string();
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let mut ret_c = head_tokens[..head_tokens.len() - 1].join(" ");
    ret_c.push_str(&pointer_prefix);
    let ret_apex = map_c_type_to_apex(&ret_c)?;

    let mut params = Vec::new();
    let mut variadic = false;
    if !params_raw.is_empty() && params_raw != "void" {
        for (i, part) in params_raw.split(',').enumerate() {
            let part = part.trim();
            if part == "..." {
                variadic = true;
                break;
            }
            let (pname, pty) = parse_param(part, i)?;
            params.push(format!("{}: {}", pname, pty));
        }
    }

    let mut signature = format!("extern(c) function {}(", name);
    signature.push_str(&params.join(", "));
    if variadic {
        if !params.is_empty() {
            signature.push_str(", ");
        }
        signature.push_str("...");
    }
    signature.push_str(&format!("): {};", ret_apex));
    Some(signature)
}

pub fn generate_bindings(header: &Path, output: Option<&Path>) -> Result<usize, String> {
    let raw = fs::read_to_string(header)
        .map_err(|e| format!("Failed to read header '{}': {}", header.display(), e))?;
    let stripped = strip_comments(&raw);

    let mut lines = Vec::new();
    lines.push("// Auto-generated by apex bindgen".to_string());
    lines.push("// Review and adjust signatures before production use.".to_string());
    lines.push(String::new());

    let mut count = 0usize;
    for stmt in stripped.split(';') {
        if let Some(sig) = generate_from_prototype(stmt) {
            lines.push(sig);
            count += 1;
        }
    }

    let out_text = lines.join("\n") + "\n";
    if let Some(path) = output {
        fs::write(path, out_text)
            .map_err(|e| format!("Failed to write output '{}': {}", path.display(), e))?;
    } else {
        print!("{}", out_text);
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::{generate_bindings, generate_from_prototype, strip_comments};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_pointer_return_prototypes() {
        let generated = generate_from_prototype("char *strdup(const char *s)")
            .expect("pointer return prototype should parse");
        assert_eq!(generated, "extern(c) function strdup(s: String): String;");
    }

    #[test]
    fn skips_function_pointer_param_prototypes_entirely() {
        let generated = generate_from_prototype(
            "void qsort(void *base, size_t n, size_t sz, int (*cmp)(const void*, const void*))",
        );
        assert!(generated.is_none());
    }

    #[test]
    fn keeps_tokens_separated_when_stripping_inline_block_comments() {
        let stripped = strip_comments("unsigned/*comment*/int count(void);");
        assert_eq!(stripped, "unsigned int count(void);");
    }

    #[test]
    fn preserves_unsigned_type_normalization() {
        let generated = generate_from_prototype("unsigned short checksum(unsigned int value)")
            .expect("unsigned integer types should normalize correctly");
        assert_eq!(
            generated,
            "extern(c) function checksum(value: Integer): Integer;"
        );
    }

    #[test]
    fn bindgen_handles_comment_only_token_boundaries_in_real_header_flow() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let header_path = std::env::temp_dir().join(format!("apex_bindgen_{unique}.h"));
        let output_path = std::env::temp_dir().join(format!("apex_bindgen_{unique}.apex"));
        let header = "unsigned/*keep-space*/int count(void);\n";
        std::fs::write(&header_path, header).expect("temporary header should be written");

        let count =
            generate_bindings(&header_path, Some(&output_path)).expect("bindgen should succeed");
        let generated =
            std::fs::read_to_string(&output_path).expect("generated bindings should be readable");

        let _ = std::fs::remove_file(&header_path);
        let _ = std::fs::remove_file(&output_path);

        assert_eq!(count, 1);
        assert!(generated.contains("extern(c) function count(): Integer;"));
    }
}
