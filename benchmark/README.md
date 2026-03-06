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

- Linux/macOS shell environment.
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

Useful options:

```bash
python3 benchmark/run.py --repeats 7 --warmup 1
python3 benchmark/run.py --bench prime_count
python3 benchmark/run.py --bench compile_project_10_files
python3 benchmark/run.py --bench incremental_rebuild_1_file
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
- `--compile-mode cold` clears artifacts between timed runs for cleaner cold-compile comparison.

For `incremental_rebuild_1_file`:
- each measured cycle does:
  1. full compile
  2. single-file source mutation
  3. second compile
- report includes `first mean` vs `second mean` per language.

## Notes

- This suite is CPU-focused and deterministic.
- Keep machine load stable for fair comparisons.
- For publication-quality results, pin CPU governor and run multiple sessions.
