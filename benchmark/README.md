# Arden Benchmark Suite

This directory contains the repository benchmark harness for Arden.

The goal is not to print one flattering number. The goal is to measure:

- runtime workloads
- cold and hot compile behavior
- incremental rebuild behavior
- optional larger synthetic graph stress cases

This benchmark suite is meant to be rerunnable by other people. If a result matters, it should be reproducible from the commands in this directory.

The CLI now prefers cleaner canonical benchmark names. Legacy benchmark IDs are still accepted as compatibility aliases, but the docs use the canonical names below.

## What The Harness Compares

The benchmark runner compares Arden against Rust and Go on equivalent workloads when the required toolchains are available.

## Main Workload Groups

### Runtime

- `sum_loop`
- `prime_count`
- `matrix_mul`
- `matrix_mul_heavy`

### Compile

- `compile_project_starter_graph`
- `compile_project_mega_graph`
- `compile_project_extreme_graph`

### Incremental Rebuild

- `incremental_rebuild_single_file`
- `incremental_rebuild_shared_core`
- `incremental_rebuild_large_project_batch`
- `incremental_rebuild_mega_graph_batch`
- `incremental_rebuild_mega_graph_mixed`
- `incremental_rebuild_extreme_graph_batch`
- `incremental_rebuild_extreme_graph_mixed`

## Why These Benchmarks Exist

The benchmark harness is trying to answer several different questions:

- how expensive is the generated machine code on simple CPU-heavy workloads?
- how much time does Arden spend compiling a non-trivial project graph?
- how much work gets reused on hot builds and partial edits?
- how does Arden compare to familiar baselines when the workload is kept comparable?

Those are different questions, which is why runtime, compile, and incremental measurements are kept separate.

## Requirements

- `python3`
- `rustc`
- `go`
- Arden binary at `target/release/arden`
- LLVM / Clang toolchain required by Arden
- `mold` on Linux, or LLVM `lld` on macOS/Windows

If the Arden binary is missing, the runner can build it with `cargo build --release` unless you pass `--no-build`.

## Start Here

Show the full CLI:

```bash
python3 benchmark/run.py --help
```

Run a small runtime benchmark:

```bash
python3 benchmark/run.py --bench sum_loop --repeats 3 --warmup 1
```

Run a compile benchmark:

```bash
python3 benchmark/run.py --bench compile_project_starter_graph --compile-mode hot --repeats 3 --warmup 1
```

Run with per-phase Arden timing breakdowns:

```bash
python3 benchmark/run.py --bench compile_project_starter_graph --compile-mode hot --arden-timings
```

## Output

Reports are written to:

- `benchmark/results/latest.json`
- `benchmark/results/latest.md`

They include:

- per-language timing data
- summary stats
- correctness checks
- optional Arden phase timings for compile-oriented runs

## Reading The Results Correctly

Use the output with the right mental model:

- runtime benchmarks are primarily about generated code quality and runtime overhead
- compile benchmarks are about frontend + LLVM + linking cost
- incremental benchmarks are about invalidation policy and cache reuse
- synthetic graph tests stress the build graph harder than many day-one hobby projects

If you publish numbers, include:

- commit or date
- machine description
- command line used
- whether the run was cold, hot, or incremental

## Local Snapshot

The following numbers were verified locally on `2026-04-07` after building `target/release/arden` from this repository. They are intentionally labeled as a quick snapshot, not a controlled lab result.

| Benchmark | Arden | Rust | Go |
| :--- | ---: | ---: | ---: |
| `matrix_mul_heavy` runtime | `0.012186s` | `0.027221s` | `0.041095s` |
| `incremental_rebuild_large_project_batch` hot rebuild after 10 edits | `0.060284s` | `1.012121s` | `0.891075s` |

Commands:

```bash
python3 benchmark/run.py --bench matrix_mul_heavy --repeats 1 --warmup 0 --no-build
python3 benchmark/run.py --bench incremental_rebuild_large_project_batch --repeats 1 --warmup 0 --no-build
```

Note: the synthetic and extreme graph compile benchmarks are also available, but on this machine the Go side was killed during quick snapshot runs. They are useful stress tests, just not the cleanest cross-language headline numbers for this specific environment.

## Notes On Interpretation

- synthetic graph benchmarks are intentionally harsh and not a claim that Arden is already representative of a massive production codebase
- hot compile and incremental results are only meaningful if you understand what got invalidated
- publication-quality results should be collected on a stable machine with controlled background load
