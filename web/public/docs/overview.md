# Arden Overview

Arden is a native systems programming language built around three priorities:

- fast compiler feedback
- strong static guarantees
- a practical all-in-one developer workflow

It targets LLVM for native code generation and ships with its own formatter, linter, test runner, benchmark command, bindgen command, and project CLI.

This page is the best starting point if you want to understand what Arden already is, what the repository actually ships today, and where to go next.

## At A Glance

Arden already ships more than "parse a file and hope for the best".

Current repository surface:

| Area | What is already there | Start here |
| --- | --- | --- |
| Language core | static types, functions, lambdas, control flow, modules | [Syntax](basics/syntax.md), [Types](basics/types.md), [Functions](features/functions.md) |
| Structured types | classes, interfaces, enums, visibility, inheritance | [Classes](features/classes.md), [Interfaces](features/interfaces.md), [Enums](features/enums.md) |
| Reusable typing | generics, generic bounds, generic interfaces, nested generic types | [Generics](advanced/generics.md), [`../examples/09_generics.arden`](../examples/09_generics.arden) |
| Safety model | ownership, borrowing, checked mutation, static validation | [Ownership](advanced/ownership.md), [Memory Management](advanced/memory_management.md) |
| Concurrency | `async`, `await`, `Task<T>` | [Async](advanced/async.md), [`../examples/14_async.arden`](../examples/14_async.arden) |
| Workflow | `new`, `check`, `run`, `fmt`, `lint`, `fix`, `test`, `bench`, `profile`, `bindgen`, `lsp` | [Quick Start](getting_started/quick_start.md), [Compiler CLI](compiler/cli.md) |
| Project mode | `arden.toml`, multi-file builds, project-aware tooling, cache state | [Projects](features/projects.md), [Projects Summary](projects.md) |
| Runtime surface | stdlib intrinsics such as `io`, `math`, `time`, `args`, `collections`, `system` | [Stdlib Overview](stdlib/overview.md) |

## What Arden Can Already Do

If you are evaluating the language, these are the capabilities you should assume exist today:

- compile and run both single files and explicit multi-file projects
- type-check static signatures before code generation
- model reusable APIs with generic functions, generic classes, and bounded type parameters
- express interface-based polymorphism with `implements` and `extends`
- model branching data with enums and use pattern matching over them
- use async tasks with typed results via `Task<T>`
- use ownership and borrowing rules to catch invalid moves and mutation hazards earlier
- call common workflow commands from one CLI instead of stitching together unrelated tools
- use built-in container/result surfaces such as `Option<T>`, `Result<T, E>`, `List<T>`, `Map<K, V>`, and `Set<T>`
- use formatter, linter, test discovery, benchmark commands, profile commands, bindgen, and LSP support from the same repo

The fastest proof is usually to read one prose page and one runnable example side by side:

- [Generics](advanced/generics.md) + [`../examples/09_generics.arden`](../examples/09_generics.arden)
- [Ownership](advanced/ownership.md) + [`../examples/10_ownership.arden`](../examples/10_ownership.arden)
- [Async](advanced/async.md) + [`../examples/14_async.arden`](../examples/14_async.arden)
- [Projects](features/projects.md) + [`../examples/starter_project/README.md`](../examples/starter_project/README.md)

## What Arden Tries To Be

Arden is not aiming to be minimal for its own sake, and it is not trying to hide systems-level constraints behind a giant runtime.

The current direction is:

- explicit enough to make ownership, mutation, and types visible
- ergonomic enough for day-to-day app and tool code
- integrated enough that common workflows are built into the compiler CLI

In practice that means Arden sits closer to "systems language with batteries included" than to "minimal compiler experiment". The repo does not just compile a file; it includes project mode, formatter/linter flows, test discovery, benchmarking commands, and example sweeps.

## What You Can Learn Here

The repository is organized so different readers can enter at different depths:

- if you want syntax and language features, stay in [`basics/`](basics/) and [`features/`](features/)
- if you want workflow, use [`getting_started/`](getting_started/) and [`compiler/cli.md`](compiler/cli.md)
- if you want runtime facilities, use [`stdlib/`](stdlib/)
- if you want to see real code first, jump to [`../examples/README.md`](../examples/README.md)
- if you want compiler internals, read [`compiler/architecture.md`](compiler/architecture.md)

## Core Concepts

### Static Types

Arden is statically typed. Variables, function signatures, generics, and collection element types are checked ahead of code generation.

Relevant docs:

- [Basics: Types](basics/types.md)
- [Features: Functions](features/functions.md)
- [Advanced: Generics](advanced/generics.md)

### Ownership And Borrowing

Arden includes ownership and borrowing checks to catch invalid moves, aliasing mistakes, and mutation hazards before runtime.

Relevant docs:

- [Advanced: Ownership](advanced/ownership.md)
- [Examples: Ownership](../examples/10_ownership.arden)

### Project Mode

Single-file workflows are supported, but Arden also has a real multi-file project mode based on `arden.toml`.

