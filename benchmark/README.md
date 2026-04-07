# Arden Production Benchmark Suite

This directory contains a structured benchmark suite that compares Arden against Rust and Go on the same workloads.

## Goals

- Use identical algorithms across languages.
- Validate correctness (same checksum/result for each workload).
- Measure repeatable wall-clock runtime.
- Export machine-readable and human-readable reports.

## Workloads

- `sum_loop`: integer-heavy pseudo-random accumulation loop.
- `prime_count`: sieve-based prime counting.
- `matrix_mul`: dense integer matrix multiplication (flattened arrays).
- `matrix_mul_heavy`: heavier dense integer matrix multiplication (220x220) for a more meaningful CPU-bound runtime pass.
- `compile_project_10_files`: compile stress benchmark on generated 10-file projects per language.
- `compile_project_synthetic_mega_graph`: compile stress benchmark on a generated 1400-file synthetic mega-graph project per language.
- `compile_project_extreme_graph`: much larger compile stress benchmark on a generated 2200-file synthetic dependency graph.
- `incremental_rebuild_1_file`: compiles a generated 10-file project, mutates one file, then recompiles.
- `incremental_rebuild_central_file`: same generated 10-file project, but mutates the shared central file before rebuild.
- `incremental_rebuild_mega_project_10_files`: compiles a generated 120-file mega-project, applies syntax-only edits to 10 files, then rebuilds to expose cold vs hot behavior.
- `incremental_rebuild_synthetic_mega_graph`: compiles a generated 1400-file synthetic mega-graph project, applies syntax-only edits to 40 spread-out files, then rebuilds.
- `incremental_rebuild_synthetic_mega_graph_mixed_invalidation`: compiles a generated 1400-file synthetic mega-graph project, then rebuilds after mixed leaf edits plus API-surface invalidation across selected groups.
- `incremental_rebuild_extreme_graph`: compiles a generated 2200-file dependency graph, applies syntax-only edits to 64 spread-out files, then rebuilds.
- `incremental_rebuild_extreme_graph_mixed_invalidation`: compiles a generated 2200-file dependency graph, then rebuilds after 40 leaf edits plus 12 shared API-surface invalidations.

## Directory Layout

```text
benchmark/
  arden/         # Arden implementations
  rust/         # Rust implementations
  go/           # Go implementations
  bin/          # Compiled binaries (generated)
  generated/    # Generated project sources for compile stress benchmark (generated)
  results/      # Benchmark reports (generated)
  run.py        # Unified benchmark runner
```

## Requirements

- Linux/macOS/Windows shell environment.
- `python3`
- `rustc` (for Rust)
- `go` (for Go)
- `mold` for Linux Arden project builds, or LLVM `lld` on macOS/Windows
- Arden binary available at `target/release/arden`

If the Arden binary is missing, the runner will build it via:

```bash
cargo build --release
```

`LLVM_SYS_211_PREFIX` is auto-detected via `llvm-config --prefix` when available.
You can override it explicitly if needed:

```bash
LLVM_SYS_211_PREFIX=/usr/lib64/llvm21 python3 benchmark/run.py
```

## Run

From repository root:

```bash
python3 benchmark/run.py
```

Default run behavior:
- runtime workloads: `sum_loop`, `prime_count`, `matrix_mul`
- compile stress in both modes: `compile_project_10_files_hot`, `compile_project_10_files_cold`
- synthetic mega-graph compile stress in both modes: `compile_project_synthetic_mega_graph_hot`, `compile_project_synthetic_mega_graph_cold`
- incremental rebuild scenarios: `incremental_rebuild_1_file`, `incremental_rebuild_central_file`
- mega incremental rebuild scenario: `incremental_rebuild_mega_project_10_files`
- synthetic mega-graph incremental rebuild scenario: `incremental_rebuild_synthetic_mega_graph`
- synthetic mega-graph mixed invalidation scenario: `incremental_rebuild_synthetic_mega_graph_mixed_invalidation`

Heavier opt-in workloads are excluded from the default suite to keep the baseline run practical:
- runtime: `matrix_mul_heavy`
- compile: `compile_project_extreme_graph`
- incremental: `incremental_rebuild_extreme_graph`, `incremental_rebuild_extreme_graph_mixed_invalidation`

Useful options:

```bash
python3 benchmark/run.py --repeats 7 --warmup 1
python3 benchmark/run.py --include-extreme
python3 benchmark/run.py --bench prime_count
python3 benchmark/run.py --bench matrix_mul_heavy
python3 benchmark/run.py --bench compile_project_10_files
python3 benchmark/run.py --bench compile_project_synthetic_mega_graph
python3 benchmark/run.py --bench compile_project_extreme_graph
python3 benchmark/run.py --bench incremental_rebuild_1_file
python3 benchmark/run.py --bench incremental_rebuild_central_file
python3 benchmark/run.py --bench incremental_rebuild_mega_project_10_files
python3 benchmark/run.py --bench incremental_rebuild_synthetic_mega_graph
python3 benchmark/run.py --bench incremental_rebuild_synthetic_mega_graph_mixed_invalidation
python3 benchmark/run.py --bench incremental_rebuild_extreme_graph
python3 benchmark/run.py --bench incremental_rebuild_extreme_graph_mixed_invalidation
python3 benchmark/run.py --bench compile_project_10_files --compile-mode cold
python3 benchmark/run.py --bench compile_project_extreme_graph --compile-mode hot --arden-timings
python3 benchmark/run.py --no-build
python3 benchmark/run.py --arden-opt-level 3
python3 benchmark/run.py --arden-target x86_64-unknown-linux-gnu
```

