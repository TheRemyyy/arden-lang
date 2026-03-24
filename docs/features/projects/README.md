# Multi-File Projects

Apex supports organizing code into multi-file projects using a project configuration file.

## Project Structure

An Apex project consists of:

```
my_project/
├── apex.toml          # Project configuration
├── src/               # Source directory
│   ├── main.apex      # Entry point
│   ├── utils.apex     # Utility functions
│   └── lib.apex       # Library code
└── README.md          # Documentation
```

## Creating a New Project

```bash
apex new my_project
cd my_project
apex run
```

This creates:
- `apex.toml` - Project configuration
- `src/main.apex` - Entry point with `main()` function
- `README.md` - Project documentation

## Project Configuration (apex.toml)

```toml
name = "my_project"
version = "1.0.0"
entry = "src/main.apex"
files = [
    "src/utils.apex",
    "src/main.apex"
]
output = "my_project"
opt_level = "3"
output_kind = "bin"
link_libs = ["ssl"]
link_search = ["native/lib"]
link_args = ["-Wl,--as-needed"]
```

### Configuration Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Project name |
| `version` | Yes | Project version |
| `entry` | Yes | Entry point file (contains `main()`) |
| `files` | Yes | List of all source files to compile |
| `output` | No | Output binary name (default: project name) |
| `opt_level` | No | Final Clang optimization level: `0`, `1`, `2`, `3`, `s`, `z`, or `fast` (default: `3`) |
| `target` | No | Target triple (optional) |
| `output_kind` | No | Final artifact kind: `bin`, `shared`, or `static` (default: `bin`) |
| `link_libs` | No | Extra libraries passed to Clang as `-l<name>` |
| `link_search` | No | Extra library search paths passed as `-L<path>` |
| `link_args` | No | Extra raw linker arguments forwarded to Clang |

Path safety:
- `entry` and every path in `files` must resolve inside the project root.
- Paths that escape through `..` segments or symlinks are rejected during validation.
- `output` must also stay inside the project root and must not collide with `apex.toml`, the configured entry file, or any file listed in `files`.
- Project auto-discovery only treats a real `apex.toml` file as the project marker; a directory named `apex.toml` is ignored.
- Source-derived namespaces also reject invalid or keyword-named path segments, so file and folder names must stay valid Apex identifiers.
- Import aliases and namespace segments also reject reserved keywords, except terminal built-in variants such as `app.Option.None`.

## Project Commands

### Build Project

```bash
apex build              # Debug build
apex build --release    # Optimized build
```

### Run Project

```bash
apex run                # Build and run
apex run --release      # Optimized build and run
apex run arg1 arg2      # Pass arguments
```

### Check Project

```bash
apex check              # Check entry point
apex check src/lib.apex # Check specific file
```

### Format Project

```bash
apex fmt                # Format files listed in apex.toml
apex fmt --check        # CI mode, fails if files need changes
apex fmt src/           # Format a specific directory tree
```

### Show Project Info

```bash
apex info
```

Output:
```
Project Information
  Name: my_project
  Version: 1.0.0
  Entry: src/main.apex
  Output: my_project
  Output Kind: Bin
  Opt Level: 3
  Target: native/default
  Root: /path/to/project

Source Files:
  - src/utils.apex
  - src/main.apex
```

## Optimization Behavior

- Project builds (`apex build`, `apex run` in a project) use `opt_level` from `apex.toml`.
- Valid values are: `0`, `1`, `2`, `3`, `s`, `z`, `fast`.
- If `opt_level` is missing, Apex defaults to `3`.
- If `opt_level` is present but invalid, Apex now rejects the config with a direct validation error instead of silently changing optimization behavior.
- If `target` is set in `apex.toml`, Apex forwards it to Clang as `--target <triple>`.
- When `target` is set, host-native tuning flags are skipped to keep target/toolchain compatibility.
- `output_kind = "shared"` emits a shared library, and `output_kind = "static"` emits a static archive.
- `link_libs`, `link_search`, and `link_args` let project builds declare native link requirements in `apex.toml`.
- Single-file mode (`apex compile file.apex`, `apex run file.apex`) defaults to maximum-performance settings.

## How It Works

1. **AST Build Pipeline**: All files listed in `files` are parsed and combined as declarations in one project AST.
2. **Import Checking**: Cross-file calls are validated by the import checker. Use explicit `import` statements when calling functions from other namespaces/modules.
3. **Deterministic Symbol Mangling**: Top-level function/class/module symbols are rewritten to namespace-qualified internal names during project build.
4. **Scope-Aware Rewrite**: Local bindings (parameters, `let` variables, loop vars, lambda params, match bindings) are never rewritten as imported/global symbols.
5. **Collision Safety**: Duplicate top-level function/class/module names across namespaces are rejected during project analysis.
6. **Entry Point**: The `entry` file must contain the `main()` function.
7. **Compilation**: Project is compiled to a single binary.

## Best Practices

1. **Organize by Functionality**: Group related functions into files
   - `math.apex` - Mathematical functions
   - `string.apex` - String utilities
   - `io.apex` - Input/output operations

2. **Use src/ Directory**: Keep source files organized in a directory

3. **Entry Point**: Keep `main.apex` minimal, delegate to other modules

4. **Documentation**: Add README.md to explain project structure

## Example Project

See `examples/multi_file_project/` for a complete example.

```toml
# apex.toml
name = "multi_file_demo"
version = "1.0.0"
entry = "src/main.apex"
files = [
    "src/math_utils.apex",
    "src/string_utils.apex",
    "src/main.apex"
]
output = "multi_file_demo"
output_kind = "bin"
```

```apex
// src/math_utils.apex
function factorial(n: Integer): Integer {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

// src/main.apex
import std.io.*;

function main(): None {
    result: Integer = factorial(5);
    println("5! = " + to_string(result));
    return None;
}
```

## Single-File Mode

You can still compile single files without a project:

```bash
apex compile file.apex
apex run file.apex
```

Note: When in a project directory, the compiler will warn you to use `apex build` instead.
