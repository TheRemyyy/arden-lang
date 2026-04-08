# Arden Performance Measurement Guide

This guide explains how to run a serious, publication-grade performance campaign against Arden and how to interpret the results for an article.

---

## Quick Start

```bash
# Build the compiler first
LLVM_SYS_211_PREFIX=/usr/lib/llvm-21 cargo build --release

# Quick sanity check (1 runtime benchmark, 3 repeats, 1 warmup)
python3 benchmark/run.py --bench matrix_mul_heavy --repeats 3 --warmup 1 --no-build

# Full default suite (runtime + compile hot/cold + incremental)
python3 benchmark/run.py --repeats 5 --warmup 2 --no-build

# Full suite with CSV export (for charts)
python3 benchmark/run.py --repeats 5 --warmup 2 --no-build --output-csv

# Full suite with per-phase Arden timings
python3 benchmark/run.py --repeats 5 --warmup 2 --no-build --arden-timings

# Full suite with Arden profile capture (runtime benchmarks)
python3 benchmark/run.py --repeats 5 --warmup 2 --no-build --capture-profile --output-csv
```

Reports are written to:

- `benchmark/results/latest.json` — machine-readable full report
- `benchmark/results/latest.md` — human-readable markdown report
- `benchmark/results/latest.csv` — tabular data for charting (with `--output-csv`)

---

## Publication-Grade Run

For numbers you intend to publish:

1. **Use a stable machine** — avoid shared CI runners. A dedicated Linux box or a reserved cloud instance with a fixed CPU family is ideal.
2. **Reduce background load** — close browsers, suspend cron jobs, disable turbo-boost if reproducibility matters more than peak speed.
3. **Use at least 5 repeats and 2 warmup runs** (`--repeats 5 --warmup 2`).
4. **Record the machine spec** — CPU model, RAM, OS, kernel version, LLVM version, rustc version, go version.
5. **Use `--arden-timings`** to capture per-phase breakdowns so readers understand where time is spent.
6. **Use `--output-csv`** to produce chartable data.

Recommended command:

```bash
python3 benchmark/run.py \
  --repeats 7 --warmup 2 \
  --no-build \
  --arden-timings \
  --capture-profile \
  --output-csv
```

Include in any published result:

- Git commit hash (`git rev-parse HEAD`)
- Date
- Machine description (CPU, RAM, OS, LLVM, rustc, go)
- The exact command used
- Whether runs were cold, hot, or incremental (the report records this per benchmark)

---

## Benchmark Groups

### Runtime

Measure generated code quality and runtime overhead. These benchmarks compile a small self-contained program and measure execution time across Arden, Rust, and Go.

| Benchmark | What it measures |
| :--- | :--- |
| `sum_loop` | Integer-heavy loop with LCG accumulation |
| `prime_count` | Trial-division prime sieve |
| `matrix_mul` | Dense 100×100 matrix multiply |
| `matrix_mul_heavy` | Dense 220×220 matrix multiply (opt-in, heavier) |
| `fibonacci_recursive` | Naive recursive fib(38) — ~126M function calls |
| `sort_heavy` | Insertion sort on 20 000 pseudo-random integers |

All three languages use the same algorithm where possible (e.g. insertion sort) so the comparison reflects code quality, not algorithm choice.

**Run only runtime benchmarks:**

```bash
python3 benchmark/run.py --bench sum_loop --repeats 5 --warmup 2 --no-build
python3 benchmark/run.py --bench fibonacci_recursive --repeats 5 --warmup 2 --no-build
python3 benchmark/run.py --bench sort_heavy --repeats 5 --warmup 2 --no-build
```

---

### Compile

Measure the full frontend-to-native-binary pipeline cost. Two modes:

- **hot** — build artifacts and Arden cache are preserved between runs (measures repeat-build cost)
- **cold** — all artifacts deleted before each timed run (measures true from-scratch cost)

