# Compiler Architecture

This document describes the internal architecture of the Apex compiler.

## Pipeline

1. **Lexing** (`lexer.rs`): Source code is converted into a stream of tokens. String interpolation tokens are handled here.
2. **Parsing** (`parser.rs`): Recursive descent parser that builds an Abstract Syntax Tree (AST) from tokens.
3. **Type Checking** (`typeck.rs`): Traverses the AST to validate types, resolve names, and ensure type safety.
4. **Borrow Checking** (`borrowck.rs`): Analyses ownership and lifetimes to ensure memory safety without GC.
5. **Code Generation** (`codegen/core.rs`, `codegen/types.rs`, `codegen/util.rs`): Lowers the AST into LLVM IR (Intermediate Representation).
6. **Linking**: LLVM IR is compiled to object files and linked through Clang. Apex uses an explicit per-platform linker policy with no fallback: Linux requires `mold`, while macOS and Windows require LLVM `lld`. Build caches are fingerprinted with that enforced linker mode.

Project rewrite and semantic passes now normalize namespace-alias constructor/type paths for module-scoped classes as well. That keeps expressions like `u.Box<Integer>(...)` and `u.M.Box<Integer>(...)` aligned with the same prefixed owner symbols used by dependency indexing, typechecking, and filtered codegen.
Codegen now also keeps synthesized user-generic class specializations (`...__spec__...`) alive through filtered declaration passes and can infer object types from constructor results, function-returned objects, and `try`-unwrapped objects when lowering method/field chains.
That object-expression inference now also covers expression-valued `if`/`match` results and block tails, so direct field/method chains on those objects survive lowering instead of failing late in codegen. Parser-side constructor heuristics now also recognize explicit generic constructors through postfix parsing paths, which keeps `Boxed<Integer>(...)` constructor syntax stable even inside nested expression branches.
The same inference layer now also handles indexed list elements and method-return values from special container helpers like `Option.unwrap()` / `Result.unwrap()`, so chained object access keeps working even when the receiver is produced by a container/indexing expression rather than a named local.
Special-method dispatch for built-in container/runtime types now also accepts expression receivers instead of only locals, so `List`, `Map`, `Range`, `Option`, and `Result` methods can be called directly on returned values without first storing them in temporaries.
Boolean helper methods such as `Option.is_some()` and `Result.is_ok()` now lower to real `i1` conditions in LLVM, which keeps direct conditional use sound when those helpers are invoked on returned/container-produced values.
Task runtime semantic checks are also kept aligned with the implemented runtime surface, so stray undocumented methods are rejected during typechecking instead of leaking through to backend `Unknown Task method` failures.
Backend method dispatch now also wires `String.length()` directly for expression receivers, keeping string literals and other non-local string values aligned with the same method surface accepted by the typechecker.
That object-type inference now also treats string concatenation and interpolation as first-class `String` producers, so string method calls continue to work even when the receiver is not a named local or plain literal.
The same inference layer now also preserves value types returned from built-in container getters such as `List.get()` and `Map.get()`, which keeps object field/method chains on those returned values valid during lowering.
Built-in `Set<T>` lowering now also performs real membership/mutation work instead of placeholder boolean stubs, keeping `add`/`contains`/`remove` behavior aligned with the typechecked container API.
The same built-in method tables are now kept aligned between typechecking and codegen for `Set<T>` as well, preventing frontend/backend drift where a method existed in one layer but not the other.
Container runtime equality/storage now also handles non-scalar generic values correctly: tagged payloads like `Option<T>` and `Result<T, E>` are compared semantically in `Set<T>`, `Map<Option<T>, V>` / `Set<Option<T>>` preserve earlier inserted tagged keys across inserts and removals, `Map<Result<T, E>, V>` uses real ABI slot sizes for non-scalar keys, and `Map<K, V>` / `Set<T>` backing allocations use target-correct element sizes instead of assuming 8-byte slots.
Typed lowering now also carries expected `Option<T>` / `Result<T, E>` layouts into static constructors in codegen-sensitive contexts, so `Result.error(...)` and similar tagged constructors no longer emit mismatched LLVM structs when the surrounding type is known.
Collection equality now also handles pointer-backed user object values by identity, which keeps `Map<Class, V>`, `Set<Class>`, and nested tagged keys like `Map<Option<Class>, V>` consistent with the rest of the runtime object model.
Enum construction now also zero-initializes inactive payload slots before inserting the active variant payload, preventing `undef` bytes from leaking into enum equality/storage paths such as `Map<Enum, V>` and `Set<Enum>`.
The current enum payload runtime model still stores payload slots as `i64`, so nested enum-by-value payloads are now rejected explicitly during typechecking instead of reaching backend codegen and failing there.
Built-in collection getters/mutators now also fail fast on invalid runtime access (`List.get/set/pop` bounds errors and missing `Map.get()` keys) instead of returning null/garbage values and crashing later during field or method lowering.
The same fail-fast bounds checks now also apply to direct list indexing syntax (`xs[i]`), so bracket access no longer bypasses the guarded collection helper path.
Binary equality lowering now also routes pointer-backed runtime values such as `List`, `Map`, and user class instances through the same shared value-equality helper used by collection internals, keeping identity-equality codegen aligned with container lookup semantics instead of failing late in backend binary-op lowering.
Codegen-side expression inference now also preserves object-valued built-in method results like `Option.unwrap()`, `Map.get()`, and `Task.await_timeout(...).unwrap()`, which keeps downstream equality and field/method chains aligned with the actual returned object types instead of degrading them to generic integer placeholders.
The same inference layer now also recognizes direct built-in constructor/function call receivers such as `Option.some(...)`, `Result.ok(...)`, and `range(...)`, so immediate method chains on those expressions survive lowering instead of failing as receiver-type inference gaps.
Direct static `Result.ok/error` constructor lowering now also uses the inferred expression type when no explicit expected type is available, which keeps untyped `Result.error(...)` expressions ABI-consistent with later equality/method use instead of falling back to placeholder payload layouts.
That same codegen-side inference now also recognizes plain constructor expressions as real class values instead of defaulting them to integers, which keeps direct object payload chains like `Option.some(Boxed(...)).unwrap().value` and `Result.ok(Boxed(...)).unwrap().value` type-stable through lowering.
Dependency/reference scanning now also records method symbols used on direct constructor receivers, so filtered project codegen can emit the required method bodies for expressions like `Boxed(23).get()` instead of linking against missing class-method symbols.
Task timeout handling now also validates that `await_timeout(ms)` receives a non-negative timeout before entering the polling loop, preventing negative integers from turning into effectively unbounded waits through unsigned loop arithmetic.
Runtime unwrap failure diagnostics now emit real newline-terminated panic messages for `Option.unwrap()` and `Result.unwrap()` instead of embedding escaped `\\n` text in stdout.

