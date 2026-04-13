# Changelog

All notable changes to the Arden will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### 🐛 Fixed

- Corrected multiple platform ABI mismatches in codegen/runtime C interop declarations and call sites (`size_t`/`long`/`time_t`/`pthread_t`), and added LLVM IR regression coverage for libc signature emission.
- Fixed async-block return-type inference to honor explicit `return` statements, which resolves false `Type mismatch: expected None, got Integer` failures in `Task.await_timeout(...)` call chains.

## [1.3.8] - 2026-04-13

### ♻️ Changed

- Reworked native artifact production around explicit linker-only backends, so Arden now emits objects directly from LLVM, links Linux outputs through `mold`, and links macOS/Windows outputs through LLVM `lld`.
- Switched Cargo self-builds away from pinned `clang` wrappers toward dedicated linker wrapper scripts in `.cargo/config.toml`, and reused exact Windows builtins paths instead of rescanning LLVM on each link.
- Added broader cross-platform linker benchmark coverage in CI, including a dedicated workflow for cold/hot/incremental timing captures plus a quick main-CI benchmark pass that uploads fresh timing artifacts on Linux, macOS, and Windows.
- Switched portable Linux release and smoke builds back to a baseline `x86-64` CPU target instead of `target-cpu=native`, while keeping native-tuned benchmark and check flows fast on CI hosts.
- Unified `arden build --timings` output with the main CLI palette instead of using a separate ad-hoc cyan header path.
- Tightened terminal styling so `arden new`, timings, and the rest of the CLI keep a consistent color path, while using a safer Windows-specific rendering path than raw ANSI everywhere.

### 🐛 Fixed

