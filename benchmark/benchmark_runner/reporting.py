from .specs import LANGUAGES
from .system import format_seconds


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

    incremental_kinds = {
        "incremental",
        "incremental_batch",
        "incremental_batch_synthetic_mega_graph",
        "incremental_mixed_synthetic_mega_graph",
        "incremental_batch_extreme_graph",
        "incremental_mixed_extreme_graph",
    }

    for bench in result["benchmarks"]:
        lines.append(f"## {bench['name']}")
        lines.append("")
        lines.append(bench["description"])
        if bench.get("compile_mode"):
            lines.extend(["", f"- compile mode: `{bench['compile_mode']}`"])
        lines.append("")

        if bench.get("kind") in incremental_kinds:
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

    return "\n".join(lines) + "\n"
