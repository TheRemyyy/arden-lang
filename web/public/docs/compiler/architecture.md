# Compiler Architecture

This document describes the internal architecture of the Apex compiler.

## Pipeline

1. **Lexing** (`lexer.rs`): Source code is converted into a stream of tokens. String interpolation tokens are handled here.
2. **Parsing** (`parser.rs`): Recursive descent parser that builds an Abstract Syntax Tree (AST) from tokens.
3. **Type Checking** (`typeck.rs`): Traverses the AST to validate types, resolve names, and ensure type safety.
4. **Borrow Checking** (`borrowck.rs`): Analyses ownership and lifetimes to ensure memory safety without GC.
5. **Code Generation** (`codegen/core.rs`, `codegen/types.rs`, `codegen/util.rs`): Lowers the AST into LLVM IR (Intermediate Representation).
6. **Linking**: LLVM IR is compiled to object files and linked through `clang -fuse-ld=lld` to produce the final executable. Apex requires `lld`; there is no fallback linker path.

## Build Caching

- **Project fingerprint cache** (`.apexcache/build_fingerprint`):
  - Hashes project config + source metadata + build-mode flags.
  - If unchanged and output artifact exists, `apex build` exits early (`Up to date ...`).
- **Parsed file cache** (`.apexcache/parsed/*.json`):
  - Stores parsed AST + namespace/import metadata keyed by source fingerprint.
  - On incremental edits, unchanged files bypass tokenization/parsing and reuse cached AST.
  - Uses a fast unchanged-file check from cached file metadata (`len + modified time`) before reading full file contents.
  - If metadata changed but file content hash is still identical, the cached parse result is still reused safely.
- **Rewritten file cache** (`.apexcache/rewritten/*.json`):
  - Stores namespace-rewritten AST fragments keyed by semantic fingerprint + per-file rewrite-context fingerprint.
  - On incremental edits, files whose semantics and relevant namespace/import context did not change bypass rewrite and are stitched directly into combined AST.
- **Import-check cache** (`.apexcache/import_check/*.json`):
  - Stores successful import-check results keyed by semantic fingerprint + per-file import/rewrite context fingerprint.
  - Unchanged files can now skip repeated import-check traversal on hot rebuilds.
- **Dependency graph cache** (`.apexcache/dependency_graph/latest.json`):
  - Stores per-file semantic/API fingerprints plus direct file dependencies resolved from same-namespace access and explicit imports.
  - Same-namespace edges are derived from AST symbol references (calls, constructions, type references, module roots) instead of treating every file in a namespace as mutually dependent.
  - Supports explicit `body-only` vs `API` change classification and reverse-dependent impact tracking between builds.
- **Semantic summary cache** (`.apexcache/semantic_summary/latest.json`):
  - Stores inferred function effect summaries and class mutating-method summaries from successful semantic passes.
  - Unchanged files can seed impacted-file type/borrow checking without re-walking all unaffected bodies.
- **Object file cache** (`.apexcache/objects/*.{o|obj}` + `*.json`):
  - Stores per-file compiled objects keyed by semantic fingerprint + per-file rewrite-context fingerprint + build options (`opt_level`, `target`, compiler version, linker mode).
  - On incremental edits, unchanged files reuse cached object files and final build performs fast relink from cached + rebuilt objects.
  - Object cache misses now emit object files directly from LLVM target machines, avoiding the old textual IR `.ll` -> `clang -c` round-trip.
  - LLVM target registries for direct object emission are initialized once per process, so parallel object rebuilds do not repeatedly pay startup cost.
- **Link manifest cache** (`.apexcache/link/latest.json`):
  - Records the ordered object input list plus final link configuration for the last successful build.
  - If a rebuild produces zero object cache misses and the manifest still matches, Apex skips the final `lld` link invocation entirely and reuses the existing output artifact.
  - When linking does run, Apex now passes large object input sets through a response file to keep `clang`/`lld` startup overhead bounded on large projects.
- **Semantic build fingerprint cache** (`.apexcache/semantic_build_fingerprint`):
  - Hashes canonicalized AST content instead of raw file text.
  - Comment-only / whitespace-only edits can now stop after parse/cache validation and return `Up to date ... (semantic cache)` without object rebuild or relink.
- **Per-file rewrite invalidation**:
  - Rewrite/object cache invalidation now hashes only the current file's namespace/import context and relevant imported namespace symbol maps.
  - Specific imports (`import lib.foo;`) additionally track owner-file API fingerprints, so unrelated API changes in the same namespace no longer fan out to those files.
  - Unrelated namespace changes no longer force global rewrite-cache/object-cache misses across the entire project.
- **Transitive codegen closure**:
  - Object-cache misses now rebuild against the changed file plus only its transitive file dependency closure.
  - Unrelated project files are no longer injected as API-only declarations into every object rebuild miss.
- **Reused API projection programs**:
  - Each rewritten file now precomputes and keeps its API-only projection once.
  - Semantic delta checking and object-cache miss codegen reuse that projected AST instead of regenerating body-stripped declarations repeatedly.
- **Codegen generic-specialization fast path**:
  - Object and full-program codegen now first checks whether the AST actually contains explicit generic call sites.
  - If none exist, Apex skips the generic-specialization rewrite pass entirely instead of cloning and rewalking the whole codegen input.
