# Changelog

All notable changes to the Apex Programming Language Compiler will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### 🐛 Fixed

- Fixed `apex test` handling for `@Ignore` without a reason:
  - tests marked with bare `@Ignore` are now skipped correctly instead of being executed
  - ignored tests are now counted in the final `Total` summary as well as `Ignored`
- Fixed string and char escape decoding in Apex source:
  - escape sequences like `\n`, `\t`, `\"`, `\\`, and escaped char literals now decode correctly at runtime
  - escaped interpolation braces (`\{` and `\}`) now remain literal text instead of incorrectly triggering string interpolation
- Fixed `apex test` ignore-reason rendering so backslashes and control characters are preserved correctly in generated runner output.
- Fixed `apex test --list` ignore-reason rendering so control characters no longer break discovery layout.
- Fixed stale example/docs interpolation snippets that used `${...}` instead of Apex `{...}` interpolation syntax.
- Fixed `range()` support so `Range<Float>` now works end-to-end instead of being rejected or miscompiled as an integer iterator.
- Fixed `range()` validation so mixed numeric arguments are rejected with a clear same-type diagnostic.
- Fixed zero-step `range()` creation so dynamic `step=0` now fails fast with a runtime error instead of producing inconsistent `has_next()/next()` behavior.
- Expanded CLI smoke coverage to assert the real `examples/24_test_attributes.apex` runner output and ignored-test totals.
- Fixed Windows LLVM setup in GitHub Actions by removing the fragile `llvm-config` shim/copy path and exporting the real LLVM prefix directly.
- Fixed Windows CI LLVM setup time by replacing Chocolatey-based installation with a direct cached prebuilt LLVM archive install shared across Windows jobs.
- Fixed macOS x64 release builds by running `x86_64-apple-darwin` on an Intel GitHub runner instead of linking against ARM Homebrew LLVM artifacts.

### ⚡ Changed

- Apex project linking now uses an explicit no-fallback policy:
  - Linux requires `mold`
  - macOS and Windows require LLVM `lld`
  - linker selection remains encoded in build cache fingerprints
- Linux CI LLVM setup now installs `mold` instead of wiring `lld` symlinks into `PATH`.

## [1.3.5] - Bug Fixes - 2026-03-08

### ✨ Added

- `apex fmt` command for formatting Apex source files.
  - Supports single-file, directory, and project-aware formatting.
  - Supports `--check` mode for CI.
- Parser support for compound assignment operators:
  - `+=`, `-=`, `*=`, `/=`
  - supports identifier targets and complex lvalues (`arr[i] += 1`, `obj.field -= 2`)
- New tooling commands:
  - `apex lint` for static source diagnostics
  - `apex fix` for safe automated cleanup
  - `apex bench` for repeated wall-time measurement
  - `apex profile` for single-run wall-time reporting
- Project linker/distribution configuration in `apex.toml`:
  - `output_kind = "bin" | "shared" | "static"`
  - `link_libs`
  - `link_search`
  - `link_args`
- Expanded CI coverage:
  - release-built CLI smoke coverage for `new`, `info`, `check`, `lint`, `fix`, `fmt`, `lex`, `parse`, `compile`, `run`, `test`, `bench`, `profile`, and `bindgen`
  - frontend typecheck, test, and production build verification
- `apex compile` now supports:
  - `--opt-level <0|1|2|3|s|z|fast>`
  - `--target <triple>`
- Benchmark runner improvements:
  - `benchmark/run.py --apex-opt-level ...` (default: `3`)
  - `benchmark/run.py --apex-target ...`
  - Apex benchmark compile now uses `--no-check` for fair runtime-focused comparisons.
  - Added Go benchmark language parity (`benchmark/go/*`, `benchmark/run.py`).
  - Added `compile_project_10_files` stress benchmark (generated 10-file project compile timing per language).
  - Added compile benchmark cache modes: `--compile-mode hot|cold` for `compile_project_10_files`.
  - Added cold-mode artifact/cache cleanup handling and Apex transient `.ll` retry guard in benchmark runner.
  - Added `incremental_rebuild_1_file` benchmark: compile once, mutate one source file, then recompile and report first/second compile timing.
  - Added `incremental_rebuild_central_file` benchmark: compile once, mutate shared core dependency file, then recompile for dependency-heavy invalidation path measurement.
  - Added `incremental_rebuild_mega_project_10_files` benchmark: compile a generated 120-file mega-project, apply syntax-only edits to 10 spread-out files, then report cold full-build vs hot rebuild timing.
  - Added `compile_project_synthetic_mega_graph` benchmark: compile a generated 1400-file synthetic mega-graph project with 96 helper functions per file to compare cold/hot compile-time scaling across languages.
  - Added `incremental_rebuild_synthetic_mega_graph` benchmark: cold-build the generated synthetic mega-graph project, apply syntax-only edits across 40 spread-out files, then measure the rebuild.
  - Added `incremental_rebuild_synthetic_mega_graph_mixed_invalidation` benchmark: cold-build the synthetic mega-graph project, then rebuild after a mixed dirty set of 24 leaf edits plus 8 API-surface group invalidations and caller rewrites.
  - `compile_project_synthetic_mega_graph` generates a layered cross-file dependency graph instead of mostly isolated leaf files, making the compile-time workload a stronger synthetic multi-file stress test.
  - Benchmark/docs wording now explicitly describes the mega-graph workloads as synthetic and not representative of a real Chromium-scale codebase.
  - `compile_project_synthetic_mega_graph` and `incremental_rebuild_synthetic_mega_graph` are now substantially heavier:
    - 1400 generated files instead of 1000
    - 96 helper functions per file instead of 64
    - wider dependency fan-out and extra cross-file wiring/surface functions per file
    - 40-file edit batches in the synthetic mega-graph rebuild benchmark instead of 25
  - Synthetic mega-graph workloads now include active group bridge modules, enabling a more realistic mixed invalidation benchmark that changes shared API surface and rewrites dependent callers instead of only appending syntax-only edits.
  - `python3 benchmark/run.py` now includes both `compile_project_10_files_hot` and `compile_project_10_files_cold` plus incremental rebuild output in one report by default.
  - Benchmark runner now normalizes executable path handling for Windows (`.exe`) and supports C compiler auto-detection (`CC`/`clang`/`gcc`).
