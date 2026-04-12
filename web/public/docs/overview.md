# Arden Overview

## Why This Matters

This page is the fastest map of what Arden already ships today, how to approach learning it, and where to dive deeper without guessing.

Arden is a native systems language with three core priorities:

- fast compile-time feedback
- strong static guarantees
- one integrated CLI workflow

## What You Get Today

Arden is not just a parser demo. The repository already includes:

- language core: types, functions, control flow, modules, classes, enums, interfaces, generics
- object model extras: visibility (`public/private/protected`), inheritance (`extends`), destructors
- safety model: ownership, borrowing, checked mutation, lifetime validation
- effect model: `@Pure`, `@Io`, `@Thread`, `@Any` (+ `@Net/@Alloc/@Unsafe` support)
- async model: `Task<T>`, `async`, `await`, task status/cancel/timeout APIs
- FFI surfaces: `extern`, ABI options (`c`/`system`), bindgen workflow, `Ptr<T>` type support
- project mode: `arden.toml`, explicit file graph, cache-aware build/check/run/test
- integrated tooling: formatter, linter/fixer, test runner, benchmark/profile, bindgen, LSP

## Recommended Learning Path

1. [Quick Start](getting_started/quick_start.md)
2. [Syntax](basics/syntax.md), [Variables](basics/variables.md), [Types](basics/types.md)
3. [Functions](features/functions.md), [Classes](features/classes.md), [Enums](features/enums.md), [Interfaces](features/interfaces.md), [Modules](features/modules.md), [Packages/Imports](features/packages_imports.md), [Attributes](features/attributes.md), [Language Edges](features/language_edges.md)
4. [Ownership](advanced/ownership.md), [Error Handling](advanced/error_handling.md), [Async](advanced/async.md), [Effects](advanced/effects.md)
5. [Extern and FFI](advanced/ffi.md), [Projects](features/projects.md), [CLI](compiler/cli.md), [Architecture](compiler/architecture.md)

## 60-Second Sanity Loop

```bash
arden check examples/single_file/basics/01_hello/01_hello.arden
arden run examples/single_file/basics/01_hello/01_hello.arden
arden test --path examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden
```

## Learn With Runnable Examples

- basics: [`01_hello`](../examples/single_file/basics/01_hello/01_hello.arden)
- ownership: [`10_ownership`](../examples/single_file/safety_and_async/10_ownership/10_ownership.arden)
- async: [`14_async`](../examples/single_file/safety_and_async/14_async/14_async.arden)
- effects: [`26_effect_system`](../examples/single_file/tooling_and_ffi/26_effect_system/26_effect_system.arden)
- extern/ffi: [`27_extern_c_interop`](../examples/single_file/tooling_and_ffi/27_extern_c_interop/27_extern_c_interop.arden)
- visibility/contracts: [`35_visibility_enforcement`](../examples/single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden), [`37_interfaces_contracts`](../examples/single_file/language_edges/37_interfaces_contracts/37_interfaces_contracts.arden)
- full example index: [examples/README](../examples/README.md)

## Mental Model

Arden code is explicit by design:

- explicit types at boundaries
- explicit ownership, mutability, and effect rules
- explicit project file graph

That makes behavior easier to reason about in large codebases and CI.

## Performance and Tuning Notes

Day-to-day commands:

- `arden check --timings`
- `arden build --timings`

Advanced large-project tuning exists through environment variables
(`ARDEN_OBJECT_SHARD_THRESHOLD`, `ARDEN_OBJECT_SHARD_SIZE`) for object-codegen
sharding. Keep these for profiling and CI tuning, not normal beginner workflow.

## Where To Go Next

- new user: [Quick Start](getting_started/quick_start.md)
- language feature lookup: [`basics/`](basics/) + [`features/`](features/)
- safety/runtime behavior: [`advanced/`](advanced/) + [`stdlib/`](stdlib/)
- command usage: [CLI Reference](compiler/cli.md)
- compiler internals: [Architecture](compiler/architecture.md)
