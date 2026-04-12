use colored::Colorize;

pub(crate) fn validate_symbol_collisions(
    function_collisions: Vec<(String, String, String)>,
    class_collisions: Vec<(String, String, String)>,
    enum_collisions: Vec<(String, String, String)>,
    interface_collisions: Vec<(String, String, String)>,
    module_collisions: Vec<(String, String, String)>,
) -> Result<(), String> {
    report_collisions(
        "Function",
        function_collisions,
        "Project contains colliding top-level function names. Use module-qualified names or rename functions.",
    )?;
    report_collisions(
        "Class",
        class_collisions,
        "Project contains colliding top-level class names. Use unique class names per project.",
    )?;
    report_collisions(
        "Enum",
        enum_collisions,
        "Project contains colliding top-level enum names. Use unique enum names per project.",
    )?;
    report_collisions(
        "Interface",
        interface_collisions,
        "Project contains colliding top-level interface names. Use unique interface names per project.",
    )?;
    report_collisions(
        "Module",
        module_collisions,
        "Project contains colliding top-level module names. Use unique module names per project.",
    )?;

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