- New language coverage examples:
  - `examples/35_visibility_enforcement.apex`
  - `examples/36_inheritance_extends.apex`
  - `examples/37_interfaces_contracts.apex`
  - `examples/38_import_aliases.apex`
  - `examples/39_compound_assign.apex`
  - `examples/40_borrow_scope_recovery.apex`
- New lint checks:
  - `L004` unused variables (`Variable 'x' is declared but never used`)
  - `L005` variable shadowing diagnostics with outer declaration offset

### ♻️ Changed

- Apex project linking now requires `lld` and uses `clang -fuse-ld=lld` exclusively.
  - Removed linker fallback behavior; missing `lld` is now a hard error.
  - Object-cache build fingerprints now encode the enforced linker mode.
- Switched internal stdlib metadata access to a shared lazy registry (`OnceLock`) instead of repeated `StdLib::new()` construction in hot paths (type checker, borrow checker, rewrite, codegen, and import-check entry points).
- Namespace-only alias imports in project rewrite now resolve module-style calls (`alias.fn(...)`) to project-mangled function symbols.
- Project object compilation for cache misses now runs in parallel (per-file LLVM context + codegen instance), then links sequentially.
- LSP rename/references now resolve symbol locations from parsed AST spans instead of raw text search.
- LSP rename/references now resolve occurrences by cursor-selected lexical binding (scope-aware), preventing cross-scope over-rename of same-name symbols.
- LSP hover now resolves keyword docs from the exact token under cursor instead of line substring matching.
- Import checking now reuses a single `StdLib` instance and shared `Arc<HashMap<...>>` namespace map instead of rebuilding/cloning per file.
- Import typo suggestion distance now uses a rolling two-row Levenshtein buffer (`O(m)` memory) instead of full matrix allocation (`O(n*m)`).
- `List<T>` now supports fixed-capacity construction with a compile-time literal argument (`List<T>(N)`) using stack-backed storage.
- List growth path now uses explicit `malloc + copy` reallocation logic, which is compatible with both heap-backed and stack-backed list buffers.
- `List<Boolean>` now uses boolean-sized element storage/codegen paths instead of integer-width storage in list internals.
- `apex fmt` now preserves source comments instead of refusing commented files.
- Borrow checker call-site param-mode resolution for methods is now type-directed from receiver type instead of first-match method-name heuristics across all classes.
- Borrow checker now tracks optional declared variable type metadata to improve method-call move/borrow analysis accuracy.
- Project builds now wire `target` from `apex.toml` into final Clang linking (`--target <triple>`).
- Project builds can now emit shared libraries and static archives via `output_kind`.
- Single-file scripts can start with a Unix shebang (`#!/usr/bin/env apex`).
- `apex info` now displays `Target` value (`native/default` when not set).
- `apex info` now displays output kind and native linker settings from `apex.toml`.
- Clang fallback flow now degrades native tuning more gracefully:
  - tries `-march=native -mtune=native`
  - then `-march=native`
  - then baseline flags
- Class visibility semantics are now enforced by type checking for field and method access.
- Class inheritance (`extends`) now participates in semantic lookup for inherited fields/methods.
- Interface semantics were tightened:
  - `implements` contracts are validated
  - interface inheritance (`interface A extends B`) is validated
  - interface types can be used in function parameters and assignments
- Import aliases (`import ... as ...`) are now supported by parser, formatter, checker, and codegen.
- Project builds now use `.apexcache` with:
  - early up-to-date skip via project fingerprint cache
  - parser-level per-file AST cache reuse for unchanged files in changed builds
  - parse cache now does a metadata fast-path (`len + mtime`) before falling back to full source hashing for unchanged-file reuse
  - persistent file dependency graph cache for explicit `body-only` vs `API` impact classification and reverse-dependent tracking
  - same-namespace dependency edges now come from AST symbol references instead of blanket namespace-wide file fanout
  - import-check success cache reuse keyed by semantic fingerprint + import/rewrite context
  - semantic summary cache for inferred function effects and mutating-method receiver behavior reused from unchanged files
  - semantic fingerprint cache to ignore comment-only / whitespace-only edits
  - rewrite-level per-file AST cache reuse keyed by per-file import/namespace context instead of whole-project context
  - specific imports now track owner-file API fingerprints instead of invalidating on unrelated API changes elsewhere in the same namespace
  - type/borrow checking now runs on an impacted semantic view (changed files + true API dependents with full bodies, API projections elsewhere)
  - rewritten file API projections are now precomputed once and reused across semantic delta checking plus per-file object rebuild misses
  - object-cache miss codegen now uses transitive file dependency closure instead of injecting API stubs from the entire project
  - changed-file object rebuilds now emit `.o/.obj` directly from LLVM target machines instead of round-tripping through textual `.ll` plus `clang -c`
  - LLVM target initialization for direct object emission is now one-time per process instead of repeating for every rebuilt object
  - full combined rewritten AST is now materialized only for `emit_llvm`; normal object-link builds stay on narrower per-file program assembly
  - object-level per-file cache reuse for unchanged files plus relink-only final stage
  - final link is now skipped entirely when all object files are cache hits and the ordered link manifest matches the previous successful link
  - final executable/shared linking now feeds object inputs to `clang` through a response file instead of expanding huge object lists directly on the command line
  - codegen now skips the explicit-generic-call specialization rewrite entirely for programs that contain no explicit generic invocations
  - `compile_filtered()` now prunes declaration-phase work to active symbols plus transitive API-referenced declaration closure instead of blanket-declaring the whole slim codegen program
  - dependency API projections fed into object-miss codegen are now filtered down to the same declaration closure instead of carrying every stub declaration from every file in the dependency slice
  - parallel multi-file parse pipeline for lower front-end wall time on larger projects
  - parallel import-check and rewrite/cache resolution pass
- CI workflow now builds Linux release compiler once and reuses the artifact for CLI smoke and examples jobs, avoiding duplicate release rebuilds.
- CI examples validation now invokes `target/release/apex-compiler` directly instead of `cargo run --release -- ...`.
- CI LLVM install steps on Ubuntu are now centralized into a reusable composite action (`.github/actions/install-llvm`) to remove duplicated workflow logic.
- CI/release Ubuntu LLVM setup now installs `lld-21` explicitly and exports `ld.lld`/`lld` into `PATH`, matching Apex's no-fallback linker requirement.
- CI job graph is now `build -> (checks, smoke, examples)` while `web` runs independently in parallel.
- Release workflow now publishes both macOS architectures (`aarch64-apple-darwin`, `x86_64-apple-darwin`).
- Windows release workflow LLVM setup now uses Chocolatey (`choco install llvm`) instead of downloading a hardcoded GitHub release tarball URL.
- Async task runtime control now uses a portable completion-state model instead of relying on Linux-only timed join APIs.
  - `Task.await_timeout(...)` now works through portable completion polling plus final join semantics.
  - Async function and async block runtimes now publish task completion/result state explicitly before final await/join.
  - Task runtime state now distinguishes `completed` from `done`, matching `is_done`, `cancel`, and timeout behavior more accurately across platforms.
