# CLI Reference

The `arden` command-line interface.

## Usage

```bash
arden <command> [arguments] [flags]
```

## Commands

| Command | Description | Example |
| :--- | :--- | :--- |
| `new <name>` | Creates a new Arden project. | `arden new my_project` |
| `build` | Builds the current project. | `arden build` |
| `run` | Builds and runs the current project or a single file. | `arden run` or `arden run file.arden` |
| `test` | Discovers and runs @Test functions. | `arden test` or `arden test --path test.arden` |
| `check` | Checks code for errors without compiling. | `arden check` |
| `info` | Shows project information. | `arden info` |
| `lint` | Runs static lint checks. | `arden lint src/main.arden` |
| `fix` | Applies safe automated source fixes. | `arden fix src/main.arden` |
| `fmt` | Formats Arden source files. | `arden fmt` or `arden fmt src/` |
| `lex` | **Debug:** Outputs the stream of tokens. | `arden lex main.arden` |
| `parse` | **Debug:** Outputs the AST. | `arden parse main.arden` |
| `lsp` | Starts the LSP server for IDE integration. | `arden lsp` |
| `compile` | Compiles a single file (legacy mode). | `arden compile file.arden` |
| `bindgen` | Generates Arden `extern` declarations from a C header. | `arden bindgen include/lib.h -o bindings.arden` |
| `bench` | Measures repeated execution time. | `arden bench file.arden --iterations 10` |
| `profile` | Runs once and reports wall time. | `arden profile file.arden` |

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
- In project mode, optimization level is controlled by `opt_level` in `arden.toml` (`0/1/2/3/s/z/fast`, default `3`).
- In project mode, `target` in `arden.toml` is passed to Clang as `--target <triple>` when set.
- In single-file mode (`arden compile file.arden` / `arden run file.arden`), Arden defaults to `-O3` and uses native tuning when available.
- Project-mode rewrite now rewrites alias-qualified and nested module-qualified nominal parents in `extends`, `implements`, and `interface ... extends ...`, so imported parents like `u.Base`, `u.Printable`, and `u.Api.Named` survive symbol mangling consistently across build/check/test flows.
- Seeded project semantic reuse now preserves those alias-qualified interface parents too, so incremental `arden build` / `arden check` runs no longer fail on valid imported interface chains like `interface Child extends u.Named` or `interface Child extends u.Api.Named`.
- The same project rewrite pass now also rewrites module-local interface parents such as `implements Named` and `extends Api.Named` inside nested modules, so local interface inheritance stays aligned with the rest of project symbol rewriting.
- Qualified nominal references inside type positions and inheritance clauses now participate in dependency tracking as dotted symbol paths instead of raw strings only, preventing incremental project builds from splitting dependent interface files into separate semantic components.
- The same dependency tracking now also covers qualified constructor expressions and qualified enum patterns such as `u.Api.Box(...)` and `u.Result.Value.Ok(v)`, so project builds keep constructor-only and match-only dependencies in the same semantic component.
- Project import checking now recognizes nested namespace aliases like `import util.Api as u;` even when the imported module path only exposes classes/enums/interfaces and no functions.
- Deep nested namespace aliases like `import util.Api.V1 as u;` now follow the same rules for constructors, interface inheritance, and enum patterns, because parsed project metadata now preserves nested module paths recursively instead of only top-level module names.
- Invalid namespace aliases are now rejected earlier even when they only appear in type and pattern positions, such as `alias.Box()`, `value: alias.Box`, `interface X extends alias.Named`, or `match (v) { alias.Result.Ok(x) => ... }`, instead of surfacing later as unrelated type/codegen failures.
- Direct module enums now also rewrite their variant field types through the same project rewrite pass, including alias-qualified, nested-module-qualified, and generic payload types inside `module { enum ... }` declarations.
- Match codegen now distinguishes built-in `Option` / `Result` from user enums with variant leaves like `Ok` and `Error`, preventing backend crashes on custom enum matches that happen to reuse those names.
- Invalid `--opt-level` / `opt_level` values are now rejected up front instead of silently falling back to `-O3`.
- `arden run` requires project `output_kind = "bin"` and now rejects shared/static library targets before starting a build; use `arden build` for library outputs.
- Project `output` paths in `arden.toml` must stay inside the project root; traversal paths like `../outside/app` are rejected during validation.
- Valid nested output paths such as `build/bin/app` now create missing parent directories automatically in both project and single-file compile flows.
- That same parent-directory creation now happens before the project-mode link-response-file step as well, so `output = "build/app"` no longer fails late during final link setup when `build/` does not already exist.