- Follow-up hardening: semantic pipeline now surfaces source-read failures instead of silently dropping files, single-source diagnostics now use full file paths (parse/type/borrow/import/entry checks), list field-method codegen no longer swallows pointer-resolution errors, and macOS temp/symlink + linker fallback working-directory handling is more robust in perf/link flows.
- Borrow-mode runtime fixes: `borrow mut` parameters now lower as true by-reference ABI arguments (not by-value copies), so scalar/list/field mutations propagate back to the caller; call sites now pass borrow-mode args with pointer semantics consistently for direct/module/method/constructor paths, and async functions/methods now reject borrow-mode parameters up front.
- Runtime/codegen: fixed nested `List` crash (`SIGSEGV`) in `run/profile`, improved signal-based crash diagnostics, and hardened temporary object cleanup warnings.
- Caching/rebuild correctness: invalidated stale project/object caches when compiler binary identity changes; improved project-mode rebuild reliability.
- CLI/test tooling: fixed `run/bench/profile/test` temporary artifact handling (unique temp outputs, consistent cleanup, explicit cleanup warnings), improved `arden test` failure diagnostics, fixed project-mode test/bench/profile output path resolution (including Windows `.exe` cases), corrected `arden bindgen` status stream routing so generated stdout output is not polluted, and prevented duplicate `std.io` import injection in generated test-runner sources when inline block comments precede imports.
- Path safety/concurrency: rejected symlinked root directories during source/test discovery and made scoped cwd switching panic-safe with explicit error propagation.
- Linking/platform fixes: stabilized linker cwd/response-file cleanup handling, improved macOS SDK/protable bundle behavior, and fixed Windows `lld-link` machine/CRT/output-path issues.
- CI/tooling robustness: fixed benchmark workflow setup/output consistency and replaced panic-style build-path invariants with explicit user-facing errors/warnings.
- Frontend/runtime correctness: fixed checked/unchecked `Char` ordered comparisons affecting Windows drive-letter path checks, and removed a panic-only fallback in call-arity codegen diagnostics in favor of safe error-path handling.
- Attribute handling: reject misplaced/invalid attribute combinations (including duplicate attributes, `@Ignore` without `@Test`, and `@Test` mixed with lifecycle hooks), fail fast for invalid test-runner signatures (`@Test/@Before/...` on async/extern/parameterized/generic/non-`None` functions), reject duplicate lifecycle hooks per suite, keep module-scoped `main` intact while stripping only top-level entry `main` in generated test runners, enforce `@Any` call boundaries (pure/non-`@Any` callers now reject `@Any` callees), close effect-check bypasses through function-value variables, function-value contract copies (`g = f`), function-value flows through `if` expressions, function-value calls via callee expressions (`(if ... )()`), higher-order function arguments, and unknown function-contract factory results (`run(make())`) in explicit non-`@Any` callers (including `@Pure`), and broaden strict effect-checking regression coverage (including transitive net-effect rejection from explicitly `@Io` callers).
- Stdlib/imports: added an explicit `std.net` namespace placeholder to known stdlib namespaces so `import std.net.*;` is recognized even before concrete network API members are implemented.
- Effect contracts/tooling: closed additional effect-check bypasses for call-produced function values (`make()`) across assignment, direct invocation, `if`/`block` flows, and higher-order arguments via explicit unknown-contract diagnostics; propagated transitive `@Any` through inferred wrappers; fixed macOS example-smoke script portability (no `mapfile`), unblocked macOS `/var`/`/tmp` symlink-alias path validation in perf tests, and resolved strict clippy warning in import-check tests.
- Effect contracts: fixed additional bound-method function-value bypasses so effect policy is now enforced for `obj.method` values from variables, constructor receivers (`Type().method`), and call receivers (`factory().method`) in both direct invocation and higher-order argument paths.
- Effect contracts: expanded bound-method receiver inference for function values across `block`/`if`/`match` receiver forms and nested method-call receivers; interface-typed method values now carry conservative unknown contracts so pure/explicit non-`@Any` callers cannot bypass effect checks through interface indirection.
- Type/borrow diagnostics: generic enum declarations now fail fast with an explicit non-panic diagnostic (`user-defined generic enums are not supported yet`) instead of cascading `Unknown type: T`/codegen crashes, and `borrow mut` parameters now behave consistently as writable aliases inside callees (reads and local updates allowed) while immutable call-site bindings are still rejected.
- No-check/codegen hardening: `arden compile --no-check` no longer panics on unsupported generic-enum payload shapes (now returns a clear `Codegen error`), and mutable-by-mode parameter handling is now aligned across typecheck/borrowck/codegen for `borrow mut` parameters.
- No-check diagnostics: generic enum rejection now happens before specialization and reports the concrete enum name plus generic parameters (for clearer `--no-check` failures).
- CLI perf tests: single-file bench/profile cleanup checks now track actual temp binary prefixes in `$TMPDIR` instead of unrelated neighboring paths.
- CLI/testing/tooling: `profile` now uses its own temp-binary prefix (`arden-profile`), test-runner workspace paths are normalized/canonicalized more defensively (including `..` inputs), test-run failure diagnostics now include the runner executable path, and Linux smoke examples avoid `mapfile`-only Bash features.
- Project/test-workspace path safety: project source/output validation now canonicalizes and checks symlink-target containment more strictly, duplicate source aliases resolving to the same file are rejected, and generated test-runner workspace file paths are normalized to prevent `..`/entry-collision regressions.
- Project config IO: improved `arden.toml` read/write/parse diagnostics to always include the concrete file path and both supported parse shapes (`root` and `[project]`) for faster production triage.
- Source discovery hardening: made recursive `.arden` collection reject symlink entries inside traversed trees (explicit fail-fast instead of silent traversal skipping).
- Cache diagnostics: corrected env-override parse warning formatting for consistent log parsing.
- Test runner diagnostics: improved compile/run error messages for generated test runners by including concrete runner source and executable paths.
- Linker diagnostics: improved linker/ar failure output to always include actionable failure details (stderr/stdout/exit code fallback) instead of empty generic failures.
- Run/bench/profile diagnostics: improved process launch and non-zero-exit errors to include the exact executable path.
- CLI/test diagnostics: standardized file-IO errors across `check`, `debug`, `fmt/lint/fix`, test runner workspace setup, and single-file compile paths so failures include concrete file/directory paths.
- `arden new` diagnostics: creation failures for project/src/main/readme now include exact target paths; fixed minor test-discovery error-message formatting typo for cleaner logs.
- macOS linker diagnostics: `xcrun` SDK path/version resolution failures now include stderr/stdout/exit-code fallback details for faster environment triage.
- Config/path diagnostics: `ProjectConfig::save` serialization failures and temporary binary name generation failures now include concrete target/source paths.
- Linker/LSP diagnostics: static-library `ar` launch/failure messages now include target output path; LSP startup runtime failure now reports explicit LSP context.
- Project discovery diagnostics: `No arden.toml found` errors now include the concrete current working directory across `build/run/check/info/bench/profile` flows.
- Test parsing diagnostics: `arden test` now reports lexer/parser failures with full test-file paths for faster pinpointing in multi-file suites.
- Diagnostic hardening sweep (20+ micro-fixes): added richer source/target-path context across debug lexer errors, codegen failure reports, linker launch paths, bindgen boundary validation/output directory handling, and recursive test/source discovery error paths.
- Project validation diagnostics: improved entry/source/output validation errors with clearer resolved-path/project-root context and more explicit output-path ancestor resolution failures.
- Import-check diagnostics: when source rendering fails during import-error formatting, diagnostics now include the underlying file read error reason.
- Bindgen boundary safety: reject missing header paths and directory output targets explicitly, auto-create output parent directories for file outputs, and add regression coverage for the new guards.
- Release portable smoke: hardened `scripts/release/smoke_portable_unix.sh` with explicit archive/install-script existence checks, per-step timeout guard (`SMOKE_STEP_TIMEOUT_SECONDS`, default `600`), and stage logs to prevent silent multi-hour hangs.
- Web homepage hero: restored vertical centering on large `100vh` layouts by aligning the top hero grid to center instead of end.
- Codegen/no-check reliability and portability: restored root-cause undefined-root diagnostics for nested member chains/generic receiver calls, and switched libc `size_t` + pointer-fallback sizing away from hardcoded 64-bit assumptions.
- CI stability: fixed effect-attributes smoke examples (`@Any` caller boundaries), tightened test-file name detection to avoid false positives like `latest.arden`, and refined macOS symlink-ancestor handling so system temp aliases don't trip path guards.

