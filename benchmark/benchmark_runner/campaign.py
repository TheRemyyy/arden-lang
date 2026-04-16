"""Full benchmark campaign orchestration for bulk data collection.

Campaign presets
----------------
quick       Fast sanity check (~2–5 min on a typical machine).
full        Publication-grade default (~15–30 min).
exhaustive  Full matrix including extreme/stress benchmarks (~60+ min).

Each preset is composed of named *stages*. Every stage maps to a single call
to ``run_selected_benchmarks`` with explicit repeat/warmup/flag settings.
Results are written per stage and then merged into a single combined report.
"""
from __future__ import annotations

import csv
import io
import json
from dataclasses import dataclass
from pathlib import Path

from .execution import run_selected_benchmarks
from .reporting import build_markdown, build_csv
from .specs import select_benchmarks
from .system import current_timestamp, run_cmd

# Mirrors the private set in reporting.py — kept in sync manually.
_INCREMENTAL_KINDS = {
    "incremental",
    "incremental_api_surface_cascade",
    "incremental_batch",
    "incremental_batch_synthetic_mega_graph",
    "incremental_mixed_synthetic_mega_graph",
    "incremental_batch_extreme_graph",
    "incremental_mixed_extreme_graph",
}


def _is_incremental(bench: dict) -> bool:
    return bench.get("kind") in _INCREMENTAL_KINDS


# ---------------------------------------------------------------------------
# Stage dataclass
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class CampaignStage:
    """Configuration for a single benchmarking stage within a campaign."""

    name: str
    description: str
    bench_names: tuple[str, ...]
    repeats: int = 5
    warmup: int = 2
    compile_mode: str = "hot"
    arden_timings: bool = False
    capture_profile: bool = False
    include_extreme: bool = False


# ---------------------------------------------------------------------------
# Benchmark group constants (reused across presets)
# ---------------------------------------------------------------------------

_RUNTIME_STANDARD: tuple[str, ...] = (
    "sum_loop",
    "prime_count",
    "matrix_mul",
    "fibonacci_recursive",
    "sort_heavy",
    "collatz_batch",
    "convolution_1d",
    "histogram_heavy",
    "prefix_sum_stream",
    "scatter_gather_mix",
    "stencil_2d",
)

_COMPILE_STANDARD: tuple[str, ...] = (
    "compile_project_starter_graph",
    "compile_project_flat_graph",
    "compile_project_layered_graph",
    "compile_project_dense_graph",
    "compile_project_worst_case_graph",
    "compile_project_mega_graph",
)

_INCREMENTAL_SMALL: tuple[str, ...] = (
    "incremental_rebuild_single_file",
    "incremental_rebuild_shared_core",
    "incremental_rebuild_api_surface_cascade",
)

_INCREMENTAL_LARGE: tuple[str, ...] = (
    "incremental_rebuild_large_project_batch",
    "incremental_rebuild_mega_graph_batch",
    "incremental_rebuild_mega_graph_mixed",
)

# ---------------------------------------------------------------------------
# Preset definitions
# ---------------------------------------------------------------------------

