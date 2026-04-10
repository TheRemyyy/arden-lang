# Multi-File Projects

Arden supports organizing code into multi-file projects using a project configuration file.

## Project Structure

An Arden project consists of:

```
my_project/
├── arden.toml          # Project configuration
├── src/               # Source directory
│   ├── main.arden      # Entry point
│   ├── utils.arden     # Utility functions
│   └── lib.arden       # Library code
└── README.md          # Documentation
```

## Creating a New Project

```bash
arden new my_project
cd my_project
arden run
```

This creates:
- `arden.toml` - Project configuration
- `src/main.arden` - Entry point with `main()` function
- `README.md` - Project documentation

## Project Configuration (arden.toml)

```toml
name = "my_project"
version = "1.0.0"
entry = "src/main.arden"
files = [
    "src/utils.arden",
    "src/main.arden"
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
| `opt_level` | No | Final native backend optimization level: `0`, `1`, `2`, `3`, `s`, `z`, or `fast` (default: `3`) |
| `target` | No | Target triple (optional) |
| `output_kind` | No | Final artifact kind: `bin`, `shared`, or `static` (default: `bin`) |
| `link_libs` | No | Extra libraries passed to the native linker backend |
| `link_search` | No | Extra library search paths passed as `-L<path>` |
| `link_args` | No | Extra raw linker arguments forwarded to the native linker backend |

Path safety:
- `entry` and every path in `files` must resolve inside the project root.
- Paths that escape through `..` segments or symlinks are rejected during validation.
- `output` must also stay inside the project root and must not collide with `arden.toml`, the configured entry file, or any file listed in `files`.
- Project auto-discovery only treats a real `arden.toml` file as the project marker; a directory named `arden.toml` is ignored.
- Source-derived namespaces also reject invalid or keyword-named path segments, so file and folder names must stay valid Arden identifiers.
- Import aliases and namespace segments also reject reserved keywords, except terminal built-in variants such as `app.Option.None`.

## Project Commands

### Build Project

```bash
arden build              # Debug build
arden build --release    # Optimized build
```

### Run Project

```bash
arden run                # Build and run
arden run --release      # Optimized build and run
arden run arg1 arg2      # Pass arguments
```

### Check Project

```bash
arden check              # Check entry point
arden check src/lib.arden # Check specific file
```

### Format Project

```bash
arden fmt                # Format files listed in arden.toml
arden fmt --check        # CI mode, fails if files need changes
arden fmt src/           # Format a specific directory tree
```

### Show Project Info

```bash
arden info
```

Output:
```
Project Information
  Name: my_project
  Version: 1.0.0
  Entry: src/main.arden
  Output: my_project
  Output Kind: Bin
  Opt Level: 3
  Target: native/default
  Root: /path/to/project

Source Files:
  - src/utils.arden
  - src/main.arden
```

## Optimization Behavior

- Project builds (`arden build`, `arden run` in a project) use `opt_level` from `arden.toml`.
- Valid values are: `0`, `1`, `2`, `3`, `s`, `z`, `fast`.
- If `opt_level` is missing, Arden defaults to `3`.
- If `opt_level` is present but invalid, Arden now rejects the config with a direct validation error instead of silently changing optimization behavior.
- If `target` is set in `arden.toml`, Arden forwards it to the LLVM/native backend toolchain for that build.
- When `target` is set, host-native tuning flags are skipped to keep target/toolchain compatibility.
- `output_kind = "shared"` emits a shared library, and `output_kind = "static"` emits a static archive.
- `link_libs`, `link_search`, and `link_args` let project builds declare native link requirements in `arden.toml`.
- Single-file mode (`arden compile file.arden`, `arden run file.arden`) defaults to maximum-performance settings.

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
   - `math.arden` - Mathematical functions
   - `string.arden` - String utilities
   - `io.arden` - Input/output operations

2. **Use src/ Directory**: Keep source files organized in a directory

3. **Entry Point**: Keep `main.arden` minimal, delegate to other modules

4. **Documentation**: Add README.md to explain project structure

## Example Projects

See `examples/starter_project/` for the minimal starter example.

See `examples/showcase_project/` for a larger project-mode showcase that combines:
- cross-file packages
- nested modules
- interfaces and inheritance
- generics and enums
- async/await plus `await_timeout`
- file I/O and interpolation-heavy reporting

```toml
# arden.toml
name = "multi_file_demo"
version = "1.0.0"
entry = "src/main.arden"
files = [
    "src/math.arden",
    "src/strings.arden",
    "src/main.arden"
]
output = "multi_file_demo"
output_kind = "bin"
```

```arden
// src/math.arden
function factorial(n: Integer): Integer {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

// src/main.arden
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
arden compile file.arden
arden run file.arden
```

Note: When in a project directory, the compiler will warn you to use `arden build` instead.
