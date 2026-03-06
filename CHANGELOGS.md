# Changelog

All notable changes to the Apex Programming Language Compiler will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### ✨ Added

- `apex fmt` command for formatting Apex source files.
  - Supports single-file, directory, and project-aware formatting.
  - Supports `--check` mode for CI.
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
- New language coverage examples:
  - `examples/35_visibility_enforcement.apex`
  - `examples/36_inheritance_extends.apex`
  - `examples/37_interfaces_contracts.apex`
  - `examples/38_import_aliases.apex`

### ♻️ Changed

- `apex fmt` now preserves source comments instead of refusing commented files.
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

### 🐛 Fixed

- Web docs routing now uses extensionless `/docs/...` URLs consistently in footer links and sitemap output.
- Markdown HTML rendered in the docs/changelog web UI is now sanitized before insertion.
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