PRESETS: dict[str, list[CampaignStage]] = {
    # ------------------------------------------------------------------ quick
    "quick": [
        CampaignStage(
            name="runtime_quick",
            description=(
                "Quick runtime sanity check across the full standard runtime suite "
                "plus the heavy matrix stressor"
            ),
            bench_names=(*_RUNTIME_STANDARD, "matrix_mul_heavy"),
            repeats=3,
            warmup=1,
            capture_profile=True,
            include_extreme=True,  # matrix_mul_heavy is opt-in
        ),
        CampaignStage(
            name="compile_cold_quick",
            description=(
                "Quick realistic cold-build check across tiny, starter, and explicit "
                "flat/layered/dense/worst-case graph shapes"
            ),
            bench_names=(
                "compile_project_tiny_graph",
                "compile_project_starter_graph",
                "compile_project_flat_graph",
                "compile_project_layered_graph",
                "compile_project_dense_graph",
                "compile_project_worst_case_graph",
            ),
            repeats=3,
            warmup=0,
            compile_mode="cold",
        ),
        CampaignStage(
            name="compile_hot_quick",
            description=(
                "Quick hot-compile follow-up on starter plus flat/layered/dense/worst-case "
                "graphs with Arden per-phase timings"
            ),
            bench_names=(
                "compile_project_starter_graph",
                "compile_project_flat_graph",
                "compile_project_layered_graph",
                "compile_project_dense_graph",
                "compile_project_worst_case_graph",
            ),
            repeats=3,
            warmup=1,
            compile_mode="hot",
            arden_timings=True,
        ),
        CampaignStage(
            name="incremental_quick",
            description=(
                "Quick incremental rebuild check — body-only single-file edit, "
                "shared-core edit, and API-surface cascade on a 10-file project"
            ),
            bench_names=(
                "incremental_rebuild_single_file",
                "incremental_rebuild_shared_core",
                "incremental_rebuild_api_surface_cascade",
            ),
            repeats=3,
            warmup=1,
            arden_timings=True,
        ),
    ],

    # ------------------------------------------------------------------ full
    "full": [
        CampaignStage(
            name="runtime",
            description="All standard runtime CPU benchmarks (loop, primes, matrix, Fibonacci, sort)",
            bench_names=_RUNTIME_STANDARD,
            repeats=5,
            warmup=2,
        ),
        CampaignStage(
            name="runtime_heavy",
            description=(
                "Heavy CPU runtime benchmark (220×220 matrix multiply) "
                "with Arden profile capture"
            ),
            bench_names=("matrix_mul_heavy",),
            repeats=5,
            warmup=2,
            capture_profile=True,
            include_extreme=True,
        ),
        CampaignStage(
            name="compile_hot",
            description=(
                "Hot-compile benchmarks on starter, explicit graph shapes, and mega-graph projects "
                "with Arden per-phase timings"
            ),
            bench_names=_COMPILE_STANDARD,
            repeats=5,
            warmup=2,
            compile_mode="hot",
            arden_timings=True,
        ),
        CampaignStage(
            name="compile_cold",
            description=(
                "Cold-compile benchmarks on starter, explicit graph shapes, and mega-graph projects "
                "(build artifacts cleared between every timed run)"
            ),
            bench_names=_COMPILE_STANDARD,
            repeats=5,
            warmup=2,
            compile_mode="cold",
        ),
        CampaignStage(
            name="incremental_small",
            description=(
                "Small-project (10-file) incremental rebuild: "
                "body-only single-file edit, shared-core body edit, "
                "and API-surface cascade edit"
            ),
            bench_names=_INCREMENTAL_SMALL,
            repeats=5,
            warmup=2,
            arden_timings=True,
        ),
        CampaignStage(
            name="incremental_large",
            description=(
                "Large and mega-graph incremental rebuild benchmarks: "
                "batch body-only edits on 120-file and 1400-file projects, "
                "plus mixed leaf+API edits on the 1400-file mega-graph"
            ),
            bench_names=_INCREMENTAL_LARGE,
            repeats=5,
            warmup=2,
            arden_timings=True,
        ),
    ],

    # -------------------------------------------------------------- exhaustive
    "exhaustive": [
        CampaignStage(
            name="runtime",
            description=(
                "All standard runtime CPU benchmarks — article-grade "
                "(7 repeats, 3 warmup)"
            ),
            bench_names=_RUNTIME_STANDARD,
            repeats=7,
            warmup=3,
        ),
        CampaignStage(
            name="runtime_heavy",
            description=(
                "Heavy CPU runtime benchmark with profile capture — article-grade "
                "(7 repeats, 3 warmup)"
            ),
            bench_names=("matrix_mul_heavy",),
            repeats=7,
            warmup=3,
            capture_profile=True,
            include_extreme=True,
        ),
        CampaignStage(
            name="compile_hot",
            description=(
                "Hot-compile benchmarks with Arden per-phase timings — article-grade "
                "(7 repeats, 3 warmup)"
            ),
            bench_names=_COMPILE_STANDARD,
            repeats=7,
            warmup=3,
            compile_mode="hot",
            arden_timings=True,
        ),
        CampaignStage(
            name="compile_cold",
            description=(
                "Cold-compile benchmarks — article-grade "
                "(7 repeats, 3 warmup)"
            ),
            bench_names=_COMPILE_STANDARD,
            repeats=7,
            warmup=3,
            compile_mode="cold",
        ),
        CampaignStage(
            name="compile_extreme_hot",
            description=(
                "Extreme-graph hot-compile stress test on a 2200-file project "
                "with Arden per-phase timings"
            ),
            bench_names=("compile_project_extreme_graph",),
            repeats=5,
            warmup=2,
            compile_mode="hot",
            arden_timings=True,
            include_extreme=True,
        ),
        CampaignStage(
            name="compile_extreme_cold",
            description=(
                "Extreme-graph cold-compile stress test on a 2200-file project "
                "(artifacts cleared between every run)"
            ),
            bench_names=("compile_project_extreme_graph",),
            repeats=5,
            warmup=2,
            compile_mode="cold",
            include_extreme=True,
        ),
        CampaignStage(
            name="incremental_small",
            description=(
                "Small-project incremental rebuild: body-only, shared-core, "
                "and API-surface cascade — article-grade (7 repeats, 3 warmup)"
            ),
            bench_names=_INCREMENTAL_SMALL,
            repeats=7,
            warmup=3,
            arden_timings=True,
        ),
        CampaignStage(
            name="incremental_large",
            description=(
                "Large and mega-graph incremental rebuild benchmarks — article-grade "
                "(7 repeats, 3 warmup)"
            ),
            bench_names=_INCREMENTAL_LARGE,
            repeats=7,
            warmup=3,
            arden_timings=True,
        ),
        CampaignStage(
            name="incremental_extreme",
            description=(
                "Extreme-graph incremental rebuild stress tests on a 2200-file project: "
                "batch body-only edits and mixed leaf+API edits"
            ),
            bench_names=(
                "incremental_rebuild_extreme_graph_batch",
                "incremental_rebuild_extreme_graph_mixed",
            ),
            repeats=5,
            warmup=2,
            arden_timings=True,
            include_extreme=True,
        ),
    ],
}


