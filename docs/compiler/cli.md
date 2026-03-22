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
| `--timings` | | Prints internal project build phase timings plus per-phase reuse/rebuild counters (`parse`, `dependency graph`, `import check`, `rewrite`, `semantic`, `object cache probe`, `object codegen`, `final link`). |

Optimization note:
- In project mode, optimization level is controlled by `opt_level` in `apex.toml` (`0/1/2/3/s/z/fast`, default `3`).
- In project mode, `target` in `apex.toml` is passed to Clang as `--target <triple>` when set.
- In single-file mode (`apex compile file.apex` / `apex run file.apex`), Apex defaults to `-O3` and uses native tuning when available.
- Invalid `--opt-level` / `opt_level` values are now rejected up front instead of silently falling back to `-O3`.
- `apex run` requires project `output_kind = "bin"` and now rejects shared/static library targets before starting a build; use `apex build` for library outputs.
- Project `output` paths in `apex.toml` must stay inside the project root; traversal paths like `../outside/app` are rejected during validation.
- Valid nested output paths such as `build/bin/app` now create missing parent directories automatically in both project and single-file compile flows.

Build cache note:
- `apex build` now writes cache metadata into `.apexcache/` in the project root.
- Broken cache paths, unreadable cache files, unreadable fingerprint-cache files, or corrupt cache payloads now surface as direct build errors instead of being silently treated as cache misses.
- If no source/config/build-mode inputs changed and output artifact exists, build exits early with `Up to date ... (build cache)`.
- For changed projects, parser-level cache is reused per unchanged file from `.apexcache/parsed/`, reducing front-end rebuild overhead.
- Rewritten AST cache is reused per unchanged file from `.apexcache/rewritten/`, reducing project rewrite overhead.
- Object cache is reused per unchanged file from `.apexcache/objects/` and changed files are rebuilt as object-only, then relinked.
- Multi-file project parse stage is parallelized to improve wall time on larger projects.
- `apex build --timings` prints internal phase timings plus per-phase counters such as `considered`, `reused`, `parsed`, `checked`, and `rebuilt`, so hot/cold rebuild tuning can target the real bottleneck instead of guessing.

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
- Without `--path`, `apex test` now uses the current project's `apex.toml` file list when available instead of scanning unrelated files under the working directory.
- When `--filter` is used, reported totals/ignored counts reflect only the filtered test set.
- When `--path` points to a directory, discovery now walks nested subdirectories as well.
- Directory discovery skips symlinked directories to avoid traversing outside the requested test tree.
- Test-file discovery matches `test/spec` case-insensitively, so names like `MathTest.apex` are included.
- Missing test directories now fail with a direct CLI error instead of reporting an empty test set.
- `--path <file>` must point to an `.apex` file; non-Apex files now fail immediately with a CLI error.
- Test execution now uses isolated temporary runner files, so `apex test` no longer overwrites or deletes neighboring `*.test_runner.apex` / `*.test_runner.exe` files in the source tree.

## Examples

### Creating a New Project

```bash
apex new my_project
cd my_project
apex run
```