## Build Caching

- **Project fingerprint cache** (`.apexcache/build_fingerprint`):
  - Hashes project config + source metadata + build-mode flags.
  - If unchanged and output artifact exists, `apex build` exits early (`Up to date ...`).
- **Parsed file cache** (`.apexcache/parsed/*.json`):
  - Stores parsed AST + namespace/import metadata keyed by source fingerprint.
  - On incremental edits, unchanged files bypass tokenization/parsing and reuse cached AST.
  - Cached parse entries now also persist extracted symbol/reference metadata (`function_names`, dependency references, qualified symbol paths, import-check fingerprint), so warm builds do not rewalk unchanged ASTs just to rebuild compiler bookkeeping.
  - Nested module declarations now contribute prefixed class/enum metadata alongside function metadata, so project rewrite and dependency/index data can resolve module-scoped types like `M__Box` consistently.
  - Uses a fast unchanged-file check from cached file metadata (`len + modified time`) before reading full file contents.
  - If metadata changed but file content hash is still identical, the cached parse result is still reused safely.
- **Rewritten file cache** (`.apexcache/rewritten/*.json`):
  - Stores namespace-rewritten AST fragments keyed by semantic fingerprint + per-file rewrite-context fingerprint.
  - On incremental edits, files whose semantics and relevant namespace/import context did not change bypass rewrite and are stitched directly into combined AST.