| Benchmark | Project size |
| :--- | :--- |
| `compile_project_starter_graph` | 10-file starter graph |
| `compile_project_mega_graph` | 1 400-file synthetic dependency graph |
| `compile_project_extreme_graph` | 2 200-file extreme graph (opt-in) |

When running the full default suite, hot and cold variants are generated automatically. To run a single benchmark in a specific mode:

```bash
python3 benchmark/run.py \
  --bench compile_project_starter_graph \
  --compile-mode cold \
  --repeats 5 --warmup 2 \
  --arden-timings \
  --no-build
```

The `--arden-timings` flag passes `--timings` to `arden build` and records per-phase time (lex, parse, type-check, borrow-check, specialization, codegen, link) in the report.

---

### Incremental Rebuild

Measure how much work each language can skip when re-building after source changes.

| Benchmark | Edit type | Project size |
| :--- | :--- | :--- |
| `incremental_rebuild_single_file` | Body-only (comment) on 1 leaf | 10-file |
| `incremental_rebuild_shared_core` | Body-only (comment) on shared core | 10-file |
| `incremental_rebuild_api_surface_cascade` | API-surface on shared core → all 10 dependents recheck | 10-file |
| `incremental_rebuild_large_project_batch` | Body-only (comment) on 10 of 120 files | 120-file |
| `incremental_rebuild_mega_graph_batch` | Body-only on many files | 1 400-file |
| `incremental_rebuild_mega_graph_mixed` | Mixed body + API-surface | 1 400-file |

#### Interpreting body-only vs API-surface results

**Body-only mutations** (all `incremental_rebuild_*_batch` and `incremental_rebuild_shared_core`): append a comment to source files. No exported function signature changes. Arden's cache tracks an *API fingerprint* separately from the *source fingerprint*. When only body content changes, Arden can skip re-typechecking dependents.

**API-surface mutations** (`incremental_rebuild_api_surface_cascade`): add an extra `_api_extra: Integer` parameter to a shared `core_blend` function and propagate call-site updates to all dependent files. Output is unchanged (parameter unused, passed as `0`). Arden's cache detects the API fingerprint change and must re-typecheck all dependents.

Comparing these two benchmarks at the same project size reveals the cost of API propagation vs pure body invalidation.

```bash
# Body-only on shared core
python3 benchmark/run.py --bench incremental_rebuild_shared_core --repeats 5 --warmup 1 --no-build

# API-surface cascade on same-sized project
python3 benchmark/run.py --bench incremental_rebuild_api_surface_cascade --repeats 5 --warmup 1 --no-build
```

---

## Arden-Specific Instrumentation

### `--timings` phase breakdown

```bash
python3 benchmark/run.py \
  --bench compile_project_starter_graph \
  --compile-mode hot \
  --arden-timings \
  --repeats 5 --warmup 2 \
  --no-build
```

The report will include a table like:

```
| Phase         | Mean (ms) | Last counters          |
|---------------|----------:|------------------------|
| lex           | 1.234     | tokens=12345           |
| parse         | 3.456     | nodes=6789             |
| type_check    | 8.901     | -                      |
| borrow_check  | 2.345     | -                      |
| specialization| 0.123     | -                      |
| codegen       | 12.567    | -                      |
| link          | 4.890     | -                      |
```

Phase timings are averaged over the measured repeats. They appear in both the markdown report and the JSON artifact.

### `arden profile` capture

```bash
python3 benchmark/run.py \
  --bench fibonacci_recursive \
  --capture-profile \
  --repeats 3 --warmup 1 \
  --no-build
```

For each runtime benchmark, this runs `arden profile <bench>.arden` once before measurements and includes the phase summary in the report. The profile output typically shows the build-vs-run split and per-phase timings for a single execution.

### Cache hit/miss evidence

When `--arden-timings` is enabled on hot compile or incremental benchmarks, the counters column in the phase timing table can show cache reuse signals (e.g., `cached=N`). A significantly shorter codegen phase on the hot run compared to the cold run is direct evidence of cache reuse.

---

## Output Files

### `latest.json`

Full machine-readable report. Structure:

```json
{
  "generated_at": "...",
  "repeats": 5,
  "warmup": 2,
  "arden_opt_level": "3",
  "arden_timings": true,
  "benchmarks": [
    {
      "name": "...",
      "kind": "runtime",
      "languages": {
        "arden": {
          "checksum": 12345,
          "samples_s": [0.012, 0.011, ...],
          "stats": { "min_s": 0.011, "mean_s": 0.012, "median_s": 0.012, "max_s": 0.013, "stddev_s": 0.0005 },
          "profile_output": "..."
        },
        "rust": { ... },
        "go": { ... }
      },
      "speedup_vs_arden": { "rust": 2.5, "go": 3.1 },
      "arden_phase_timing_sections": [ ... ]
    }
  ]
}
```

### `latest.md`

Human-readable markdown. Includes:

- Configuration header
- **Summary table** (Arden mean + speedups at a glance)
- **Methodology** section explaining cold/hot/incremental and mutation types
- Per-benchmark detail tables with min/mean/median/max/stddev
- Incremental benchmarks: cold build mean vs rebuild mean + ratio
- Arden phase timing tables (when `--arden-timings` enabled)
- Arden profile output (when `--capture-profile` enabled)

### `latest.csv`

Tabular data for charting. Columns:

```
generated_at, benchmark, kind, phase, language, min_s, mean_s, median_s, max_s, stddev_s, checksum
```

For incremental benchmarks, two rows per language are emitted: one for `cold_build` and one for `rebuild`. Import into Excel, Google Sheets, or Python (`pandas.read_csv`) for visualisation.

---

## Requirements

- `python3` (3.10+)
- `rustc` (any recent stable)
- `go` (1.21+)
- Arden binary at `target/release/arden` (or let the harness build it)
- LLVM 21 (`LLVM_SYS_211_PREFIX=/usr/lib/llvm-21`)
- `mold` (Linux) or LLVM `lld` (macOS/Windows)

No proprietary tools required.

---

## Caveats and Honest Interpretation

- **Toolchain differences matter.** Rust is compiled with `rustc -C opt-level=3 -C target-cpu=native` (single-file). Go uses `go build -trimpath`. Arden uses `opt_level = "3"` via `arden.toml`. Linkers differ per platform (mold/lld). These are the defaults each language recommends for optimised builds.

- **Single-file vs project-mode rustc.** Rust incremental benchmarks use `rustc main.rs` (no Cargo), which recompiles from scratch on every change. Cargo with incremental compilation would be faster. This makes the Arden vs Rust incremental comparison generous to Arden. If you want a fairer Rust comparison, adapt the harness to use `cargo build` with a `Cargo.toml` for Rust.

- **Synthetic graphs are not representative of production codebases.** The mega-graph and extreme-graph compile benchmarks use auto-generated projects with dense dependency chains. These are stress tests, not realistic workloads.

- **Wall-clock vs CPU time.** All timings are wall-clock. On a loaded machine, variance increases. Use `--warmup 2` or higher and run on a quiet machine for publication numbers.

- **Cache state matters.** Hot compile and incremental benchmarks assume the cache is populated from earlier runs in the same cycle. Cold benchmarks explicitly clear all artifacts. Read the `compile_mode` field in the report to understand which applies.

- **Checksum verification.** All three languages must produce the same integer checksum to pass. This ensures the benchmarks measure equivalent computations, not different programs.

---

## Debugging

If a benchmark fails:

```bash
# Check Arden compiler is present
ls -la target/release/arden

# Test Arden can run a benchmark file
./target/release/arden run benchmark/arden/sum_loop.arden

# Test Rust compilation
rustc -C opt-level=3 benchmark/rust/sum_loop.rs -o /tmp/sum_loop_rust && /tmp/sum_loop_rust

# Test Go compilation
go build -o /tmp/sum_loop_go benchmark/go/sum_loop.go && /tmp/sum_loop_go
```

If results vary wildly between runs, check system load and reduce background processes.