## Output

- JSON report: `benchmark/results/latest.json`
- Markdown report: `benchmark/results/latest.md`

Both include:

- per-language timings
- summary stats (min/mean/median/stddev/max)
- speedups relative to Arden
- correctness checksums
- optional Arden per-phase build timings when `--arden-timings` is enabled

For `compile_project_10_files` and `compile_project_synthetic_mega_graph`:
- `--compile-mode hot` keeps compile caches/artifacts between runs (incremental-friendly).
- `--compile-mode cold` clears artifacts between timed runs; for Arden this also removes `.ardencache`.

For compile and incremental scenarios with `--arden-timings`:
- the runner passes `--timings` to Arden project builds
- Markdown and JSON reports include averaged Arden phase timings plus the last observed per-phase counters
- this makes it easier to see whether Arden time is going into parse, semantic, object codegen, or final link instead of only comparing one wall-clock number

For `compile_project_synthetic_mega_graph` specifically:
- each language compiles a generated 1400-file project with 96 helper functions per file
- files are connected by a layered cross-file dependency graph with wider fan-out instead of all calling one shared core helper
- the graph also includes group-level bridge modules that sit on active build paths, so invalidation tests can rewrite shared API surface and dependent callers instead of only touching leaf files
- each file exports a hot path plus multiple extra cross-file wiring functions to stress declaration volume, symbol resolution, invalidation, and code generation on a wide multi-file DAG
- this is a synthetic stress benchmark, not a model of a real Chromium-scale codebase
- this is the compile-time counterpart to the synthetic mega-graph incremental rebuild benchmark

For `compile_project_extreme_graph` specifically:
- each language compiles a generated 2200-file project with 112 helper functions per file
- the dependency graph is wider and deeper than the regular synthetic mega-graph benchmark
- the goal is to put more pressure on parser throughput, graph construction, semantic invalidation, and final link behavior
- this is intentionally expensive and therefore opt-in rather than part of the default suite

For `incremental_rebuild_1_file` and `incremental_rebuild_central_file`:
- each measured cycle does:
  1. full compile
  2. single-file source mutation
  3. second compile
- report includes `first mean` vs `second mean` per language.

For `incremental_rebuild_mega_project_10_files`:
- each measured cycle generates a much larger 120-file project with 320 helper functions per file
- the first timing is a cold full build on a fresh project
- the second timing is a hot rebuild after syntax-only edits in 10 spread-out files
- this is the benchmark intended to show how much Arden incremental caching shrinks rebuild cost on very large codebases

For `incremental_rebuild_synthetic_mega_graph`:
- each measured cycle generates the same synthetic 1400-file dependency graph used by the mega compile benchmark
- the first timing is a cold full build on a fresh project
- the second timing is a hot rebuild after syntax-only edits in 40 spread-out files
- this is the benchmark meant to measure non-trivial rebuild work after real source changes, not a no-op hot rebuild
- despite the scale, it is still synthetic and should not be read as representative of a real Chromium-like product build graph

For `incremental_rebuild_synthetic_mega_graph_mixed_invalidation`:
- each measured cycle generates the same synthetic 1400-file dependency graph and cold-builds it from scratch
- the rebuild phase applies two edit classes together:
- syntax-only edits across 24 spread-out leaf files
- API-surface changes across 8 group bridge modules, plus caller rewrites in every file that depends on those groups
- this is the benchmark intended to approximate a more realistic "dirty set" than pure no-op or comment-only rebuilds
- it is still synthetic, but it exercises parser, declaration, type/symbol, and dependency invalidation paths together

For `incremental_rebuild_extreme_graph`:
- each measured cycle generates the same 2200-file extreme dependency graph used by `compile_project_extreme_graph`
- the first timing is a cold build on a fresh project
- the second timing is a hot rebuild after syntax-only edits in 64 spread-out files
- this is the "pressure test" version of the synthetic incremental benchmark

For `incremental_rebuild_extreme_graph_mixed_invalidation`:
- each measured cycle cold-builds the 2200-file extreme graph, then rebuilds after a mixed dirty set
- the rebuild combines syntax-only edits across 40 leaf files with API-surface changes across 12 shared bridge groups
- this is the harshest compile benchmark in the suite and is intended to stress non-trivial invalidation rather than cache-hit demos

## Notes

- This suite is CPU-focused and deterministic.
- Keep machine load stable for fair comparisons.
- For publication-quality results, pin CPU governor and run multiple sessions.
- On Windows, executable suffixes are handled automatically (`.exe`), but required toolchains still must be available in `PATH`.
