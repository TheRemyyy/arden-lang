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

### `arden info`

```bash
arden info
```

Prints resolved project configuration (entry, files, output, optimization and
related settings). Use this first when build/run behavior looks unexpected.

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

### Host CPU Native Codegen (Advanced)

By default, Arden emits code for a stable baseline CPU (`x86-64` or `generic`)
to keep outputs portable across machines/CI runners.

To opt into host-specific tuning for local builds/runs:

```bash
ARDEN_CODEGEN_NATIVE_CPU=1 arden run file.arden
```

Notes:

- this is intended for local performance experiments
- avoid enabling it in shared CI/release pipelines unless you fully control runtime CPUs
- explicit `--target` builds keep target-driven behavior

### Platform Linker Overrides (Advanced Troubleshooting)

Use only when diagnosing platform linker/toolchain setup issues.

- `ARDEN_LLVM_REAL_PREFIX`
  - override LLVM prefix lookup used by linker/toolchain discovery
  - mainly relevant on custom/local LLVM installs
- `ARDEN_WINDOWS_BUILTINS_LIB` (Windows)
  - explicit path to `clang_rt.builtins-x86_64.lib` when auto-detection fails

Example (Windows PowerShell):

```powershell
$env:ARDEN_WINDOWS_BUILTINS_LIB = "C:\\llvm\\lib\\clang\\22\\lib\\windows\\clang_rt.builtins-x86_64.lib"
arden build --release
```

### Tooling / CI Environment Variables

These are useful for repository tooling and diagnostics.

- `ARDEN_COMPILER_PATH`
  - override compiler binary path in smoke scripts (for example, debug build vs release build)
- `CI_SKIP_COMPILER_BUILD=1`
  - skip rebuilding compiler in smoke scripts before execution
- `ARDEN_FAILURE_SOURCE`
- `ARDEN_FAILURE_SOURCES`
- `ARDEN_FAILURE_CONTEXT`
- `ARDEN_FAILURE_OUTPUT_ROOT`
  - control CI crash artifact dump scripts (`scripts/ci/*emit_codegen_artifacts*`)

Internal test-only markers (not user-facing CLI settings):

- `ARDEN_BAD_UTF8_ENV`
- `__ARDEN_TEST_START__`
- `__ARDEN_TEST_PASS__`
- `__ARDEN_TEST_SKIP__`
- `__ARDEN_TEST_SKIP_REASON__`

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
