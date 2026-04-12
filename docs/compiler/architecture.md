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
