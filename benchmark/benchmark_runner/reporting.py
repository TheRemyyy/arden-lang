import csv
import io

from .specs import LANGUAGES
from .system import format_seconds

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


def _summary_table(benchmarks: list[dict]) -> list[str]:
    """Return a compact top-level summary table (Arden mean + speedup vs peers)."""
    lines: list[str] = [
        "## Summary",
        "",
        "Arden mean (s) and speedup relative to each peer language (higher is better for Arden).",
        "",
        "| Benchmark | Metric | Arden mean (s) | vs Rust | vs Go |",
        "|---|---|---:|---:|---:|",
    ]
    for bench in benchmarks:
        if _is_incremental(bench):
            metric_label = bench.get("phase_two_label") or "rebuild mean (s)"
            arden_mean = bench["languages"]["arden"]["second_stats"]["mean_s"]
        else:
            metric_label = "runtime mean (s)" if bench.get("kind") == "runtime" else "compile mean (s)"
            arden_mean = bench["languages"]["arden"]["stats"]["mean_s"]
        vs_rust = bench["speedup_vs_arden"].get("rust", float("nan"))
        vs_go = bench["speedup_vs_arden"].get("go", float("nan"))
        lines.append(
            f"| `{bench['name']}` | {metric_label} | {format_seconds(arden_mean)} "
            f"| {vs_rust:.3f}x | {vs_go:.3f}x |"
        )
    lines.append("")
    return lines


def build_markdown(result: dict) -> str:
    lines: list[str] = ["# Benchmark Report", ""]
    lines.append(f"- Generated: `{result['generated_at']}`")
    lines.append(f"- Repeats: `{result['repeats']}`")
    lines.append(f"- Warmup runs: `{result['warmup']}`")
    lines.append(f"- Arden opt level: `{result.get('arden_opt_level', 'n/a')}`")
    lines.append(f"- Arden target: `{result.get('arden_target') or 'native/default'}`")
    lines.append(f"- Arden phase timings: `{'enabled' if result.get('arden_timings') else 'disabled'}`")
    lines.append(f"- Compile mode: `{result.get('compile_mode', 'n/a')}`")
    lines.append("")

    benchmarks = result["benchmarks"]
    if benchmarks:
        lines.extend(_summary_table(benchmarks))

    lines.append("---")
    lines.append("")
    lines.append("## Methodology")
    lines.append("")
    lines.append(
        "Timings are wall-clock seconds measured with `time.perf_counter` around the "
        "subprocess call. Each benchmark runs the configured number of warmup iterations "
        "(excluded from stats) followed by the measured repeats. Stats are computed over "
        "measured repeats only. Cross-language correctness is verified by comparing a "
        "deterministic integer checksum printed to stdout by each binary."
    )
    lines.append("")
    lines.append(
        "**Cold compile**: build artifacts and Arden `.ardencache/` are deleted before every "
        "timed run. **Hot compile**: artifacts are kept between runs. **Incremental rebuild**: "
        "a full cold build is performed first (excluded from the rebuild stat), source files are "
        "mutated, then the rebuild is timed."
    )
    lines.append("")
    lines.append(
        "**Body-only mutations** append a comment (no change to exported function signatures). "
        "**API-surface mutations** add an extra ignored parameter to a shared function and "
        "propagate the call-site update to all dependents; output is unchanged (parameter is "
        "unused and passed as `0`)."
    )
    lines.append("")
    lines.append("---")
    lines.append("")

    for bench in benchmarks:
        lines.append(f"## {bench['name']}")
        lines.append("")
        lines.append(bench["description"])
        if bench.get("compile_mode"):
            lines.extend(["", f"- compile mode: `{bench['compile_mode']}`"])
        lines.append("")

        if _is_incremental(bench):
            phase_one_label = bench.get("phase_one_label", "first mean (s)")
            phase_two_label = bench.get("phase_two_label", "second mean (s)")
            ratio_label = bench.get("ratio_label", "second/first")
            lines.append(f"| Language | Checksum | {phase_one_label} | {phase_two_label} | {ratio_label} |")
            lines.append("|---|---:|---:|---:|---:|")
            for lang in LANGUAGES:
                entry = bench["languages"][lang]
                first_mean = entry["first_stats"]["mean_s"]
                second_mean = entry["second_stats"]["mean_s"]
                ratio = second_mean / first_mean if first_mean > 0 else float("inf")
                lines.append(
                    f"| {lang} | {entry['checksum']} | {format_seconds(first_mean)} | {format_seconds(second_mean)} | {ratio:.3f}x |"
                )
        else:
            lines.append("| Language | Checksum | min (s) | mean (s) | median (s) | stddev (s) | max (s) |")
            lines.append("|---|---:|---:|---:|---:|---:|---:|")
            for lang in LANGUAGES:
                entry = bench["languages"][lang]
                stats = entry["stats"]
                lines.append(
                    f"| {lang} | {entry['checksum']} | {format_seconds(stats['min_s'])} | {format_seconds(stats['mean_s'])} | {format_seconds(stats['median_s'])} | {format_seconds(stats['stddev_s'])} | {format_seconds(stats['max_s'])} |"
                )

        lines.extend(["", "| Relative to Arden (mean) | Value |", "|---|---:|"])
        for lang in LANGUAGES:
            if lang == "arden":
                continue
            lines.append(f"| {lang.capitalize()} speedup | {bench['speedup_vs_arden'][lang]:.3f}x |")
        lines.append("")

        for section in bench.get("arden_phase_timing_sections") or []:
            lines.append(f"{section['label']} (`--timings`, mean of measured runs):")
            lines.extend(["", "| Phase | Mean (ms) | Last counters |", "|---|---:|---|"])
            for phase in section.get("phases", []):
                counters = ", ".join(
                    f"{key}={value}" for key, value in phase.get("counters", {}).items()
                )
                lines.append(f"| {phase['label']} | {phase['mean_ms']:.3f} | {counters or '-'} |")
            lines.append("")

        arden_profile = bench["languages"]["arden"].get("profile_output")
        if arden_profile:
            lines.append("Arden `profile` output (build + run phase summary):")
            lines.append("")
            lines.append("```")
            lines.append(arden_profile.strip())
            lines.append("```")
            lines.append("")

    return "\n".join(lines) + "\n"