## [1.3.7] - 2026-04-10

### ♻️ Changed

- Upgraded the LLVM integration to LLVM 22.1.x across Cargo, CI, release packaging, benchmark tooling, and install docs, including the move to `inkwell` `llvm22-1` and `LLVM_SYS_221_PREFIX`.
- Reworked function-value name resolution in the type checker to use a cached leaf-name index instead of repeatedly scanning every known function symbol, cutting large synthetic project cold-build time from roughly `52.7s` to `1.9s` while keeping 10-file body-only rebuilds around `0.7s` in the `--timings` benchmark flow.
- Expanded `arden build --timings` coverage and breakdowns for cold, warm, 10-file body-only rebuilds, larger mixed nominal-type stress projects, and hotter codegen subphases.
- Trimmed redundant scalar type validation in codegen `Assign` and `Return` hot paths, improving the 32k-function synthetic cold build from `1.079s` to `1.061s` in A/B `--timings` runs.
- Trimmed repeated call-path lookups and small argument-buffer reallocations in codegen so hot `expr_call` work scales better on larger mixed `interface`/`enum`/`class` synthetic projects.
- Reused per-namespace local enum sets during project rewrite instead of rebuilding them in recursive hot paths, dropping XL mixed synthetic rewrite worker time from about `2.36s` to `2.06s` in `--timings` runs.
- Cached mangled class symbol names for project rewrite call handling instead of rescanning every class on each rewritten call, cutting XL mixed synthetic rewrite worker time to about `0.15s` and cold build time from about `1.10s` to `0.96s`.
- Collapsed shard-local closure body symbol collection into a single pass per shard, cutting that object-codegen step from roughly `0.02s` to `0.003s` on the 32k-function synthetic cold build.
- Precomputed top-level declaration/body filter decisions once per codegen pass, cutting mixed-project filter overhead and improving synthetic cold builds to about `1.046s` on 32k functions and `1.068s` on the XL mixed stress project.
- Lowered the default large-project object-codegen shard size from `8` to `4`, improving LLVM 22 cold `--timings` runs to about `1.021s` on the 32k-functions stress project and `0.908s` on the XL mixed stress project while keeping warm and 10-file rebuilds near-instant.

## [1.3.6] - Compiler UX, Type/Codegen Correctness, and Project Reliability - 2026-04-08

### ✨ Added

- Added broader language and tooling coverage across formatter, lint/fix flows, benchmarking, import handling, and project rewrite paths.

### ♻️ Changed