# ---------------------------------------------------------------------------
# Combined reporting helpers
# ---------------------------------------------------------------------------

def _master_summary_table(campaign: dict) -> list[str]:
    """Return a compact master summary table spanning all stages."""
    lines: list[str] = [
        "## Master Summary",
        "",
        "Arden mean (s) and speedup relative to each peer. Higher speedup = better for Arden.",
        "",
        "| Stage | Benchmark | Metric | Arden mean (s) | vs Rust | vs Go |",
        "|---|---|---|---:|---:|---:|",
    ]
    for stage in campaign["stages"]:
        for bench in stage["benchmarks"]:
            if _is_incremental(bench):
                metric_label = bench.get("phase_two_label") or "rebuild mean (s)"
                arden_mean = bench["languages"]["arden"]["second_stats"]["mean_s"]
            else:
                metric_label = (
                    "runtime mean (s)" if bench.get("kind") == "runtime" else "compile mean (s)"
                )
                arden_mean = bench["languages"]["arden"]["stats"]["mean_s"]
            vs_rust = bench["speedup_vs_arden"].get("rust", float("nan"))
            vs_go = bench["speedup_vs_arden"].get("go", float("nan"))
            lines.append(
                f"| `{stage['name']}` | `{bench['name']}` | {metric_label} "
                f"| {arden_mean:.6f} | {vs_rust:.3f}x | {vs_go:.3f}x |"
            )
    lines.append("")
    return lines


def _stages_overview_table(campaign: dict) -> list[str]:
    lines: list[str] = [
        "## Campaign Stages",
        "",
        "| # | Stage | repeats | warmup | compile mode | arden timings | capture profile |",
        "|---|---|---:|---:|---|---|---|",
    ]
    for idx, stage in enumerate(campaign["stages"], start=1):
        lines.append(
            f"| {idx} | `{stage['name']}` | {stage['repeats']} | {stage['warmup']} "
            f"| `{stage['compile_mode']}` "
            f"| {'✓' if stage['arden_timings'] else '—'} "
            f"| {'✓' if stage['capture_profile'] else '—'} |"
        )
    lines.append("")
    return lines


def _extract_stage_body(stage_md: str) -> str:
    """Return the per-benchmark detail sections from a build_markdown output.

    Strips the H1 heading and metadata block (everything before the first
    ``---`` separator) so the content can be embedded inside a larger document
    without duplicate top-level headings.
    """
    sep = "\n---\n"
    first = stage_md.find(sep)
    if first < 0:
        return stage_md
    return stage_md[first + len(sep):]


