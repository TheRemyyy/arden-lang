# Showcase Project

This is the largest example project in the repository.

Use it when you want to see several features interacting in one project instead of isolated snippets.

It combines:

- packages and nested modules
- alias imports
- interfaces, classes, inheritance, and visibility
- generics and enum-heavy flows
- async / await and task control helpers
- stdlib usage across I/O, strings, args, time, and math
- file I/O through the runtime helper path

## Run It

```bash
cd examples/showcase_project
arden info
arden check
arden build
arden run
```

From the repository root:

```bash
(cd examples/showcase_project && cargo run --manifest-path ../../Cargo.toml -- build)
./examples/showcase_project/showcase_project
```

## Layout

```text
showcase_project/
├── arden.toml
└── src/
    ├── analytics.arden
    ├── domain.arden
    ├── main.arden
    ├── runtime.arden
    └── score.arden
```

## What Each File Shows

- `main.arden` ties the whole project together and exercises the public surface
- `domain.arden` contains interfaces, enums, classes, inheritance, and pattern matching helpers
- `analytics.arden` shows generics, nested modules, `Result<Option<T>>`, and cross-module types
- `score.arden` extends the analytics package with async task-based work
- `runtime.arden` shows async orchestration, file I/O, and result shaping

## What To Notice

- imports use both package names and aliases
- the project mixes classes, enums, generics, closures, tasks, and stdlib calls in one code path
- `main.arden` exercises nested matches on `Result<Option<Box<Float>>, String>`
- async work is visible through `Task<String>` and `await_timeout(...)`
- runtime helpers write and delete a temporary report file, so the example is doing more than just pure computation

## Why This Example Matters

Many language repos have either tiny syntax samples or huge unreadable demos. This project is meant to sit in the middle:

- large enough to feel like a real project
- small enough to read in one sitting
- broad enough to prove that Arden features can interact in one codebase

If you are trying to understand what Arden feels like beyond toy snippets, start here after the smaller multi-file examples.