- CI LLVM setup on Windows now derives a usable `llvm-sys` configuration even when the Chocolatey LLVM install does not ship `llvm-config.exe`.
  - The shared GitHub Actions LLVM install step now discovers LLVM 21 via `clang.exe` when needed and prepares a Windows `llvm-config.exe` shim for `llvm-sys`.
  - GitHub Actions build/check steps now rely on `GITHUB_ENV` exports from the shared LLVM install action instead of re-overriding LLVM env vars per step.

### 🐛 Fixed

- Fixed duplicate global string symbol collisions (`str.*`, `fmt.*`, file mode constants) during object-cache relink builds on multi-file projects by using private linkage for internal string globals.
- Fixed stale-object reuse after object-linkage strategy updates by bumping object cache schema to invalidate incompatible cached `.o/.obj` artifacts.
- Web docs routing now uses extensionless `/docs/...` URLs consistently in footer links and sitemap output.
- Markdown HTML rendered in the docs/changelog web UI is now sanitized before insertion.
- Fixed parser handling where compound assignments on parsed postfix targets from identifier-leading expressions were rejected (`items[0] -= 1` path).
- Fixed borrow checker scope-exit borrow release logic:
  - mutable borrow state reset now checks active borrows for the same variable only
  - immutable borrow counts are properly recomputed/decremented after scope exit
- Fixed lambda capture borrow semantics by analyzing free identifiers in lambda bodies and applying move/borrow behavior for captured outer variables.
- Fixed false-positive lambda capture diagnostics where owned captures were reported as use-after-move inside the lambda expression itself.
- Fixed borrow checker assignment validation for nested lvalues (`obj.field = ...`, `arr[i] = ...`) so owner borrow state is enforced consistently (not only plain identifier targets).
- Fixed method-call borrow mode resolution for `this.method(...)` receiver calls by inferring receiver class from `this` type metadata.
- Fixed type checker validation gap for built-in generic constructors:
  - `List<T>` now validates constructor args (`0` args, or optional integer capacity)
  - `Map<K,V>`, `Set<T>`, `Option<T>`, `Result<T,E>` constructors now reject unexpected value arguments
- Fixed borrow checker state handling after invalid assignment:
  - ownership state is no longer force-reset to `Owned` when assignment is rejected due to active borrows
  - follow-up diagnostics (e.g. subsequent move-while-borrowed) are now preserved in the same flow
- Fixed import checker precedence so same-file local functions correctly shadow stdlib names (e.g. local `print(...)` no longer incorrectly requires `import std.io.print;`).
- Fixed import checking for stdlib module-style calls (`Math.abs(...)`) so missing `std.math` imports are now reported.
- Fixed import checker handling for namespace aliases (`import std.io as io;`) so aliased stdlib calls like `io.println(...)`, `math.abs(...)`, and `str.len(...)` are validated correctly.
- Fixed `apex check` behavior in project mode:
  - running `apex check` without an explicit file now performs project-aware validation (same multi-file pipeline as project builds), not entry-file-only checking.
  - project build/check now runs type checker and borrow checker on the rewritten combined project AST before codegen/link.
- Improved import-check hint text for module-style stdlib calls (`Math.abs`, `Str.len`, `System.os`) to suggest namespace wildcard imports (`import std.math.*;`) instead of mangled symbol names.
- Fixed parser built-in generic handling gap for enum named fields so `Ptr<T>` now parses as `Type::Ptr` (instead of generic fallback).
- Fixed import checker traversal gaps so missing imports are now reported inside:
  - class methods
  - constructors/destructors
  - module functions
  - interface default method implementations
- Fixed namespace extraction for module functions to use only mangled symbols (`Module__func`) and avoid leaking unqualified names.
- Fixed test-runner code generation to declare mutable test counters (`mut tests_total/tests_passed/tests_failed/tests_ignored`), preventing invalid assignments in generated runner code.
- Fixed test-runner source rewriting to strip non-plain entry signatures like `public function main(...)`, preventing duplicate-main generation.
- Fixed lint `L003` behavior to correctly flag unused specific imports for stdlib symbols (for example `import std.math.abs;` when unused).
- Fixed lint `L004` unused-variable analysis to include `for (...)` loop variables.
- Fixed lint `L005` shadowing analysis to detect shadowing against:
  - function/class method/constructor parameters
  - `for` loop variables that shadow outer names
- Fixed test-runner import injection detection to only match real import lines (comment text like `// import std.io.*;` no longer suppresses injection).
- Fixed test-runner package handling for sources with leading comments containing `;`, so stdio import insertion keeps correct package-first ordering.
- Fixed test-runner `main` stripping to avoid false positives on comments mentioning `function main(...)`.
- Fixed test-runner `main` stripping to support `public/private/protected` and `async` main signatures.
- Fixed parser string interpolation fallback for unclosed `{...` sequences so they remain literal text instead of being interpreted as expressions.
- Fixed project namespace alias call lowering so valid imports like `import math_utils as mu; mu.factorial(...)` compile and run in multi-file projects.
- Fixed project namespace alias lowering for nested module chains:
  - `import lib as l; l.Tools.ping()` now rewrites to the correct mangled symbol.
  - deep chains like `l.A.X.f()` now resolve end-to-end instead of falling through to undefined-variable/linker failures.
- Fixed import checker diagnostics for unknown namespace aliases so invalid alias usage is reported during import checking (instead of surfacing later as generic undefined-variable/codegen failures).
- Fixed unknown namespace-alias import-check hint text to be actionable (`import <namespace> as <alias>;`) instead of emitting invalid synthetic import suggestions.
- Fixed invalid dotted namespace alias handling (`import nope.ns as n; n.call();`) to consistently produce unknown-namespace-alias diagnostics during import checking.
- Fixed dotted module-alias imports so paths like `import lib.A.X as ax; ax.f()` resolve and compile correctly in project mode.
- Fixed CI `cli-smoke` compiler path resolution when reusing downloaded release artifact:
  - `scripts/ci_cli_smoke.sh` now normalizes relative `APEX_COMPILER_PATH` to absolute path before changing working directories.
  - CI now passes absolute compiler path via `${{ github.workspace }}/target/release/apex-compiler`.
