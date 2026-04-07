# Compiler Architecture

This document describes the current high-level structure of the Arden compiler.

It is intentionally architectural, not a changelog dump.

## Pipeline

Arden roughly follows this flow:

1. lex source into tokens
2. parse tokens into an AST
3. resolve and type-check declarations and expressions
4. run borrow checking
5. lower to LLVM IR
6. compile and link a native artifact

## Main Source Areas

### Frontend

- `src/lexer/` - tokenization
- `src/parser/` - AST construction
- `src/ast/` - AST definitions

### Semantic Analysis

- `src/typeck/` - type collection, resolution, checking, effects
- `src/borrowck/` - ownership and borrowing validation
- `src/import_check/` - import validation
- `src/project_rewrite/` - project-mode rewriting / symbol normalization

### Project / Build Logic

- `src/project/` - `arden.toml` loading and project configuration
- `src/cache/` - project cache and reuse metadata
- `src/dependency/` - dependency graph and invalidation logic
- `src/symbol_lookup/` - symbol indexing and lookup support

### Backend

- `src/codegen/` - LLVM IR lowering
- `src/linker/` - final artifact linking
- `src/stdlib/` - intrinsic stdlib wiring used by semantic/codegen stages

### Tooling

- `src/formatter/` - `arden fmt`
- `src/lint/` - `arden lint` / `arden fix`
- `src/test_runner/` - `arden test`
- `src/bindgen/` - `arden bindgen`
- `src/lsp/` - `arden lsp`

### Tests

- `src/tests/` - integration-style compiler tests
- module-local `tests.rs` files - unit and behavior-focused coverage

## Project Mode

Project mode is driven by `arden.toml`.

Important behavior:

- files are listed explicitly
- the entrypoint is explicit
- project commands such as `build`, `run`, `check`, `test`, `fmt`, and `info` use project configuration
- project builds can reuse cached work from `.ardencache/`

Related docs:

- [CLI reference](cli.md)
- [Projects](../features/projects.md)

## Native Toolchain

Arden lowers through LLVM and links through Clang.

Current linker policy is explicit:

- Linux requires `mold`
- macOS requires LLVM `lld`
- Windows requires LLVM `lld`

This is also reflected in CI and release workflows.

## Build Cache

Project builds maintain cache data under `.ardencache/`.

At a high level, that cache is used to:

- detect unchanged builds
- reuse parsed or rewritten project state where possible
- reduce rebuild work after partial edits

For user-facing timing inspection:

```bash
arden build --timings
arden check --timings
```

## CLI Entry

The main CLI entrypoint is:

- `src/main.rs`

That command surface is documented in:

- [CLI reference](cli.md)