Build cache note:
- `arden build` now writes cache metadata into `.ardencache/` in the project root.
- Broken cache paths, unreadable cache files, unreadable fingerprint-cache files, or corrupt cache payloads now surface as direct build errors instead of being silently treated as cache misses.
- If no source/config/build-mode inputs changed and output artifact exists, build exits early with `Up to date ... (build cache)`.
- For changed projects, parser-level cache is reused per unchanged file from `.ardencache/parsed/`, reducing front-end rebuild overhead.
- Rewritten AST cache is reused per unchanged file from `.ardencache/rewritten/`, reducing project rewrite overhead.
- Object cache is reused per unchanged file from `.ardencache/objects/` and changed files are rebuilt as object-only, then relinked.
- Multi-file project parse stage is parallelized to improve wall time on larger projects.
- `arden build --timings` prints internal phase timings plus per-phase counters such as `considered`, `reused`, `parsed`, `checked`, and `rebuilt`, so hot/cold rebuild tuning can target the real bottleneck instead of guessing.

## Compile Command Flags

The `compile` command supports extra codegen controls for single-file workflows:

```bash
arden compile file.arden --opt-level 3
arden compile file.arden --target x86_64-unknown-linux-gnu
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
arden test

# Run tests in a specific file
arden test --path tests/math_test.arden

# List tests without running
arden test --list

# Filter tests by name
arden test --filter "math"
```

### Test Options

| Option | Abbreviation | Description |
| :--- | :--- | :--- |
| `--path <file>` | `-p` | Path to test file or directory. |
| `--list` | `-l` | Lists tests without running them. |
| `--filter <pattern>` | `-f` | Filters tests by name pattern. |

Filter note:
- Without `--path`, `arden test` now uses every source file from the current project's `arden.toml` file list when available instead of scanning unrelated files under the working directory or skipping valid tests in non-`*test*` filenames.
- When `--filter` is used, reported totals/ignored counts reflect only the filtered test set.
- When `--path` points to a directory, discovery now walks nested subdirectories as well.
- Directory discovery skips symlinked directories to avoid traversing outside the requested test tree.
- Test-file discovery matches `test/spec` case-insensitively, so names like `MathTest.arden` are included.
- Missing test directories now fail with a direct CLI error instead of reporting an empty test set.
- `--path <file>` must point to an `.arden` file; non-Arden files now fail immediately with a CLI error.
- Test execution now uses isolated temporary runner files, so `arden test` no longer overwrites or deletes neighboring `*.test_runner.arden` / `*.test_runner.exe` files in the source tree.
- In project mode, test execution now builds the generated runner inside an isolated temporary copy of the project, so tests can keep using project-local package imports and aliases without colliding with the original entrypoint `main()`.
- Relative project file paths passed through `--path` now resolve correctly against the current project root, so commands like `arden test --path src/main.arden` work the same as absolute paths.
- Bare relative filenames passed through `--path` now also validate correctly from the current directory, so commands like `arden test --path smoke_test.arden` no longer fail on parent-directory resolution before test execution starts.
- Generated runners now still inject `import std.io.*;` when that text appears only inside block comments, and shebang scripts keep the shebang as line 1 while receiving the injected import on the next line.
- Existing user `main(...)` stripping is now comment/string aware, so braces inside strings, `// ...`, or `/* ... */` no longer leak pieces of the original main body into the generated runner or swallow following declarations.

## Examples

### Creating a New Project

```bash
arden new my_project
cd my_project
arden run
```

Project-name note:
- `arden new <name>` now rejects names containing quotes, spaces, path separators, or other special characters that would generate invalid `arden.toml`, invalid Arden source, or unsafe output filenames.

### Building a Release Binary

```bash
arden build --release
```

### Running Tests

```bash
# Run all tests
arden test

# Run with verbose output
arden test --list
arden test --filter "math"
```

