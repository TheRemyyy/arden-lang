# Arden Benchmark Suite

This directory contains the benchmark harness for Arden. It compares Arden, Rust, and Go on equivalent workloads: CPU-heavy runtime programs, cold and hot project compiles, and incremental rebuild scenarios.

If a result matters, it should be reproducible from the commands in this file.

---

## Choose Your Workflow

| Goal | Command | Output |
| :--- | :--- | :--- |
| **Smoke test** — does the harness work at all? | `python3 benchmark/run.py --bench matrix_mul_heavy --repeats 1 --warmup 0 --no-build` | `results/latest.{json,md}` |
| **Quick check** — sanity pass across several benchmarks | `python3 benchmark/full_campaign.py --preset quick --no-build` | `results/campaign_<ts>/` |
| **Full campaign** — publication-grade data collection | `python3 benchmark/full_campaign.py --preset full --no-build` | `results/campaign_<ts>/` |
| **Article-grade / exhaustive** — full matrix + stress benchmarks | `python3 benchmark/full_campaign.py --preset exhaustive --no-build` | `results/campaign_<ts>/` |
| **Single benchmark with phase breakdowns** | `python3 benchmark/run.py --bench compile_project_starter_graph --compile-mode hot --arden-timings --repeats 5 --warmup 2 --no-build` | `results/latest.{json,md}` |
| **Single benchmark with profile capture** | `python3 benchmark/run.py --bench fibonacci_recursive --capture-profile --repeats 3 --warmup 1 --no-build` | `results/latest.{json,md}` |
| **Single benchmark with CSV export** | `python3 benchmark/run.py --repeats 5 --warmup 2 --output-csv --no-build` | `results/latest.{json,md,csv}` |

Build the compiler once before running benchmarks (if not already built):

```bash
LLVM_SYS_211_PREFIX=/usr/lib/llvm-21 cargo build --release
```

---

## Entrypoints

There are two scripts. Use the one that matches your goal.

### `benchmark/run.py` — Single benchmark runs

Runs one benchmark (or the full default suite) in a single pass. Outputs always go to `benchmark/results/latest.{json,md}` (and optionally `latest.csv`).

```bash
# Show full CLI
python3 benchmark/run.py --help

# One runtime benchmark, fast
python3 benchmark/run.py --bench matrix_mul_heavy --repeats 3 --warmup 1 --no-build

# One compile benchmark in cold mode with phase timings
python3 benchmark/run.py \
  --bench compile_project_starter_graph \
  --compile-mode cold \
  --arden-timings \
  --repeats 5 --warmup 2 \
  --no-build

# One incremental benchmark
python3 benchmark/run.py \
  --bench incremental_rebuild_large_project_batch \
  --repeats 5 --warmup 2 \
  --no-build

# Full default suite with CSV export
python3 benchmark/run.py --repeats 5 --warmup 2 --output-csv --no-build
```

### `benchmark/full_campaign.py` — Multi-stage campaigns

Orchestrates multiple benchmark stages across all groups, writes a timestamped results directory, and produces a combined CSV suitable for charting.

```bash
# Preview the plan without running anything
python3 benchmark/full_campaign.py --preset full --dry-run

# Quick sanity check (~2–5 min, 3 stages)
python3 benchmark/full_campaign.py --preset quick --no-build

# Full publication-grade campaign (~15–30 min, 6 stages)
python3 benchmark/full_campaign.py --preset full --no-build

# Exhaustive campaign including 2 200-file stress benchmarks (~60+ min, 9 stages)
python3 benchmark/full_campaign.py --preset exhaustive --no-build
```

| Preset | Stages | Estimated time | Use case |
| :--- | ---: | :--- | :--- |
| `quick` | 3 | ~2–5 min | Harness validation / sanity check |
| `full` | 6 | ~15–30 min | Publication-grade data collection |
| `exhaustive` | 9 | ~60+ min | Full matrix + extreme stress benchmarks |

**What the `full` preset measures:**