- Fixed macOS async runtime example/link failures caused by `Task.await_timeout(...)` depending on unavailable `pthread_timedjoin_np`.
- Fixed async task completion races by initializing task back-pointers before worker thread spawn and publishing completion state atomically.
- Fixed Windows GitHub Actions `llvm-sys` detection failures when the LLVM 21 Chocolatey install provides `clang.exe` but no `llvm-config.exe`.
- Fixed parser handling of empty interpolation braces (`{}`) to preserve braces as literal text.
- Fixed parser string interpolation normalization so all-literal interpolation parts are merged back into plain string literals.
- Fixed import checker alias semantics: namespace alias imports no longer implicitly import all symbols as unqualified calls.
- Fixed import checker false negatives where `import std.math as math;` incorrectly allowed direct `Math__abs(...)` calls without proper import.
- Fixed import checker false negatives where `import std.math as math;` incorrectly allowed `Math.abs(...)` calls without proper import.
- Fixed lint `L003` unused-specific-import detection to use alias binding names (for `import ... as alias`) when determining usage.
- Fixed lint `L003` messages for aliased specific imports to include full import identity (`path as alias`) instead of only raw path.
- Fixed direct specific import aliases for stdlib symbols (for example `import std.math.Math__abs as abs_fn;`) so calls like `abs_fn(...)` now compile and run end-to-end.
- Fixed codegen alias resolution to avoid hardcoded namespace-to-module mapping (`std.math` -> `Math`, etc.) and resolve module aliases dynamically from the registered stdlib symbol table.
- Fixed rewrite/typecheck alias resolution logic to avoid brittle string-prefix checks (`starts_with("std.")`) and rely on canonical symbol registry lookups instead.
- Fixed generic function type parameters being resolved as class names during type checking:
  - function/method generic params now bind to internal type variables in signatures and body checks
  - call sites like `id<Integer>(1)` no longer fail with spurious `expected T, got Integer` errors.
- Fixed explicit generic call handling end-to-end:
  - parser now supports explicit type arguments on method and module calls (`obj.fn<T>(...)`, `Module.fn<T>(...)`)
  - non-generic functions called with explicit type arguments now fail
  - explicit generic arity mismatch now fails
  - unknown explicit type argument types now fail
- Fixed borrow checker argument-mode fallback for member calls with expression receivers (`mk().use(x)`):
  - unresolved receiver types no longer default call arguments to `Owned` moves.
- Fixed mutable receiver borrow validation to reject mutating method calls on immutable variables.
- Fixed assignment mutability enforcement for nested lvalues:
  - immutable owners now reject `obj.field = ...`
  - immutable owners now reject `arr[i] = ...`
- Fixed examples regression in `examples/12_string_interp.apex` by adding missing `std.math` import for `Math.abs(...)`.
- Fixed project rewrite handling of stdlib namespace aliases (`import std.io as io;`, `import std.math as math;`) so project-mode `check/build` no longer rewrites aliases into invalid mangled module identifiers.
- Fixed type checker alias resolution precedence: a local variable named like an import alias (for example `io`) no longer gets treated as stdlib module alias in method-call resolution (`io.println(...)` now correctly errors on non-module variable types).
- Replaced hardcoded stdlib alias mapping in type checking/project rewrite with stdlib-registry-based resolution:
  - alias calls now resolve generically from `StdLib` (`std.io`, `std.math`, `std.string`, `std.system`, `std.time`, `std.fs`, ...).
  - avoids per-namespace/per-module hardcoded branching and reduces risk of future alias regressions when stdlib surface changes.
- Fixed import checker traversal bug where alias-resolved callee handling (`io.println(...)`) could skip validation of nested argument calls; missing imports inside arguments (for example `Math.abs` without `std.math`) are now correctly reported.
- Fixed nested module function namespace extraction/collection to recurse through deep module trees (`Outer__Inner__f`), avoiding missed symbol ownership during import checking and project rewrite.
- Fixed type checker declaration collection for nested modules to recursively register deep module function signatures (`A__X__id`, `A__Y__add`), preventing false `Undefined variable` errors after correct project rewrite mangling.
- Fixed import-check local function collection for nested modules to preserve full module prefix chains (`A__X__f` instead of truncated `X__f`), reducing false import diagnostics in deeply nested module files.
- Fixed single-file borrow checking for stdlib namespace aliases (`import std.io as io; io.println(s);`):
  - alias calls now resolve to stdlib borrow-mode signatures in borrow checker (no false move on borrowed stdlib args),
  - and borrow arguments in function calls are now treated as temporary call-site borrows released after the call expression.
- Fixed borrow checker lifetime tracking for reference-return call initializers:
  - `r: &T = f_borrowing(x)` now keeps the source borrow active in the surrounding scope, preventing invalid moves/assignments of `x` while `r` is alive.
- Fixed lambda borrow captures to apply at lambda creation scope (outer scope), so captured borrows correctly block invalid moves after closure creation.
- Fixed borrow checker method receiver analysis:
  - mutating receiver calls now infer receiver mode from class method bodies (including transitive `this.other_method()` mutating paths),
  - receiver borrows are now temporary call-site borrows (no stale borrow after `obj.method()` returns),
  - nested receiver chains (`a.b.touch()`) now propagate receiver borrow mode to root owner (`a`) for correct borrow conflict detection.
- Fixed codegen index access on `List<T>` values to handle materialized list structs (`{capacity,length,data}`) without pointer-cast panics.
- Fixed codegen lvalue generation for indexed assignment (`xs[i] = ...`, `xs[i] += ...`) so check-pass programs no longer fail with `Invalid lvalue`.
- Fixed codegen lvalue support for indexed assignment through field-owned lists (`obj.list[i] += ...`) and nested field assignment type resolution (`a.b.v += ...`).
- Fixed filtered project codegen for nested modules:
  - nested module functions are now declared/compiled recursively when a parent module namespace is active,
  - prevents `undefined reference` link failures for deep module symbols referenced through namespace aliases.
