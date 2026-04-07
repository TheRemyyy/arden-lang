# Insane Showcase Project

This example is the heavy multi-file Arden showcase.

It intentionally combines:

- Java-style packages across files
- nested modules and alias imports
- interfaces, classes, inheritance, generics
- enums and nested `Option<Result<...>>` flows
- higher-order functions and function values
- async/await with `await_timeout`
- file I/O, string utilities, math, args
- float interpolation through deep qualified calls

## Run

```bash
cd examples/insane_showcase_project
arden build
./insane_showcase_project
```

Or from the repo root:

```bash
(cd examples/insane_showcase_project && cargo run --quiet --manifest-path ../../Cargo.toml -- build)
./examples/insane_showcase_project/insane_showcase_project
```
