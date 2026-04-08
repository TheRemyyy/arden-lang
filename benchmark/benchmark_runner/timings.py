import statistics
from pathlib import Path


def parse_build_timings(output: str) -> dict[str, dict]:
    lines = output.splitlines()
    try:
        start = lines.index("Build timings") + 1
    except ValueError:
        return {}

    timings: dict[str, dict] = {}
    for raw_line in lines[start:]:
        line = raw_line.strip()
        if not line or " ms" not in line:
            continue

        ms_index = line.rfind(" ms")
        number_start = ms_index - 1
        while number_start >= 0 and (
            line[number_start].isdigit() or line[number_start] == "."
        ):
            number_start -= 1
        number_start += 1
        label = line[:number_start].strip()
        if not label:
            continue
        try:
            ms_value = float(line[number_start:ms_index].strip())
        except ValueError:
            continue

        counters: dict[str, int] = {}
        counters_part = line[ms_index + len(" ms"):].strip()
        if counters_part:
            for item in counters_part.split(","):
                key, sep, value = item.strip().partition("=")
                if not sep:
                    continue
                try:
                    counters[key] = int(value)
                except ValueError:
                    continue

        timings[label] = {"ms": ms_value, "counters": counters}
    return timings


def summarize_arden_phase_timings(samples: list[dict[str, dict]]) -> list[dict]:
    if not samples:
        return []

    labels: list[str] = []
    for sample in samples:
        for label in sample:
            if label not in labels:
                labels.append(label)

    summary: list[dict] = []
    for label in labels:
        label_samples = [sample[label] for sample in samples if label in sample]
        if not label_samples:
            continue
        mean_ms = statistics.mean(item["ms"] for item in label_samples)
        summary.append(
            {
                "label": label,
                "mean_ms": mean_ms,
                "runs": len(label_samples),
                "counters": label_samples[-1]["counters"],
            }
        )
    return summary


def run_arden_profile(
    root: Path,
    bench_name: str,
    build_env: dict[str, str],
) -> str:
    """Run `arden profile <bench>.arden` and return the raw output.

    Returns an empty string when the compiler is unavailable or the command
    fails so that the benchmark run is not aborted by an optional feature.
    """
    from .system import run_cmd

    compiler = root / "target" / "release" / "arden"
    if not compiler.exists():
        return ""
    src = root / "benchmark" / "arden" / f"{bench_name}.arden"
    if not src.exists():
        return ""
    proc = run_cmd([str(compiler), "profile", str(src)], root, env=build_env)
    if proc.returncode != 0:
        return ""
    combined = (proc.stdout or "") + (proc.stderr or "")
    return combined.strip()