- **Import-check cache** (`.apexcache/import_check/*.json`):
  - Stores successful import-check results keyed by a narrower import/reference fingerprint plus per-file import/rewrite context fingerprint.
  - Rewrite/import context now prefers exact owner-file API fingerprints for actually used imported symbols instead of hashing whole namespaces whenever that can be resolved safely.
  - Same-namespace symbol usage now also hashes exact owner-file API fingerprints instead of pessimistically hashing the whole current namespace.
  - Body-only rewrites that do not change imports or referenced symbols can now reuse import-check results instead of invalidating on every semantic-body fingerprint change.
  - Namespace and wildcard imports fall back to namespace fingerprints only when Apex cannot narrow the dependency to exact owner files.
  - Fingerprint inputs are serialized in deterministic sorted order so hot rebuild reuse is stable across runs instead of depending on hash iteration order.
  - Unchanged files can now skip repeated import-check traversal on hot rebuilds with fewer false invalidations from unrelated namespace churn.
- **Dependency graph cache** (`.apexcache/dependency_graph/latest.json`):
  - Stores per-file semantic/API fingerprints plus direct file dependencies resolved from same-namespace access and explicit imports.
  - Same-namespace edges are derived from AST symbol references (calls, constructions, type references, module roots) instead of treating every file in a namespace as mutually dependent.
  - Wildcard imports and namespace aliases now try to resolve only the owner files of actually used imported symbols instead of depending on every file in the imported namespace by default.
  - Supports explicit `body-only` vs `API` change classification and reverse-dependent impact tracking between builds.
- **Semantic summary cache** (`.apexcache/semantic_summary/latest.json`):
  - Stores inferred function effect summaries and class mutating-method summaries from successful semantic passes.
  - Stores both per-file ownership metadata and per-component summary membership.
  - Unchanged files can seed impacted-file type/borrow checking without re-walking all unaffected bodies.
  - Entire unchanged dependency-graph components can now skip type checking and borrow checking even when some other project component changed in the same build.
- **Object file cache** (`.apexcache/objects/*.{o|obj}` + `*.json`):
  - Stores per-file compiled objects keyed by semantic fingerprint + per-file rewrite-context fingerprint + build options (`opt_level`, `target`, compiler version, linker mode).
  - On incremental edits, unchanged files reuse cached object files and final build performs fast relink from cached + rebuilt objects.
  - Object cache misses now emit object files directly from LLVM target machines, avoiding the old textual IR `.ll` -> `clang -c` round-trip.
  - LLVM target registries for direct object emission are initialized once per process, so parallel object rebuilds do not repeatedly pay startup cost.
- **Link manifest cache** (`.apexcache/link/latest.json`):
  - Records the ordered object input list plus final link configuration for the last successful build.
  - If a rebuild produces zero object cache misses and the manifest still matches, Apex skips the final linker invocation entirely and reuses the existing output artifact.
  - When linking does run, Apex now passes large object input sets through a response file to keep Clang + linker startup overhead bounded on large projects.
- **Semantic build fingerprint cache** (`.apexcache/semantic_build_fingerprint`):
  - Hashes canonicalized AST content instead of raw file text.
  - Comment-only / whitespace-only edits can now stop after parse/cache validation and return `Up to date ... (semantic cache)` without object rebuild or relink.
- **Per-file rewrite invalidation**:
  - Rewrite/object cache invalidation now hashes only the current file's namespace/import context and relevant imported namespace symbol maps.
  - Specific imports (`import lib.foo;`) additionally track owner-file API fingerprints, so unrelated API changes in the same namespace no longer fan out to those files.
  - Same-namespace references now track owner-file API fingerprints directly, which prevents wide `global` namespaces from invalidating rewrite/object caches project-wide after a small API edit.
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
- **Declaration-closure pruning for filtered codegen**:
  - Per-file object rebuilds no longer blanket-declare every symbol from the slim codegen program.
  - Apex now computes a declaration closure from the changed file's active symbols plus transitive API-visible references of dependency files, so filtered codegen only predeclares symbols that can actually be reached.
  - Dependency API projection inputs are trimmed to that same closure, so object-miss codegen also stops carrying unrelated stub declarations through the front of the pipeline.
  - Qualified import paths used through namespace aliases (for example `import util as u; f = u.add1`) now seed that closure too, so imported function values and alias-qualified calls pull in the right owner declarations during filtered object rebuilds.
  - Alias-qualified class/module references (for example `u.Box(...)`) now seed dependency edges and declaration closure entries too, so constructor/object codegen sees the owning type declarations instead of treating alias-rooted constructors as isolated files.
  - Filtered object emission now also activates closure-discovered body symbols that belong to the rebuilt source file itself, so direct-constructor receiver calls such as `Boxed(...).get()` emit the required methods without duplicating imported dependency bodies in the caller object.
  - Project rewrite now also normalizes local qualified nested-module expression paths, so local forms like `M.E.A(...)`, `match (...) { M.E.A(v) => ... }`, and module-body references like `M.mk()` / `M.Box(...)` survive the second module-local rewrite pass without leaving stale `app__M.*` chains behind.
  - Nested module declarations are now rewritten recursively with the local symbol set of the current nested module, so deeper chains like `M.N.mk()` and `await(M.N.mk()).value` preserve the correct single `M__N__...` prefix instead of either skipping rewrite or doubling the nested module segment.