- Fixed explicit generic free/module function calls in codegen via on-demand specialization:
  - calls like `id<Integer>(77)`, `A.X.id<Integer>(...)`, and alias-based variants now compile and run without runtime crashes.
  - generated specializations are emitted with stable symbol naming (`__spec__...`) and explicit type-argument rewriting.
- Fixed filtered project codegen linkage for generated generic specializations:
  - specialization symbols are now compiled even under active-symbol filtering,
  - preventing `undefined reference to ...__spec__...` linker errors in project builds.
- Fixed parser behavior where visibility modifiers on `constructor`/`destructor` were silently ignored.
- Fixed formatter roundtrip stability for expression statements starting with `match`/`if`:
  - `apex fmt` now emits parenthesized expression-statement forms (`(match (...){...});`, `(if (...){...} else {...});`) to avoid reparsing as statement nodes with different semantics.
- Added parser/type/smoke regression coverage for `if` expression branches containing `match (...) { ... };` and lambda-valued `if` expression branches compiled through full codegen.
- Fixed `match` expression runtime codegen to evaluate arm predicates correctly (literal/variant/wildcard) instead of falling back to wildcard/default behavior.
- Fixed `match` expression runtime codegen for exhaustive non-wildcard matches (e.g. booleans) to return the selected arm value instead of a constant zero fallback.
- Fixed import checker traversal gaps for expression forms:
  - `if` expressions now validate condition and branch expression calls,
  - `require(...)` now validates calls in both condition and message expressions.
- Fixed duplicate typechecker diagnostics in `if`/`match` expressions where expression statements were previously type-checked twice in the same branch/arm.
- Fixed lint duplicate-import detection to account for aliases (`path + alias` identity), preventing false `L001` on distinct alias imports.
- Fixed `apex fix` import cleanup to keep imports with trailing inline comments (`import ...; // comment`) instead of dropping them from rewritten output.
- Fixed formatter shebang handling so `apex fmt` preserves `#!/usr/bin/env apex` script headers.
- Fixed parser call/construction disambiguation for uppercase identifiers:
  - uppercase function calls such as `Foo()` now parse as function calls when `Foo` is a known function symbol (not forced constructor syntax).
- Fixed import checker traversal for `async { ... }` expression blocks so missing imports used inside async bodies are now reported.
- Fixed `apex fix` shebang handling (`#!/usr/bin/env apex`) so fix + format pipelines preserve script headers instead of failing lexing after import rewrites.
- Fixed `match` codegen literal comparison for `String` patterns in both statement and expression forms by using string-content comparison instead of integer-only fallback logic.
- Fixed `match` Option/Result binding type propagation in codegen:
  - `Some(v)` / `Ok(v)` / `Error(e)` bindings now inherit the real payload type instead of hardcoded `Integer`/`String`,
  - removes invalid LLVM IR on boolean payload matches (`br i64`, phi type mismatch).
- Fixed borrow checker async capture lifetime handling:
  - non-moved captures used inside `async` blocks are now treated as active borrows after async block creation,
  - subsequent moves/assignments on captured values are correctly rejected while borrow is active.
- Fixed parser generic/call handling:
  - forward uppercase function calls (`Foo()`) are now parsed as function calls instead of constructor expressions when `Foo` resolves to a function symbol,
  - explicit generic function call syntax (`id<Integer>(...)`) now parses correctly.
- Extended pattern parser support to include float literals, char literals, and negative integer literals in `match` patterns.
- Fixed borrow checker async mutable-capture handling to keep mutable borrow state (not downgraded to immutable), so later `&x`/`&mut x` operations now report correct mutable-borrow conflicts.
- Fixed `apex fix` import rewrite to preserve imports with trailing block comments (`import x; /* ... */`) instead of dropping required imports.
  - parser now emits an explicit error (`Visibility modifiers are not supported on constructors/destructors`) instead of accepting misleading syntax.
- Fixed parser expression coverage by adding `if (...) { ... } else { ... }` expression parsing (`Expr::IfExpr`) in expression contexts.
- `if` expression parsing now supports both forms:
  - with `else` (value-producing branches),
  - and without `else` (valid expression form typed as `None`).
- Fixed borrow checker control-flow handling for constant boolean branches:
  - short-circuit move analysis now skips unreachable RHS for `true || ...` and `false && ...`,
  - constant `if`/`while(false)` branches are treated as unreachable in borrow analysis where applicable,
  - constant `if` with early termination now prevents false-positive analysis of unreachable following statements.
- Fixed mutating-method inference to respect short-circuit constants when scanning method bodies (`true || this.mutating_call()` no longer marks method as mutating due to unreachable RHS).
- Fixed type checker visibility diagnostic spans by replacing synthetic `0..0` spans with declaration-context spans for interface/function/class signature checks.
- Fixed `match` expression type safety:
  - arm result types are now validated for compatibility,
  - non-exhaustive `match` expressions are now rejected for `Boolean`, `Option<T>`, and `Result<T, E>` unless a catch-all arm exists.
- Fixed project config compatibility for `apex.toml`:
  - `ProjectConfig::load` now supports both flat-key format and `[project]` table format.
- Fixed class visibility enforcement gaps:
  - `private`/`protected` class types are now validated at construction sites and variable type declarations.
  - external construction of private classes (for example `Secret()`) is now rejected.
  - class visibility is now also enforced in function/method signatures and inheritance (`extends`) checks.
- Fixed import parser inconsistency:
  - wildcard imports can no longer be combined with aliases (`import x.* as y` now errors explicitly instead of being accepted and failing later in import checking).
- Added import checker regression tests for local-vs-stdlib shadowing and `Math.*` import enforcement paths.
- Added import checker regression test for aliased stdlib module calls (`std.io`/`std.math`/`std.string` alias flow).
- Added import checker regression test for nested-call validation under aliased stdlib callees.
- Added CLI smoke regression coverage for project-level `apex check` catching cross-file type errors.
- Added CLI smoke regression coverage for project-mode stdlib alias imports (`io.println`, `math.abs`) in `apex check`.
- Added type checker regression test for local-variable shadowing over stdlib import aliases.
- Expanded CLI smoke regression suite with:
  - explicit historical regression scenarios (constructor validation, borrow-state consistency, stdlib import enforcement, local shadowing, project alias handling, cross-file type checks),
  - and an automated batch of 100 generated `apex check` cases (pass/fail mix) for broader real-world syntax/type/import/borrow coverage.
