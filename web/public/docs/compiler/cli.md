# CLI Reference

Arden ships with an integrated command-line workflow for building, checking, formatting, testing, benchmarking, and debugging source.

Base usage:

```bash
arden <command> [options]
```

## Commands

| Command | What it does |
| :--- | :--- |
| `new` | Create a project skeleton |
| `build` | Build the current project |
| `run` | Build and run a project or single file |
| `compile` | Compile a single `.arden` file |
| `check` | Parse, type-check, and borrow-check source |
| `info` | Print project configuration and build settings |
| `lint` | Report static findings |
| `fix` | Apply safe fixes and reformat the result |
| `fmt` | Format Arden source |
| `lex` | Print lexer tokens |
| `parse` | Print the parsed AST |
| `lsp` | Start the language server |
| `test` | Discover and run `@Test` suites |
| `bindgen` | Generate Arden `extern` bindings from a C header |
| `bench` | Measure end-to-end execution time |
| `profile` | Run once and print a timing summary |

## Global Flags

```bash
arden --help
arden --version
```

## Mental Model

The command surface splits roughly into four groups:

- project creation and build flow: `new`, `info`, `build`, `run`, `check`
- source hygiene: `fmt`, `lint`, `fix`
- debugging and inspection: `lex`, `parse`, `profile`
- testing and interop: `test`, `bench`, `bindgen`, `lsp`

If you are new to Arden, the most useful commands to learn first are usually `run`, `check`, `info`, `fmt`, and `test`.

## Command Details

### `arden new`

```bash
arden new my_project
arden new my_project --path /tmp/my_project
```

- creates `arden.toml`, `src/main.arden`, and a starter `README.md`
- rejects unsafe or invalid project names
- gives you the smallest project-mode scaffold in the repository's style

Use this when you are done experimenting with a single `.arden` file and want explicit project config.

### `arden build`

```bash
arden build
arden build --release
arden build --timings
arden build --emit-llvm
```

Options:

- `--release`
- `--emit-llvm`
- `--no-check`
- `--timings`

Notes:

- project optimization is controlled by `opt_level` in `arden.toml`
- project builds use explicit linker policy: Linux uses direct `mold`, macOS uses LLVM `lld`, Windows uses `lld-link`
- build cache metadata lives in `.ardencache/` in the project root
- `--timings` is the fastest way to inspect build phases and cache reuse

### `arden run`

```bash
arden run
arden run hello.arden
arden run hello.arden arg1 arg2
arden run --release
```

Options:

- `--release`
- `--no-check`
- `--timings`

Notes:

- without a file path, `arden run` uses the current project
- with a file path, `arden run` builds and runs that single file
- trailing args are forwarded to the compiled program

This is usually the default day-to-day command for examples and local iteration.

### `arden compile`

```bash
arden compile hello.arden
arden compile hello.arden -o hello_bin
arden compile hello.arden --opt-level 3
arden compile hello.arden --target x86_64-unknown-linux-gnu
```

Options:

- `-o, --output <path>`
- `--opt-level <0|1|2|3|s|z|fast>`
- `--target <triple>`
- `--emit-llvm`
- `--no-check`

Use this when you want a single-file native artifact without creating a full project.

### `arden check`

```bash
arden check
arden check src/main.arden
arden check --timings
```

Notes:

- without a file path, project mode checks the configured project graph
- with a file path, only that file is checked
- `--timings` applies in project mode

This is the right command when you want semantic validation and borrow checking without spending time on a final native build.

### `arden info`

```bash
arden info
```

Prints:

- project name and version
- entry file
- output path and kind
- optimization level
- target
- project root
- configured source files

When project mode behaves unexpectedly, `arden info` should be one of the first commands you run.

### `arden lint`

```bash
arden lint
arden lint src/main.arden
```

Designed to report static findings without rewriting source.

### `arden fix`

```bash
arden fix
arden fix src/main.arden
```

Applies safe automatic fixes and reformats the result.

### `arden fmt`

```bash
arden fmt
arden fmt src/
arden fmt src/main.arden
arden fmt --check
```

Options:

- `--check`

Use `--check` in validation scripts or CI when you want formatting enforcement without mutating files.

### `arden lex`

```bash
arden lex file.arden
```

Useful for debugging tokenizer behavior.

### `arden parse`

```bash
arden parse file.arden
```

Useful for debugging parser output.

### `arden lsp`

```bash
arden lsp
```

Starts the language server process.

### `arden test`

```bash
arden test
arden test --list
arden test --filter math
arden test --path tests/
arden test --path tests/math_test.arden
```

Options:

- `-p, --path <file-or-dir>`
- `-l, --list`
- `-f, --filter <pattern>`

Notes:

- project mode uses project files by default
- directory paths are walked recursively
- generated runners are isolated from the source tree

Reference:

- [Testing](../features/testing.md)

### `arden bindgen`

```bash
arden bindgen sample.h
arden bindgen sample.h -o bindings.arden
```

Generates Arden `extern` declarations from a C header.

Useful pairings:

- `examples/27_extern_c_interop.arden`
- `examples/34_bindgen_workflow.arden`

### `arden bench`

```bash
arden bench hello.arden
arden bench hello.arden --iterations 10
arden bench
```

Options:

- `-i, --iterations <n>`

Measures end-to-end execution time for a project or single file.

This is intentionally lightweight and different from the repository benchmark harness under `benchmark/`.

### `arden profile`

```bash
arden profile hello.arden
arden profile
```

Runs once and prints a timing summary.

## Suggested Workflows

### Single File

```bash
arden check hello.arden
arden fmt hello.arden
arden run hello.arden
arden profile hello.arden
```

### Project

```bash
arden info
arden check --timings
arden test
arden build --release
```

### Investigating A Frontend Bug

```bash
arden lex suspicious.arden
arden parse suspicious.arden
arden check suspicious.arden
```

## Common Command Choices

| If you want to... | Use |
| :--- | :--- |
| run one file quickly | `arden run file.arden` |
| validate types/borrows without building | `arden check` |
| inspect project config | `arden info` |
| enforce style in CI | `arden fmt --check` |
| run only selected tests | `arden test --filter name` |
| inspect compile timing shape | `arden build --timings` |
| debug lexer/parser behavior | `arden lex` / `arden parse` |