- **Impacted semantic view**:
  - Type checking and borrow checking now run with full bodies only for changed files and real API dependents.
  - Unchanged unaffected files participate through API projections plus cached semantic summaries.
  - Unchanged connected components can now bypass semantic passes entirely when their component fingerprints still match the previous successful build.
- **Lazy full-program assembly**:
  - Normal object-link builds no longer materialize the full combined rewritten AST up front.
  - Full-project merged AST construction is deferred to the `emit_llvm` path that actually needs a single monolithic IR module.
- **Parallel project parse phase**:
  - Multi-file project parsing now runs in parallel workers (file read + lex + parse/cache lookup).
  - Import checks and rewrite/cache resolution run in parallel per file.
  - Symbol map/collision resolution and final declaration merge still run deterministically.
- **Codegen relevant-file pruning**:
  - Per-file object rebuilds now feed LLVM only the files reached by the declaration closure walk, not the full transitive dependency file closure.
  - This keeps object-miss program assembly closer to the set of declarations actually needed for the rebuilt unit.

## Recent Correctness Hardening

- **Scope-aware LSP rename/references**:
  - Symbol rename/reference resolution now follows lexical bindings selected at cursor position.
  - Prevents accidental edits of unrelated same-name symbols in nested/outer scopes.
  - Pattern-bound names in `match` arms now enter the scoped binding table too, so rename/references on `Some(v)` / `Error(err)` resolve the real arm-local binding instead of falling back to plain text matching.
- **Precise LSP hover token targeting**:
  - Hover docs are now resolved from the exact token under cursor, not from broad line substring checks.
 - **Nested declaration lookup in LSP**:
  - Go-to-definition now recursively searches nested modules for classes, enums, interfaces, and functions instead of stopping at one module layer or only seeing nested free functions.
- **If-expression parsing in expression positions**:
  - Parser now supports `if (...) { ... } else { ... }` as `Expr::IfExpr` where an expression is expected.
  - `if (...) { ... }` without `else` remains valid and is `None`-typed in type analysis.
- **Borrow checker constant-branch flow pruning**:
  - Unreachable RHS of `true || ...` and `false && ...` is no longer analyzed for move/borrow effects.
  - Constant `if` and `while(false)` paths are handled as unreachable in borrow analysis where possible.
  - Constant `if` with early termination no longer triggers false-positive downstream move/use errors.
  - Borrowed constructor parameters now initialize with the same borrowed state as ordinary function parameters, so constructors do not accidentally treat `borrow` / `borrow mut` inputs as owned values.
  - Nested module function calls now retain full mangled owner prefixes in borrow-mode lookup, so calls like `Outer.Inner.keep(s)` resolve the declared borrow signature instead of silently defaulting to move semantics.
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
  - Invalid namespace alias direct calls like `alias()` now fail during import-check instead of surviving until later undefined-name passes.
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
  - Recursive project rewrite now carries the full nested-module prefix into deeper module descendants, so nested local class/type references inside `A.B.C` bodies no longer collapse back to `A.B`.
- **Generic call safety hardening**:
  - Codegen now performs on-demand specialization for explicit generic free/module calls (for example `id<T>(...)`, `A.X.id<T>(...)`) and rewrites call sites to concrete specialized symbols.
  - Project filtered compilation now emits dependency-owned generated class specializations only from the owning object while still allowing call-site generic method specializations to lower in the consumer object, avoiding both duplicate-symbol and missing-symbol linker regressions.
- **Shared stdlib registry**:
  - Compiler stages now reuse a single lazy-initialized stdlib registry (`OnceLock`) instead of repeatedly constructing stdlib lookup maps during hot-path analysis and lowering.