- Consolidated exact-import, namespace-alias, and zero-argument stdlib alias handling so the same forms behave consistently across checked builds, unchecked builds, and multi-file project builds.
- Tightened unchecked codegen to fail at semantic boundaries earlier instead of letting invalid programs degrade into LLVM or Clang backend failures.
- Refined CLI output for `build`, `run`, `fmt`, `fix`, and `check` with consistent project-style status lines, build durations, and web-aligned terminal colors instead of the previous mix of ad-hoc cyan/green messages.
- Switched CLI build and timing summaries to seconds with enough precision for hot-cache runs, so near-instant builds no longer look absurdly tiny or rounded inconsistently in raw milliseconds.
- Polished developer-facing terminal UX again by aligning `arden lsp` lifecycle logs and the generated `arden test` runner output with the newer neutral CLI presentation instead of the older plain-text banners.
- Simplified `arden test` runtime output further by dropping runner banners and suite/summary noise in favor of plain per-test `ok` and `skip` lines with CLI-side coloring.
- Shortened and clarified docs around function/test returns and assertion helpers so examples match actual language behavior without over-explaining every small variant.

### 🐛 Fixed

- Fixed a broad set of unchecked codegen holes around type boundaries, including assignments, returns, branch joins, loop bindings, constructor/call arguments, function-value adapters, and container payload/key writes.
- Fixed unchecked builtin specialization mismatches for invariant containers and heap wrappers such as `List`, `Map`, `Box`, `Rc`, and `Arc`, so empty or pointer-shaped values with the wrong specialization no longer compile just because their lowered layout matches.
- Fixed unchecked explicit builtin constructor specialization mismatches for nested invariant containers such as `Option<List<T>>`, while keeping imported builtin variant aliases like `Present` and `Success` working normally.
- Fixed the silent-feeling `arden lsp` startup path by emitting basic lifecycle logs immediately, including an explicit waiting-for-handshake state, so local launches no longer appear dead.
- Fixed scope restoration across blocks, `if`, `match`, and `for`, eliminating branch-local binding leaks and several invalid-IR paths.
- Fixed contextual typing and adaptation for lambdas, enum-variant function values, exact-import constructor values, and interface-backed/bound method values.
- Fixed project rewrite and import-resolution edge cases across exact imports, wildcard imports, namespace aliases, nested modules, root aliases, builtin `Option`/`Result` constructors and patterns, and zero-argument stdlib aliases.
- Fixed stale-cache and rebuild correctness issues in project mode, including corrupted cache recovery, namespace-alias invalidation, and import-check/cache reuse across declaration signatures.
- Fixed unchecked and checked diagnostics for module-local nominal types and nested receiver failures so errors now report the real root cause with user-facing type names.
- Fixed remaining parser and import-check gaps around builtin `Option.None`, builtin aliases, and package/root-qualified constructor and pattern syntax.
- Fixed parser diagnostics so missing semicolons and other expectation errors now use human-readable token names, aligned source gutters, and insertion-point carets instead of internal token dumps or next-token-only blame, including identifier-led statements that previously lost their true start column.
- Fixed single-file import diagnostics so unresolved imports and alias issues now render with source context, identifier-accurate carets, and compact fix hints instead of a detached summary block.
- Fixed assorted project-mode regressions around generic inheritance, generic bounds, nested-module rewrites, current-package alias constructors, and extern/link-name preservation.
- Fixed test-runner output consistency so ignored tests, suite headings, and final summaries now read as one coherent UI instead of a mix of legacy banner styles.


## [1.3.5] - Tooling, Performance, and Build Reliability - 2026-03-08

### ✨ Added

- Added `arden fmt`, `arden lint`, `arden fix`, `arden bench`, and `arden profile`, plus new lint rules, compound-assignment parsing, richer examples, and broader CLI/frontend/CI smoke coverage.
- Added project linker/output configuration in `arden.toml`, compile target/optimization flags, and expanded benchmark coverage including hot/cold compile modes, incremental rebuild scenarios, and larger synthetic mega-graph workloads.

### ♻️ Changed

- Reworked project builds around `.ardencache`, parallel parse/import/rewrite/object stages, narrower rebuild slices, direct object emission, response-file linking, and stricter `lld`-only linking.
- Improved LSP rename/reference/hover accuracy, borrow-check/type-check metadata reuse, stdlib metadata sharing, async runtime portability, CI/release packaging, and project/build reporting.
- Expanded language semantics around inheritance, interface validation, import aliases, list construction/storage, shebang scripts, and project target/output handling.