### Checking Code

```bash
# Check current project
arden check

# Check specific file
arden check src/utils.arden
```

Behavior:
- `arden check` (without a file path) is project-aware and validates the full project graph (`arden.toml` files list), including cross-file imports/types/borrows.
- `arden check <file.arden>` checks only that single file.
- `arden check` validates explicit generic call arguments on functions/methods/modules (`f<T>(...)`), including:
  - non-generic calls with type arguments (rejected),
  - generic arity mismatch (rejected),
  - unknown explicit type arguments (rejected).
- `arden check` now also rejects constant-zero `range` step expressions such as `range(0, 3, 1 - 1)` and `range(0.0, 3.0, 0.5 - 0.5)` before codegen/runtime.
- Assignment mutability checks now apply to nested targets too (`obj.field = ...`, `arr[i] = ...`), not only direct identifier targets.
- Invalid `import ... as alias` usages now report an actionable unknown-namespace-alias error instead of a bogus synthetic import suggestion.

### Formatting Code

```bash
# Format current project
arden fmt

# Format one file
arden fmt src/main.arden

# CI/check mode
arden fmt --check
```

Notes:
- When run inside a project without an explicit path, `arden fmt` formats the files listed in `arden.toml`.
- Those project file paths are now validated against the project root before formatting, so `arden.toml` cannot point `fmt` at files outside the workspace.
- Recursive directory formatting skips symlinked directories to avoid traversing outside the requested tree.
- You can point `arden fmt` at either a single `.arden` file or a directory.
- Formatter output now preserves ambiguous expression statements (`if`/`match`) by emitting them as parenthesized expressions in statement position, so `fmt` round-trips without changing semantics.
- Comments inside inline expression blocks such as `async { ... }`, `if (...) { ... } else { ... }`, and `match (...) { ... }` are preserved in place, including trailing comments after the last statement and comments inside otherwise-empty blocks, instead of being moved outside the expression.
- Formatter now preserves a script shebang line (`#!/usr/bin/env arden`) during rewrite and `--check` runs.
- Leading comments before a `package ...;` declaration are preserved above the package line instead of being moved below it.
- Constant integer divide/modulo-by-zero expressions are rejected during `arden check`, and dynamic integer zero divisors now fail with explicit runtime diagnostics instead of crashing with a raw arithmetic fault.
- Constant negative `Task.await_timeout(...)` arguments are rejected during `arden check`, while dynamic negative timeout values still fail fast at runtime with an explicit diagnostic.
- Constant negative `Time.sleep(...)` arguments are rejected during `arden check`, while dynamic negative sleep values still fail fast at runtime with an explicit diagnostic.
- Constant negative collection/string indices are rejected during `arden check` for `List.get`, `List.set`, `list[index]`, and `string[index]`, while dynamic negative indices still fail fast at runtime with explicit diagnostics.
- Constant out-of-bounds indices on string literals like `"abc"[5]` are also rejected during `arden check` instead of being deferred to runtime.
- Constant string-literal indexing also uses Unicode character positions, so `"🚀"[1]` is rejected during `arden check` and `"🚀"[0]` yields the expected `Char`.
- Dynamic indexing on string literals also uses Unicode character positions at runtime, so `"🚀"[idx]` behaves consistently with `Char` semantics instead of indexing raw UTF-8 bytes.
- Dynamic indexing on `String` values also uses Unicode character positions at runtime, so `s[idx]` is aligned with `Char` semantics instead of indexing raw UTF-8 bytes.
- `String.length()` also uses Unicode character count semantics at runtime, so `"🚀".length()` and `s.length()` now return `1` instead of the UTF-8 byte count.
- `Args.get(...)` now rejects constant negative indices during `arden check`, and dynamic negative or out-of-bounds argument indices fail fast at runtime with explicit diagnostics.
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
- The entrypoint `main()` must be synchronous, non-generic, parameterless, non-extern, and return either `None` or `Integer`; invalid signatures are now rejected during `arden check` instead of leaking into backend crashes.

