# CLI Reference

The `apex` command-line interface.

## Usage

```bash
apex <command> [arguments] [flags]
```

## Commands

| Command | Description | Example |
| :--- | :--- | :--- |
| `new <name>` | Creates a new Apex project. | `apex new my_project` |
| `build` | Builds the current project. | `apex build` |
| `run` | Builds and runs the current project or a single file. | `apex run` or `apex run file.apex` |
| `test` | Discovers and runs @Test functions. | `apex test` or `apex test --path test.apex` |
| `check` | Checks code for errors without compiling. | `apex check` |
| `info` | Shows project information. | `apex info` |
| `lex` | **Debug:** Outputs the stream of tokens. | `apex lex main.apex` |
| `parse` | **Debug:** Outputs the AST. | `apex parse main.apex` |
| `lsp` | Starts the LSP server for IDE integration. | `apex lsp` |
| `compile` | Compiles a single file (legacy mode). | `apex compile file.apex` |
| `bindgen` | Generates Apex `extern` declarations from a C header. | `apex bindgen include/lib.h -o bindings.apex` |

## Global Flags

| Flag | Abbreviation | Description |
| :--- | :--- | :--- |
| `--help` | `-h` | Shows help information. |
| `--version` | `-V` | Shows version information. |

## Build & Run Flags

| Flag | Abbreviation | Description |
| :--- | :--- | :--- |
| `--release` | `-r` | Builds with optimizations enabled. |
| `--emit-llvm` | | Emits the LLVM IR (`.ll` file) instead of a binary. |
| `--no-check` | | Skips type checking. **Warning: Unsafe.** |

Optimization note:
- In project mode, optimization level is controlled by `opt_level` in `apex.toml` (`0/1/2/3/s/z/fast`, default `3`).
- In single-file mode (`apex compile file.apex` / `apex run file.apex`), Apex uses maximum-performance optimization by default.

## Test Command

The `test` command discovers and runs functions marked with `@Test`:

```bash
# Run all tests in current project
apex test

# Run tests in a specific file
apex test --path tests/math_test.apex

# List tests without running
apex test --list

# Filter tests by name
apex test --filter "math"
```

### Test Options

| Option | Abbreviation | Description |
| :--- | :--- | :--- |
| `--path <file>` | `-p` | Path to test file or directory. |
| `--list` | `-l` | Lists tests without running them. |
| `--filter <pattern>` | `-f` | Filters tests by name pattern. |

## Examples

### Creating a New Project

```bash
apex new my_project
cd my_project
apex run
```

### Building a Release Binary

```bash
apex build --release
```

### Running Tests

```bash
# Run all tests
apex test

# Run with verbose output
apex test --list
apex test --filter "math"
```

### Checking Code

```bash
# Check current project
apex check

# Check specific file
apex check src/utils.apex
```

### Debug Output

```bash
# Show tokens
apex lex main.apex

# Show AST
apex parse main.apex

# Show LLVM IR
apex build --emit-llvm
```

### Bindgen

```bash
# Print generated extern declarations to stdout
apex bindgen include/lib.h

# Write generated declarations into a file
apex bindgen include/lib.h --output src/bindings.apex
```

Notes:
- Generated signatures are a starting point. Review and adjust ABI/types before production use.
- Current generator targets C function prototypes and variadic `...` declarations.
