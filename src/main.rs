//! Apex Programming Language Compiler

mod ast;
mod borrowck;
mod codegen;
mod import_check;
mod lexer;
mod lsp;
mod namespace;
mod parser;
mod project;
mod stdlib;
mod test_runner;
mod typeck;

use clap::{Parser as ClapParser, Subcommand};
use colored::*;
use inkwell::context::Context;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ast::{Decl, Expr, ImportDecl, Program, Stmt};
use crate::borrowck::BorrowChecker;
use crate::codegen::Codegen;
use crate::import_check::ImportChecker;
use crate::parser::Parser;
use crate::project::{find_project_root, ProjectConfig};
use crate::test_runner::{discover_tests, generate_test_runner_with_source, print_discovery};
use crate::typeck::TypeChecker;

#[derive(ClapParser)]
#[command(name = "apex")]
#[command(author = "Remyyy")]
#[command(version = "1.3.1")]
#[command(about = "Apex Programming Language Compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Apex project
    New {
        /// Project name
        name: String,
        /// Project path (default: current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Build the current project
    Build {
        /// Release build (optimized)
        #[arg(short, long)]
        release: bool,
        /// Emit LLVM IR
        #[arg(long)]
        emit_llvm: bool,
        /// Skip type checking
        #[arg(long)]
        no_check: bool,
    },
    /// Build and run the current project (or a single file)
    Run {
        /// Input file (optional, runs project if not specified)
        file: Option<PathBuf>,
        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        /// Release build (optimized)
        #[arg(short, long)]
        release: bool,
        /// Skip type checking
        #[arg(long)]
        no_check: bool,
    },
    /// Compile a single file (legacy mode)
    Compile {
        /// Input file
        file: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Emit LLVM IR
        #[arg(long)]
        emit_llvm: bool,
        /// Skip type checking
        #[arg(long)]
        no_check: bool,
    },
    /// Check syntax and types of a file
    Check {
        /// Input file (default: project entry point)
        file: Option<PathBuf>,
    },
    /// Show project info
    Info,
    /// Show tokens (debug)
    Lex {
        /// Input file
        file: PathBuf,
    },
    /// Show AST (debug)
    Parse {
        /// Input file
        file: PathBuf,
    },
    /// Start LSP server
    Lsp,
    /// Run tests
    Test {
        /// Input file or directory (default: project test files)
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// List tests without running them
        #[arg(short, long)]
        list: bool,
        /// Filter tests by name pattern
        #[arg(short, long)]
        filter: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::New { name, path } => new_project(&name, path.as_deref()),
        Commands::Build {
            release,
            emit_llvm,
            no_check,
        } => build_project(release, emit_llvm, !no_check),
        Commands::Run {
            file,
            args,
            release,
            no_check,
        } => {
            if let Some(f) = file {
                run_single_file(&f, &args, release, !no_check)
            } else {
                run_project(&args, release, !no_check)
            }
        }
        Commands::Compile {
            file,
            output,
            emit_llvm,
            no_check,
        } => compile_file(&file, output.as_deref(), emit_llvm, !no_check),
        Commands::Check { file } => check_file(file.as_deref()),
        Commands::Info => show_project_info(),
        Commands::Lex { file } => lex_file(&file),
        Commands::Parse { file } => parse_file(&file),
        Commands::Lsp => {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(lsp::run_lsp_server());
            Ok(())
        }
        Commands::Test { path, list, filter } => {
            run_tests(path.as_deref(), list, filter.as_deref())
        }
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    std::process::exit(0);
}

/// Create a new project
fn new_project(name: &str, path: Option<&Path>) -> Result<(), String> {
    let project_path = path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(name));

    if project_path.exists() {
        return Err(format!(
            "{}: Directory '{}' already exists",
            "error".red().bold(),
            project_path.display()
        ));
    }

    // Create project directory structure
    fs::create_dir_all(&project_path).map_err(|e| {
        format!(
            "{}: Failed to create project directory: {}",
            "error".red().bold(),
            e
        )
    })?;

    fs::create_dir_all(project_path.join("src")).map_err(|e| {
        format!(
            "{}: Failed to create src directory: {}",
            "error".red().bold(),
            e
        )
    })?;

    // Create project config
    let config = ProjectConfig::new(name);
    let config_path = project_path.join("apex.toml");
    config.save(&config_path)?;

    // Create main.apex
    let main_content = format!(
        r#"// Welcome to {}!
// This is the entry point of your application.

function main(): None {{
    println("Hello from {}!");
    return None;
}}
"#,
        name, name
    );

    let main_path = project_path.join("src").join("main.apex");
    fs::write(&main_path, main_content).map_err(|e| {
        format!(
            "{}: Failed to create main.apex: {}",
            "error".red().bold(),
            e
        )
    })?;

    // Create README.md
    let readme_content = format!(
        r#"# {}

Apex project created with `apex new`.

## Project Structure

```
.
├── apex.toml       # Project configuration
├── src/
│   └── main.apex   # Entry point
└── README.md       # This file
```

## Commands

- `apex build` - Build the project
- `apex run` - Build and run the project
- `apex info` - Show project information

## Configuration

Edit `apex.toml` to customize your project:

```toml
name = "{}"
version = "0.1.0"
entry = "src/main.apex"
files = ["src/main.apex"]
output = "{}"
opt_level = "3"
```
"#,
        name, name, name
    );

    let readme_path = project_path.join("README.md");
    fs::write(&readme_path, readme_content).map_err(|e| {
        format!(
            "{}: Failed to create README.md: {}",
            "error".red().bold(),
            e
        )
    })?;

    println!(
        "{} {} project '{}'",
        "Created".green().bold(),
        "Apex".cyan(),
        name
    );
    println!(
        "  {} {}",
        "Location:".dimmed(),
        project_path
            .canonicalize()
            .unwrap_or(project_path)
            .display()
    );
    println!("\nTo get started:");
    println!("  cd {}", name);
    println!("  apex run");

    Ok(())
}