- **Lint scope analysis hardening**:
  - Shadowing diagnostics now account for parameters and `for`-loop variables.
  - Unused-variable diagnostics now include unused `for`-loop iterator variables.
- **String interpolation parser hardening**:
  - Unclosed interpolation fragments (`"{...`) are preserved as literal text.
  - Empty braces (`{}`) are preserved as literal text.
  - Interpolation nodes that end up fully literal are normalized back to plain string literals.
- **Compiler hardening loops**:
  - `src/parser.rs` now includes ignored deterministic stress tests for generated lexer/parser noise.
  - `fuzz/fuzz_targets/lexer_parser.rs` provides a `cargo-fuzz` entrypoint for lexer+parser panic hunting outside the default test suite.
- **Generic parameter parser hardening**:
  - Empty declaration lists like `function f<>` are rejected directly.
  - Trailing commas in declaration generics like `function f<T,>` are rejected directly.
- **List parser hardening**:
  - Trailing commas in function parameter lists are rejected directly.
  - Trailing commas in extern parameter lists are rejected directly.
  - Trailing commas in call argument lists are rejected directly.
- **Declaration list parser hardening**:
  - Trailing commas in class `implements` lists are rejected directly.
  - Trailing commas in interface `extends` lists are rejected directly.
  - Trailing commas in enum field lists are rejected directly.
- **Extern header parser hardening**:
  - Empty `extern(...)` option lists are rejected directly.
  - Trailing commas in `extern(...)` option lists are rejected directly.
  - Extra `extern(...)` option arguments beyond ABI and optional link name are rejected directly.
- **Enum/pattern list parser hardening**:
  - Trailing commas in enum variant lists are rejected directly.
  - Trailing commas in pattern binding lists are rejected directly.
- **Declaration inheritance parser hardening**:
  - Empty class `implements` lists are rejected directly.
  - Empty interface `extends` lists are rejected directly.
- **Else-if parser/formatter hardening**:
  - Statement-form and expression-form `else if` chains are now accepted by `src/parser.rs`.
  - `src/formatter.rs` preserves nested `else if` chains instead of rewriting them as `else { if ... }`.
- **Formatter roundtrip stability**:
  - `src/formatter.rs` now preserves parser-valid parameter mode ordering (`borrow mut x: T`) instead of emitting `mut borrow x: T`.
  - Multi-bound generic constraints stay comma-separated in formatter output so the parser can read them back without a syntax drift step.
- **Declaration-header parser hardening**:
  - Visibility modifiers on `module` declarations now produce a direct parser error instead of falling through to a token mismatch.
  - `class ... extends ...` now rejects comma-suffixed multi-base syntax with a direct single-base-class diagnostic.
- **Match parser hardening**:
  - Empty `match` statement and expression bodies are now rejected directly in `src/parser.rs` instead of surviving into later compiler stages.
- **Declaration/path diagnostic hardening**:
  - Leading-dot `import` and `package` paths are now rejected directly.
  - Empty class `extends` clauses are now rejected directly.
  - Visibility modifiers on `import` and `package` declarations now produce dedicated parser diagnostics.
- **Formatter precedence hardening**:
  - `src/formatter.rs` now preserves parentheses when lambda / `if` / `match` / `async` expressions are used as call callees, preventing roundtrip drift.
- **Generic class typechecking hardening**:
  - `src/typeck.rs` now preserves user-defined generic class instantiations as `Class("Name<...>")` and substitutes class-level type variables through fields, constructors, and methods.
  - `src/codegen/core.rs` now normalizes generic constructor names back to the base class symbol during constructor codegen.
  - `src/codegen/core.rs` now emits implicit zero-argument constructors for classes that omit `constructor`, so `C()` works end to end without a handwritten constructor body.
  - `src/codegen/core.rs` now specializes explicit generic method calls inside class bodies and call sites, so `obj.id<Integer>(...)` no longer survives into unsupported raw codegen.
  - `src/codegen/util.rs` now resolves `Type::Generic(name, ...)` back to the owning class for generic instance field/method dispatch.
- **Closure callee codegen hardening**:
  - `src/codegen/core.rs` now allows non-identifier closure-valued expressions to go through the same indirect-call path as named function variables, so lambda callees compile instead of failing with `Invalid callee`.
  - `src/typeck.rs` and `src/codegen/core.rs` now distinguish function-valued fields from methods, so `obj.f(...)` routes through closure-call checking/lowering instead of member-method dispatch.