### 🐛 Fixed

- Fixed import, alias, wildcard, and module-local scope leaks across parser, import-check, type-check, lint, autofix, and unchecked codegen paths.
- Fixed unchecked dispatch and diagnostics for interfaces, enums, constructor/function values, nested receivers, generic bounds, and module-local nominal types.
- Fixed argument/return lowering gaps for direct calls, module-qualified calls, wildcard imports, enum constructors, compound assignment, and unchecked return validation.
- Fixed project rewrite and cache invalidation bugs across generic bases, nested modules, namespace aliases, declaration signatures, stale object reuse, and duplicate import-check reporting.
- Fixed tooling/runtime issues including parser edge cases for compound assignment and builtin generics, lambda capture borrow handling, test-runner codegen, async timeout portability, web docs sanitization/routing, and release/workflow setup.


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
  - Extern callsites now use C ABI argument lowering (no Arden env pointer).
- **Pointer Interop Type**: Added generic `Ptr<T>` as a first-class type for raw FFI pointer signatures.
  - Parser/typechecker/codegen support for `Ptr<T>` declarations and extern interop.
  - `Ptr<T>` is now accepted as an FFI-safe extern signature type.
- **C Header Bindings**: Added CLI command `arden bindgen` to generate Arden `extern(c)` declarations from `.h` files.
  - Supports common C prototypes and variadic signatures.
  - Supports stdout output or `--output <file>` generation.
- **New Feature Examples**:
  - `examples/single_file/tooling_and_ffi/26_effect_system/26_effect_system.arden`
  - `examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden`
  - `examples/single_file/tooling_and_ffi/28_async_runtime_control/28_async_runtime_control.arden`
  - `examples/single_file/tooling_and_ffi/29_effect_inference_and_any/29_effect_inference_and_any.arden`
  - `examples/single_file/tooling_and_ffi/30_extern_variadic_printf/30_extern_variadic_printf.arden`
  - `examples/single_file/tooling_and_ffi/31_extern_abi_link_name/31_extern_abi_link_name.arden`
  - `examples/single_file/tooling_and_ffi/32_extern_safe_wrapper/32_extern_safe_wrapper.arden`
  - `examples/single_file/tooling_and_ffi/33_extern_ptr_types/33_extern_ptr_types.arden`
  - `examples/single_file/tooling_and_ffi/34_bindgen_workflow/34_bindgen_workflow.arden`
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
- **Project `opt_level` Wiring**: `arden.toml` `opt_level` now actually drives final Clang optimization level (`0/1/2/3/s/z/fast`). Missing/invalid values default safely to maximum-performance `-O3`.

### 🐛 Fixed

- **Namespace Collisions**: Collision handling now fails early with clear function+namespace diagnostics.
- **Documentation Consistency**: Updated `arden` CLI usage, module syntax notes, and compiler architecture file map.
- **Class/Module Collisions**: Top-level class and module name collisions now fail early across namespaces.
- **List Capacity Growth**: Fixed `List.push()` codegen to grow backing storage with `realloc` when `length >= capacity`, preventing heap corruption (`malloc(): corrupted top size`) in large workloads like `benchmark/arden/matrix_mul.arden`.
- **Map IR Block Ordering**: Fixed invalid LLVM IR generation in `Map.set()` control-flow block ordering (late-created `map_set.cont/update`), which caused Clang parse failures in `examples/single_file/safety_and_async/17_comprehensive/17_comprehensive.arden`.

## [1.3.2] - Range Types - 2026-02-22

### ✨ New Features

- **Range Type**: Full iterator-based range type for numeric sequences
  - `Range<T>` generic type with `range(start, end)` and `range(start, end, step)` functions
  - Iterator protocol with `has_next()` and `next()` methods
  - Support for ascending and descending ranges (negative steps)
  - LLVM struct-based implementation with heap allocation
  - New example: `examples/single_file/stdlib_and_system/25_range_types/25_range_types.arden`
  - Documentation: `docs/features/ranges.md`