/// Build the current project with proper namespace checking
fn build_project(_release: bool, emit_llvm: bool, do_check: bool) -> Result<(), String> {
    let project_root = find_project_root(&std::env::current_dir().unwrap())
        .ok_or_else(|| format!("{}: No apex.toml found. Are you in a project directory?\nRun `apex new <name>` to create a new project.",
            "error".red().bold()))?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;

    // Validate project
    config.validate(&project_root)?;

    println!(
        "{} {} v{}",
        "Building".green().bold(),
        config.name.cyan(),
        config.version.dimmed()
    );

    // Phase 1: Parse all files and extract namespace information
    let files = config.get_source_files(&project_root);
    let mut parsed_files: Vec<(PathBuf, String, Program, Vec<ImportDecl>)> = Vec::new();
    let mut global_function_map: HashMap<String, String> = HashMap::new(); // func_name -> namespace
    let mut global_class_map: HashMap<String, String> = HashMap::new(); // class_name -> namespace
    let mut global_module_map: HashMap<String, String> = HashMap::new(); // module_name -> namespace
    let mut namespace_class_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut namespace_module_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut function_collisions: Vec<(String, String, String)> = Vec::new();
    let mut class_collisions: Vec<(String, String, String)> = Vec::new();
    let mut module_collisions: Vec<(String, String, String)> = Vec::new();

    for file in &files {
        let source = fs::read_to_string(file).map_err(|e| {
            format!(
                "{}: Failed to read '{}': {}",
                "error".red().bold(),
                file.display(),
                e
            )
        })?;

        let filename = file.file_name().unwrap().to_str().unwrap_or("unknown.apex");

        // Parse the file
        let tokens = lexer::tokenize(&source).map_err(|e| {
            format!(
                "{}: Lexer error in {}: {}",
                "error".red().bold(),
                filename,
                e
            )
        })?;

        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().map_err(|e| {
            format!(
                "{}: Parse error in {}: {}",
                "error".red().bold(),
                filename,
                e.message
            )
        })?;

        // Extract namespace from package declaration
        let namespace = program
            .package
            .clone()
            .unwrap_or_else(|| "global".to_string());

        // Extract imports
        let imports: Vec<ImportDecl> = program
            .declarations
            .iter()
            .filter_map(|d| match &d.node {
                Decl::Import(import) => Some(import.clone()),
                _ => None,
            })
            .collect();

        // Extract symbol definitions for global maps
        let class_entry = namespace_class_map.entry(namespace.clone()).or_default();
        let module_entry = namespace_module_map.entry(namespace.clone()).or_default();
        for decl in &program.declarations {
            match &decl.node {
                Decl::Function(func) => {
                    if let Some(existing_ns) = global_function_map.get(&func.name) {
                        if existing_ns != &namespace {
                            function_collisions.push((
                                func.name.clone(),
                                existing_ns.clone(),
                                namespace.clone(),
                            ));
                        }
                    } else {
                        global_function_map.insert(func.name.clone(), namespace.clone());
                    }
                }
                Decl::Class(class) => {
                    class_entry.insert(class.name.clone());
                    if let Some(existing_ns) = global_class_map.get(&class.name) {
                        if existing_ns != &namespace {
                            class_collisions.push((
                                class.name.clone(),
                                existing_ns.clone(),
                                namespace.clone(),
                            ));
                        }
                    } else {
                        global_class_map.insert(class.name.clone(), namespace.clone());
                    }
                }
                Decl::Module(module) => {
                    module_entry.insert(module.name.clone());
                    if let Some(existing_ns) = global_module_map.get(&module.name) {
                        if existing_ns != &namespace {
                            module_collisions.push((
                                module.name.clone(),
                                existing_ns.clone(),
                                namespace.clone(),
                            ));
                        }
                    } else {
                        global_module_map.insert(module.name.clone(), namespace.clone());
                    }
                }
                _ => {}
            }
        }

        parsed_files.push((file.clone(), namespace, program, imports));
    }

    if !function_collisions.is_empty() {
        eprintln!(
            "{} Function name collisions detected across namespaces:",
            "error".red().bold()
        );
        for (func, ns_a, ns_b) in function_collisions {
            eprintln!(
                "  → '{}' is defined in both '{}' and '{}'",
                func, ns_a, ns_b
            );
        }
        return Err(
            "Project contains colliding top-level function names. Use module-qualified names or rename functions."
                .to_string(),
        );
    }
    if !class_collisions.is_empty() {
        eprintln!(
            "{} Class name collisions detected across namespaces:",
            "error".red().bold()
        );
        for (name, ns_a, ns_b) in class_collisions {
            eprintln!(
                "  â†’ '{}' is defined in both '{}' and '{}'",
                name, ns_a, ns_b
            );
        }
        return Err(
            "Project contains colliding top-level class names. Use unique class names per project."
                .to_string(),
        );
    }
    if !module_collisions.is_empty() {
        eprintln!(
            "{} Module name collisions detected across namespaces:",
            "error".red().bold()
        );
        for (name, ns_a, ns_b) in module_collisions {
            eprintln!(
                "  â†’ '{}' is defined in both '{}' and '{}'",
                name, ns_a, ns_b
            );
        }
        return Err(
            "Project contains colliding top-level module names. Use unique module names per project."
                .to_string(),
        );
    }

    // Phase 2: Check imports for each file
    if do_check {
        println!("{} Checking imports...", "→".cyan());

        for (file, namespace, program, imports) in &parsed_files {
            let mut checker = ImportChecker::new(
                global_function_map.clone(),
                namespace.clone(),
                imports.clone(),
            );

            if let Err(errors) = checker.check_program(program) {
                let filename = file.file_name().unwrap().to_str().unwrap_or("unknown");
                eprintln!("{} Import errors in {}:", "error".red().bold(), filename);
                for err in errors {
                    eprintln!("  → {}", err.format());
                }
                return Err("Import check failed".to_string());
            }
        }
    }

    let entry_path = config.get_entry_path(&project_root);
    let mut namespace_functions: HashMap<String, HashSet<String>> = HashMap::new();
    for (_, ns, program, _) in &parsed_files {
        let entry = namespace_functions.entry(ns.clone()).or_default();
        for decl in &program.declarations {
            if let Decl::Function(func) = &decl.node {
                entry.insert(func.name.clone());
            }
        }
    }

    let entry_namespace = parsed_files
        .iter()
        .find(|(file, _, _, _)| file == &entry_path)
        .map(|(_, ns, _, _)| ns.clone())
        .unwrap_or_else(|| "global".to_string());

    // Phase 3: Build combined AST with deterministic namespace mangling.
    let mut combined_program = Program {
        package: None,
        declarations: Vec::new(),
    };
    for (_, namespace, program, imports) in &parsed_files {
        let rewritten = rewrite_program_for_project(
            program,
            namespace,
            &entry_namespace,
            &namespace_functions,
            &global_function_map,
            &namespace_class_map,
            &global_class_map,
            &namespace_module_map,
            &global_module_map,
            imports,
        );
        combined_program
            .declarations
            .extend(rewritten.declarations.into_iter());
    }

    // Compile combined program AST (import/type checks already done above).
    let output_path = project_root.join(&config.output);
    compile_program_ast(
        &combined_program,
        &entry_path,
        &output_path,
        emit_llvm,
        Some(&config.opt_level),
    )?;

    println!(
        "{} {} -> {}",
        "Built".green().bold(),
        config.name.cyan(),
        output_path.display()
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn rewrite_program_for_project(
    program: &Program,
    current_namespace: &str,
    entry_namespace: &str,
    namespace_functions: &HashMap<String, HashSet<String>>,
    global_function_map: &HashMap<String, String>,
    namespace_classes: &HashMap<String, HashSet<String>>,
    global_class_map: &HashMap<String, String>,
    namespace_modules: &HashMap<String, HashSet<String>>,
    global_module_map: &HashMap<String, String>,
    imports: &[ImportDecl],
) -> Program {
    let local_functions = namespace_functions
        .get(current_namespace)
        .cloned()
        .unwrap_or_default();
    let local_classes = namespace_classes
        .get(current_namespace)
        .cloned()
        .unwrap_or_default();
    let local_modules = namespace_modules
        .get(current_namespace)
        .cloned()
        .unwrap_or_default();

    let mut imported_map: HashMap<String, String> = HashMap::new();
    let mut imported_classes: HashMap<String, String> = HashMap::new();
    let mut imported_modules: HashMap<String, String> = HashMap::new();
    for import in imports {
        if import.path.ends_with(".*") {
            let ns = import.path.trim_end_matches(".*");
            if let Some(funcs) = namespace_functions.get(ns) {
                for name in funcs {
                    imported_map.insert(name.clone(), ns.to_string());
                }
            }
            if let Some(classes) = namespace_classes.get(ns) {
                for name in classes {
                    imported_classes.insert(name.clone(), ns.to_string());
                }
            }
            if let Some(modules) = namespace_modules.get(ns) {
                for name in modules {
                    imported_modules.insert(name.clone(), ns.to_string());
                }
            }
        } else if import.path.contains('.') {
            let mut parts = import.path.split('.').collect::<Vec<_>>();
            if let Some(name) = parts.pop() {
                let ns = parts.join(".");
                imported_map.insert(name.to_string(), ns.clone());
                imported_classes.insert(name.to_string(), ns.clone());
                imported_modules.insert(name.to_string(), ns);
            }
        }
    }

    Program {
        package: None,
        declarations: program
            .declarations
            .iter()
            .filter(|d| !matches!(d.node, Decl::Import(_)))
            .map(|d| {
                let node = match &d.node {
                    Decl::Function(func) => {
                        let mut f = func.clone();
                        let mut scopes = vec![f.params.iter().map(|p| p.name.clone()).collect()];
                        f.params = f
                            .params
                            .iter()
                            .map(|p| ast::Parameter {
                                name: p.name.clone(),
                                ty: rewrite_type_for_project(
                                    &p.ty,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    entry_namespace,
                                ),
                                mutable: p.mutable,
                                mode: p.mode,
                            })
                            .collect();
                        f.return_type = rewrite_type_for_project(
                            &f.return_type,
                            current_namespace,
                            &local_classes,
                            &imported_classes,
                            global_class_map,
                            entry_namespace,
                        );
                        f.body = rewrite_block_calls_for_project(
                            &f.body,
                            current_namespace,
                            entry_namespace,
                            &local_functions,
                            &imported_map,
                            global_function_map,
                            &local_classes,
                            &imported_classes,
                            global_class_map,
                            &local_modules,
                            &imported_modules,
                            global_module_map,
                            &mut scopes,
                        );
                        f.name = mangle_project_symbol(current_namespace, entry_namespace, &f.name);
                        Decl::Function(f)
                    }
                    Decl::Class(class) => {
                        let mut c = class.clone();
                        c.name = mangle_project_symbol(current_namespace, entry_namespace, &c.name);
                        c.fields = c
                            .fields
                            .iter()
                            .map(|field| ast::Field {
                                name: field.name.clone(),
                                ty: rewrite_type_for_project(
                                    &field.ty,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    entry_namespace,
                                ),
                                mutable: field.mutable,
                                visibility: field.visibility,
                            })
                            .collect();
                        if let Some(ctor) = &class.constructor {
                            let mut new_ctor = ctor.clone();
                            let mut scopes: Vec<HashSet<String>> =
                                vec![new_ctor.params.iter().map(|p| p.name.clone()).collect()];
                            if let Some(scope) = scopes.last_mut() {
                                scope.insert("this".to_string());
                            }
                            new_ctor.params = new_ctor
                                .params
                                .iter()
                                .map(|p| ast::Parameter {
                                    name: p.name.clone(),
                                    ty: rewrite_type_for_project(
                                        &p.ty,
                                        current_namespace,
                                        &local_classes,
                                        &imported_classes,
                                        global_class_map,
                                        entry_namespace,
                                    ),
                                    mutable: p.mutable,
                                    mode: p.mode,
                                })
                                .collect();
                            new_ctor.body = rewrite_block_calls_for_project(
                                &new_ctor.body,
                                current_namespace,
                                entry_namespace,
                                &local_functions,
                                &imported_map,
                                global_function_map,
                                &local_classes,
                                &imported_classes,
                                global_class_map,
                                &local_modules,
                                &imported_modules,
                                global_module_map,
                                &mut scopes,
                            );
                            c.constructor = Some(new_ctor);
                        }
                        c.methods = class
                            .methods
                            .iter()
                            .map(|m| {
                                let mut nm = m.clone();
                                let mut scopes: Vec<HashSet<String>> =
                                    vec![nm.params.iter().map(|p| p.name.clone()).collect()];
                                if let Some(scope) = scopes.last_mut() {
                                    scope.insert("this".to_string());
                                }
                                nm.params = nm
                                    .params
                                    .iter()
                                    .map(|p| ast::Parameter {
                                        name: p.name.clone(),
                                        ty: rewrite_type_for_project(
                                            &p.ty,
                                            current_namespace,
                                            &local_classes,
                                            &imported_classes,
                                            global_class_map,
                                            entry_namespace,
                                        ),
                                        mutable: p.mutable,
                                        mode: p.mode,
                                    })
                                    .collect();
                                nm.return_type = rewrite_type_for_project(
                                    &nm.return_type,
                                    current_namespace,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    entry_namespace,
                                );
                                nm.body = rewrite_block_calls_for_project(
                                    &nm.body,
                                    current_namespace,
                                    entry_namespace,
                                    &local_functions,
                                    &imported_map,
                                    global_function_map,
                                    &local_classes,
                                    &imported_classes,
                                    global_class_map,
                                    &local_modules,
                                    &imported_modules,
                                    global_module_map,
                                    &mut scopes,
                                );
                                nm
                            })
                            .collect();
                        Decl::Class(c)
                    }
                    Decl::Module(module) => {
                        let mut m = module.clone();
                        m.name = mangle_project_symbol(current_namespace, entry_namespace, &m.name);
                        Decl::Module(m)
                    }
                    Decl::Enum(en) => {
                        let mut e = en.clone();
                        e.name = mangle_project_symbol(current_namespace, entry_namespace, &e.name);
                        e.variants = e
                            .variants
                            .iter()
                            .map(|v| ast::EnumVariant {
                                name: v.name.clone(),
                                fields: v
                                    .fields
                                    .iter()
                                    .map(|f| ast::EnumField {
                                        name: f.name.clone(),
                                        ty: rewrite_type_for_project(
                                            &f.ty,
                                            current_namespace,
                                            &local_classes,
                                            &imported_classes,
                                            global_class_map,
                                            entry_namespace,
                                        ),
                                    })
                                    .collect(),
                            })
                            .collect();
                        Decl::Enum(e)
                    }
                    _ => d.node.clone(),
                };
                ast::Spanned::new(node, d.span.clone())
            })
            .collect(),
    }
}

fn mangle_project_symbol(namespace: &str, entry_namespace: &str, name: &str) -> String {
    if name == "main" && namespace == entry_namespace {
        "main".to_string()
    } else {
        format!("{}__{}", namespace.replace('.', "__"), name)
    }
}

fn rewrite_type_for_project(
    ty: &ast::Type,
    current_namespace: &str,
    local_classes: &HashSet<String>,
    imported_classes: &HashMap<String, String>,
    global_class_map: &HashMap<String, String>,
    entry_namespace: &str,
) -> ast::Type {
    match ty {
        ast::Type::Named(name) => {
            if local_classes.contains(name) {
                ast::Type::Named(mangle_project_symbol(
                    current_namespace,
                    entry_namespace,
                    name,
                ))
            } else if let Some(ns) = imported_classes
                .get(name)
                .or_else(|| global_class_map.get(name))
            {
                ast::Type::Named(mangle_project_symbol(ns, entry_namespace, name))
            } else {
                ast::Type::Named(name.clone())
            }
        }
        ast::Type::Generic(name, args) => ast::Type::Generic(
            if local_classes.contains(name) {
                mangle_project_symbol(current_namespace, entry_namespace, name)
            } else if let Some(ns) = imported_classes
                .get(name)
                .or_else(|| global_class_map.get(name))
            {
                mangle_project_symbol(ns, entry_namespace, name)
            } else {
                name.clone()
            },
            args.iter()
                .map(|a| {
                    rewrite_type_for_project(
                        a,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        entry_namespace,
                    )
                })
                .collect(),
        ),
        ast::Type::Function(params, ret) => ast::Type::Function(
            params
                .iter()
                .map(|p| {
                    rewrite_type_for_project(
                        p,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        entry_namespace,
                    )
                })
                .collect(),
            Box::new(rewrite_type_for_project(
                ret,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
        ),
        ast::Type::Option(inner) => ast::Type::Option(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Result(ok, err) => ast::Type::Result(
            Box::new(rewrite_type_for_project(
                ok,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
            Box::new(rewrite_type_for_project(
                err,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
        ),
        ast::Type::List(inner) => ast::Type::List(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Map(k, v) => ast::Type::Map(
            Box::new(rewrite_type_for_project(
                k,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
            Box::new(rewrite_type_for_project(
                v,
                current_namespace,
                local_classes,
                imported_classes,
                global_class_map,
                entry_namespace,
            )),
        ),
        ast::Type::Set(inner) => ast::Type::Set(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Ref(inner) => ast::Type::Ref(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::MutRef(inner) => ast::Type::MutRef(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Box(inner) => ast::Type::Box(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Rc(inner) => ast::Type::Rc(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Arc(inner) => ast::Type::Arc(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Task(inner) => ast::Type::Task(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        ast::Type::Range(inner) => ast::Type::Range(Box::new(rewrite_type_for_project(
            inner,
            current_namespace,
            local_classes,
            imported_classes,
            global_class_map,
            entry_namespace,
        ))),
        _ => ty.clone(),
    }
}

fn is_shadowed(name: &str, scopes: &[HashSet<String>]) -> bool {
    scopes.iter().rev().any(|scope| scope.contains(name))
}

fn push_scope(scopes: &mut Vec<HashSet<String>>) {
    scopes.push(HashSet::new());
}

fn pop_scope(scopes: &mut Vec<HashSet<String>>) {
    if scopes.len() > 1 {
        scopes.pop();
    }
}

fn bind_pattern_locals(pattern: &ast::Pattern, scope: &mut HashSet<String>) {
    match pattern {
        ast::Pattern::Ident(name) => {
            scope.insert(name.clone());
        }
        ast::Pattern::Variant(_, bindings) => {
            for b in bindings {
                scope.insert(b.clone());
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn rewrite_block_calls_for_project(
    block: &ast::Block,
    current_namespace: &str,
    entry_namespace: &str,
    local_functions: &HashSet<String>,
    imported_map: &HashMap<String, String>,
    global_function_map: &HashMap<String, String>,
    local_classes: &HashSet<String>,
    imported_classes: &HashMap<String, String>,
    global_class_map: &HashMap<String, String>,
    local_modules: &HashSet<String>,
    imported_modules: &HashMap<String, String>,
    global_module_map: &HashMap<String, String>,
    scopes: &mut Vec<HashSet<String>>,
) -> ast::Block {
    block
        .iter()
        .map(|stmt| {
            ast::Spanned::new(
                rewrite_stmt_calls_for_project(
                    &stmt.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                stmt.span.clone(),
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn rewrite_stmt_calls_for_project(
    stmt: &Stmt,
    current_namespace: &str,
    entry_namespace: &str,
    local_functions: &HashSet<String>,
    imported_map: &HashMap<String, String>,
    global_function_map: &HashMap<String, String>,
    local_classes: &HashSet<String>,
    imported_classes: &HashMap<String, String>,
    global_class_map: &HashMap<String, String>,
    local_modules: &HashSet<String>,
    imported_modules: &HashMap<String, String>,
    global_module_map: &HashMap<String, String>,
    scopes: &mut Vec<HashSet<String>>,
) -> Stmt {
    match stmt {
        Stmt::Let {
            name,
            ty,
            value,
            mutable,
        } => {
            let rewritten = Stmt::Let {
                name: name.clone(),
                ty: rewrite_type_for_project(
                    ty,
                    current_namespace,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    entry_namespace,
                ),
                value: ast::Spanned::new(
                    rewrite_expr_calls_for_project(
                        &value.node,
                        current_namespace,
                        entry_namespace,
                        local_functions,
                        imported_map,
                        global_function_map,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_modules,
                        imported_modules,
                        global_module_map,
                        scopes,
                    ),
                    value.span.clone(),
                ),
                mutable: *mutable,
            };
            if let Some(scope) = scopes.last_mut() {
                scope.insert(name.clone());
            }
            rewritten
        }
        Stmt::Assign { target, value } => Stmt::Assign {
            target: ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &target.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                target.span.clone(),
            ),
            value: ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &value.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                value.span.clone(),
            ),
        },
        Stmt::Expr(expr) => Stmt::Expr(ast::Spanned::new(
            rewrite_expr_calls_for_project(
                &expr.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            ),
            expr.span.clone(),
        )),
        Stmt::Return(Some(expr)) => Stmt::Return(Some(ast::Spanned::new(
            rewrite_expr_calls_for_project(
                &expr.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            ),
            expr.span.clone(),
        ))),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            let condition = ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                condition.span.clone(),
            );
            push_scope(scopes);
            let then_block = rewrite_block_calls_for_project(
                then_block,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            );
            pop_scope(scopes);
            let else_block = else_block.as_ref().map(|b| {
                push_scope(scopes);
                let rewritten = rewrite_block_calls_for_project(
                    b,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                );
                pop_scope(scopes);
                rewritten
            });
            Stmt::If {
                condition,
                then_block,
                else_block,
            }
        }
        Stmt::While { condition, body } => {
            let condition = ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &condition.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                condition.span.clone(),
            );
            push_scope(scopes);
            let body = rewrite_block_calls_for_project(
                body,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            );
            pop_scope(scopes);
            Stmt::While { condition, body }
        }
        Stmt::For {
            var,
            var_type,
            iterable,
            body,
        } => {
            let iterable = ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &iterable.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                iterable.span.clone(),
            );
            push_scope(scopes);
            if let Some(scope) = scopes.last_mut() {
                scope.insert(var.clone());
            }
            let body = rewrite_block_calls_for_project(
                body,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            );
            pop_scope(scopes);
            Stmt::For {
                var: var.clone(),
                var_type: var_type.as_ref().map(|t| {
                    rewrite_type_for_project(
                        t,
                        current_namespace,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        entry_namespace,
                    )
                }),
                iterable,
                body,
            }
        }
        Stmt::Match { expr, arms } => Stmt::Match {
            expr: ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &expr.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                expr.span.clone(),
            ),
            arms: arms
                .iter()
                .map(|arm| {
                    push_scope(scopes);
                    if let Some(scope) = scopes.last_mut() {
                        bind_pattern_locals(&arm.pattern, scope);
                    }
                    let body = rewrite_block_calls_for_project(
                        &arm.body,
                        current_namespace,
                        entry_namespace,
                        local_functions,
                        imported_map,
                        global_function_map,
                        local_classes,
                        imported_classes,
                        global_class_map,
                        local_modules,
                        imported_modules,
                        global_module_map,
                        scopes,
                    );
                    pop_scope(scopes);
                    ast::MatchArm {
                        pattern: arm.pattern.clone(),
                        body,
                    }
                })
                .collect(),
        },
        _ => stmt.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn rewrite_expr_calls_for_project(
    expr: &Expr,
    current_namespace: &str,
    entry_namespace: &str,
    local_functions: &HashSet<String>,
    imported_map: &HashMap<String, String>,
    global_function_map: &HashMap<String, String>,
    local_classes: &HashSet<String>,
    imported_classes: &HashMap<String, String>,
    global_class_map: &HashMap<String, String>,
    local_modules: &HashSet<String>,
    imported_modules: &HashMap<String, String>,
    global_module_map: &HashMap<String, String>,
    scopes: &mut Vec<HashSet<String>>,
) -> Expr {
    match expr {
        Expr::Call { callee, args } => {
            let rewritten_callee = match &callee.node {
                Expr::Ident(name) => {
                    if is_shadowed(name, scopes) {
                        Expr::Ident(name.clone())
                    } else if local_functions.contains(name) {
                        Expr::Ident(mangle_project_symbol(
                            current_namespace,
                            entry_namespace,
                            name,
                        ))
                    } else if let Some(ns) = imported_map.get(name) {
                        Expr::Ident(mangle_project_symbol(ns, entry_namespace, name))
                    } else if let Some(ns) = global_function_map.get(name) {
                        Expr::Ident(mangle_project_symbol(ns, entry_namespace, name))
                    } else {
                        Expr::Ident(name.clone())
                    }
                }
                other => rewrite_expr_calls_for_project(
                    other,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
            };
            Expr::Call {
                callee: Box::new(ast::Spanned::new(rewritten_callee, callee.span.clone())),
                args: args
                    .iter()
                    .map(|a| {
                        ast::Spanned::new(
                            rewrite_expr_calls_for_project(
                                &a.node,
                                current_namespace,
                                entry_namespace,
                                local_functions,
                                imported_map,
                                global_function_map,
                                local_classes,
                                imported_classes,
                                global_class_map,
                                local_modules,
                                imported_modules,
                                global_module_map,
                                scopes,
                            ),
                            a.span.clone(),
                        )
                    })
                    .collect(),
            }
        }
        Expr::Binary { op, left, right } => Expr::Binary {
            op: *op,
            left: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &left.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                left.span.clone(),
            )),
            right: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &right.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                right.span.clone(),
            )),
        },
        Expr::Unary { op, expr } => Expr::Unary {
            op: *op,
            expr: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &expr.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                expr.span.clone(),
            )),
        },
        Expr::Field { object, field } => {
            let rewritten_object = match &object.node {
                Expr::Ident(name) if !is_shadowed(name, scopes) => {
                    if local_modules.contains(name) {
                        Expr::Ident(mangle_project_symbol(
                            current_namespace,
                            entry_namespace,
                            name,
                        ))
                    } else if let Some(ns) = imported_modules.get(name) {
                        Expr::Ident(mangle_project_symbol(ns, entry_namespace, name))
                    } else if let Some(ns) = global_module_map.get(name) {
                        Expr::Ident(mangle_project_symbol(ns, entry_namespace, name))
                    } else {
                        Expr::Ident(name.clone())
                    }
                }
                _ => rewrite_expr_calls_for_project(
                    &object.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
            };
            Expr::Field {
                object: Box::new(ast::Spanned::new(rewritten_object, object.span.clone())),
                field: field.clone(),
            }
        }
        Expr::Index { object, index } => Expr::Index {
            object: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &object.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                object.span.clone(),
            )),
            index: Box::new(ast::Spanned::new(
                rewrite_expr_calls_for_project(
                    &index.node,
                    current_namespace,
                    entry_namespace,
                    local_functions,
                    imported_map,
                    global_function_map,
                    local_classes,
                    imported_classes,
                    global_class_map,
                    local_modules,
                    imported_modules,
                    global_module_map,
                    scopes,
                ),
                index.span.clone(),
            )),
        },
        Expr::Construct { ty, args } => Expr::Construct {
            ty: if local_classes.contains(ty) {
                mangle_project_symbol(current_namespace, entry_namespace, ty)
            } else if let Some(ns) = imported_classes
                .get(ty)
                .or_else(|| global_class_map.get(ty))
            {
                mangle_project_symbol(ns, entry_namespace, ty)
            } else {
                ty.clone()
            },
            args: args
                .iter()
                .map(|a| {
                    ast::Spanned::new(
                        rewrite_expr_calls_for_project(
                            &a.node,
                            current_namespace,
                            entry_namespace,
                            local_functions,
                            imported_map,
                            global_function_map,
                            local_classes,
                            imported_classes,
                            global_class_map,
                            local_modules,
                            imported_modules,
                            global_module_map,
                            scopes,
                        ),
                        a.span.clone(),
                    )
                })
                .collect(),
        },
        Expr::Lambda { params, body } => {
            push_scope(scopes);
            if let Some(scope) = scopes.last_mut() {
                for param in params {
                    scope.insert(param.name.clone());
                }
            }
            let rewritten_body = rewrite_expr_calls_for_project(
                &body.node,
                current_namespace,
                entry_namespace,
                local_functions,
                imported_map,
                global_function_map,
                local_classes,
                imported_classes,
                global_class_map,
                local_modules,
                imported_modules,
                global_module_map,
                scopes,
            );
            pop_scope(scopes);
            Expr::Lambda {
                params: params
                    .iter()
                    .map(|p| ast::Parameter {
                        name: p.name.clone(),
                        ty: rewrite_type_for_project(
                            &p.ty,
                            current_namespace,
                            local_classes,
                            imported_classes,
                            global_class_map,
                            entry_namespace,
                        ),
                        mutable: p.mutable,
                        mode: p.mode,
                    })
                    .collect(),
                body: Box::new(ast::Spanned::new(rewritten_body, body.span.clone())),
            }
        }
        _ => expr.clone(),
    }
}

fn compile_program_ast(
    program: &Program,
    source_path: &Path,
    output_path: &Path,
    emit_llvm: bool,
    opt_level: Option<&str>,
) -> Result<(), String> {
    let context = Context::create();
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let mut codegen = Codegen::new(&context, module_name);
    codegen
        .compile(program)
        .map_err(|e| format!("{}: Codegen error: {}", "error".red().bold(), e.message))?;

    if emit_llvm {
        let ll_path = output_path.with_extension("ll");
        codegen.write_ir(&ll_path)?;
        println!("{} {}", "LLVM IR".green().bold(), ll_path.display());
    } else {
        let ir_path = output_path.with_extension("ll");
        codegen.write_ir(&ir_path)?;
        compile_ir(&ir_path, output_path, opt_level)?;
        let _ = fs::remove_file(&ir_path);
    }

    Ok(())
}

/// Build and run the current project
fn run_project(args: &[String], release: bool, do_check: bool) -> Result<(), String> {
    build_project(release, false, do_check)?;

    let project_root = find_project_root(&std::env::current_dir().unwrap())
        .ok_or_else(|| format!("{}: No apex.toml found", "error".red().bold()))?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;
    let output_path = project_root.join(&config.output);

    println!("{} {}", "Running".cyan().bold(), config.name);
    println!();

    let status = Command::new(&output_path)
        .args(args)
        .status()
        .map_err(|e| format!("{}: Failed to run: {}", "error".red().bold(), e))?;

    if !status.success() {
        return Err(format!(
            "\n{}: Program exited with code: {}",
            "error".red().bold(),
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Run a single file (legacy mode)
fn run_single_file(
    file: &Path,
    args: &[String],
    _release: bool,
    do_check: bool,
) -> Result<(), String> {
    #[cfg(windows)]
    let output = file.with_extension("run.exe");
    #[cfg(not(windows))]
    let output = file.with_extension("run");

    compile_file(file, Some(&output), false, do_check)?;

    println!("{}", "Running...".cyan().bold());
    println!();

    let status = Command::new(&output)
        .args(args)
        .status()
        .map_err(|e| format!("{}: Failed to run: {}", "error".red().bold(), e))?;

    let _ = fs::remove_file(&output);

    if !status.success() {
        return Err(format!(
            "{}: Program exited with code: {}",
            "error".red().bold(),
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Compile a single file (legacy mode)
fn compile_file(
    file: &Path,
    output: Option<&Path>,
    emit_llvm: bool,
    do_check: bool,
) -> Result<(), String> {
    // Check if we're in a project
    if let Some(project_root) = find_project_root(&std::env::current_dir().unwrap()) {
        if file.starts_with(&project_root) {
            println!(
                "{}",
                "Note: Running in project context. Consider using `apex build` instead.".yellow()
            );
        }
    }

    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let output_path = output.map(PathBuf::from).unwrap_or_else(|| {
        #[cfg(windows)]
        {
            file.with_extension("exe")
        }
        #[cfg(not(windows))]
        {
            file.with_extension("")
        }
    });

    compile_source(&source, file, &output_path, emit_llvm, do_check, None)?;

    println!("{} {}", "Output".green().bold(), output_path.display());
    Ok(())
}

/// Compile source code
fn compile_source(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    emit_llvm: bool,
    do_check: bool,
    opt_level: Option<&str>,
) -> Result<(), String> {
    let filename = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("input.apex");

    // Tokenize
    let tokens = lexer::tokenize(source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    // Parse
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, source, filename))?;

    // Type check
    if do_check {
        // Import check
        let namespace = extract_namespace(&program);
        let imports = extract_imports(&program);
        let mut import_checker = ImportChecker::new(HashMap::new(), namespace, imports);
        if let Err(errors) = import_checker.check_program(&program) {
            eprintln!("{} Import errors:", "error".red().bold());
            for err in errors {
                eprintln!("  → {}", err.format());
            }
            return Err("Import check failed".to_string());
        }

        let mut type_checker = TypeChecker::new(source.to_string());
        if let Err(errors) = type_checker.check(&program) {
            return Err(typeck::format_errors(&errors, source, filename));
        }

        let mut borrow_checker = BorrowChecker::new();
        if let Err(errors) = borrow_checker.check(&program) {
            return Err(borrowck::format_borrow_errors(&errors, source, filename));
        }
    }

    // Codegen
    let context = Context::create();
    let module_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let mut codegen = Codegen::new(&context, module_name);
    codegen
        .compile(&program)
        .map_err(|e| format!("{}: Codegen error: {}", "error".red().bold(), e.message))?;

    if emit_llvm {
        let ll_path = output_path.with_extension("ll");
        codegen.write_ir(&ll_path)?;
        println!("{} {}", "LLVM IR".green().bold(), ll_path.display());
    } else {
        let ir_path = output_path.with_extension("ll");
        codegen.write_ir(&ir_path)?;

        compile_ir(&ir_path, output_path, opt_level)?;
        let _ = fs::remove_file(&ir_path);
    }

    Ok(())
}

fn resolve_clang_opt_flag(opt_level: Option<&str>) -> &'static str {
    let normalized = opt_level
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match normalized.as_str() {
        "" | "3" => "-O3",
        "0" => "-O0",
        "1" => "-O1",
        "2" => "-O2",
        "s" => "-Os",
        "z" => "-Oz",
        "fast" => "-Ofast",
        _ => "-O3",
    }
}

/// Compile LLVM IR using clang
fn compile_ir(ir_path: &Path, output_path: &Path, opt_level: Option<&str>) -> Result<(), String> {
    let opt_flag = resolve_clang_opt_flag(opt_level);
    let run_clang = |tuned: bool| {
        let mut cmd = Command::new("clang");
        cmd.arg(ir_path)
            .arg("-o")
            .arg(output_path)
            .arg("-Wno-override-module")
            .arg(opt_flag);

        if tuned {
            cmd.arg("-march=native").arg("-mtune=native");
        }

        #[cfg(windows)]
        cmd.arg("-llegacy_stdio_definitions");

        #[cfg(not(windows))]
        cmd.arg("-lm").arg("-pthread");

        cmd.output()
    };

    // Prefer native-tuned binaries for maximum local performance, but keep a safe fallback.
    let tuned = run_clang(true);
    match tuned {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => {
            let fallback = run_clang(false);
            match fallback {
                Ok(output) => {
                    if output.status.success() {
                        Ok(())
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        Err(format!(
                            "{}: Clang failed: {}",
                            "error".red().bold(),
                            stderr
                        ))
                    }
                }
                Err(_) => Err(format!(
                    "{}: Clang not found. Install clang to compile.",
                    "error".red().bold()
                )),
            }
        }
        Err(_) => Err(format!(
            "{}: Clang not found. Install clang to compile.",
            "error".red().bold()
        )),
    }
}

/// Check a single file
fn check_file(file: Option<&Path>) -> Result<(), String> {
    let file_path = if let Some(f) = file {
        f.to_path_buf()
    } else {
        // Use project entry point
        let project_root =
            find_project_root(&std::env::current_dir().unwrap()).ok_or_else(|| {
                format!(
                    "{}: No apex.toml found. Specify a file or run from a project directory.",
                    "error".red().bold()
                )
            })?;

        let config_path = project_root.join("apex.toml");
        let config = ProjectConfig::load(&config_path)?;
        config.get_entry_path(&project_root)
    };

    println!("{} {}", "Checking".cyan().bold(), file_path.display());

    let source = fs::read_to_string(&file_path)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let filename = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("input.apex");

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format_parse_error(&e, &source, filename))?;

    // Run import checker
    let namespace = extract_namespace(&program);
    let imports = extract_imports(&program);
    let mut import_checker = ImportChecker::new(HashMap::new(), namespace, imports);
    if let Err(errors) = import_checker.check_program(&program) {
        eprintln!("{} Import errors:", "error".red().bold());
        for err in errors {
            eprintln!("  → {}", err.format());
        }
        return Err("Import check failed".to_string());
    }

    let mut type_checker = TypeChecker::new(source.clone());
    if let Err(errors) = type_checker.check(&program) {
        return Err(typeck::format_errors(&errors, &source, filename));
    }

    let mut borrow_checker = BorrowChecker::new();
    if let Err(errors) = borrow_checker.check(&program) {
        return Err(borrowck::format_borrow_errors(&errors, &source, filename));
    }

    println!("{}", "No errors found.".green());
    Ok(())
}

/// Extract namespace from a program
fn extract_namespace(program: &ast::Program) -> String {
    program
        .package
        .clone()
        .unwrap_or_else(|| "global".to_string())
}

/// Extract imports from a program
fn extract_imports(program: &ast::Program) -> Vec<ast::ImportDecl> {
    program
        .declarations
        .iter()
        .filter_map(|d| match &d.node {
            ast::Decl::Import(import) => Some(import.clone()),
            _ => None,
        })
        .collect()
}

/// Show project information
fn show_project_info() -> Result<(), String> {
    let project_root = find_project_root(&std::env::current_dir().unwrap()).ok_or_else(|| {
        format!(
            "{}: No apex.toml found in current directory or parents.",
            "error".red().bold()
        )
    })?;

    let config_path = project_root.join("apex.toml");
    let config = ProjectConfig::load(&config_path)?;

    println!("{}", "Project Information".cyan().bold());
    println!("  {}: {}", "Name".dimmed(), config.name);
    println!("  {}: {}", "Version".dimmed(), config.version);
    println!("  {}: {}", "Entry".dimmed(), config.entry);
    println!("  {}: {}", "Output".dimmed(), config.output);
    println!("  {}: {}", "Opt Level".dimmed(), config.opt_level);
    println!("  {}: {}", "Root".dimmed(), project_root.display());

    println!("\n{}", "Source Files:".dimmed());
    for file in &config.files {
        println!("  - {}", file);
    }

    if !config.dependencies.is_empty() {
        println!("\n{}", "Dependencies:".dimmed());
        for (name, version) in &config.dependencies {
            println!("  - {} = {}", name, version);
        }
    }

    Ok(())
}

/// Show tokens (debug)
fn lex_file(file: &Path) -> Result<(), String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    println!("{} tokens:", "Found".cyan().bold());
    for (token, span) in tokens {
        println!("  {:?} @ {}..{}", token, span.start, span.end);
    }

    Ok(())
}

/// Show AST (debug)
fn parse_file(file: &Path) -> Result<(), String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("{}: Failed to read file: {}", "error".red().bold(), e))?;

    let tokens = lexer::tokenize(&source)
        .map_err(|e| format!("{}: Lexer error: {}", "error".red().bold(), e))?;

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format!("{}: Parse error: {}", "error".red().bold(), e.message))?;

    println!("{}", "AST:".cyan().bold());
    println!("{:#?}", program);

    Ok(())
}

/// Format parse error with source context
fn format_parse_error(error: &parser::ParseError, source: &str, filename: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();

    let mut line_num: usize = 1;
    let mut col: usize = 1;
    for (i, ch) in source.char_indices() {
        if i >= error.span.start {
            break;
        }
        if ch == '\n' {
            line_num += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    let mut output = String::new();
    output.push_str(&format!("\x1b[1;31merror\x1b[0m: {}\n", error.message));
    output.push_str(&format!(
        "  \x1b[1;34m-->\x1b[0m {}:{}:{}\n",
        filename, line_num, col
    ));
    output.push_str("   \x1b[1;34m|\x1b[0m\n");

    if line_num <= lines.len() {
        output.push_str(&format!(
            "\x1b[1;34m{:3} |\x1b[0m {}\n",
            line_num,
            lines[line_num - 1]
        ));

        let underline_start = col.saturating_sub(1);
        let underline_len = (error.span.end - error.span.start).max(1);
        output.push_str(&format!(
            "   \x1b[1;34m|\x1b[0m {}\x1b[1;31m{}\x1b[0m\n",
            " ".repeat(underline_start),
            "^".repeat(underline_len.min(50))
        ));
    }

    output
}

/// Run tests for a file or project
fn run_tests(
    test_path: Option<&Path>,
    list_only: bool,
    filter: Option<&str>,
) -> Result<(), String> {
    // Determine which file(s) to test
    let test_files = if let Some(path) = test_path {
        if path.is_file() {
            vec![path.to_path_buf()]
        } else {
            // Look for test files in directory
            find_test_files(path)?
        }
    } else {
        // Default: look for test files in current project
        let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
        find_test_files(&current_dir)?
    };

    if test_files.is_empty() {
        println!("{}", "No test files found.".yellow());
        println!("Create test files with functions marked with @Test attribute.");
        return Ok(());
    }

    // Process each test file
    let mut all_tests_found = false;

    for test_file in &test_files {
        let source = fs::read_to_string(test_file)
            .map_err(|e| format!("Failed to read test file: {}", e))?;

        // Parse the test file
        let tokens = lexer::tokenize(&source).map_err(|e| format!("Lexer error: {}", e))?;
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse_program()
            .map_err(|e| format!("Parse error: {}", e.message))?;

        // Discover tests
        let discovery = discover_tests(&program);

        if discovery.total_tests == 0 {
            continue;
        }

        all_tests_found = true;

        // Apply filter if specified
        let filtered_suites: Vec<_> = if let Some(pattern) = filter {
            discovery
                .suites
                .into_iter()
                .map(|mut suite| {
                    suite.tests.retain(|t| t.name.contains(pattern));
                    suite
                })
                .filter(|s| !s.tests.is_empty())
                .collect()
        } else {
            discovery.suites
        };

        if filtered_suites.is_empty() {
            println!(
                "{}: No tests matching filter '{}'",
                test_file.display(),
                filter.unwrap_or("")
            );
            continue;
        }

        let filtered_discovery = test_runner::TestDiscovery {
            suites: filtered_suites,
            total_tests: discovery.total_tests,
            ignored_tests: discovery.ignored_tests,
        };

        // List or run tests
        if list_only {
            println!("\n{}", test_file.display().to_string().cyan().bold());
            print_discovery(&filtered_discovery);
        } else {
            // Generate and run test runner - include original source + test runner main
            let runner_code = generate_test_runner_with_source(&filtered_discovery, &source);

            // Create temporary file for test runner
            let runner_path = test_file.with_extension("test_runner.apex");
            fs::write(&runner_path, &runner_code)
                .map_err(|e| format!("Failed to write test runner: {}", e))?;

            // Compile and run the test runner
            let exe_path = test_file.with_extension("test_runner.exe");
            let result = compile_and_run_test(&runner_path, &exe_path);

            // Clean up temporary files
            let _ = fs::remove_file(&runner_path);
            let _ = fs::remove_file(&exe_path);

            result?;
        }
    }

    if !all_tests_found {
        println!("{}", "No tests found in any test files.".yellow());
        println!("Mark functions with @Test to create tests:");
        println!("  {} function myTest(): None {{ ... }}", "@Test".cyan());
    }

    Ok(())
}

/// Find test files in a directory
fn find_test_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut test_files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "apex").unwrap_or(false) {
                // Check if file name suggests it's a test file
                let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if file_name.contains("test") || file_name.contains("spec") {
                    test_files.push(path);
                }
            }
        }
    }

    Ok(test_files)
}

/// Compile and run a test file
fn compile_and_run_test(source_path: &Path, exe_path: &Path) -> Result<(), String> {
    use std::process::Command;

    // Compile the test runner
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read test runner: {}", e))?;

    compile_source(&source, source_path, exe_path, false, true, None)?;

    // Run the compiled test
    println!("\n{}", "Running tests...".cyan().bold());
    println!();

    let output = Command::new(exe_path)
        .output()
        .map_err(|e| format!("Failed to run tests: {}", e))?;

    // Print output
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));

    // Check exit code
    if !output.status.success() {
        return Err("Tests failed".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod project_rewrite_tests {
    use super::*;

    fn sp<T>(node: T) -> ast::Spanned<T> {
        ast::Spanned::new(node, 0..0)
    }

    #[test]
    fn keeps_shadowed_function_call_unmangled() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                return_type: ast::Type::None,
                body: vec![
                    sp(Stmt::Let {
                        name: "foo".to_string(),
                        ty: ast::Type::Integer,
                        value: sp(Expr::Literal(ast::Literal::Integer(1))),
                        mutable: false,
                    }),
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Ident("foo".to_string()))),
                        args: vec![],
                    }))),
                ],
                is_async: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "lib.foo".to_string(),
        }];
        let namespace_functions = HashMap::from([
            ("app".to_string(), HashSet::from(["main".to_string()])),
            ("lib".to_string(), HashSet::from(["foo".to_string()])),
        ]);
        let global_function_map = HashMap::from([("foo".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &global_function_map,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(expr_stmt) = &func.body[1].node else {
            panic!("expected call statement");
        };
        let Expr::Call { callee, .. } = &expr_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Ident(name) = &callee.node else {
            panic!("expected ident callee");
        };
        assert_eq!(name, "foo");
    }

    #[test]
    fn rewrites_imported_class_construct_and_module_field() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                return_type: ast::Type::None,
                body: vec![
                    sp(Stmt::Let {
                        name: "w".to_string(),
                        ty: ast::Type::Named("Widget".to_string()),
                        value: sp(Expr::Construct {
                            ty: "Widget".to_string(),
                            args: vec![],
                        }),
                        mutable: false,
                    }),
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("Utils".to_string()))),
                            field: "make".to_string(),
                        })),
                        args: vec![],
                    }))),
                ],
                is_async: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![
            ast::ImportDecl {
                path: "lib.Widget".to_string(),
            },
            ast::ImportDecl {
                path: "lib.Utils".to_string(),
            },
        ];
        let namespace_functions =
            HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]);
        let namespace_classes =
            HashMap::from([("lib".to_string(), HashSet::from(["Widget".to_string()]))]);
        let namespace_modules =
            HashMap::from([("lib".to_string(), HashSet::from(["Utils".to_string()]))]);
        let global_class_map = HashMap::from([("Widget".to_string(), "lib".to_string())]);
        let global_module_map = HashMap::from([("Utils".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &HashMap::new(),
            &namespace_classes,
            &global_class_map,
            &namespace_modules,
            &global_module_map,
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Let { ty, value, .. } = &func.body[0].node else {
            panic!("expected let statement");
        };
        assert_eq!(ty, &ast::Type::Named("lib__Widget".to_string()));
        let Expr::Construct { ty, .. } = &value.node else {
            panic!("expected construct expression");
        };
        assert_eq!(ty, "lib__Widget");

        let Stmt::Expr(expr_stmt) = &func.body[1].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &expr_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Field { object, .. } = &callee.node else {
            panic!("expected field expression");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected module ident");
        };
        assert_eq!(name, "lib__Utils");
    }

    #[test]
    fn keeps_shadowed_module_ident_unmangled() {
        let program = Program {
            package: Some("app".to_string()),
            declarations: vec![sp(Decl::Function(ast::FunctionDecl {
                name: "main".to_string(),
                generic_params: vec![],
                params: vec![],
                return_type: ast::Type::None,
                body: vec![
                    sp(Stmt::Let {
                        name: "Utils".to_string(),
                        ty: ast::Type::Integer,
                        value: sp(Expr::Literal(ast::Literal::Integer(0))),
                        mutable: false,
                    }),
                    sp(Stmt::Expr(sp(Expr::Call {
                        callee: Box::new(sp(Expr::Field {
                            object: Box::new(sp(Expr::Ident("Utils".to_string()))),
                            field: "make".to_string(),
                        })),
                        args: vec![],
                    }))),
                ],
                is_async: false,
                visibility: ast::Visibility::Private,
                attributes: vec![],
            }))],
        };

        let imports = vec![ast::ImportDecl {
            path: "lib.Utils".to_string(),
        }];
        let namespace_functions =
            HashMap::from([("app".to_string(), HashSet::from(["main".to_string()]))]);
        let namespace_modules =
            HashMap::from([("lib".to_string(), HashSet::from(["Utils".to_string()]))]);
        let global_module_map = HashMap::from([("Utils".to_string(), "lib".to_string())]);

        let rewritten = rewrite_program_for_project(
            &program,
            "app",
            "app",
            &namespace_functions,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &namespace_modules,
            &global_module_map,
            &imports,
        );

        let Decl::Function(func) = &rewritten.declarations[0].node else {
            panic!("expected function declaration");
        };
        let Stmt::Expr(expr_stmt) = &func.body[1].node else {
            panic!("expected expr statement");
        };
        let Expr::Call { callee, .. } = &expr_stmt.node else {
            panic!("expected call expression");
        };
        let Expr::Field { object, .. } = &callee.node else {
            panic!("expected field expression");
        };
        let Expr::Ident(name) = &object.node else {
            panic!("expected module ident");
        };
        assert_eq!(name, "Utils");
    }
}