- Added borrow checker regression tests for:
  - use-after-move
  - move while borrowed
  - double mutable borrow
  - scope-exit borrow release movability
  - lambda capture move behavior
  - compound assignment on borrowed variable
  - assignment through borrowed owner (`obj.field += ...`)
  - `this` receiver method param-mode lookup
  - stdlib alias call borrow semantics in single-file mode (`io.println(s)` keeps `s` movable after the call)
  - reference-return borrow lifetime retention
  - temporary receiver borrow behavior across repeated mutating calls
  - nested receiver borrow conflict detection (`a.b.touch()` while `a` is borrowed)
- Added type checker regression tests for built-in generic constructor validation (invalid List/Map/Set args + valid constructor paths).
- CI smoke checks now include borrow-checker edge-case expected-fail/expected-pass scenarios.
- CI smoke checks now include compile-time regression coverage for nested field assignment (`a.b.v += ...`) and list index codegen assignment paths.
- Removed duplicate Vercel routing config from `web/public/vercel.json`; `web/vercel.json` is now the only deploy config.
- Removed machine-specific LLVM/linker paths from `.cargo/config.toml`.
- `apex new` now scaffolds `src/main.apex` with the required `import std.io.*;`, so a fresh project checks and runs immediately.
- Added regression coverage for shebang tokenization, lint import rules, and project linker config parsing.
- Fixed `apex new` default config paths:
  - `entry` is now `src/main.apex`
  - `files` includes `src/main.apex`
- Fixed test runner generation:
  - generated hook/test invocations now call functions (`testName();`) instead of expression statements (`testName;`)
  - `main()` stripping logic no longer over-consumes source after the main function.
  - generated test runner sources now inject `import std.io.*;` when needed.
- Fixed `apex test --filter` summary counters:
  - `total`/`ignored` now reflect the filtered subset, not the unfiltered discovery set.
- Fixed previously accepted invalid access to `private` and `protected` members from outside class boundaries.
- Fixed stale docs wording around class visibility defaults and interface implementation behavior.

## [1.3.4] - Async Runtime, FFI, Bindgen - 2026-03-05

### ✨ Added

- **Threaded Async Runtime**: `Task<T>` now uses a real thread-backed runtime with result caching.
  - `async function` calls now spawn task workers immediately via `pthread`.
  - `await` now joins unfinished tasks and reuses the stored result on subsequent awaits.
  - `async { ... }` blocks now compile to real task runners with captured environments.
- **Task Control APIs**: Added `Task.is_done()`, `Task.cancel()`, and `Task.await_timeout(ms)` runtime methods.
  - `await_timeout(ms)` returns `Option<T>` and performs a timed join.
  - `cancel()` marks task as done and provides a safe default result for later `await`.
- **Effect System**: Added function effect attributes `@Pure`, `@Io`, `@Net`, `@Alloc`, `@Unsafe`, `@Thread`, and `@Any` with call-site checks.
  - Built-in effect requirements are enforced for IO/thread-sensitive APIs.
  - Calls to user functions/methods now propagate declared effects.
  - Non-annotated functions now get effect inference from the call graph.
  - Invalid combinations (`@Pure` + explicit effects / `@Pure` + `@Any`) now fail at type-check time.
- **C Interop**: Added `extern function ...;` declarations for C ABI calls.
  - Supports typed extern signatures without function bodies.
  - Supports variadic extern declarations (e.g. `extern function printf(fmt: String, ...): Integer;`).
  - Supports explicit ABI and symbol aliasing (e.g. `extern(c, "puts") function c_puts(...): Integer;`, `extern(system, "printf") ...`).
  - Enforces FFI-safe extern signatures and variadic argument types at type-check time.
  - Extern callsites now use C ABI argument lowering (no Apex env pointer).
- **Pointer Interop Type**: Added generic `Ptr<T>` as a first-class type for raw FFI pointer signatures.
  - Parser/typechecker/codegen support for `Ptr<T>` declarations and extern interop.
  - `Ptr<T>` is now accepted as an FFI-safe extern signature type.
- **C Header Bindings**: Added CLI command `apex bindgen` to generate Apex `extern(c)` declarations from `.h` files.
  - Supports common C prototypes and variadic signatures.
  - Supports stdout output or `--output <file>` generation.
- **New Feature Examples**:
  - `examples/26_effect_system.apex`
  - `examples/27_extern_c_interop.apex`
  - `examples/28_async_runtime_control.apex`
  - `examples/29_effect_inference_and_any.apex`
  - `examples/30_extern_variadic_printf.apex`
  - `examples/31_extern_abi_link_name.apex`
  - `examples/32_extern_safe_wrapper.apex`
  - `examples/33_extern_ptr_types.apex`
  - `examples/34_bindgen_workflow.apex`
  - `examples/README.md` (coverage index)

### ♻️ Changed

- **Codegen Task Representation**: `Task<T>` codegen now uses an internal runtime task handle instead of the previous direct `T` value stub.
- **Async Documentation**: Updated async docs to reflect real runtime behavior (thread-backed scheduling, parallel task execution, await-as-join, cached result).
- **Function Documentation**: Added docs for `extern function` and effect attributes.
- **Safety of unwrap**: `Option.unwrap()` and `Result.unwrap()` now emit runtime panic+exit on invalid states instead of unchecked loads.

## [1.3.3] - Compiler/LSP/Docs Sync - 2026-03-05

### ✨ Added

- **Module Dot Syntax**: Added semantic/codegen support for `Module.function(...)` while keeping `Module__function(...)` compatibility.
- **Enum Variant Constructors**: Added enum metadata and constructor support for `Enum.Variant(...)` in codegen/type checking.
- **LSP Diagnostics on Edit**: Added diagnostics publication on open/change (lexer/parser errors) and baseline go-to-definition symbol lookup.

- **Integration Test Coverage**: Added tests for multi-file project rewrite edge cases:
  - Shadowed local function identifiers are not mangled.
  - Imported class constructor and module field accesses are mangled deterministically.
  - Shadowed module identifiers are not mangled.

### ♻️ Changed

- **Match Codegen**: Expanded to support enum-style variant dispatch and payload binding flows.
- **Map Methods**: `set`, `insert`, `get`, and `contains` now use functional linear-lookup behavior with update semantics.
- **Project Build Pipeline**: Now combines parsed AST declarations directly instead of text-merging source strings.
- **Docs Mirroring**: Full docs mirror is maintained under `web/public/docs/` from `docs/`.
- **Scope-Aware Rewriting**: Project call rewriting now preserves params, locals, loop vars, lambda params, and match bindings.
- **Architecture Docs**: Project documentation now covers AST combining, deterministic mangling, scope-aware behavior, and collision policy.
- **Native Clang Tuning**: Final IR compilation now prefers `-march=native -mtune=native` with a safe fallback to baseline `-O3` if tuned flags are unavailable.
- **Project `opt_level` Wiring**: `apex.toml` `opt_level` now actually drives final Clang optimization level (`0/1/2/3/s/z/fast`). Missing/invalid values default safely to maximum-performance `-O3`.

