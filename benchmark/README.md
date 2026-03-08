# Apex Production Benchmark Suite

This directory contains a structured benchmark suite that compares Apex against C, Rust, and Go on the same workloads.

## Goals

- Use identical algorithms across languages.
- Validate correctness (same checksum/result for each workload).
- Measure repeatable wall-clock runtime.
- Export machine-readable and human-readable reports.

## Workloads

- `sum_loop`: integer-heavy pseudo-random accumulation loop.
- `prime_count`: sieve-based prime counting.
- `matrix_mul`: dense integer matrix multiplication (flattened arrays).
- `compile_project_10_files`: compile stress benchmark on generated 10-file projects per language.
- `incremental_rebuild_1_file`: compiles a generated 10-file project, mutates one file, then recompiles.
- `incremental_rebuild_central_file`: same generated 10-file project, but mutates the shared central file before rebuild.
- `incremental_rebuild_mega_project_10_files`: compiles a generated 120-file mega-project, applies syntax-only edits to 10 files, then rebuilds to expose cold vs hot behavior.

## Directory Layout

```text
benchmark/
  apex/         # Apex implementations
  c/            # C implementations
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
- `clang` (for C)
- `rustc` (for Rust)
- `go` (for Go)
- Apex compiler binary available at `target/release/apex-compiler`

If the Apex compiler binary is missing, the runner will build it via:

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
- incremental rebuild scenarios: `incremental_rebuild_1_file`, `incremental_rebuild_central_file`
- mega incremental rebuild scenario: `incremental_rebuild_mega_project_10_files`

Useful options:

```bash
python3 benchmark/run.py --repeats 7 --warmup 1
python3 benchmark/run.py --bench prime_count
python3 benchmark/run.py --bench compile_project_10_files
python3 benchmark/run.py --bench incremental_rebuild_1_file
python3 benchmark/run.py --bench incremental_rebuild_central_file
python3 benchmark/run.py --bench incremental_rebuild_mega_project_10_files
python3 benchmark/run.py --bench compile_project_10_files --compile-mode cold
python3 benchmark/run.py --no-build
python3 benchmark/run.py --apex-opt-level 3
python3 benchmark/run.py --apex-target x86_64-unknown-linux-gnu
```

## Output

- JSON report: `benchmark/results/latest.json`
- Markdown report: `benchmark/results/latest.md`

Both include:

- per-language timings
- summary stats (min/mean/median/stddev/max)
- speedups relative to Apex
- correctness checksums

For `compile_project_10_files`:
- `--compile-mode hot` keeps compile caches/artifacts between runs (incremental-friendly).
- `--compile-mode cold` clears artifacts between timed runs; for Apex this also removes `.apexcache`.

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
- this is the benchmark intended to show how much Apex incremental caching shrinks rebuild cost on very large codebases

## Notes

- This suite is CPU-focused and deterministic.
- Keep machine load stable for fair comparisons.
- For publication-quality results, pin CPU governor and run multiple sessions.
- On Windows, executable suffixes are handled automatically (`.exe`), but required toolchains still must be available in `PATH`.