Project-name note:
- `apex new <name>` now rejects names containing quotes, spaces, path separators, or other special characters that would generate invalid `apex.toml`, invalid Apex source, or unsafe output filenames.

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
- `apex check` now also rejects constant-zero `range` step expressions such as `range(0, 3, 1 - 1)` and `range(0.0, 3.0, 0.5 - 0.5)` before codegen/runtime.
- Assignment mutability checks now apply to nested targets too (`obj.field = ...`, `arr[i] = ...`), not only direct identifier targets.
- Invalid `import ... as alias` usages now report an actionable unknown-namespace-alias error instead of a bogus synthetic import suggestion.

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
- Those project file paths are now validated against the project root before formatting, so `apex.toml` cannot point `fmt` at files outside the workspace.
- Recursive directory formatting skips symlinked directories to avoid traversing outside the requested tree.
- You can point `apex fmt` at either a single `.apex` file or a directory.
- Formatter output now preserves ambiguous expression statements (`if`/`match`) by emitting them as parenthesized expressions in statement position, so `fmt` round-trips without changing semantics.
- Comments inside inline expression blocks such as `async { ... }`, `if (...) { ... } else { ... }`, and `match (...) { ... }` are preserved in place, including trailing comments after the last statement and comments inside otherwise-empty blocks, instead of being moved outside the expression.
- Formatter now preserves a script shebang line (`#!/usr/bin/env apex`) during rewrite and `--check` runs.
- Leading comments before a `package ...;` declaration are preserved above the package line instead of being moved below it.
- Constant integer divide/modulo-by-zero expressions are rejected during `apex check`, and dynamic integer zero divisors now fail with explicit runtime diagnostics instead of crashing with a raw arithmetic fault.
- Constant negative `Task.await_timeout(...)` arguments are rejected during `apex check`, while dynamic negative timeout values still fail fast at runtime with an explicit diagnostic.
- Constant negative `Time.sleep(...)` arguments are rejected during `apex check`, while dynamic negative sleep values still fail fast at runtime with an explicit diagnostic.
- Constant negative collection/string indices are rejected during `apex check` for `List.get`, `List.set`, `list[index]`, and `string[index]`, while dynamic negative indices still fail fast at runtime with explicit diagnostics.
- Constant out-of-bounds indices on string literals like `"abc"[5]` are also rejected during `apex check` instead of being deferred to runtime.
- Constant string-literal indexing also uses Unicode character positions, so `"🚀"[1]` is rejected during `apex check` and `"🚀"[0]` yields the expected `Char`.
- Dynamic indexing on string literals also uses Unicode character positions at runtime, so `"🚀"[idx]` behaves consistently with `Char` semantics instead of indexing raw UTF-8 bytes.
- Dynamic indexing on `String` values also uses Unicode character positions at runtime, so `s[idx]` is aligned with `Char` semantics instead of indexing raw UTF-8 bytes.
- `String.length()` also uses Unicode character count semantics at runtime, so `"🚀".length()` and `s.length()` now return `1` instead of the UTF-8 byte count.
- `Args.get(...)` now rejects constant negative indices during `apex check`, and dynamic negative or out-of-bounds argument indices fail fast at runtime with explicit diagnostics.
- `System.exec(...)` now captures full stdout instead of truncating longer command output to a fixed small buffer.
- `System.exec(...)` now also rejects embedded NUL bytes and invalid UTF-8 at the boundary instead of silently truncating binary stdout or failing later in unrelated string operations.
- `System.getenv(...)` now validates environment values at the boundary, so invalid UTF-8 fails immediately instead of surfacing later in unrelated string operations.
- On POSIX hosts, `System.shell(...)` now returns the decoded process exit code instead of the raw `system()` wait-status word.
- `System.cwd()` now returns the full working directory for deep paths instead of collapsing to an empty string once the current path exceeds the old fixed 1024-byte buffer.
- `File.read()` now fails fast on embedded NUL bytes instead of silently truncating binary-looking content at the first `0x00`.
- `File.read()` now also validates UTF-8 at load time, so invalid text bytes fail immediately instead of slipping through and only crashing later in string operations.
- `File.read()` now also fails fast when the target path cannot be opened instead of silently returning an empty string for missing or inaccessible files.
- `File.write()` now returns `false` when the write or final flush/close fails instead of reporting success just because the file handle opened.
- `File.exists()` now returns `false` for directories instead of treating any readable path as a file hit.
- `File.delete()` now returns `false` for directories instead of deleting them as if they were regular files.
- `File.read()` now rejects FIFOs and other non-seekable paths with a direct runtime diagnostic instead of producing misleading follow-on string errors.
- `Str.len(...)` now matches `String.length()` and Unicode indexing semantics by returning character count instead of raw UTF-8 byte count.
- `Time.now(format)` now handles long format strings without corrupting the returned string through a too-small fixed output buffer.
- `read_line()` imported from `std.io.*` now typechecks correctly, and long input lines are no longer truncated to a tiny fixed buffer.
- The entrypoint `main()` must be synchronous, non-generic, parameterless, non-extern, and return either `None` or `Integer`; invalid signatures are now rejected during `apex check` instead of leaking into backend crashes.

Lint/fix note:
- `apex lint <path>` and `apex fix <path>` now validate explicit paths up front and reject directories or non-`.apex` files with a direct CLI error.
- `apex lex`, `apex parse`, and `apex compile` also validate explicit source paths up front and reject directories or non-`.apex` files with a direct CLI error.
- `apex check <path>` also validates explicit source paths up front and rejects directories or non-`.apex` files with a direct CLI error.
- Explicit file-path commands also reject symlinked `.apex` files that resolve outside the requested directory tree instead of following them to external sources.

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
- Inline C comments in prototypes are normalized safely, so headers like `unsigned/*abi*/int fn(void);` no longer collapse into invalid pseudo-types.
- Array parameters are normalized into valid Apex names and pointer-decay types, and `inline` prototypes are no longer skipped.
