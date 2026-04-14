use colored::Colorize;
use std::fmt;

#[derive(Debug)]
enum CollisionCheckError {
    Function(String),
    Class(String),
    Enum(String),
    Interface(String),
    Module(String),
}

impl fmt::Display for CollisionCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function(message)
            | Self::Class(message)
            | Self::Enum(message)
            | Self::Interface(message)
            | Self::Module(message) => write!(f, "{message}"),
        }
    }
}

impl From<CollisionCheckError> for String {
    fn from(value: CollisionCheckError) -> Self {
        value.to_string()
    }
}

impl From<String> for CollisionCheckError {
    fn from(value: String) -> Self {
        Self::Function(value)
    }
}

pub(crate) fn validate_symbol_collisions(
    function_collisions: Vec<(String, String, String)>,
    class_collisions: Vec<(String, String, String)>,
    enum_collisions: Vec<(String, String, String)>,
    interface_collisions: Vec<(String, String, String)>,
    module_collisions: Vec<(String, String, String)>,
) -> Result<(), String> {
    validate_symbol_collisions_impl(
        function_collisions,
        class_collisions,
        enum_collisions,
        interface_collisions,
        module_collisions,
    )
    .map_err(Into::into)
}

fn validate_symbol_collisions_impl(
    function_collisions: Vec<(String, String, String)>,
    class_collisions: Vec<(String, String, String)>,
    enum_collisions: Vec<(String, String, String)>,
    interface_collisions: Vec<(String, String, String)>,
    module_collisions: Vec<(String, String, String)>,
) -> Result<(), CollisionCheckError> {
    report_collisions(
        "Function",
        function_collisions,
        "Project contains colliding top-level function names. Use module-qualified names or rename functions.",
    )
    .map_err(CollisionCheckError::Function)?;
    report_collisions(
        "Class",
        class_collisions,
        "Project contains colliding top-level class names. Use unique class names per project.",
    )
    .map_err(CollisionCheckError::Class)?;
    report_collisions(
        "Enum",
        enum_collisions,
        "Project contains colliding top-level enum names. Use unique enum names per project.",
    )
    .map_err(CollisionCheckError::Enum)?;
    report_collisions(
        "Interface",
        interface_collisions,
        "Project contains colliding top-level interface names. Use unique interface names per project.",
    )
    .map_err(CollisionCheckError::Interface)?;
    report_collisions(
        "Module",
        module_collisions,
        "Project contains colliding top-level module names. Use unique module names per project.",
    )
    .map_err(CollisionCheckError::Module)?;

    Ok(())
}

fn report_collisions(
    symbol_kind: &str,
    collisions: Vec<(String, String, String)>,
    error_message: &str,
) -> Result<(), String> {
    if collisions.is_empty() {
        return Ok(());
    }

    eprintln!(
        "{} {symbol_kind} name collisions detected across namespaces:",
        "error".red().bold()
    );
    for (name, namespace_a, namespace_b) in collisions {
        eprintln!(
            "  → '{}' is defined in both '{}' and '{}'",
            name, namespace_a, namespace_b
        );
    }
    Err(error_message.to_string())
}