def build_csv(result: dict) -> str:
    """Return a CSV string with one row per language per benchmark.

    Columns:
    - generated_at, benchmark, kind, phase, language,
      min_s, mean_s, median_s, max_s, stddev_s, checksum
    """
    output = io.StringIO()
    writer = csv.writer(output)
    writer.writerow([
        "generated_at", "benchmark", "kind", "phase",
        "language", "min_s", "mean_s", "median_s", "max_s", "stddev_s", "checksum",
    ])
    generated_at = result.get("generated_at", "")
    for bench in result["benchmarks"]:
        bench_name = bench["name"]
        kind = bench.get("kind", "runtime")
        if _is_incremental(bench):
            for lang in LANGUAGES:
                entry = bench["languages"][lang]
                for phase, stats_key in (("cold_build", "first_stats"), ("rebuild", "second_stats")):
                    stats = entry[stats_key]
                    writer.writerow([
                        generated_at, bench_name, kind, phase, lang,
                        stats["min_s"], stats["mean_s"], stats["median_s"],
                        stats["max_s"], stats["stddev_s"],
                        entry["checksum"],
                    ])
        else:
            for lang in LANGUAGES:
                entry = bench["languages"][lang]
                stats = entry["stats"]
                writer.writerow([
                    generated_at, bench_name, kind, "timed", lang,
                    stats["min_s"], stats["mean_s"], stats["median_s"],
                    stats["max_s"], stats["stddev_s"],
                    entry["checksum"],
                ])
    return output.getvalue()