def build_combined_markdown(campaign: dict) -> str:
    """Return a combined campaign markdown report."""
    lines: list[str] = [
        "# Arden Full Benchmark Campaign Report",
        "",
        f"- Generated: `{campaign['generated_at']}`",
        f"- Preset: `{campaign['preset']}`",
        f"- Arden opt level: `{campaign.get('arden_opt_level', '3')}`",
        f"- Arden target: `{campaign.get('arden_target') or 'native/default'}`",
        f"- Stages: `{len(campaign['stages'])}`",
        "",
    ]

    lines.extend(_stages_overview_table(campaign))
    lines.extend(_master_summary_table(campaign))

    lines += [
        "---",
        "",
        "## Methodology",
        "",
        "Timings are wall-clock seconds measured with `time.perf_counter` around the "
        "subprocess call. Each benchmark runs the configured warmup iterations (excluded "
        "from stats) followed by the measured repeats. Stats are computed over measured "
        "repeats only. Cross-language output correctness is verified by integer checksum "
        "comparison.",
        "",
        "**Cold compile**: build artifacts and `.ardencache/` cleared before every timed "
        "run. **Hot compile**: artifacts kept between runs. **Incremental rebuild**: cold "
        "build first (excluded from rebuild stat), source files mutated, then rebuild timed.",
        "",
        "**Body-only mutations** append a comment (no exported signature change). "
        "**API-surface mutations** add an extra ignored parameter to a shared function and "
        "propagate call-site updates; output checksum is unchanged.",
        "",
        "Per-stage raw results are available in the `stage_*.json` files alongside this "
        "document. Per-stage markdown detail reports are in `stage_*.md`.",
        "",
    ]

    for idx, stage in enumerate(campaign["stages"], start=1):
        lines.append(f"---")
        lines.append("")
        lines.append(f"## Stage {idx}: `{stage['name']}`")
        lines.append("")
        lines.append(stage["description"])
        lines.append("")
        lines.append(
            f"repeats=`{stage['repeats']}`, warmup=`{stage['warmup']}`, "
            f"compile_mode=`{stage['compile_mode']}`, "
            f"arden_timings=`{stage['arden_timings']}`, "
            f"capture_profile=`{stage['capture_profile']}`"
        )
        lines.append("")

        # Embed per-benchmark detail from build_markdown (strip the header/metadata block)
        stage_report = {
            "generated_at": campaign["generated_at"],
            "repeats": stage["repeats"],
            "warmup": stage["warmup"],
            "arden_opt_level": campaign.get("arden_opt_level", "3"),
            "arden_target": campaign.get("arden_target"),
            "arden_timings": stage["arden_timings"],
            "capture_profile": stage["capture_profile"],
            "compile_mode": stage["compile_mode"],
            "benchmarks": stage["benchmarks"],
        }
        stage_body = _extract_stage_body(build_markdown(stage_report))
        lines.append(stage_body.rstrip())
        lines.append("")

    lines.append("---")
    lines.append("")

    return "\n".join(lines) + "\n"


def build_combined_csv(campaign: dict) -> str:
    """Return a CSV string covering all stages and all benchmarks.

    Columns:
    - campaign_preset  The preset name (quick / full / exhaustive).
    - stage            Campaign stage name (e.g. runtime, compile_hot).
    - generated_at     ISO-style timestamp from the campaign run.
    - benchmark        Benchmark name (e.g. matrix_mul_heavy).
    - kind             Benchmark category: runtime / compile / incremental /
                       incremental_batch / incremental_api_surface_cascade / etc.
    - phase            Measurement phase within the benchmark:
                       ``timed``       – runtime or compile (single timed value).
                       ``cold_build``  – first phase of an incremental benchmark.
                       ``rebuild``     – second phase (the actual incremental rebuild).
    - language         arden / rust / go.
    - min_s … stddev_s Per-benchmark stats in seconds.
    - checksum         Integer checksum printed by the binary; used for cross-language
                       correctness verification.
    """
    output = io.StringIO()
    writer = csv.writer(output)
    writer.writerow([
        "campaign_preset",
        "stage",
        "generated_at",
        "benchmark",
        "kind",
        "phase",
        "language",
        "min_s",
        "mean_s",
        "median_s",
        "max_s",
        "stddev_s",
        "checksum",
    ])
    generated_at = campaign.get("generated_at", "")
    preset = campaign.get("preset", "")
    for stage in campaign["stages"]:
        stage_name = stage["name"]
        for bench in stage["benchmarks"]:
            bench_name = bench["name"]
            kind = bench.get("kind", "runtime")
            for lang, lang_data in bench["languages"].items():
                if _is_incremental(bench):
                    for phase, stats_key in (
                        ("cold_build", "first_stats"),
                        ("rebuild", "second_stats"),
                    ):
                        stats = lang_data.get(stats_key, {})
                        writer.writerow([
                            preset,
                            stage_name,
                            generated_at,
                            bench_name,
                            kind,
                            phase,
                            lang,
                            stats.get("min_s", ""),
                            stats.get("mean_s", ""),
                            stats.get("median_s", ""),
                            stats.get("max_s", ""),
                            stats.get("stddev_s", ""),
                            lang_data.get("checksum", ""),
                        ])
                else:
                    stats = lang_data.get("stats", {})
                    writer.writerow([
                        preset,
                        stage_name,
                        generated_at,
                        bench_name,
                        kind,
                        "timed",
                        lang,
                        stats.get("min_s", ""),
                        stats.get("mean_s", ""),
                        stats.get("median_s", ""),
                        stats.get("max_s", ""),
                        stats.get("stddev_s", ""),
                        lang_data.get("checksum", ""),
                    ])
    return output.getvalue()


