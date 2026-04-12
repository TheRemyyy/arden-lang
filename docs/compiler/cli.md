# CLI Reference

## Why This Matters

This is the operational contract for day-to-day Arden development and CI.

## Base Usage

```bash
arden <command> [options]
```

## Command Map

| Command | Purpose |
| :--- | :--- |
| `new` | create a project scaffold |
| `build` | build current project |
| `run` | build + run project or single file |
| `compile` | compile single file |
| `check` | parse + type + borrow checks |
| `info` | print resolved project settings |
| `test` | discover/run `@Test` suites |
| `fmt` | format source |
| `lint` | report static findings |
| `fix` | apply safe fixes + format |
| `lex` | print lexer tokens |
| `parse` | print parsed AST |
| `bench` | measure end-to-end runtime |
| `profile` | one-run timing summary |
| `bindgen` | generate Arden extern bindings from C header |
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

Program args passthrough:

```bash
arden run app.arden -- --mode ci --limit 10
```

Quality loop:

```bash
arden test
arden fmt
arden lint
```

## Detailed Options

### `arden new`

```bash
arden new <NAME> [--path <DIR>]
```

- `name` required
- `--path` optional output directory (default `./<NAME>`)

### `arden build`

```bash
arden build [--release] [--emit-llvm] [--no-check] [--timings]
```

- `--release` optimized codegen
- `--emit-llvm` write LLVM IR instead of final artifact
- `--no-check` skip type/borrow checks
- `--timings` print internal phase timings

### `arden run`

```bash
arden run [FILE] [--release] [--no-check] [--timings] [-- <PROGRAM_ARGS...>]
```

- optional `FILE`; if omitted, runs current project
- trailing `-- ...` passes args to compiled program

### `arden compile`

```bash
arden compile <FILE> [-o <OUT>] [--opt-level <L>] [--target <TRIPLE>] [--emit-llvm] [--no-check]
```

- `-o, --output` output path
- `--opt-level` one of: `0`, `1`, `2`, `3`, `s`, `z`, `fast`
- `--target` backend target triple
- `--emit-llvm` write LLVM IR
- `--no-check` skip type/borrow checks

### `arden check`

```bash
arden check [FILE] [--timings]
```

- optional file; otherwise project entry point
- `--timings` timing breakdown in project mode

## Advanced Build Knobs (Project Mode)

These knobs are useful when you profile large project builds.

```bash
ARDEN_OBJECT_SHARD_THRESHOLD=1 ARDEN_OBJECT_SHARD_SIZE=2 arden build --timings
```

- `ARDEN_OBJECT_SHARD_THRESHOLD` minimum active-file count before object sharding is enabled
- `ARDEN_OBJECT_SHARD_SIZE` max files per object-codegen shard

Defaults in current compiler:

- `ARDEN_OBJECT_SHARD_THRESHOLD=256`
- `ARDEN_OBJECT_SHARD_SIZE=4`

Important:

- these are advanced performance tuning env vars
- they affect build cache/codegen behavior, not language semantics
- treat them as implementation-level controls, not stable language guarantees

### `arden test`

```bash
arden test [--path <PATH>] [--list] [--filter <PATTERN>]
```

- `--path` file or directory target
- `--list` list discovered tests without execution
- `--filter` run only tests with matching name substring

### `arden fmt`

```bash
arden fmt [PATH] [--check]
```

- format file/directory
- `--check` validates formatting without writing

### `arden lint`

```bash
arden lint [PATH]
```

### `arden fix`

```bash
arden fix [PATH]
```

### `arden lex`

```bash
arden lex <FILE>
```

### `arden parse`

```bash
arden parse <FILE>
```

### `arden bindgen`

```bash
arden bindgen <HEADER> [-o <OUT_FILE>]
```

### `arden bench`

```bash
arden bench [FILE] [--iterations <N>]
```

- default iterations: `5`

### `arden profile`

```bash
arden profile [FILE]
```

### `arden lsp`

```bash
arden lsp
```

## Related

- [Quick Start](../getting_started/quick_start.md)
- [Projects](../features/projects.md)
- [Architecture](architecture.md)
