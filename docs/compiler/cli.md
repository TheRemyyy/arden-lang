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
| `lint` | Runs static lint checks. | `apex lint src/main.apex` |
| `fix` | Applies safe automated source fixes. | `apex fix src/main.apex` |
| `fmt` | Formats Apex source files. | `apex fmt` or `apex fmt src/` |
| `lex` | **Debug:** Outputs the stream of tokens. | `apex lex main.apex` |
| `parse` | **Debug:** Outputs the AST. | `apex parse main.apex` |
| `lsp` | Starts the LSP server for IDE integration. | `apex lsp` |
| `compile` | Compiles a single file (legacy mode). | `apex compile file.apex` |
| `bindgen` | Generates Apex `extern` declarations from a C header. | `apex bindgen include/lib.h -o bindings.apex` |
| `bench` | Measures repeated execution time. | `apex bench file.apex --iterations 10` |
| `profile` | Runs once and reports wall time. | `apex profile file.apex` |

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
- In project mode, `target` in `apex.toml` is passed to Clang as `--target <triple>` when set.
- In single-file mode (`apex compile file.apex` / `apex run file.apex`), Apex defaults to `-O3` and uses native tuning when available.

Build cache note:
- `apex build` now writes cache metadata into `.apexcache/` in the project root.
- If no source/config/build-mode inputs changed and output artifact exists, build exits early with `Up to date ... (build cache)`.
- For changed projects, parser-level cache is reused per unchanged file from `.apexcache/parsed/`, reducing front-end rebuild overhead.
- Rewritten AST cache is reused per unchanged file from `.apexcache/rewritten/`, reducing project rewrite overhead.
- Object cache is reused per unchanged file from `.apexcache/objects/` and changed files are rebuilt as object-only, then relinked.
- Multi-file project parse stage is parallelized to improve wall time on larger projects.

## Compile Command Flags

The `compile` command supports extra codegen controls for single-file workflows:

```bash
apex compile file.apex --opt-level 3
apex compile file.apex --target x86_64-unknown-linux-gnu
```

| Flag | Description |
| :--- | :--- |
| `--opt-level <0|1|2|3|s|z|fast>` | Sets final Clang optimization level for the file compile. |
| `--target <triple>` | Passes target triple to Clang (`--target <triple>`). |
| `--emit-llvm` | Emits LLVM IR (`.ll`) instead of linking a binary. |
| `--no-check` | Skips import/type/borrow checks before codegen. |

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

Filter note:
- When `--filter` is used, reported totals/ignored counts reflect only the filtered test set.

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

Behavior:
- `apex check` (without a file path) is project-aware and validates the full project graph (`apex.toml` files list), including cross-file imports/types/borrows.
- `apex check <file.apex>` checks only that single file.
- `apex check` validates explicit generic call arguments on functions/methods/modules (`f<T>(...)`), including:
  - non-generic calls with type arguments (rejected),
  - generic arity mismatch (rejected),
  - unknown explicit type arguments (rejected).
- Assignment mutability checks now apply to nested targets too (`obj.field = ...`, `arr[i] = ...`), not only direct identifier targets.

### Formatting Code

```bash
# Format current project
apex fmt

# Format one file
apex fmt src/main.apex

# CI/check mode
apex fmt --check
```

Notes:
- When run inside a project without an explicit path, `apex fmt` formats the files listed in `apex.toml`.
- You can point `apex fmt` at either a single `.apex` file or a directory.
- Formatter output now preserves ambiguous expression statements (`if`/`match`) by emitting them as parenthesized expressions in statement position, so `fmt` round-trips without changing semantics.
- Formatter now preserves a script shebang line (`#!/usr/bin/env apex`) during rewrite and `--check` runs.

### Linting and Safe Fixes

```bash
apex lint src/main.apex
apex fix src/main.apex
```

Current `apex lint` rules cover:
- duplicate imports
- unsorted imports
- apparently unused specific imports
- unused local variables (`L004`, underscore-prefixed names are ignored)
- variable shadowing inside nested scopes (`L005`)

`apex fix` currently applies safe import deduping/sorting and then runs formatter output normalization.

Import cleanup behavior:
- alias imports are treated as distinct imports (`import std.io as io;` and `import std.io as io2;` are not duplicates).
- imports with trailing inline comments are still parsed as imports by `apex fix` and are not dropped accidentally.
- imports with trailing block comments (`import ...; /* ... */`) are also preserved by `apex fix`.
- script shebang headers (`#!/usr/bin/env apex`) are preserved by `apex fix`.

### Benchmarking and Profiling

```bash
apex bench file.apex --iterations 10
apex profile file.apex
```

`apex bench` reports min/mean/max wall time across repeated runs.  
`apex profile` currently reports wall time for one run.

### Script Entry Points

```bash
chmod +x hello.apex
./hello.apex
```

Apex source files can start with a Unix shebang such as `#!/usr/bin/env apex`. The lexer strips the shebang before normal parsing, which makes single-file scripts runnable on Unix-like systems.

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