def _write_campaign_readme(out_dir: Path, preset: str, timestamp: str, command: str) -> None:
    """Write a README.md describing the campaign results directory."""
    lines: list[str] = [
        "# Campaign Results",
        "",
        f"- Preset: `{preset}`",
        f"- Generated: `{timestamp}`",
        "",
        "## How to Reproduce",
        "",
        "```bash",
        command,
        "```",
        "",
        "## Files",
        "",
        "| File | Contents |",
        "|---|---|",
        "| `campaign_summary.json` | Full machine-readable report (all stages combined) |",
        "| `campaign_summary.md` | Human-readable combined report with master summary table |",
        "| `campaign_summary.csv` | Tabular export for charting — one row per language per phase |",
        "| `stage_NN_<name>.json` | Per-stage raw JSON results |",
        "| `stage_NN_<name>.md` | Per-stage markdown detail report |",
        "",
        "## Using the CSV",
        "",
        "The `stage` column in `campaign_summary.csv` identifies which campaign stage "
        "produced each row. Import into any spreadsheet application or use pandas:",
        "",
        "```python",
        "import pandas as pd",
        "df = pd.read_csv('campaign_summary.csv')",
        "# Filter to a specific stage",
        "df[df['stage'] == 'runtime']",
        "# Pivot to compare Arden vs Rust",
        "df.pivot_table(index='benchmark', columns='language', values='mean_s')",
        "```",
        "",
        "## Caveats",
        "",
        "- All timings are wall-clock seconds. Run on a quiet machine for publication numbers.",
        "- Extreme-graph benchmarks (`compile_extreme_*`, `incremental_extreme`) are "
        "  stress tests, not representative of typical projects.",
        "- The `stage` column lets you filter hot-compile vs cold-compile results separately.",
        "",
    ]
    (out_dir / "README.md").write_text("\n".join(lines), encoding="utf-8")


# ---------------------------------------------------------------------------
# Campaign runner
# ---------------------------------------------------------------------------

