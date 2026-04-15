use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
enum BindgenError {
    HeaderValidation(String),
    HeaderRead(String),
    OutputPathValidation(String),
    OutputDirCreate(String),
    OutputWrite(String),
}

impl fmt::Display for BindgenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderValidation(message)
            | Self::HeaderRead(message)
            | Self::OutputPathValidation(message)
            | Self::OutputDirCreate(message)
            | Self::OutputWrite(message) => write!(f, "{message}"),
        }
    }
}

impl From<BindgenError> for String {
    fn from(value: BindgenError) -> Self {
        value.to_string()
    }
}

impl From<String> for BindgenError {
    fn from(value: String) -> Self {
        Self::OutputWrite(value)
    }
}

pub(crate) fn strip_comments(input: &str) -> String {
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
    let mut normalized = raw.trim().replace('\t', " ");
    for qualifier in ["restrict", "__restrict", "__restrict__"] {
        normalized = normalized.replace(&format!("*{qualifier}"), "*");
        normalized = normalized.replace(&format!("{qualifier}*"), "*");
    }

    normalized
        .split_whitespace()
        .filter(|token| {
            !matches!(
                *token,
                "const"
                    | "volatile"
                    | "register"
                    | "extern"
                    | "static"
                    | "inline"
                    | "restrict"
                    | "__restrict"
                    | "__restrict__"
                    | "__inline"
                    | "__inline__"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn map_c_type_to_arden(c_type: &str) -> Option<String> {
    let t = normalize_c_type(c_type);
    let tokens: Vec<&str> = t.split_whitespace().collect();
    let compact = t.replace(' ', "");
    if compact.is_empty() {
        return None;
    }

    if compact.ends_with('*') {
        let pointer_depth = compact.chars().rev().take_while(|c| *c == '*').count();
        let base = compact.trim_end_matches('*');
        if base == "char" && pointer_depth == 1 {
            return Some("String".to_string());
        }
        return Some("Ptr<None>".to_string());
    }

    if compact == "signed" {
        return Some("Integer".to_string());
    }

    if !tokens.is_empty()
        && tokens.iter().all(|token| {
            matches!(
                *token,
                "signed" | "unsigned" | "short" | "long" | "int" | "char"
            )
        })
        && !(tokens.len() == 1 && tokens[0] == "char")
    {
        return Some("Integer".to_string());
    }

    match compact.as_str() {
        "void" => Some("None".to_string()),
        "char" => Some("Char".to_string()),
        "float" | "double" | "longdouble" => Some("Float".to_string()),
        "bool" | "_Bool" => Some("Boolean".to_string()),
        "int"
        | "short"
        | "long"
        | "longint"
        | "longlong"
        | "longlongint"
        | "size_t"
        | "ssize_t"
        | "intptr_t"
        | "uintptr_t"
        | "uint8_t"
        | "uint16_t"
        | "uint32_t"
        | "uint64_t"
        | "int8_t"
        | "int16_t"
        | "int32_t"
        | "int64_t"
        | "unsigned"
        | "unsignedchar"
        | "unsignedint"
        | "unsignedshort"
        | "unsignedlong"
        | "unsignedlongint"
        | "unsignedlonglong"
        | "unsignedlonglongint"
        | "signedchar"
        | "signedint"
        | "signedshort"
        | "signedlong"
        | "signedlongint"
        | "signedlonglong"
        | "signedlonglongint" => Some("Integer".to_string()),
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

    let mut array_depth = 0usize;
    let mut name_index = tokens.len() - 1;
    while name_index > 0 && tokens[name_index].starts_with('[') && tokens[name_index].ends_with(']')
    {
        array_depth += 1;
        name_index -= 1;
    }

    let mut name = tokens[name_index].to_string();
    let mut type_part = tokens[..name_index].join(" ");
    while let Some(open) = name.find('[') {
        let close = name[open..].find(']')?;
        name.replace_range(open..open + close + 1, "");
        array_depth += 1;
    }
    if name.chars().all(|c| c == '*') {
        type_part = p.to_string();
        name = format!("arg{}", index);
    } else if name.starts_with('*') {
        let stars = name.chars().take_while(|c| *c == '*').count();
        name = name[stars..].to_string();
        type_part.push_str(&"*".repeat(stars));
    } else if name_index == 0 && tokens.len() == 1 {
        // No explicit parameter name in prototype.
        type_part = p.to_string();
        name = format!("arg{}", index);
    }
    if array_depth > 0 {
        type_part.push_str(&"*".repeat(array_depth));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }

    let arden_ty = map_c_type_to_arden(&type_part)?;
    Some((name, arden_ty))
}

pub(crate) fn generate_from_prototype(proto: &str) -> Option<String> {
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
    let ret_arden = map_c_type_to_arden(&ret_c)?;

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
    signature.push_str(&format!("): {};", ret_arden));
    Some(signature)
}

pub fn generate_bindings(header: &Path, output: Option<&Path>) -> Result<usize, String> {
    generate_bindings_impl(header, output).map_err(Into::into)
}

fn generate_bindings_impl(header: &Path, output: Option<&Path>) -> Result<usize, BindgenError> {
    if !header.exists() {
        return Err(BindgenError::HeaderValidation(format!(
            "Header '{}' does not exist",
            crate::format_cli_path(header)
        )));
    }
    if !header.is_file() {
        return Err(BindgenError::HeaderValidation(format!(
            "Header '{}' is not a file",
            crate::format_cli_path(header)
        )));
    }

    let raw = fs::read_to_string(header).map_err(|e| {
        BindgenError::HeaderRead(format!(
            "Failed to read header '{}': {}",
            crate::format_cli_path(header),
            e
        ))
    })?;
    let stripped = strip_comments(&raw);

    let mut lines = Vec::new();
    lines.push("// Auto-generated by arden bindgen".to_string());
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
        if path.exists() && path.is_dir() {
            return Err(BindgenError::OutputPathValidation(format!(
                "Bindgen output path '{}' is a directory; expected a file path",
                crate::format_cli_path(path)
            )));
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| {
                    BindgenError::OutputDirCreate(format!(
                        "Failed to create bindgen output directory '{}': {}",
                        crate::format_cli_path(parent),
                        e
                    ))
                })?;
            }
        }
        fs::write(path, out_text).map_err(|e| {
            BindgenError::OutputWrite(format!(
                "Failed to write output '{}': {}",
                crate::format_cli_path(path),
                e
            ))
        })?;
    } else {
        print!("{}", out_text);
    }

    Ok(count)
}