- **Testing Framework**: Full testing framework with attributes and assertions
  - `@Test` attribute to mark test functions
  - `@Ignore` attribute to skip tests (with optional reason: `@Ignore("not ready")`)
  - `@Before`, `@After` for setup/teardown around each test
  - `@BeforeAll`, `@AfterAll` for suite-level setup/teardown
  - New CLI command: `arden test` - Discover and run all @Test functions
  - Assertion functions: `assert()`, `assert_eq()`, `assert_ne()`, `assert_true()`, `assert_false()`, `fail()`
  - New example: `examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden`

- **LSP (Language Server Protocol)**: Arden now has a built-in LSP server for IDE integration
  - New CLI command: `arden lsp` - Start the language server
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

- **Multi-File Project Support**: Arden now supports organizing code into projects with multiple source files.
  - Project configuration via `arden.toml`
  - New CLI commands: `arden new`, `arden build`, `arden run`, `arden info`
  - Automatic merging and compilation of multiple source files
  - Entry point configuration for main function location

- **Project Commands**:
  - `arden new <name>` - Create a new project with standard structure
  - `arden build` - Build current project
  - `arden run` - Build and run current project
  - `arden info` - Display project information
  - `arden check [file]` - Check project or specific file

### 📁 Configuration

- **arden.toml Format**:
  ```toml
  name = "my_project"
  version = "1.0.0"
  entry = "src/main.arden"
  files = ["src/utils.arden", "src/main.arden"]
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

## [1.2.0] - Performance Push & Codegen Refactor - 2026-02-21

Focused on making native output materially faster while breaking the original codegen monolith into modules that were easier to maintain and extend.

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

## [1.1.4] - Stdlib Expansion & Runtime Utilities - 2025-12-29

This release substantially widened the built-in runtime surface with arguments, time, string helpers, and system integration primitives.

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
- **New Examples**: Added `19_time.arden`, `20_system.arden`, `21_conversions.arden`, `22_args.arden`, `23_str_utils.arden`.

### ♻️ Changed

- **Math Unification**: All mathematical functions (sqrt, sin, abs, etc.) now require the `Math.` prefix for consistency and better namespacing.
- **Improved For Loops**: Loop ranges now support variables (e.g., `for (i in 0..count)`), allowing for dynamic iteration.
- **Standard Library Expansion**: Continued efforts to expand the builtin library capabilities.

### 🐛 Fixed

- **Boolean String Conversion**: `to_string(bool)` now correctly returns "true" or "false" instead of garbage values.

## [1.1.3] - File I/O & Example Coverage - 2025-12-29

The focus here was practical scripting support: file operations, better examples, and the first dedicated example verification flow.

### ✨ Added

- **File I/O Support**: Added native support for file system operations via the `File` static object.
  - `File.read(path)`: Reads entire file to String.
  - `File.write(path, content)`: Writes content to file.
  - `File.exists(path)`: Checks for file existence.
  - `File.delete(path)`: Deletes a file.
- **New Examples**: Added `18_file_io.arden` and `app_notes.arden` demonstrating file system interactions.
- **Test Infrastructure**: Added `test_examples.bat` for automated verification of all example programs.

### ♻️ Changed

- **Standard Library Ownership**: Relaxed borrow checker rules for standard library functions (`strlen`, `println`, etc.). These functions now borrow their arguments instead of consuming them, allowing variables to be reused after being printed or measured.
- **Compiler Intrinsics**: Optimized C binding generation for standard library calls in the LLVM backend.

### 🐛 Fixed

- **Borrow Checker**: Fixed a bug where standard library calls would incorrectly mark string variables as moved.

## [1.1.2] - Runtime Stability Fixes - 2025-12-28

This patch concentrated on correctness issues in generated LLVM and container behavior, especially around `List` handling and `match` lowering.

### 🐛 Fixed

- **Critical Runtime Crash**: Fixed a bug where classes starting with "List" (e.g., `ListNode`) were incorrectly compiled as generic lists, causing stack corruption and runtime crashes.
- **List.set()**: Implemented missing `set(index, value)` method for `List<T>` in codegen.
- **Match Statements**: Fixed invalid LLVM IR generation (orphan blocks) for `match` statements.
- **Clippy Warnings**: Resolved `collapsible_match` and other lints in `codegen.rs`.

## [1.1.1] - Documentation Rebuild - 2025-12-27

This release reorganized the project’s docs into a real documentation tree and turned the repository root into a cleaner entry point.

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