### 🐛 Fixed

- **Namespace Collisions**: Collision handling now fails early with clear function+namespace diagnostics.
- **Documentation Consistency**: Updated `apex` CLI usage, module syntax notes, and compiler architecture file map.
- **Class/Module Collisions**: Top-level class and module name collisions now fail early across namespaces.
- **List Capacity Growth**: Fixed `List.push()` codegen to grow backing storage with `realloc` when `length >= capacity`, preventing heap corruption (`malloc(): corrupted top size`) in large workloads like `benchmark/apex/matrix_mul.apex`.
- **Map IR Block Ordering**: Fixed invalid LLVM IR generation in `Map.set()` control-flow block ordering (late-created `map_set.cont/update`), which caused Clang parse failures in `examples/17_comprehensive.apex`.

## [1.3.2] - Range Types - 2026-02-22

### ✨ New Features

- **Range Type**: Full iterator-based range type for numeric sequences
  - `Range<T>` generic type with `range(start, end)` and `range(start, end, step)` functions
  - Iterator protocol with `has_next()` and `next()` methods
  - Support for ascending and descending ranges (negative steps)
  - LLVM struct-based implementation with heap allocation
  - New example: `examples/25_range_types.apex`
  - Documentation: `docs/features/ranges.md`

- **Testing Framework**: Full testing framework with attributes and assertions
  - `@Test` attribute to mark test functions
  - `@Ignore` attribute to skip tests (with optional reason: `@Ignore("not ready")`)
  - `@Before`, `@After` for setup/teardown around each test
  - `@BeforeAll`, `@AfterAll` for suite-level setup/teardown
  - New CLI command: `apex test` - Discover and run all @Test functions
  - Assertion functions: `assert()`, `assert_eq()`, `assert_ne()`, `assert_true()`, `assert_false()`, `fail()`
  - New example: `examples/24_test_attributes.apex`

- **LSP (Language Server Protocol)**: Apex now has a built-in LSP server for IDE integration
  - New CLI command: `apex lsp` - Start the language server
  - Autocomplete support for keywords, types, and functions
  - Hover documentation for language keywords
  - Go to definition support (prepared)

- **Improved Error Messages**: Better developer experience with helpful error messages
  - "Did you mean?" suggestions for typos using Levenshtein distance
  - Contextual hints for missing imports
  - Color-coded error output with source location

### 🔧 Technical

- **Range Type Implementation**:
  - Added `Type::Range(Box<Type>)` to AST
  - Added `ResolvedType::Range(Box<ResolvedType>)` to typeck
  - Parser support for `Range<T>` syntax
  - LLVM codegen: struct `{ i64, i64, i64, i64 }` (start, end, step, current)
  - `create_range()`, `range_has_next()`, `range_next()` helper functions
  - `compile_range_method()` for method calls

- Added `test_runner.rs` module for test discovery and execution
- Added `Attribute` enum to AST for function annotations
- Added `@` token to lexer
- Updated parser to parse attributes before function declarations
- Added parser unit tests for attribute parsing
- Added assert functions to codegen with proper LLVM generation
- Added `lsp.rs` module with tower-lsp integration
- Added fuzzy string matching for error suggestions
- Updated `import_check.rs` with suggestion engine

## [1.3.1] - Import System Fixes - 2026-02-22

### 🐛 Bug Fixes

- **Wildcard Imports for Stdlib**: Fixed wildcard imports (`import std.io.*;`) not properly importing stdlib functions like `println`, `print`.
- **Duplicate Import Check**: Fixed duplicate import validation in multi-file projects where imports were checked twice (once during analysis, once during compilation).
- **Examples Updated**: Added missing `import std.io.*;` and `import std.string.*;` statements to multi-file project examples.

### 🔧 Technical

- `import_check.rs`: Added stdlib function resolution for wildcard imports
- `main.rs`: Skip redundant import check during compilation phase (already done in analysis phase)
- All examples now properly import required modules
- Full `cargo fmt` and `cargo clippy` compliance

## [1.3.0] - Multi-File & Namespace System - 2026-02-21

### 🚀 Major Release: Complete Project System

This release introduces a complete multi-file project system with Java-style namespaces and mandatory imports.

### ✨ New Features

- **Multi-File Project Support**: Apex now supports organizing code into projects with multiple source files.
  - Project configuration via `apex.toml`
  - New CLI commands: `apex new`, `apex build`, `apex run`, `apex info`
  - Automatic merging and compilation of multiple source files
  - Entry point configuration for main function location
  
- **Project Commands**:
  - `apex new <name>` - Create a new project with standard structure
  - `apex build` - Build current project
  - `apex run` - Build and run current project  
  - `apex info` - Display project information
  - `apex check [file]` - Check project or specific file

### 📁 Configuration

- **apex.toml Format**:
  ```toml
  name = "my_project"
  version = "1.0.0"
  entry = "src/main.apex"
  files = ["src/utils.apex", "src/main.apex"]
  output = "my_project"
  opt_level = "3"
  ```

### 📚 Documentation

- Added comprehensive multi-file project documentation
- New example: `examples/multi_file_project/`
- Updated test suite to include multi-file project testing

### 🔧 Technical

- Added `namespace.rs` - Namespace resolution system
- Added `import_check.rs` - Import validation with helpful error messages
- Added `project.rs` - Project configuration management
- Updated lexer with `Package`, `Import`, `Star` tokens
- Updated parser for package/import syntax
- CI workflow tests all 32 examples including multi-file projects
- Full clippy compliance, cargo fmt applied

### 🐛 Behavior Changes

- **BREAKING**: Functions from other files are **NOT** automatically available
- Must use `import namespace.function;` or `import namespace.*;`
- Same-namespace functions work without imports (local scope)
- Functions without package declaration are in `global` namespace

## [1.2.0] - 2026-02-21

### 🚀 Performance & Optimization

