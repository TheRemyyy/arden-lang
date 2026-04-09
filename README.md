<div align="center">

<img src="LOGO.png" alt="Arden logo" width="160" />

# Arden

**A native systems language focused on fast feedback, strong static checks, and practical tooling.**

[![Website](https://img.shields.io/badge/Website-Arden-white?style=flat-square&logo=vercel)](https://arden-lang.dev/)
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg?style=flat-square)](https://www.rust-lang.org/)
[![LLVM](https://img.shields.io/badge/LLVM-22.1+-blue.svg?style=flat-square)](https://llvm.org/)

[Documentation](docs/) • [Examples](examples/) • [Benchmarks](benchmark/) • [Web Docs](https://www.arden-lang.dev/docs/overview)

</div>

---

## Why Arden

Arden is built for people who want native output, compiler-enforced safety, and an integrated workflow without stitching together five separate tools.

Today the repository already includes:

- LLVM-backed native code generation
- a real CLI for `build`, `run`, `check`, `fmt`, `lint`, `fix`, `test`, `bench`, `profile`, `bindgen`, `lex`, `parse`, and `lsp`
- multi-file project builds via `arden.toml`
- ownership and borrowing checks
- async tasks and runtime control helpers
- formatter, linter, test runner, benchmark harness, and CI smoke coverage in the same repo

This is still an experimental language, but it is not a toy parser demo.

## What You Get In This Repository

This repository is not just the compiler binary. It also contains the material a new user needs to go from "what is this?" to "I can build something with it":

- source documentation under [`docs/`](docs/)
- runnable language and project examples under [`examples/`](examples/)
- compiler and project smoke scripts under [`scripts/`](scripts/)
- the benchmark harness under [`benchmark/`](benchmark/)
- CI and release automation under [`.github/`](.github/)

The intended learning loop is:

1. install the toolchain
2. run one single-file example
3. create a project with `arden new`
4. inspect project mode with `arden info`
5. move into testing, formatting, benchmarking, and larger examples

## What Arden Looks Like

```arden
import std.io.*;

function main(): None {
    mut sum: Integer = 0;

    for (value in range(0, 5)) {
        sum += value;
    }

    println("sum = {sum}");
    return None;
}
```

## Quick Start

### Requirements

- Rust `1.85+`
- LLVM `22.1+`
- Clang
- `mold` on Linux, or LLVM `lld` on macOS/Windows

Detailed platform notes live in [docs/getting_started/installation.md](docs/getting_started/installation.md).

### Build From Source

```bash
git clone https://github.com/TheRemyyy/arden-lang.git arden
cd arden
cargo build --release
```

The compiler binary will be available at:

- `target/release/arden`
- `target/release/arden.exe` on Windows

### Run A Single File

```bash
cat > hello.arden <<'EOF'
import std.io.*;

function main(): None {
    println("Hello, Arden!");
    return None;
}
EOF

./target/release/arden run hello.arden
```

### Create A Project

```bash
./target/release/arden new hello_project
cd hello_project
../target/release/arden run
```

That scaffold is intentionally small, but it already gives you the pieces Arden uses for project mode:

- `arden.toml` declares the project name, entry file, output kind, output path, and explicit source file list
- `src/main.arden` is the entrypoint used by `arden run` and `arden build`
- `README.md` records the local workflow so the generated project is not a dead skeleton

To inspect exactly what the compiler sees, run:

```bash
../target/release/arden info
```

## CLI Surface

Arden ships with a broader workflow than just `compile`.

```text
new      Create a project skeleton
build    Build the current project
run      Build and run a project or single file
compile  Compile a single Arden file
check    Parse, type-check, and borrow-check source
info     Print project configuration and build settings
lint     Report static findings
fix      Apply safe fixes and reformat the result
fmt      Format Arden source
lex      Print lexer tokens
parse    Print the parsed AST
lsp      Start the language server
test     Discover and run @Test suites
bindgen  Generate Arden extern bindings from a C header
bench    Measure end-to-end execution time
profile  Run once and print a timing summary
```

Reference: [docs/compiler/cli.md](docs/compiler/cli.md)

## Language Snapshot

Arden currently supports:

- functions, lambdas, modules, packages, and imports
- classes, inheritance, interfaces, and visibility rules
- enums, pattern matching, `Option<T>`, and `Result<T, E>`
- generics and generic bounds
- ownership, borrowing, and mutability checking
- async / await with `Task<T>`
- intrinsic standard library modules for I/O, math, time, args, strings, collections, and system access

Good starting points:

- [docs/overview.md](docs/overview.md)
- [docs/getting_started/quick_start.md](docs/getting_started/quick_start.md)
- [docs/features/projects.md](docs/features/projects.md)
- [docs/features/testing.md](docs/features/testing.md)
- [docs/stdlib/overview.md](docs/stdlib/overview.md)

## How Arden Works

At a high level, the compiler pipeline is:

1. lex source text into tokens
2. parse the token stream into an AST
3. resolve names, types, and effects
4. run ownership and borrow validation
5. lower the checked program to LLVM IR
6. link a native executable or library

That matters for users because many CLI commands stop at different layers:

- `arden lex` shows tokenizer output
- `arden parse` shows parser output
- `arden check` runs semantic and borrow checks without building a native binary
- `arden build` goes through codegen and linking
- `arden run` builds and executes

More detail lives in [docs/compiler/architecture.md](docs/compiler/architecture.md).

## Project Mode In Practice

Single-file programs are useful for experiments, but most real Arden work happens in project mode.

Project mode gives you:

- explicit source graph control through `arden.toml`
- a stable entry file instead of magic directory scanning
- reusable build metadata in `.ardencache/`
- project-aware `build`, `run`, `check`, `fmt`, `test`, and `info`

This is one of the bigger differences between Arden and parser-demo style language repos: there is an opinionated workflow for building multi-file code, not just compiling one example file at a time.

Reference: [docs/features/projects.md](docs/features/projects.md)

## Examples

The repo includes both focused feature examples and larger project-style samples.

Recommended first passes:

- [examples/01_hello.arden](examples/01_hello.arden)
- [examples/10_ownership.arden](examples/10_ownership.arden)
- [examples/14_async.arden](examples/14_async.arden)
- [examples/24_test_attributes.arden](examples/24_test_attributes.arden)
- [examples/35_visibility_enforcement.arden](examples/35_visibility_enforcement.arden)
- [examples/starter_project/README.md](examples/starter_project/README.md)
- [examples/showcase_project/README.md](examples/showcase_project/README.md)

Overview: [examples/README.md](examples/README.md)

If you are learning the language, a good order is:

1. start with `01_hello`, `02_variables`, and `04_control_flow`
2. move to `05_classes`, `08_modules`, and `09_generics`
3. then read `10_ownership`, `13_error_handling`, and `14_async`
4. after that, switch to `starter_project/` and `showcase_project/`

That path mirrors the way the docs are structured, so you can alternate between prose and runnable code instead of reading one giant manual first.

## Benchmarks

Arden includes a benchmark harness that compares Arden, Rust, and Go on shared workloads.

The suite covers:

- CPU-focused runtime workloads
- cold and hot project compile benchmarks
- incremental rebuild benchmarks
- optional larger synthetic graph stress tests

There are two entrypoints:

- **`benchmark/run.py`** — single benchmark runs, outputs to `benchmark/results/latest.*`
- **`benchmark/full_campaign.py`** — multi-stage campaigns with presets, outputs to a timestamped `benchmark/results/campaign_*/` directory

Quick start:

```bash
# Smoke test — does the harness work?
python3 benchmark/run.py --bench matrix_mul_heavy --repeats 1 --warmup 0 --no-build

# Quick sanity pass across all groups (~2–5 min)
python3 benchmark/full_campaign.py --preset quick --no-build

# Full publication-grade campaign (~15–30 min)
python3 benchmark/full_campaign.py --preset full --no-build
```

Full documentation, command map, output layout, instrumentation flags, and methodology caveats: [benchmark/README.md](benchmark/README.md)

The benchmark harness is intentionally part of the repository instead of an external gist so numbers can be regenerated, challenged, and updated. If benchmark results are published, they should always be tied to a command, machine, and date rather than presented as timeless marketing.

### Local Snapshot

Verified locally on `2026-04-07` with `target/release/arden` built from this repository and single-sample runs (`--repeats 1 --warmup 0`):

| Benchmark | Arden | Rust | Go |
| :--- | ---: | ---: | ---: |
| `matrix_mul_heavy` runtime | `0.012186s` | `0.027221s` | `0.041095s` |
| `incremental_rebuild_large_project_batch` hot rebuild after 10 edits | `0.060284s` | `1.012121s` | `0.891075s` |

Commands used:

```bash
python3 benchmark/run.py --bench matrix_mul_heavy --repeats 1 --warmup 0 --no-build
python3 benchmark/run.py --bench incremental_rebuild_large_project_batch --repeats 1 --warmup 0 --no-build
```

Treat this as a repository snapshot, not a publication-grade claim. For stable reporting, rerun with more repeats on a quieter machine.

On this machine, the even larger synthetic and extreme graph compile benchmarks were available in the harness but the Go side was killed during quick snapshot runs, so they are better treated as stress tests than as a clean three-way headline comparison.

## Repository Map

- [docs/](docs/) - language, stdlib, project, and compiler documentation
- [examples/](examples/) - feature-focused examples and multi-file sample projects
- [benchmark/](benchmark/) - benchmark harness and report generation
- [scripts/](scripts/) - smoke tests, example runners, and maintenance scripts
- [.github/workflows/](.github/workflows/) - CI and release automation
- [src/](src/) - compiler implementation

## Documentation Map

If you want a structured reading order:

- [docs/overview.md](docs/overview.md) for the broad mental model
- [docs/getting_started/installation.md](docs/getting_started/installation.md) for toolchain setup
- [docs/getting_started/quick_start.md](docs/getting_started/quick_start.md) for the first runnable steps
- [docs/compiler/cli.md](docs/compiler/cli.md) for the command surface
- [docs/features/projects.md](docs/features/projects.md) and [docs/features/testing.md](docs/features/testing.md) for day-to-day workflow
- [docs/compiler/architecture.md](docs/compiler/architecture.md) if you want to understand how the compiler is arranged internally

If you prefer code first:

- [examples/README.md](examples/README.md)
- [benchmark/README.md](benchmark/README.md)
- [scripts/README.md](scripts/README.md)

## Project Status

Arden is actively evolving. The docs in this repository aim to describe what is implemented now, not an aspirational roadmap.

That means:

- examples are intended to run against the current compiler
- CLI docs follow the current `--help` output
- benchmark docs describe the actual shipped harness
- web docs are generated from the repository sources in `docs/`

That also means docs should become richer over time, but not looser. If a feature is incomplete, the docs should say so plainly.

## Contributing

If you want to improve the compiler, docs, examples, or tooling, start with:

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [docs/compiler/architecture.md](docs/compiler/architecture.md)
- [scripts/README.md](scripts/README.md)
