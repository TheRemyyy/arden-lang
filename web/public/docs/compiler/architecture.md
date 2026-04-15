# Compiler Architecture

## Why This Matters

When you change compiler behavior, this map tells you where to change it safely and where to add tests.

## Pipeline

High-level flow:

1. lex source
2. parse AST
3. resolve + type-check
4. borrow-check
5. lower to LLVM IR
6. compile/link native artifact

## Main Source Areas

### Frontend

- `src/lexer/`
- `src/parser/`
- `src/ast/`

### Semantic Stages

- `src/typeck/`
- `src/borrowck/`
- `src/import_check/`
- `src/project/` rewrite/semantic pipeline pieces

### Backend

- `src/codegen/`
- `src/linker/`
- `src/stdlib/` intrinsic wiring

### Tooling

- `src/formatter/`
- `src/lint/`
- `src/test_runner/`
- `src/bindgen/`
- `src/lsp/`

### Tests

- integration-style suites in `src/tests/`
- module-focused coverage in local test modules

## Project Mode Architecture

Project mode centers around `arden.toml`:

- explicit entry + files list
- import graph validation
- semantic/build cache reuse via `.ardencache/`

### Build Cache Layers (Project Mode)

High-level cache flow:

1. parse/index cache
2. semantic cache gate
3. rewrite cache
4. object cache (including shard-level cache for large projects)
5. final link manifest cache

The compiler can reuse different layers independently, which is why `--timings`
often shows partial rebuilds instead of all-or-nothing behavior.

### Object Codegen Sharding

For large projects, object codegen is grouped into shards to reduce fixed LLVM
module/object overhead.

Current tuning env vars:

- `ARDEN_OBJECT_SHARD_THRESHOLD` (default `256`)
- `ARDEN_OBJECT_SHARD_SIZE` (default `4`)
- `ARDEN_CODEGEN_NATIVE_CPU` (default disabled; set `1/true/yes/on` for host-native CPU tuning)

Example profiling command:

```bash
ARDEN_OBJECT_SHARD_THRESHOLD=1 ARDEN_OBJECT_SHARD_SIZE=2 arden build --timings
```

These knobs are intended for performance investigation and CI tuning.
They are not language-level syntax/settings.

## Linker Policy

Repo-default linkers:

- Linux: `mold`
- macOS: `lld`
- Windows: `lld-link`

## Debugging Build Stages

Use timings:

```bash
arden build --timings
arden check --timings
```

And parse/lex commands for frontend debugging:

```bash
arden lex file.arden
arden parse file.arden
```