- **LLVM Aggressive Optimizations**: Switched from `OptimizationLevel::Default` to `OptimizationLevel::Aggressive` for maximum performance.
- **Native CPU Targeting**: Changed from generic CPU to `native` with `+avx2,+fma` features for host-specific optimizations.
- **Function Attributes**: Added optimization attributes:
  - `alwaysinline` for small functions (≤3 params)
  - `nounwind` for exception-free code
  - `willreturn` for functions guaranteed to return
- **Tail Call Optimization**: Enabled `set_tail_call(true)` on all function calls.
- **Loop Rotation**: Implemented loop rotation optimization for better branch prediction and reduced branching overhead.

### 📊 Benchmarks

- **Fibonacci(35)**: ~0.12s (comparable to C/Rust)
- **Prime Sieve**: ~0.08s (faster than C/Rust!)
- **Overall Speedup**: 3x faster than original implementation

### 🏗️ Code Refactoring

- **Modular Architecture**: Split monolithic `codegen.rs` (6666 lines) into focused modules:
  - `codegen/core.rs` (3876 lines): Main codegen logic
  - `codegen/types.rs` (1590 lines): Built-in type implementations
  - `codegen/util.rs` (1223 lines): Utilities and C library bindings
- **Cleaner Imports**: Removed all unused imports, clippy-clean with `-D warnings`.

### 🐛 Fixed

- **LLVM Attribute Errors**: Removed problematic attributes (`uwtable`, `call_convention`) causing Clang failures.
- **Code Formatting**: Applied `cargo fmt` across entire codebase.

## [1.1.4] - 2025-12-29

### ✨ Added

- **Args Module**: Introduced support for command-line arguments via the `Args` object.
  - `Args.count()`: Returns the number of arguments.
  - `Args.get(index)`: Retrieves a specific argument.
- **Str Module**: Introduced the `Str` static object for string manipulation (renamed from `String` to avoid type name collisions).
  - `Str.len(s)`: Get string length.
  - `Str.compare(a, b)`: Compare two strings.
  - `Str.concat(a, b)`: Concatenate two strings.
  - `Str.upper(s)`: Convert to uppercase (stub).
  - `Str.lower(s)`: Convert to lowercase.
  - `Str.trim(s)`: Remove leading/trailing whitespace.
  - `Str.contains(s, sub)`: Check if string contains substring.
  - `Str.startsWith(s, pre)`: Check if string starts with prefix.
  - `Str.endsWith(s, suf)`: Check if string ends with suffix.
- **System Module Improvements**:
  - `System.getenv(name)`: Get environment variables.
  - `System.shell(cmd)`: Run shell command (exit code).
  - `System.exec(cmd)`: Run shell command and capture stdout.
  - `System.cwd()`: Get current working directory.
  - `System.os()`: Get operating system name.
  - `System.exit(code)`: Terminate program with exit code.
- **Math Module Improvements**: Added `Math.pi()`, `Math.e()`, and `Math.random()`.
- **Time Module**: Added native support for time-related operations.
  - `Time.now(format)`: Returns formatted local time.
  - `Time.unix()`: Returns raw Unix timestamp.
  - `Time.sleep(ms)`: Suspends program execution.
- **List Improvements**:
  - `List.pop()`: Remove and return the last element.
- **New Examples**: Added `19_time.apex`, `20_system.apex`, `21_conversions.apex`, `22_args.apex`, `23_str_utils.apex`.

### ♻️ Changed

- **Math Unification**: All mathematical functions (sqrt, sin, abs, etc.) now require the `Math.` prefix for consistency and better namespacing.
- **Improved For Loops**: Loop ranges now support variables (e.g., `for (i in 0..count)`), allowing for dynamic iteration.
- **Standard Library Expansion**: Continued efforts to expand the builtin library capabilities.

### 🐛 Fixed

- **Boolean String Conversion**: `to_string(bool)` now correctly returns "true" or "false" instead of garbage values.

## [1.1.3] - 2025-12-29

### ✨ Added

- **File I/O Support**: Added native support for file system operations via the `File` static object.
  - `File.read(path)`: Reads entire file to String.
  - `File.write(path, content)`: Writes content to file.
  - `File.exists(path)`: Checks for file existence.
  - `File.delete(path)`: Deletes a file.
- **New Examples**: Added `18_file_io.apex` and `app_notes.apex` demonstrating file system interactions.
- **Test Infrastructure**: Added `test_examples.bat` for automated verification of all example programs.

### ♻️ Changed

- **Standard Library Ownership**: Relaxed borrow checker rules for standard library functions (`strlen`, `println`, etc.). These functions now borrow their arguments instead of consuming them, allowing variables to be reused after being printed or measured.
- **Compiler Intrinsics**: Optimized C binding generation for standard library calls in the LLVM backend.

### 🐛 Fixed

- **Borrow Checker**: Fixed a bug where standard library calls would incorrectly mark string variables as moved.

## [1.1.2] - 2025-12-28

### 🐛 Fixed

- **Critical Runtime Crash**: Fixed a bug where classes starting with "List" (e.g., `ListNode`) were incorrectly compiled as generic lists, causing stack corruption and runtime crashes.
- **List.set()**: Implemented missing `set(index, value)` method for `List<T>` in codegen.
- **Match Statements**: Fixed invalid LLVM IR generation (orphan blocks) for `match` statements.
- **Clippy Warnings**: Resolved `collapsible_match` and other lints in `codegen.rs`.

## [1.1.1] - 2025-12-27

### 🚀 Major Changes

- **Complete Documentation Refactor**: The documentation has been completely overhauled and moved to a dedicated `docs/` directory.
- **Simplified README**: `README.md` is now a clean entry point, linking to specific documentation sections.

### ✨ Added

- **New Documentation Structure**:
  - `docs/getting_started/`: Installation, Quick Start (Hello World), Editor Setup.
  - `docs/basics/`: Syntax, Variables, Types, Control Flow.
  - `docs/features/`: Functions, Classes, Interfaces, Enums, Modules.
  - `docs/advanced/`: Ownership, Generics, Async/Await, Error Handling, Memory Management.
  - `docs/stdlib/`: Standard Library Overview (Math, String).
  - `docs/compiler/`: CLI Reference, Architecture internals.
- **Changelog**: Added `changelogs.md` to track project history.

### ♻️ Changed

- **CONTRIBUTING.md**: Updated contribution guidelines to reflect the new project structure and documentation workflow.
- **README.md**: Removed monolithic content and replaced it with an organized index of links.
