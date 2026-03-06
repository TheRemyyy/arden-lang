# Compiler Architecture

This document describes the internal architecture of the Apex compiler.

## Pipeline

1. **Lexing** (`lexer.rs`): Source code is converted into a stream of tokens. String interpolation tokens are handled here.
2. **Parsing** (`parser.rs`): Recursive descent parser that builds an Abstract Syntax Tree (AST) from tokens.
3. **Type Checking** (`typeck.rs`): Traverses the AST to validate types, resolve names, and ensure type safety.
4. **Borrow Checking** (`borrowck.rs`): Analyses ownership and lifetimes to ensure memory safety without GC.
5. **Code Generation** (`codegen/core.rs`, `codegen/types.rs`, `codegen/util.rs`): Lowers the AST into LLVM IR (Intermediate Representation).
6. **Linking**: LLVM IR is compiled to an object file and linked (using `clang`/`cc`) to produce the final executable.

## Build Caching

- **Project fingerprint cache** (`.apexcache/build_fingerprint`):
  - Hashes project config + source metadata + build-mode flags.
  - If unchanged and output artifact exists, `apex build` exits early (`Up to date ...`).
- **Parsed file cache** (`.apexcache/parsed/*.json`):
  - Stores parsed AST + namespace/import metadata keyed by source fingerprint.
  - On incremental edits, unchanged files bypass tokenization/parsing and reuse cached AST.

## Directory Structure

- `src/main.rs`: Entry point, CLI argument parsing.
- `src/ast.rs`: Definitions of all AST nodes (Expr, Stmt, Type).
- `src/lexer.rs`: Tokenizer implementation.
- `src/parser.rs`: Parser implementation.
- `src/typeck.rs`: Type checker implementation.
- `src/borrowck.rs`: Borrow checker implementation.
- `src/formatter.rs`: AST-driven source formatter used by `apex fmt`.
- `src/codegen/mod.rs`: Codegen module entry.
- `src/codegen/core.rs`: Core IR generation and lowering.
- `src/codegen/types.rs`: Built-in collection/Option/Result/Range codegen helpers.
- `src/codegen/util.rs`: C runtime bindings and utility helpers.

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for details on how to set up the dev environment and submit PRs.