Lint/fix note:
- `arden lint <path>` and `arden fix <path>` now validate explicit paths up front and reject directories or non-`.arden` files with a direct CLI error.
- `arden lex`, `arden parse`, and `arden compile` also validate explicit source paths up front and reject directories or non-`.arden` files with a direct CLI error.
- `arden check <path>` also validates explicit source paths up front and rejects directories or non-`.arden` files with a direct CLI error.
- Explicit file-path commands also reject symlinked `.arden` files that resolve outside the requested directory tree instead of following them to external sources.
- `arden fix` now preserves leading file-header comments and comments between `package ...;` and the import block while sorting/deduplicating imports, instead of moving those comments below the imports.
- Block-commented import-like text now stays inside its original block comment during `arden fix`; only real top-level imports are rewritten.
- `arden lint` `L003` now also treats imports as used when they only appear inside explicit generic call arguments (`List<Boxed>()`, `List<u.Box>()`) or inside interface default method bodies.
- `arden lint` now also applies `L004` and `L005` inside interface default method bodies, including locals, loop variables, lambda parameters, and match bindings.
- `arden lint` `L003` now also treats imports as used when they appear only in generic bounds on functions, classes, enums, interfaces, or class methods.
- Qualified names are now accepted in inheritance/implementation clauses too, so `class Child extends u.Base`, `class Child implements u.Api.Named`, and `interface Child extends u.Base` parse correctly instead of stopping at the first dot.
- `arden lint` `L003` now also treats namespace aliases used only in `extends` / `implements` clauses as used.
- Typechecking now also resolves those qualified inheritance references through aliases and module paths, so valid forms like `class Child extends u.Base` and `class Book implements u.Api.Printable` no longer fail later as unknown classes/interfaces.

### Linting and Safe Fixes

```bash
arden lint src/main.arden
arden fix src/main.arden
```

Current `arden lint` rules cover:
- duplicate imports
- unsorted imports
- apparently unused specific imports
- unused local variables (`L004`, underscore-prefixed names are ignored)
- variable shadowing inside nested scopes (`L005`)

Scope note:
- `L004` and `L005` now also cover locals introduced inside `async` expressions, expression-valued `if` branches, `match` pattern bindings, and lambda parameter lists instead of only statement-bodied scopes.
- `L003` also treats imports referenced only from `match` patterns as used, including exact variant aliases, enum aliases, and namespace-qualified pattern paths.

`arden fix` currently applies safe import deduping/sorting and then runs formatter output normalization.

Import cleanup behavior:
- alias imports are treated as distinct imports (`import std.io as io;` and `import std.io as io2;` are not duplicates).
- imports with trailing inline comments are still parsed as imports by `arden fix` and are not dropped accidentally.
- imports with trailing block comments (`import ...; /* ... */`) are also preserved by `arden fix`.
- script shebang headers (`#!/usr/bin/env arden`) are preserved by `arden fix`.

### Benchmarking and Profiling

```bash
arden bench file.arden --iterations 10
arden profile file.arden
```

`arden bench` reports min/mean/max wall time across repeated runs.  
`arden profile` currently reports wall time for one run.

### Script Entry Points

```bash
chmod +x hello.arden
./hello.arden
```

Arden source files can start with a Unix shebang such as `#!/usr/bin/env arden`. The lexer strips the shebang before normal parsing, which makes single-file scripts runnable on Unix-like systems.

### Debug Output

```bash
# Show tokens
arden lex main.arden

# Show AST
arden parse main.arden

# Show LLVM IR
arden build --emit-llvm
```

### Bindgen

```bash
# Print generated extern declarations to stdout
arden bindgen include/lib.h

# Write generated declarations into a file
arden bindgen include/lib.h --output src/bindings.arden
```

Notes:
- Generated signatures are a starting point. Review and adjust ABI/types before production use.
- Current generator targets C function prototypes and variadic `...` declarations.
- Inline C comments in prototypes are normalized safely, so headers like `unsigned/*abi*/int fn(void);` no longer collapse into invalid pseudo-types.
- Array parameters are normalized into valid Arden names and pointer-decay types, and `inline` prototypes are no longer skipped.
- `restrict` / `__restrict__` qualifiers are normalized correctly even when attached to pointer stars like `char *restrict dst`.
- Reordered integer spellings like `long unsigned int` and bare `signed` now lower to Arden `Integer` instead of being skipped.