def run_campaign(
    preset: str,
    root: Path,
    out_dir: Path,
    build_env: dict[str, str],
    arden_opt_level: str = "3",
    arden_target: str | None = None,
    no_build: bool = False,
    dry_run: bool = False,
    command: str = "",
) -> dict:
    """Run the full benchmark campaign for the given preset.

    Args:
        preset:           One of ``"quick"``, ``"full"``, ``"exhaustive"``.
        root:             Repository root directory.
        out_dir:          Directory where all results will be written.
        build_env:        Extra environment variables (e.g. LLVM prefix).
        arden_opt_level:  Arden optimisation level (default ``"3"``).
        arden_target:     Optional cross-compilation target triple.
        no_build:         Skip ``cargo build --release``.
        dry_run:          Print the campaign plan without running anything.
        command:          Canonical command string recorded in the output README.

    Returns:
        The combined campaign result dict, or an empty dict for ``dry_run``.
    """
    if preset not in PRESETS:
        raise RuntimeError(
            f"Unknown preset: {preset!r}. Valid presets: {', '.join(PRESETS)}"
        )

    stages = PRESETS[preset]
    timestamp = current_timestamp()

    bar = "=" * 70
    print(f"\n{bar}", flush=True)
    print("Arden Full Benchmark Campaign", flush=True)
    print(f"  Preset   : {preset}", flush=True)
    print(f"  Stages   : {len(stages)}", flush=True)
    print(f"  Output   : {out_dir}", flush=True)
    print(f"  Generated: {timestamp}", flush=True)
    print(bar, flush=True)

    if dry_run:
        print("\nDRY RUN — campaign plan only, no benchmarks executed.\n", flush=True)
        for idx, stage in enumerate(stages, start=1):
            print(f"  Stage {idx:02d}: {stage.name}", flush=True)
            print(f"    {stage.description}", flush=True)
            print(f"    benchmarks : {', '.join(stage.bench_names)}", flush=True)
            print(
                f"    repeats={stage.repeats}, warmup={stage.warmup}, "
                f"compile_mode={stage.compile_mode}, "
                f"arden_timings={stage.arden_timings}, "
                f"capture_profile={stage.capture_profile}",
                flush=True,
            )
            print("", flush=True)
        return {}

    out_dir.mkdir(parents=True, exist_ok=True)
    bin_dir = out_dir / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)

    if not no_build:
        print("\nBuilding target/release/arden...", flush=True)
        proc = run_cmd(["cargo", "build", "--release"], root, env=build_env)
        if proc.returncode != 0:
            raise RuntimeError(f"Failed to build Arden:\n{proc.stderr}")
        print("Built target/release/arden", flush=True)

    campaign: dict = {
        "generated_at": timestamp,
        "preset": preset,
        "arden_opt_level": arden_opt_level,
        "arden_target": arden_target,
        "stages": [],
    }

    total_stages = len(stages)
    for stage_idx, stage in enumerate(stages, start=1):
        print(f"\n{bar}", flush=True)
        print(f"Stage {stage_idx}/{total_stages}: {stage.name}", flush=True)
        print(f"  {stage.description}", flush=True)
        print(f"  benchmarks : {', '.join(stage.bench_names)}", flush=True)
        print(
            f"  repeats={stage.repeats}, warmup={stage.warmup}, "
            f"compile_mode={stage.compile_mode}, "
            f"arden_timings={stage.arden_timings}, "
            f"capture_profile={stage.capture_profile}",
            flush=True,
        )
        print(bar, flush=True)

        selected = []
        for name in stage.bench_names:
            specs = select_benchmarks(name, stage.include_extreme)
            selected.extend(specs)

        benchmarks = run_selected_benchmarks(
            selected,
            root,
            bin_dir,
            build_env,
            arden_opt_level,
            arden_target,
            stage.compile_mode,
            stage.warmup,
            stage.repeats,
            stage.arden_timings,
            stage.capture_profile,
        )

        stage_result = {
            "name": stage.name,
            "description": stage.description,
            "repeats": stage.repeats,
            "warmup": stage.warmup,
            "compile_mode": stage.compile_mode,
            "arden_timings": stage.arden_timings,
            "capture_profile": stage.capture_profile,
            "benchmarks": benchmarks,
        }

        stage_file = out_dir / f"stage_{stage_idx:02d}_{stage.name}.json"
        stage_file.write_text(json.dumps(stage_result, indent=2), encoding="utf-8")

        # Per-stage markdown detail report
        stage_md_report = {
            "generated_at": timestamp,
            "repeats": stage.repeats,
            "warmup": stage.warmup,
            "arden_opt_level": arden_opt_level,
            "arden_target": arden_target,
            "arden_timings": stage.arden_timings,
            "capture_profile": stage.capture_profile,
            "compile_mode": stage.compile_mode,
            "benchmarks": benchmarks,
        }
        stage_md_file = out_dir / f"stage_{stage_idx:02d}_{stage.name}.md"
        stage_md_file.write_text(build_markdown(stage_md_report), encoding="utf-8")

        print(f"\n  Wrote: {stage_file}", flush=True)
        print(f"  Wrote: {stage_md_file}", flush=True)

        campaign["stages"].append(stage_result)

    # Combined outputs
    json_out = out_dir / "campaign_summary.json"
    md_out = out_dir / "campaign_summary.md"
    csv_out = out_dir / "campaign_summary.csv"

    json_out.write_text(json.dumps(campaign, indent=2), encoding="utf-8")
    md_out.write_text(build_combined_markdown(campaign), encoding="utf-8")
    csv_out.write_text(build_combined_csv(campaign), encoding="utf-8")
    _write_campaign_readme(out_dir, preset, timestamp, command)

    print(f"\n{bar}", flush=True)
    print("Campaign complete.", flush=True)
    print(f"  Wrote: {json_out}", flush=True)
    print(f"  Wrote: {md_out}", flush=True)
    print(f"  Wrote: {csv_out}", flush=True)
    print(f"  Wrote: {out_dir / 'README.md'}", flush=True)
    print(bar, flush=True)

    return campaign