| Stage | Benchmarks | Repeats | Warmup | Extras |
| :--- | :--- | ---: | ---: | :--- |
| `runtime` | sum_loop, prime_count, matrix_mul, fibonacci_recursive, sort_heavy | 5 | 2 | — |
| `runtime_heavy` | matrix_mul_heavy (220×220) | 5 | 2 | `--capture-profile` |
| `compile_hot` | starter graph, mega-graph | 5 | 2 | `--arden-timings` |
| `compile_cold` | starter graph, mega-graph | 5 | 2 | — |
| `incremental_small` | single-file, shared-core, API-surface cascade | 5 | 2 | `--arden-timings` |
| `incremental_large` | large-batch, mega-graph-batch, mega-graph-mixed | 5 | 2 | `--arden-timings` |

---

## Benchmark Groups

### Runtime

Measure generated code quality and runtime overhead. Arden, Rust, and Go compile the same algorithm and the execution time is compared.

| Benchmark | What it measures |
| :--- | :--- |
| `sum_loop` | Integer-heavy loop with LCG accumulation |
| `prime_count` | Trial-division prime sieve |
| `matrix_mul` | Dense 100×100 matrix multiply |
| `matrix_mul_heavy` | Dense 220×220 matrix multiply (heavier, opt-in) |
| `fibonacci_recursive` | Naive recursive fib(38) — ~126 M function calls |
| `sort_heavy` | Insertion sort on 20 000 pseudo-random integers |

All three languages use the same algorithm so the comparison reflects code quality, not algorithm choice. A checksum is verified to confirm equivalent output.

### Compile

Measure the full frontend-to-native-binary pipeline cost.

- **hot** — build artifacts and Arden cache are preserved between runs (repeat-build cost)
- **cold** — all artifacts deleted before each timed run (true from-scratch cost)

| Benchmark | Project size |
| :--- | :--- |
| `compile_project_starter_graph` | 10-file starter graph |
| `compile_project_mega_graph` | 1 400-file synthetic dependency graph |
| `compile_project_extreme_graph` | 2 200-file extreme graph (opt-in) |

When running the full default suite, hot and cold variants are generated automatically. To run one in a specific mode:

```bash
python3 benchmark/run.py \
  --bench compile_project_starter_graph \
  --compile-mode cold \
  --repeats 5 --warmup 2 \
  --arden-timings \
  --no-build
```

### Incremental Rebuild

Measure how much work each language skips when re-building after source changes.

| Benchmark | Edit type | Project size |
| :--- | :--- | :--- |
| `incremental_rebuild_single_file` | Body-only (comment) on 1 leaf | 10-file |
| `incremental_rebuild_shared_core` | Body-only (comment) on shared core | 10-file |
| `incremental_rebuild_api_surface_cascade` | API-surface change on shared core → all 10 dependents re-check | 10-file |
| `incremental_rebuild_large_project_batch` | Body-only on 10 of 120 files | 120-file |
| `incremental_rebuild_mega_graph_batch` | Body-only on many files | 1 400-file |
| `incremental_rebuild_mega_graph_mixed` | Mixed body + API-surface | 1 400-file |

**Body-only vs API-surface explained:**

*Body-only mutations* append a comment to source files. No exported function signature changes. Arden tracks an *API fingerprint* separately from the *source fingerprint*, so dependents are not re-typechecked when only the body changes.

*API-surface mutations* change a shared function's signature (extra parameter) and update all call sites. Arden detects the API fingerprint change and must re-typecheck all dependents. Comparing the two benchmarks at the same project size shows the cost of API propagation vs pure body invalidation.

```bash
# Body-only — dependents can be skipped
python3 benchmark/run.py --bench incremental_rebuild_shared_core --repeats 5 --warmup 1 --no-build

# API-surface cascade — all dependents must re-check
python3 benchmark/run.py --bench incremental_rebuild_api_surface_cascade --repeats 5 --warmup 1 --no-build
```

---

## Arden-Specific Instrumentation

### `--arden-timings` — per-phase breakdown

