# CLI Reference

## Why This Matters

This is the operational contract for day-to-day Arden development and CI.

## Base Usage

```bash
arden <command> [options]
```

## Core Commands

| Command | Purpose |
| :--- | :--- |
| `new` | create a project scaffold |
| `build` | build project artifact |
| `run` | build + run project or file |
| `compile` | compile single file |
| `check` | parse + type + borrow checks |
| `info` | print resolved project settings |
| `test` | run `@Test` suites |
| `fmt` | format source |
| `lint` | report static findings |
| `fix` | apply safe fixes + format |
| `lex` | print tokens |
| `parse` | print AST |
| `bench` | run benchmark flow |
| `profile` | one-run timing summary |
| `bindgen` | generate extern bindings |
| `lsp` | start language server |

## New User Starter Set

Learn these first:

- `run`
- `check`
- `info`
- `test`
- `fmt`

## Practical Recipes

Single file:

```bash
arden check examples/single_file/basics/01_hello/01_hello.arden
arden run examples/single_file/basics/01_hello/01_hello.arden
```

Project mode:

```bash
arden new app
cd app
arden info
arden check
arden run
```

Quality loop:

```bash
arden test
arden fmt
arden lint
```

## Command Options Snapshot

### `arden run`

- optional file argument: `arden run [FILE]`
- pass runtime args through: `arden run app.arden -- arg1 arg2`
- flags: `--release`, `--no-check`, `--timings`

### `arden compile`

- required file: `arden compile <FILE>`
- output path: `-o, --output`
- optimization: `--opt-level`
- backend target triple: `--target`
- emit LLVM IR: `--emit-llvm`
- skip semantic checks: `--no-check`

### `arden check`

- optional file: `arden check [FILE]`
- project timing breakdown: `--timings`

### `arden fmt`

- file or directory input: `arden fmt [PATH]`
- check-only mode (no write): `arden fmt --check [PATH]`

### `arden test`

- path selection: `--path <PATH>`
- list tests only: `--list`
- name filter: `--filter <PATTERN>`

### `arden bench`

- optional file/project default
- iteration count: `--iterations <N>`

### `arden profile`

- single-run timing summary for file or project

## Related

- [Quick Start](../getting_started/quick_start.md)
- [Projects](../features/projects.md)
- [Architecture](architecture.md)