- `src/typeck.rs`, `src/project_rewrite.rs`, and filtered project codegen now also treat namespace-alias function values as first-class functions, so expressions like `u.add1` and calls like `u.add1(2)` survive typecheck, rewrite, and object-only codegen consistently.
- The same alias rewrite/codegen path now handles nested module-qualified alias calls like `u.M.add1`, not just single-segment `u.func` lookups.
- Namespace alias constructor calls like `u.Box(2)` now lower through project rewrite into constructor expressions and carry matching dependency edges/import-check knowledge, so class-only namespaces work with alias-based construction too.
- Aliased constructors now work on all currently supported paths: namespace-alias enum variants like `u.E.A(1)`, exact imported enum aliases like `import util.E as Enum; Enum.A(1)`, and exact imported class constructor aliases like `import util.Box as B; B(1)`.
- Exact imported enum variant aliases like `import app.E.B as Variant; Variant(2)` now also resolve end to end in project mode; the dependency graph treats them as depending on the parent enum owner file so semantic checking keeps the enum metadata in the same component.
- The same exact imported variant aliases now rewrite in pattern positions too, so `match (e) { Variant(v) => ... }` lowers to the owning mangled enum variant before semantic checking.
- The nested-module forms now work too: exact aliases like `import app.M.E as Enum` / `import app.M.E.B as Variant` and namespace aliases like `import app as u` all resolve nested enum type, constructor, and pattern paths through the same project rewrite and dependency-closure logic.
- The same nested-module rewrite machinery now covers local nested classes and exact imported nested module functions: paths like `M.Box(2)`, `b: M.Box`, `import app.M.Box as Boxed`, and `import app.M.mk as mk; mk(2).get()` all rewrite through stable owner symbols instead of leaking pseudo-module identifiers into semantic/codegen stages.
- Type-name canonicalization now also runs end-to-end for dotted module type annotations and for user-defined generic classes that reuse built-in container spellings, so paths like `item: util.Item` and `class Box<T> { ... }` no longer split into parallel semantic/backend identities or fall back to built-in runtime lowering.
- Generic-function specialization in codegen now also remaps module-local class shadowing before lowering specialized bodies, so nested module helpers like `module M { class Box<T> ...; function mk<T>(v: T): Box<T> { return Box<T>(v); } }` no longer miscompile the constructor call as the built-in `Box<T>` runtime allocation path.
- The same module-aware remap now also runs during generic-class specialization rewrites for ordinary nested-module helper bodies, so cross-package project builds keep local `Box<Integer>(...)` constructor calls bound to `M__N__Box__spec__I64` instead of falling back to the built-in `Box` runtime path.
- The same codegen specialization pass now does the corresponding owner-aware rewrite for generic methods on specialized classes, so nested module code like `M.Box<Integer>(2).map<Integer>(inc).get()` no longer falls through unsupported explicit-generic method calls or returns zero-initialized objects.
- Filtered project codegen now compiles only the requested class methods when a class is pulled in solely through method-symbol activation, which prevents duplicate base symbol emission while still allowing imported expression-receiver chains like `import app.M.make as make; make<Integer>(2).map<Integer>(inc).get()`.
- Parser precedence now keeps postfix chains outside `await`, so `await(make_box()).get()` is interpreted as “await the task result, then call `get()`” instead of accidentally treating `.get()` as part of the awaited operand.
- Direct map indexing now shares the same typed lookup/fail-fast path as `Map.get(...)`, so `m[key]` respects the actual `Map<K, V>` key type and no longer falls through a bogus raw-pointer indexing path in codegen.
- Direct string indexing now also goes through an explicit bounds check before loading a `Char`, so `"abc"[i]` no longer relies on unchecked pointer arithmetic when `i` is out of range.
- Map index assignment now desugars cleanly at codegen time into the same typed update path as `Map.set(...)`, so `m[key] = value` no longer falls through a list-only lvalue implementation.
- List index assignment now shares the same bounds checks as list reads, so `xs[i] = value` no longer bypasses runtime safety guards on negative or out-of-range indices.
- Backend binary equality now has a dedicated `String` path using `strcmp`, so string comparisons no longer depend on integer/float-only binary lowering.
- Exact imported enum aliases now also rewrite in type positions, so declarations like `e: Enum` stay consistent with `Enum.A(...)` constructor paths during project-mode typechecking.
- Local enum type annotations and local enum variant constructor expressions inside function bodies now rewrite too, so body-local declarations like `e: E = E.A(1)` and lambda params typed as `E` stay consistent with the mangled project enum name.
- Parser type syntax now accepts qualified names like `u.Box` and `u.Box<Integer>` in type positions, which lets alias-qualified project types flow through the same rewrite/typecheck path as expression-level alias calls.
- Namespace-alias qualified types and constructor-like generic type strings (for example `b: u.Box` and `List<u.Box>()`) now rewrite through the same AST type normalization path, so annotations and constructor expressions agree on mangled owner types.
- Project rewrite now also recurses through expression-only containers like `if` expressions, `match` expressions, async blocks, `await`, string interpolation, `require`, borrow/deref, `try`, and range endpoints, so alias/function rewriting no longer silently stops at those expression boundaries.
- Payload-less enum variants now lower as first-class enum values in project mode, including direct forms like `E.A` and alias-qualified forms like `Enum.A` and `u.E.A`, instead of falling through field-access code paths.
- Higher-order codegen now handles `try`-unwrapped and dereferenced function values as indirect callees, so expressions like `(choose()?)(1)` and `(*f)(1)` use the same closure-call lowering path as other function values.
- Formatter precedence now preserves dereferenced and `try`-unwrapped function-value callees too, so `apex fmt` does not rewrite `(*f)(1)` into `*f(1)` or `(choose()?)(1)` into `choose()?(1)`.
- Async soundness is stricter now: async blocks/functions may not return values containing borrowed references, async functions may not accept borrowed-reference-containing parameters, and async blocks may not capture outer variables whose types already contain borrowed references.
- Extern functions remain callable, but `src/typeck.rs` now rejects them as first-class values up front, so expressions like `f = puts` fail in semantic analysis instead of surviving to a late backend error.
  - Match patterns now also accept qualified enum variant names, so forms like `Enum.A(v)` and `util.E.B(w)` survive parse, typecheck, formatting, and codegen instead of failing on the first `.` token or dropping payload bindings in backend lowering.
  - Higher-order generic methods that return closures now survive specialization and subsequent invocation without confusing generated method symbols for fields.
  - `src/typeck.rs` now parses function-type strings nested inside generic wrappers during normalization/substitution, so wrapper types containing function values compare correctly.
  - `src/typeck.rs` now recognizes `Option.some/none` and `Result.ok/error` as frontend static constructors instead of treating `Option`/`Result` as undefined variables.