Pass `--arden-timings` to any `run.py` command (or to full_campaign.py stages that set it automatically). The flag forwards `--timings` to `arden build` and records lex / parse / type-check / borrow-check / specialization / codegen / link time per run. Results are averaged over the measured repeats and appear in both the markdown and JSON report.

Example report table produced:

```
| Phase          | Mean (ms) | Last counters     |
|----------------|----------:|-------------------|
| lex            |     1.234 | tokens=12345      |
| parse          |     3.456 | nodes=6789        |
| type_check     |     8.901 | -                 |
| borrow_check   |     2.345 | -                 |
| specialization |     0.123 | -                 |
| codegen        |    12.567 | -                 |
| link           |     4.890 | -                 |
```

When `--arden-timings` is used on hot-compile or incremental benchmarks, the counters column can show cache-reuse signals (e.g. `cached=N`). A significantly shorter codegen phase on the hot run compared to the cold run is direct evidence of cache reuse.

### `--capture-profile` — `arden profile` output

```bash
python3 benchmark/run.py \
  --bench fibonacci_recursive \
  --capture-profile \
  --repeats 3 --warmup 1 \
  --no-build
```

For each runtime benchmark, this runs `arden profile <bench>.arden` once before the timed measurements and includes the phase summary in the report. The profile output shows the build-vs-run split and per-phase timings for a single execution.

---

## Output Files

### `run.py` — `benchmark/results/latest.*`

Every `run.py` invocation overwrites the `latest.*` files:

| File | Contents |
| :--- | :--- |
| `latest.json` | Full machine-readable report (all benchmarks, all languages, stats, phase timings) |
| `latest.md` | Human-readable markdown with summary table, per-benchmark detail, phase timings, profile output |
| `latest.csv` | Tabular export for charting (with `--output-csv` only) |

### `full_campaign.py` — `benchmark/results/campaign_<YYYYMMDD_HHMMSS>/`

Each campaign run creates a timestamped directory:

```
benchmark/results/campaign_<YYYYMMDD_HHMMSS>/
├── README.md                  # Exact command used + how to reproduce
├── campaign_summary.json      # All stages combined (machine-readable)
├── campaign_summary.md        # Master summary + per-stage detail tables
├── campaign_summary.csv       # One row per language per phase across all stages
├── stage_01_runtime.json
├── stage_01_runtime.md
├── stage_02_runtime_heavy.json
├── stage_02_runtime_heavy.md
└── ...
```

The CSV columns are:

```
campaign_preset, stage, generated_at, benchmark, kind, phase,
language, min_s, mean_s, median_s, max_s, stddev_s, checksum
```

The `stage` column lets you filter hot-compile vs cold-compile, or runtime vs incremental, without manual post-processing. Import into Excel, Google Sheets, or `pandas.read_csv` for charts.

### JSON report structure (`latest.json` / `stage_NN_*.json`)

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
          "samples_s": [0.012, 0.011],
          "stats": { "min_s": 0.011, "mean_s": 0.012, "median_s": 0.012, "max_s": 0.013, "stddev_s": 0.0005 },
          "profile_output": "..."
        },
        "rust": {},
        "go": {}
      },
      "speedup_vs_arden": { "rust": 2.5, "go": 3.1 },
      "arden_phase_timing_sections": []
    }
  ]
}
```

---

## Requirements

- `python3` (3.10+)
- `rustc` (any recent stable)
- `go` (1.21+)
- Arden binary at `target/release/arden` (built with `cargo build --release`)
- LLVM 21 (`LLVM_SYS_211_PREFIX=/usr/lib/llvm-21`)
- `mold` on Linux, or LLVM `lld` on macOS/Windows

No proprietary tools required. If the Arden binary is missing and `--no-build` is not set, the harness builds it automatically.

---

## Publication-Grade Run

For numbers you intend to publish:

1. **Use a stable machine** — avoid shared CI runners. A dedicated Linux box or reserved cloud instance with a fixed CPU family is ideal.
2. **Reduce background load** — close browsers, suspend cron jobs, disable turbo-boost if reproducibility matters more than peak speed.
3. **Use at least 5 repeats and 2 warmup runs.**
4. **Record the machine spec** — CPU model, RAM, OS, kernel version, LLVM version, rustc version, go version.
5. **Use `--arden-timings`** to capture per-phase breakdowns so readers understand where time is spent.
6. **Use `--output-csv`** or the campaign CSV to produce chartable data.

Recommended single-pass command:

```bash
python3 benchmark/run.py \
  --repeats 7 --warmup 2 \
  --no-build \
  --arden-timings \
  --capture-profile \
  --output-csv