Project mode includes:

- explicit file lists
- project-aware `build`, `run`, `check`, `test`, `fmt`, and `info`
- build caching and timing output

Relevant docs:

- [Features: Projects](features/projects.md)
- [Projects summary](projects.md)

### Built-In Workflow Commands

Arden intentionally keeps common tasks under one CLI instead of expecting separate ad-hoc tools for each stage. In current repository form, that includes:

- project creation with `arden new`
- checking and building with `arden check`, `arden build`, and `arden run`
- formatting and linting with `arden fmt`, `arden lint`, and `arden fix`
- test discovery with `arden test`
- simple performance inspection with `arden bench` and `arden profile`
- C binding generation with `arden bindgen`

### Native Toolchain Output

Arden lowers to LLVM IR and then produces native artifacts using Clang plus a platform-specific linker policy.

Relevant docs:

- [Getting Started: Installation](getting_started/installation.md)
- [Compiler: Architecture](compiler/architecture.md)

## Language Features Available Today

The current compiler surface includes:

- functions and lambdas
- classes, interfaces, inheritance, and visibility
- enums and pattern matching
- generics and generic bounds
- modules, packages, and imports
- `Option<T>` and `Result<T, E>`
- async / await with `Task<T>`
- intrinsic stdlib modules such as `Math`, `Str`, `Time`, `System`, `Args`, and file I/O helpers
- built-in test attributes such as `@Test`, `@Before`, and `@Ignore`

## Feature Map

Use this as the short index when you want to jump directly to a capability instead of reading the docs in order.

### Core Syntax And Type System

- [Syntax](basics/syntax.md)
- [Types](basics/types.md)
- [Control Flow](basics/control_flow.md)
- [Functions And Lambdas](features/functions.md)

### Reusable Abstractions

- [Generics](advanced/generics.md)
- [Interfaces](features/interfaces.md)
- [Classes](features/classes.md)
- [Enums](features/enums.md)
- [Modules And Imports](features/modules.md)

### Safety And Runtime Behavior

- [Ownership](advanced/ownership.md)
- [Memory Management](advanced/memory_management.md)
- [Error Handling](advanced/error_handling.md)
- [Async](advanced/async.md)

### Tooling And Workflow

- [Installation](getting_started/installation.md)
- [Quick Start](getting_started/quick_start.md)
- [Testing](features/testing.md)
- [Compiler CLI](compiler/cli.md)
- [Projects](features/projects.md)
- [Stdlib Overview](stdlib/overview.md)

For a broad but runnable tour of these features, the examples directory is often faster than prose:

- [`../examples/17_comprehensive.arden`](../examples/17_comprehensive.arden)
- [`../examples/24_test_attributes.arden`](../examples/24_test_attributes.arden)
- [`../examples/35_visibility_enforcement.arden`](../examples/35_visibility_enforcement.arden)
- [`../examples/showcase_project/README.md`](../examples/showcase_project/README.md)

## CLI Workflow

Arden intentionally bundles common workflows into one CLI:

```text
new, build, run, compile, check, info, lint, fix, fmt,
lex, parse, lsp, test, bindgen, bench, profile
```

Reference:

- [Compiler CLI](compiler/cli.md)

## Suggested Learning Paths

### Path 1: I Want To Run Something In Five Minutes

1. [Installation](getting_started/installation.md)
2. [Quick Start](getting_started/quick_start.md)
3. [`../examples/01_hello.arden`](../examples/01_hello.arden)
4. [`../examples/10_ownership.arden`](../examples/10_ownership.arden)
5. [Projects](features/projects.md)

### Path 2: I Want To Understand The Language Surface

1. [Syntax](basics/syntax.md)
2. [Types](basics/types.md)
3. [Functions](features/functions.md)
4. [Classes](features/classes.md)
5. [Modules](features/modules.md)
6. [Enums](features/enums.md)
7. [Ranges](features/ranges.md)

### Path 3: I Want To Contribute To The Compiler

1. [Compiler CLI](compiler/cli.md)
2. [Compiler Architecture](compiler/architecture.md)
3. [Projects](features/projects.md)
4. [`../scripts/README.md`](../scripts/README.md)
5. [`../CONTRIBUTING.md`](../CONTRIBUTING.md)

## Suggested Reading Order

If you are new to the language:

1. [Installation](getting_started/installation.md)
2. [Quick Start](getting_started/quick_start.md)
3. [Syntax](basics/syntax.md)
4. [Types](basics/types.md)
5. [Projects](features/projects.md)
6. [Testing](features/testing.md)
7. [Standard Library](stdlib/overview.md)

If you want real code quickly:

- [Examples index](../examples/README.md)
- [Multi-file project example](../examples/starter_project/README.md)
- [Showcase project](../examples/showcase_project/README.md)

## Accuracy Policy

The source docs in this repository are meant to describe the current compiler, examples, and CLI behavior. If you notice a mismatch between docs and reality, the intended fix is to update the documentation or example corpus rather than leave contradictory material around.