- `src/project_rewrite.rs` now rewrites bare function identifiers used as values in project mode, and the rewrite cache schema was bumped so old pre-fix rewrites are dropped.
- The rewrite cache schema is bumped whenever alias/type rewriting changes in a cache-incompatible way, preventing stale `.apexcache` entries from reintroducing fixed project-rewrite bugs.

## Directory Structure

- `src/main.rs`: Entry point, CLI argument parsing.
- `src/ast.rs`: Definitions of all AST nodes (Expr, Stmt, Type).
- `src/lexer.rs`: Tokenizer implementation.
- `src/parser.rs`: Parser implementation.
- `src/typeck.rs`: Type checker implementation.
- `src/borrowck.rs`: Borrow checker implementation.
- `src/formatter.rs`: AST-driven source formatter used by `apex fmt`.
- `src/test_runner.rs`: Test discovery and generated runner pipeline; now recurses nested modules for `@Test` and lifecycle hooks.
- `src/bindgen.rs`: Lightweight C header bridge; now keeps pointer-return prototypes and rejects whole function-pointer-param signatures instead of emitting truncated bindings.
- `src/codegen/mod.rs`: Codegen module entry.
- `src/codegen/core.rs`: Core IR generation and lowering.
- `src/codegen/types.rs`: Built-in collection/Option/Result/Range codegen helpers.
- `src/codegen/util.rs`: C runtime bindings and utility helpers.
- `fuzz/fuzz_targets/lexer_parser.rs`: `cargo-fuzz` lexer/parser hardening target.

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for details on how to set up the dev environment and submit PRs.