```

Or use the campaign runner for structured, reproducible bulk data:

```bash
python3 benchmark/full_campaign.py --preset full --no-build
```

Include in any published result:

- Git commit hash (`git rev-parse HEAD`)
- Date
- Machine description (CPU, RAM, OS, LLVM, rustc, go)
- The exact command used
- Whether runs were cold, hot, or incremental (the report records this per benchmark)

---

## Local Snapshot

Verified locally on `2026-04-07` after building `target/release/arden` from this repository. Single-sample runs (`--repeats 1 --warmup 0`) — treat as a quick snapshot, not a publication-grade claim.

| Benchmark | Arden | Rust | Go |
| :--- | ---: | ---: | ---: |
| `matrix_mul_heavy` runtime | `0.012186s` | `0.027221s` | `0.041095s` |
| `incremental_rebuild_large_project_batch` hot rebuild after 10 edits | `0.060284s` | `1.012121s` | `0.891075s` |

```bash
python3 benchmark/run.py --bench matrix_mul_heavy --repeats 1 --warmup 0 --no-build
python3 benchmark/run.py --bench incremental_rebuild_large_project_batch --repeats 1 --warmup 0 --no-build
```

The synthetic and extreme graph compile benchmarks are available in the harness but were excluded from this snapshot because the Go side was killed during quick runs on this machine. They are useful stress tests, not clean three-way headline numbers.

---

## Methodology Caveats

- **Toolchain defaults differ.** Rust uses `rustc -C opt-level=3 -C target-cpu=native` (single-file, no Cargo). Go uses `go build -trimpath`. Arden uses `opt_level = "3"` via `arden.toml`. Linkers differ per platform (mold/lld). These are the defaults each language recommends for optimised builds.

- **Rust incremental is conservative here.** Rust incremental benchmarks use `rustc main.rs` (no Cargo), which recompiles from scratch on every change. Cargo with incremental compilation would be faster. The Arden vs Rust incremental comparison is therefore generous to Arden. Adapt the harness to use `cargo build` with a `Cargo.toml` for a fairer comparison.

- **Synthetic graphs are not realistic production workloads.** The mega-graph and extreme-graph compile benchmarks use auto-generated projects with dense dependency chains. They are stress tests, not claims about typical large codebases.

- **Wall-clock vs CPU time.** All timings are wall-clock. On a loaded machine, variance increases. Use `--warmup 2` or higher and run on a quiet machine for publication numbers.

- **Cache state matters.** Hot compile and incremental benchmarks assume the cache is populated from earlier runs in the same cycle. Cold benchmarks explicitly clear all artifacts. Read the `compile_mode` field in the report to understand which applies.

- **Checksum verification.** All three languages must produce the same integer checksum to pass. This ensures the benchmarks measure equivalent computations.

---

## Debugging

```bash
# Confirm the Arden binary is present
ls -la target/release/arden

# Test Arden can run a benchmark file directly
./target/release/arden run benchmark/arden/sum_loop.arden

# Test Rust compilation
rustc -C opt-level=3 benchmark/rust/sum_loop.rs -o /tmp/sum_loop_rust && /tmp/sum_loop_rust

# Test Go compilation
go build -o /tmp/sum_loop_go benchmark/go/sum_loop.go && /tmp/sum_loop_go
```

If results vary wildly between runs, check system load and reduce background processes.