- **Impacted semantic view**:
  - Type checking and borrow checking now run with full bodies only for changed files and real API dependents.
  - Unchanged unaffected files participate through API projections plus cached semantic summaries.
- **Lazy full-program assembly**:
  - Normal object-link builds no longer materialize the full combined rewritten AST up front.
  - Full-project merged AST construction is deferred to the `emit_llvm` path that actually needs a single monolithic IR module.
- **Parallel project parse phase**:
  - Multi-file project parsing now runs in parallel workers (file read + lex + parse/cache lookup).
  - Import checks and rewrite/cache resolution run in parallel per file.
  - Symbol map/collision resolution and final declaration merge still run deterministically.

## Recent Correctness Hardening

- **Scope-aware LSP rename/references**:
  - Symbol rename/reference resolution now follows lexical bindings selected at cursor position.
  - Prevents accidental edits of unrelated same-name symbols in nested/outer scopes.
- **Precise LSP hover token targeting**:
  - Hover docs are now resolved from the exact token under cursor, not from broad line substring checks.
- **If-expression parsing in expression positions**:
  - Parser now supports `if (...) { ... } else { ... }` as `Expr::IfExpr` where an expression is expected.
  - `if (...) { ... }` without `else` remains valid and is `None`-typed in type analysis.
- **Borrow checker constant-branch flow pruning**:
  - Unreachable RHS of `true || ...` and `false && ...` is no longer analyzed for move/borrow effects.
  - Constant `if` and `while(false)` paths are handled as unreachable in borrow analysis where possible.
  - Constant `if` with early termination no longer triggers false-positive downstream move/use errors.
- **Improved type-check diagnostic spans**:
  - Visibility/signature diagnostics now use declaration-context spans instead of synthetic `0..0` placeholders.
- **Match-expression correctness checks**:
  - Type checker now validates compatible result types across all match-expression arms.
  - Exhaustiveness checks are enforced for `Boolean`, `Option<T>`, and `Result<T, E>` unless a catch-all arm exists.
- **Import-check traversal hardening**:
  - Import checking now traverses class constructors/destructors/methods, module functions, and interface default implementations (not only top-level functions).
  - Module function namespace extraction uses mangled names consistently (`Module__func`) during import resolution.
  - Nested-module local function collection now preserves full prefix chains (`A__X__f`) for correct local-shadow/import checks.
  - Namespace alias imports (`import ... as alias`) no longer implicitly grant unqualified access to all symbols in that namespace.
- **Alias resolution hardening**:
  - Specific-symbol aliases (for example `import std.math.Math__abs as abs_fn`) are resolved across type checking and code generation, so aliased calls compile and execute correctly.
  - Alias canonicalization now uses symbol-table/registry lookups instead of brittle namespace-prefix checks.
  - Project rewrite now resolves namespace-only alias imports (`import math_utils as mu`) for module-style calls (`mu.factorial(...)`) to correct mangled symbols.
  - Project rewrite now also resolves nested module calls through namespace aliases (for example `import lib as l; l.Tools.ping()` and `l.A.X.f()`) to deep mangled project symbols.
  - Dotted module alias imports (for example `import lib.A.X as ax`) now resolve module-style calls (`ax.f()`) to the correct deep mangled symbols.
- **Import alias diagnostics hardening**:
  - Unknown namespace aliases are now surfaced during import checking when used in module-style calls, reducing delayed downstream failures.
  - Unknown alias diagnostics now emit actionable guidance for valid alias imports instead of suggesting invalid synthetic import paths.
  - Invalid dotted namespace alias paths now consistently route to namespace-alias diagnostics (`import <namespace> as <alias>;`) instead of falling through to generic later-stage errors.
- **Nested module codegen hardening**:
  - Function symbol extraction now recurses through nested modules in project parsing/import-check phases.
  - Type-check declaration collection now also recurses nested modules, so deep mangled symbols are available to semantic checks (`A__X__id`, `A__Y__add`, ...).
  - Filtered project compilation now recursively declares and compiles nested module symbols when a parent module namespace is active, preventing missing-symbol linker failures for deep module calls.
- **Generic call safety hardening**:
  - Codegen now performs on-demand specialization for explicit generic free/module calls (for example `id<T>(...)`, `A.X.id<T>(...)`) and rewrites call sites to concrete specialized symbols.
  - Project filtered compilation now always emits generated specialization bodies to avoid missing-symbol linker regressions.
- **Shared stdlib registry**:
  - Compiler stages now reuse a single lazy-initialized stdlib registry (`OnceLock`) instead of repeatedly constructing stdlib lookup maps during hot-path analysis and lowering.
- **Lint scope analysis hardening**:
  - Shadowing diagnostics now account for parameters and `for`-loop variables.
  - Unused-variable diagnostics now include unused `for`-loop iterator variables.
- **String interpolation parser hardening**:
  - Unclosed interpolation fragments (`"{...`) are preserved as literal text.
  - Empty braces (`{}`) are preserved as literal text.
  - Interpolation nodes that end up fully literal are normalized back to plain string literals.

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
