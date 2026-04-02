#!/usr/bin/env python3
import argparse
import json
import os
import shutil
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List


@dataclass(frozen=True)
class BenchmarkSpec:
    name: str
    description: str
    kind: str = "runtime"
    default_enabled: bool = True


@dataclass(frozen=True)
class SyntheticGraphConfig:
    file_count: int
    funcs_per_file: int
    mutate_count: int
    max_deps: int
    group_size: int
    mixed_leaf_edits: int
    mixed_group_edits: int


@dataclass(frozen=True)
class TimedCompileResult:
    elapsed_s: float
    stdout: str
    stderr: str


BENCHMARKS: List[BenchmarkSpec] = [
    BenchmarkSpec("sum_loop", "Integer-heavy pseudo-random accumulation loop"),
    BenchmarkSpec("prime_count", "Prime counting via sieve"),
    BenchmarkSpec("matrix_mul", "Dense matrix multiplication (100x100)"),
    BenchmarkSpec(
        "matrix_mul_heavy",
        "Dense integer matrix multiplication (220x220) for a heavier CPU-bound runtime pass",
        default_enabled=False,
    ),
    BenchmarkSpec(
        "compile_project_10_files",
        "Compile stress test on generated 10-file project per language",
        kind="compile",
    ),
    BenchmarkSpec(
        "compile_project_synthetic_mega_graph",
        "Compile stress test on a generated 1400-file synthetic mega-graph project per language",
        kind="compile",
    ),
    BenchmarkSpec(
        "compile_project_extreme_graph",
        "Compile stress test on a generated 2200-file extreme synthetic dependency graph per language",
        kind="compile",
        default_enabled=False,
    ),
    BenchmarkSpec(
        "incremental_rebuild_1_file",
        "Compile 10-file project, mutate one file, then recompile",
        kind="incremental",
    ),
    BenchmarkSpec(
        "incremental_rebuild_central_file",
        "Compile 10-file project with shared core dependency, mutate central file, then recompile",
        kind="incremental",
    ),
    BenchmarkSpec(
        "incremental_rebuild_mega_project_10_files",
        "Compile a generated mega-project, apply syntax-only edits to 10 files, then rebuild",
        kind="incremental_batch",
    ),
    BenchmarkSpec(
        "incremental_rebuild_synthetic_mega_graph",
        "Compile a generated synthetic mega-graph project, apply syntax-only edits to many files, then rebuild",
        kind="incremental_batch_synthetic_mega_graph",
    ),
    BenchmarkSpec(
        "incremental_rebuild_synthetic_mega_graph_mixed_invalidation",
        "Compile a generated synthetic mega-graph project, then rebuild after mixed leaf edits and API-surface invalidation",
        kind="incremental_mixed_synthetic_mega_graph",
    ),
    BenchmarkSpec(
        "incremental_rebuild_extreme_graph",
        "Compile a generated 2200-file extreme dependency graph, apply syntax-only edits to many files, then rebuild",
        kind="incremental_batch_extreme_graph",
        default_enabled=False,
    ),
    BenchmarkSpec(
        "incremental_rebuild_extreme_graph_mixed_invalidation",
        "Compile a generated 2200-file extreme dependency graph, then rebuild after leaf edits plus shared API invalidation",
        kind="incremental_mixed_extreme_graph",
        default_enabled=False,
    ),
]

LANGUAGES = ("apex", "rust", "go")
SYNTHETIC_MEGA_GRAPH_CONFIG = SyntheticGraphConfig(
    file_count=1400,
    funcs_per_file=96,
    mutate_count=40,
    max_deps=6,
    group_size=50,
    mixed_leaf_edits=24,
    mixed_group_edits=8,
)
EXTREME_GRAPH_CONFIG = SyntheticGraphConfig(
    file_count=2200,
    funcs_per_file=112,
    mutate_count=64,
    max_deps=8,
    group_size=44,
    mixed_leaf_edits=40,
    mixed_group_edits=12,
)


def select_synthetic_graph_config(bench_name: str) -> SyntheticGraphConfig:
    if "extreme_graph" in bench_name:
        return EXTREME_GRAPH_CONFIG
    return SYNTHETIC_MEGA_GRAPH_CONFIG


def is_windows() -> bool:
    return os.name == "nt"


def exe_path(path: Path) -> Path:
    if is_windows() and path.suffix.lower() != ".exe":
        return path.with_suffix(".exe")
    return path


def run_cmd(cmd: List[str], cwd: Path, env: Dict[str, str] | None = None) -> subprocess.CompletedProcess:
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)
    return subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=merged_env,
        check=False,
    )


def ensure_tool(name: str) -> None:
    if shutil.which(name) is None:
        raise RuntimeError(f"Required tool not found in PATH: {name}")


def parse_checksum(output: str) -> int:
    line = output.strip().splitlines()[-1].strip()
    return int(line)


def compile_apex(
    root: Path,
    bench: str,
    out: Path,
    build_env: Dict[str, str],
    opt_level: str,
    target: str | None,
) -> None:
    compiler = root / "target" / "release" / "apex-compiler"
    if not compiler.exists():
        raise RuntimeError(
            f"Apex compiler missing at {compiler}. Build it first or run without --no-build."
        )

    src = root / "benchmark" / "apex" / f"{bench}.apex"
    cmd = [
        str(compiler),
        "compile",
        str(src),
        "-o",
        str(out),
        "--no-check",
        "--opt-level",
        opt_level,
    ]
    if target:
        cmd.extend(["--target", target])
    proc = run_cmd(cmd, root, env=build_env)
    if proc.returncode != 0:
        raise RuntimeError(f"Failed to compile Apex benchmark {bench}:\n{proc.stderr}")


def compile_rust(root: Path, bench: str, out: Path) -> None:
    src = root / "benchmark" / "rust" / f"{bench}.rs"
    cmd = ["rustc", "-C", "opt-level=3", "-C", "target-cpu=native", str(src), "-o", str(out)]
    proc = run_cmd(cmd, root)
    if proc.returncode != 0:
        raise RuntimeError(f"Failed to compile Rust benchmark {bench}:\n{proc.stderr}")


def compile_go(root: Path, bench: str, out: Path) -> None:
    src = root / "benchmark" / "go" / f"{bench}.go"
    cmd = ["go", "build", "-trimpath", "-ldflags", "-s -w", "-o", str(out), str(src)]
    proc = run_cmd(cmd, root, env={"GO111MODULE": "off"})
    if proc.returncode != 0:
        raise RuntimeError(f"Failed to compile Go benchmark {bench}:\n{proc.stderr}")


def timed_compile(
    cmd: List[str], cwd: Path, env: Dict[str, str] | None = None
) -> TimedCompileResult:
    start = time.perf_counter()
    proc = run_cmd(cmd, cwd, env=env)
    elapsed = time.perf_counter() - start
    if proc.returncode != 0:
        raise RuntimeError(
            f"Compile failed: {' '.join(cmd)}\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return TimedCompileResult(elapsed_s=elapsed, stdout=proc.stdout, stderr=proc.stderr)


def timed_compile_with_retry(lang: str, job: Dict, retries: int = 1) -> TimedCompileResult:
    for attempt in range(retries + 1):
        try:
            return timed_compile(job["cmd"], job["cwd"], env=job["env"])
        except RuntimeError as exc:
            msg = str(exc)
            transient_ll_missing = (
                lang == "apex"
                and ".ll" in msg
                and "no such file or directory" in msg.lower()
            )
            if transient_ll_missing and attempt < retries:
                time.sleep(0.05)
                continue
            raise

    raise RuntimeError("unreachable")


def parse_build_timings(output: str) -> Dict[str, Dict]:
    lines = output.splitlines()
    try:
        start = lines.index("Build timings") + 1
    except ValueError:
        return {}

    timings: Dict[str, Dict] = {}
    for raw_line in lines[start:]:
        line = raw_line.strip()
        if not line:
            continue
        if " ms" not in line:
            continue

        ms_index = line.rfind(" ms")
        if ms_index == -1:
            continue
        number_start = ms_index - 1
        while number_start >= 0 and (line[number_start].isdigit() or line[number_start] == "."):
            number_start -= 1
        number_start += 1
        label = line[:number_start].strip()
        if not label:
            continue
        try:
            ms_value = float(line[number_start:ms_index].strip())
        except ValueError:
            continue

        counters: Dict[str, int] = {}
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


def summarize_apex_phase_timings(samples: List[Dict[str, Dict]]) -> List[Dict]:
    if not samples:
        return []

    labels: List[str] = []
    for sample in samples:
        for label in sample:
            if label not in labels:
                labels.append(label)

    summary: List[Dict] = []
    for label in labels:
        label_samples = [sample[label] for sample in samples if label in sample]
        if not label_samples:
            continue
        mean_ms = statistics.mean(item["ms"] for item in label_samples)
        counters = label_samples[-1]["counters"]
        summary.append(
            {
                "label": label,
                "mean_ms": mean_ms,
                "runs": len(label_samples),
                "counters": counters,
            }
        )
    return summary


def run_checksum(binary: Path, cwd: Path) -> int:
    proc = run_cmd([str(binary)], cwd)
    if proc.returncode != 0:
        raise RuntimeError(
            f"Binary execution failed: {binary}\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return parse_checksum(proc.stdout)


def pick_mutation_targets(part_files: List[Path], mutate_count: int, profile: str) -> List[Path]:
    if not part_files:
        return []

    mutate_count = max(1, min(mutate_count, len(part_files)))
    if profile == "central":
        middle = len(part_files) // 2
        return [part_files[middle]]
    if profile == "batch_spread":
        if mutate_count == 1:
            return [part_files[-1]]
        last_index = len(part_files) - 1
        indices = {
            round((last_index * i) / (mutate_count - 1))
            for i in range(mutate_count)
        }
        return [part_files[i] for i in sorted(indices)]
    return part_files[-mutate_count:]


def generate_compile_project_10_files(
    root: Path,
    bench_name: str,
    mutation_profile: str = "leaf",
    file_count: int = 10,
    funcs_per_file: int = 180,
    mutate_count: int = 1,
) -> Dict[str, Dict[str, Path]]:
    generated_root = root / "benchmark" / "generated" / bench_name
    if generated_root.exists():
        shutil.rmtree(generated_root)
    generated_root.mkdir(parents=True, exist_ok=True)

    apex_dir = generated_root / "apex"
    apex_src = apex_dir / "src"
    apex_src.mkdir(parents=True, exist_ok=True)
    apex_files = []
    apex_part_files: List[Path] = []
    apex_core = apex_src / "core.apex"
    apex_core.write_text(
        "\n".join(
            [
                "import std.io.*;",
                "",
                "function core_mix(x: Integer, k: Integer): Integer {",
                "    return x + k;",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    apex_files.append("src/core.apex")
    for i in range(file_count):
        part = f"part_{i:02d}"
        apex_files.append(f"src/{part}.apex")
        apex_part_files.append(apex_src / f"{part}.apex")
        lines: List[str] = ["import std.io.*;", ""]
        for j in range(funcs_per_file):
            lines.append(
                f"function {part}_f{j:03d}(x: Integer): Integer {{ return core_mix(x, {i + j + 1}); }}"
            )
        lines.append("")
        lines.append(f"function {part}_apply(x: Integer): Integer {{")
        lines.append("    mut y: Integer = x;")
        for j in range(funcs_per_file):
            lines.append(f"    y = {part}_f{j:03d}(y);")
        lines.append("    return y;")
        lines.append("}")
        (apex_src / f"{part}.apex").write_text("\n".join(lines) + "\n", encoding="utf-8")

    main_lines: List[str] = ["import std.io.*;", "", "function main(): None {", "    mut acc: Integer = 0;"]
    for i in range(file_count):
        main_lines.append(f"    acc = part_{i:02d}_apply(acc);")
    main_lines.extend(['    println(to_string(acc));', "    return None;", "}"])
    (apex_src / "main.apex").write_text("\n".join(main_lines) + "\n", encoding="utf-8")
    apex_files.append("src/main.apex")

    toml_lines = [
        f'name = "{bench_name}"',
        'version = "0.1.0"',
        'entry = "src/main.apex"',
        "files = [",
    ]
    toml_lines.extend([f'    "{f}",' for f in apex_files])
    toml_lines.extend(["]", f'output = "{bench_name}"', 'opt_level = "3"'])
    (apex_dir / "apex.toml").write_text("\n".join(toml_lines) + "\n", encoding="utf-8")

    rust_dir = generated_root / "rust"
    rust_dir.mkdir(parents=True, exist_ok=True)
    rust_part_files: List[Path] = []
    rust_main = ["mod core;"]
    for i in range(file_count):
        rust_main.append(f"mod part_{i:02d};")
    rust_main.extend(["", "fn main() {", "    let mut acc: i64 = 0;"])
    for i in range(file_count):
        rust_main.append(f"    acc = part_{i:02d}::apply(acc);")
    rust_main.extend(['    println!("{acc}");', "}"])
    (rust_dir / "main.rs").write_text("\n".join(rust_main) + "\n", encoding="utf-8")
    for i in range(file_count):
        part = f"part_{i:02d}"
        rust_part_files.append(rust_dir / f"{part}.rs")
        lines = ["pub fn apply(x: i64) -> i64 {", "    let mut y = x;"]
        for j in range(funcs_per_file):
            lines.append(f"    y = crate::core::mix(y, {i + j + 1});")
        lines.extend(["    y", "}"])
        (rust_dir / f"{part}.rs").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (rust_dir / "core.rs").write_text(
        "\n".join(
            [
                "pub fn mix(x: i64, k: i64) -> i64 {",
                "    x + k",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )

    go_dir = generated_root / "go"
    go_dir.mkdir(parents=True, exist_ok=True)
    go_part_files: List[Path] = []
    (go_dir / "go.mod").write_text("module compile10\n\ngo 1.22\n", encoding="utf-8")
    go_main = ['package main', "", 'import "fmt"', "", "func main() {", "    var acc int64 = 0"]
    for i in range(file_count):
        go_main.append(f"    acc = part_{i:02d}_apply(acc)")
    go_main.extend(["    fmt.Println(acc)", "}"])
    (go_dir / "main.go").write_text("\n".join(go_main) + "\n", encoding="utf-8")
    for i in range(file_count):
        part = f"part_{i:02d}"
        go_part_file = go_dir / f"unit_{i:04d}.go"
        go_part_files.append(go_part_file)
        lines = ["package main", "", "func " + part + "_apply(x int64) int64 {", "    y := x"]
        for j in range(funcs_per_file):
            lines.append(f"    y = coreMix(y, {i + j + 1})")
        lines.extend(["    return y", "}"])
        go_part_file.write_text("\n".join(lines) + "\n", encoding="utf-8")
    (go_dir / "core.go").write_text(
        "\n".join(
            [
                "package main",
                "",
                "func coreMix(x int64, k int64) int64 {",
                "    return x + k",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )

    apex_mutate_sources = pick_mutation_targets(apex_part_files, mutate_count, mutation_profile)
    rust_mutate_sources = pick_mutation_targets(rust_part_files, mutate_count, mutation_profile)
    go_mutate_sources = pick_mutation_targets(go_part_files, mutate_count, mutation_profile)
    if mutation_profile == "central":
        apex_mutate_sources = [apex_core]
        rust_mutate_sources = [rust_dir / "core.rs"]
        go_mutate_sources = [go_dir / "core.go"]

    return {
        "apex": {
            "project_dir": apex_dir,
            "binary": apex_dir / bench_name,
            "mutate_source": apex_mutate_sources[0],
            "mutate_sources": apex_mutate_sources,
        },
        "rust": {
            "project_dir": rust_dir,
            "binary": rust_dir / f"{bench_name}_rust",
            "mutate_source": rust_mutate_sources[0],
            "mutate_sources": rust_mutate_sources,
        },
        "go": {
            "project_dir": go_dir,
            "binary": go_dir / f"{bench_name}_go",
            "mutate_source": go_mutate_sources[0],
            "mutate_sources": go_mutate_sources,
        },
    }


def generate_incremental_rebuild_mega_project_10_files(root: Path, bench_name: str) -> Dict[str, Dict[str, Path]]:
    return generate_compile_project_10_files(
        root,
        bench_name,
        mutation_profile="batch_spread",
        file_count=120,
        funcs_per_file=320,
        mutate_count=10,
    )


def synthetic_graph_dependency_indices(index: int, max_deps: int) -> List[int]:
    if index <= 0:
        return []

    candidates = [
        index - 1,
        index - 3,
        index - 7,
        index - 15,
        index // 2,
        index - 32,
        index - 64,
        (index * 7) // 11,
        (index * 5) // 8,
        (index * 3) // 5,
    ]
    deps: List[int] = []
    for candidate in candidates:
        if 0 <= candidate < index and candidate not in deps:
            deps.append(candidate)
        if len(deps) == max_deps:
            break
    return deps


def generate_compile_project_synthetic_graph(
    root: Path,
    bench_name: str,
    config: SyntheticGraphConfig,
    mutate_count: int | None = None,
) -> Dict[str, Dict[str, Path]]:
    file_count = config.file_count
    funcs_per_file = config.funcs_per_file
    group_size = config.group_size
    effective_mutate_count = mutate_count if mutate_count is not None else config.mutate_count
    generated_root = root / "benchmark" / "generated" / bench_name
    if generated_root.exists():
        shutil.rmtree(generated_root)
    generated_root.mkdir(parents=True, exist_ok=True)

    part_names = [f"part_{i:04d}" for i in range(file_count)]
    group_count = (file_count + group_size - 1) // group_size
    group_names = [f"group_{i:02d}" for i in range(group_count)]

    apex_dir = generated_root / "apex"
    apex_src = apex_dir / "src"
    apex_src.mkdir(parents=True, exist_ok=True)
    rust_dir = generated_root / "rust"
    rust_dir.mkdir(parents=True, exist_ok=True)
    go_dir = generated_root / "go"
    go_dir.mkdir(parents=True, exist_ok=True)

    apex_files = ["src/core.apex"]
    apex_part_files: List[Path] = []
    rust_part_files: List[Path] = []
    go_part_files: List[Path] = []
    apex_group_plans: List[Dict] = []
    rust_group_plans: List[Dict] = []
    go_group_plans: List[Dict] = []

    apex_core = apex_src / "core.apex"
    apex_core.write_text(
        "\n".join(
            [
                "function core_mix(x: Integer, k: Integer): Integer {",
                "    return x + k;",
                "}",
                "",
                "function core_fold(a: Integer, b: Integer, salt: Integer): Integer {",
                "    return a + b + salt;",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    (rust_dir / "core.rs").write_text(
        "\n".join(
            [
                "pub fn mix(x: i64, k: i64) -> i64 {",
                "    x + k",
                "}",
                "",
                "pub fn fold(a: i64, b: i64, salt: i64) -> i64 {",
                "    a + b + salt",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    (go_dir / "go.mod").write_text("module compile10\n\ngo 1.22\n", encoding="utf-8")
    (go_dir / "core.go").write_text(
        "\n".join(
            [
                "package main",
                "",
                "func coreMix(x int64, k int64) int64 {",
                "    return x + k",
                "}",
                "",
                "func coreFold(a int64, b int64, salt int64) int64 {",
                "    return a + b + salt",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )

    for group_idx, group_name in enumerate(group_names):
        group_salt = 1000 + group_idx * 37

        apex_group_file = apex_src / f"{group_name}.apex"
        apex_group_file.write_text(
            "\n".join(
                [
                    f"// MUTATION_SURFACE_{group_name.upper()}",
                    f"function {group_name}_bridge(x: Integer, salt: Integer): Integer {{",
                    f"    return core_fold(x, salt, {group_salt});",
                    "}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        apex_files.append(f"src/{group_name}.apex")

        rust_group_file = rust_dir / f"{group_name}.rs"
        rust_group_file.write_text(
            "\n".join(
                [
                    f"// MUTATION_SURFACE_{group_name.upper()}",
                    f"pub fn {group_name}_bridge(x: i64, salt: i64) -> i64 {{",
                    f"    crate::core::fold(x, salt, {group_salt})",
                    "}",
                    "",
                ]
            ),
            encoding="utf-8",
        )

        go_group_file = go_dir / f"{group_name}.go"
        go_group_file.write_text(
            "\n".join(
                [
                    "package main",
                    "",
                    f"// MUTATION_SURFACE_{group_name.upper()}",
                    f"func {group_name}_bridge(x int64, salt int64) int64 {{",
                    f"    return coreFold(x, salt, {group_salt})",
                    "}",
                    "",
                ]
            ),
            encoding="utf-8",
        )

        apex_group_plans.append(
            {"group_name": group_name, "group_index": group_idx, "call_salt": group_salt, "surface_files": [apex_group_file], "caller_files": []}
        )
        rust_group_plans.append(
            {"group_name": group_name, "group_index": group_idx, "call_salt": group_salt, "surface_files": [rust_group_file], "caller_files": []}
        )
        go_group_plans.append(
            {"group_name": group_name, "group_index": group_idx, "call_salt": group_salt, "surface_files": [go_group_file], "caller_files": []}
        )

    for i, part in enumerate(part_names):
        deps = synthetic_graph_dependency_indices(i, config.max_deps)
        group_idx = i // group_size
        group_name = group_names[group_idx]
        group_salt = 1000 + group_idx * 37

        apex_files.append(f"src/{part}.apex")
        apex_part_file = apex_src / f"{part}.apex"
        apex_part_files.append(apex_part_file)
        apex_group_plans[group_idx]["caller_files"].append(apex_part_file)
        apex_lines: List[str] = []
        for j in range(funcs_per_file):
            apex_lines.append(
                f"function {part}_f{j:03d}(x: Integer): Integer {{ return core_mix(x, {i + j + 1}); }}"
            )
        apex_lines.extend(["", f"function {part}_apply(x: Integer): Integer {{", "    mut y: Integer = x;"])
        for j in range(funcs_per_file):
            apex_lines.append(f"    y = {part}_f{j:03d}(y);")
        apex_lines.append(f"    y = {group_name}_bridge(y, {group_salt}); // MUTATION_CALL_{group_name.upper()}")
        apex_lines.extend(["    return y;", "}"])
        apex_lines.extend(["", f"function {part}_chain(x: Integer): Integer {{", f"    mut y: Integer = {part}_apply(x);"])
        for j in range(0, funcs_per_file, 3):
            apex_lines.append(f"    y = {part}_f{j:03d}(y);")
        apex_lines.extend(["    return y;", "}"])
        apex_lines.extend(["", f"function {part}_wire(x: Integer): Integer {{", f"    mut y: Integer = {part}_chain(x);"])
        for dep in deps:
            dep_part = part_names[dep]
            apex_lines.append(f"    y = core_fold(y, {dep_part}_apply(x), {i + dep + 1});")
        for dep in deps:
            dep_part = part_names[dep]
            apex_lines.append(f"    y = core_fold(y, {dep_part}_wire(x), {i + dep + 33});")
        apex_lines.extend(["    return y;", "}"])
        apex_lines.extend(["", f"function {part}_fanout(x: Integer): Integer {{", f"    mut y: Integer = {part}_wire(x);"])
        for dep in deps:
            dep_part = part_names[dep]
            apex_lines.append(f"    y = core_fold(y, {dep_part}_chain(x), {i + dep + 65});")
        apex_lines.extend(["    return y;", "}"])
        apex_lines.extend(["", f"function {part}_signature(seed: Integer): Integer {{", f"    mut y: Integer = {part}_fanout(seed);"])
        for dep in deps:
            dep_part = part_names[dep]
            apex_lines.append(f"    y = core_fold(y, {dep_part}_apply(seed), {i + dep + 97});")
        apex_lines.extend(["    return y;", "}"])
        apex_part_file.write_text("\n".join(apex_lines) + "\n", encoding="utf-8")

        rust_part_file = rust_dir / f"{part}.rs"
        rust_part_files.append(rust_part_file)
        rust_group_plans[group_idx]["caller_files"].append(rust_part_file)
        rust_lines: List[str] = [
            *[
                f"pub fn f{j:03d}(x: i64) -> i64 {{ crate::core::mix(x, {i + j + 1}) }}"
                for j in range(funcs_per_file)
            ],
            "",
            "pub fn apply(x: i64) -> i64 {",
            "    let mut y = x;",
        ]
        for j in range(funcs_per_file):
            rust_lines.append(f"    y = f{j:03d}(y);")
        rust_lines.append(f"    y = crate::{group_name}::{group_name}_bridge(y, {group_salt}); // MUTATION_CALL_{group_name.upper()}")
        rust_lines.extend(["    y", "}"])
        rust_lines.extend(["", "pub fn chain(x: i64) -> i64 {", "    let mut y = apply(x);"])
        for j in range(0, funcs_per_file, 3):
            rust_lines.append(f"    y = f{j:03d}(y);")
        rust_lines.extend(["    y", "}"])
        rust_lines.extend(["", "pub fn wire(x: i64) -> i64 {", "    let mut y = chain(x);"])
        for dep in deps:
            dep_part = part_names[dep]
            rust_lines.append(f"    y = crate::core::fold(y, crate::{dep_part}::apply(x), {i + dep + 1});")
        for dep in deps:
            dep_part = part_names[dep]
            rust_lines.append(f"    y = crate::core::fold(y, crate::{dep_part}::wire(x), {i + dep + 33});")
        rust_lines.extend(["    y", "}"])
        rust_lines.extend(["", "pub fn fanout(x: i64) -> i64 {", "    let mut y = wire(x);"])
        for dep in deps:
            dep_part = part_names[dep]
            rust_lines.append(f"    y = crate::core::fold(y, crate::{dep_part}::chain(x), {i + dep + 65});")
        rust_lines.extend(["    y", "}"])
        rust_lines.extend(["", "pub fn signature(seed: i64) -> i64 {", "    let mut y = fanout(seed);"])
        for dep in deps:
            dep_part = part_names[dep]
            rust_lines.append(f"    y = crate::core::fold(y, crate::{dep_part}::apply(seed), {i + dep + 97});")
        rust_lines.extend(["    y", "}"])
        rust_part_file.write_text("\n".join(rust_lines) + "\n", encoding="utf-8")

        go_part_file = go_dir / f"unit_{i:04d}.go"
        go_part_files.append(go_part_file)
        go_group_plans[group_idx]["caller_files"].append(go_part_file)
        go_lines = ["package main", ""]
        for j in range(funcs_per_file):
            go_lines.append(f"func {part}_f{j:03d}(x int64) int64 {{ return coreMix(x, {i + j + 1}) }}")
        go_lines.extend(["", f"func {part}_apply(x int64) int64 {{", "    y := x"])
        for j in range(funcs_per_file):
            go_lines.append(f"    y = {part}_f{j:03d}(y)")
        go_lines.append(f"    y = {group_name}_bridge(y, {group_salt}) // MUTATION_CALL_{group_name.upper()}")
        go_lines.extend(["    return y", "}"])
        go_lines.extend(["", f"func {part}_chain(x int64) int64 {{", f"    y := {part}_apply(x)"])
        for j in range(0, funcs_per_file, 3):
            go_lines.append(f"    y = {part}_f{j:03d}(y)")
        go_lines.extend(["    return y", "}"])
        go_lines.extend(["", f"func {part}_wire(x int64) int64 {{", f"    y := {part}_chain(x)"])
        for dep in deps:
            dep_part = part_names[dep]
            go_lines.append(f"    y = coreFold(y, {dep_part}_apply(x), {i + dep + 1})")
        for dep in deps:
            dep_part = part_names[dep]
            go_lines.append(f"    y = coreFold(y, {dep_part}_wire(x), {i + dep + 33})")
        go_lines.extend(["    return y", "}"])
        go_lines.extend(["", f"func {part}_fanout(x int64) int64 {{", f"    y := {part}_wire(x)"])
        for dep in deps:
            dep_part = part_names[dep]
            go_lines.append(f"    y = coreFold(y, {dep_part}_chain(x), {i + dep + 65})")
        go_lines.extend(["    return y", "}"])
        go_lines.extend(["", f"func {part}_signature(seed int64) int64 {{", f"    y := {part}_fanout(seed)"])
        for dep in deps:
            dep_part = part_names[dep]
            go_lines.append(f"    y = coreFold(y, {dep_part}_apply(seed), {i + dep + 97})")
        go_lines.extend(["    return y", "}"])
        go_part_file.write_text("\n".join(go_lines) + "\n", encoding="utf-8")

    main_lines: List[str] = ["import std.io.*;", "", "function main(): None {", "    mut acc: Integer = 0;"]
    for part in part_names:
        main_lines.append(f"    acc = {part}_apply(acc);")
    main_lines.extend(['    println(to_string(acc));', "    return None;", "}"])
    (apex_src / "main.apex").write_text("\n".join(main_lines) + "\n", encoding="utf-8")
    apex_files.append("src/main.apex")
    toml_lines = [f'name = "{bench_name}"', 'version = "0.1.0"', 'entry = "src/main.apex"', "files = ["]
    toml_lines.extend([f'    "{f}",' for f in apex_files])
    toml_lines.extend(["]", f'output = "{bench_name}"', 'opt_level = "3"'])
    (apex_dir / "apex.toml").write_text("\n".join(toml_lines) + "\n", encoding="utf-8")

    rust_main = ["mod core;"]
    for group_name in group_names:
        rust_main.append(f"mod {group_name};")
    for part in part_names:
        rust_main.append(f"mod {part};")
    rust_main.extend(["", "fn main() {", "    let mut acc: i64 = 0;"])
    for part in part_names:
        rust_main.append(f"    acc = {part}::apply(acc);")
    rust_main.extend(['    println!("{acc}");', "}"])
    (rust_dir / "main.rs").write_text("\n".join(rust_main) + "\n", encoding="utf-8")

    go_main = ['package main', "", 'import "fmt"', "", "func main() {", "    var acc int64 = 0"]
    for part in part_names:
        go_main.append(f"    acc = {part}_apply(acc)")
    go_main.extend(["    fmt.Println(acc)", "}"])
    (go_dir / "main.go").write_text("\n".join(go_main) + "\n", encoding="utf-8")

    apex_mutate_sources = pick_mutation_targets(apex_part_files, effective_mutate_count, "batch_spread")
    rust_mutate_sources = pick_mutation_targets(rust_part_files, effective_mutate_count, "batch_spread")
    go_mutate_sources = pick_mutation_targets(go_part_files, effective_mutate_count, "batch_spread")

    def spread_group_plans(plans: List[Dict], count: int) -> List[Dict]:
        if not plans:
            return []
        count = max(1, min(count, len(plans)))
        if count == 1:
            return [plans[-1]]
        last_index = len(plans) - 1
        indices = {round((last_index * i) / (count - 1)) for i in range(count)}
        return [plans[i] for i in sorted(indices)]

    return {
        "apex": {
            "project_dir": apex_dir,
            "binary": apex_dir / bench_name,
            "mutate_source": apex_mutate_sources[0],
            "mutate_sources": apex_mutate_sources,
            "mixed_leaf_sources": pick_mutation_targets(
                apex_part_files, config.mixed_leaf_edits, "batch_spread"
            ),
            "mixed_groups": spread_group_plans(apex_group_plans, config.mixed_group_edits),
        },
        "rust": {
            "project_dir": rust_dir,
            "binary": rust_dir / f"{bench_name}_rust",
            "mutate_source": rust_mutate_sources[0],
            "mutate_sources": rust_mutate_sources,
            "mixed_leaf_sources": pick_mutation_targets(
                rust_part_files, config.mixed_leaf_edits, "batch_spread"
            ),
            "mixed_groups": spread_group_plans(rust_group_plans, config.mixed_group_edits),
        },
        "go": {
            "project_dir": go_dir,
            "binary": go_dir / f"{bench_name}_go",
            "mutate_source": go_mutate_sources[0],
            "mutate_sources": go_mutate_sources,
            "mixed_leaf_sources": pick_mutation_targets(
                go_part_files, config.mixed_leaf_edits, "batch_spread"
            ),
            "mixed_groups": spread_group_plans(go_group_plans, config.mixed_group_edits),
        },
    }


def make_compile_jobs(
    root: Path,
    compile_projects: Dict[str, Dict[str, Path]],
    build_env: Dict[str, str],
    apex_timings: bool,
) -> Dict[str, Dict]:
    compiler = root / "target" / "release" / "apex-compiler"
    apex_cmd = [str(compiler), "build", "--no-check"]
    if apex_timings:
        apex_cmd.append("--timings")
    return {
        "apex": {
            "cmd": apex_cmd,
            "cwd": compile_projects["apex"]["project_dir"],
            "env": build_env,
            "binary": exe_path(compile_projects["apex"]["binary"]),
            "mutate_source": compile_projects["apex"]["mutate_source"],
            "mutate_sources": compile_projects["apex"].get("mutate_sources", []),
            "mixed_leaf_sources": compile_projects["apex"].get("mixed_leaf_sources", []),
            "mixed_groups": compile_projects["apex"].get("mixed_groups", []),
        },
        "rust": {
            "cmd": [
                "rustc",
                "-C",
                "opt-level=3",
                "-C",
                "target-cpu=native",
                "main.rs",
                "-o",
                str(compile_projects["rust"]["binary"]),
            ],
            "cwd": compile_projects["rust"]["project_dir"],
            "env": None,
            "binary": exe_path(compile_projects["rust"]["binary"]),
            "mutate_source": compile_projects["rust"]["mutate_source"],
            "mutate_sources": compile_projects["rust"].get("mutate_sources", []),
            "mixed_leaf_sources": compile_projects["rust"].get("mixed_leaf_sources", []),
            "mixed_groups": compile_projects["rust"].get("mixed_groups", []),
        },
        "go": {
            "cmd": [
                "go",
                "build",
                "-trimpath",
                "-o",
                str(compile_projects["go"]["binary"]),
                ".",
            ],
            "cwd": compile_projects["go"]["project_dir"],
            "env": {
                "GO111MODULE": "on",
                "GOCACHE": str(compile_projects["go"]["project_dir"] / ".gocache"),
            },
            "binary": exe_path(compile_projects["go"]["binary"]),
            "go_cache_dir": compile_projects["go"]["project_dir"] / ".gocache",
            "mutate_source": compile_projects["go"]["mutate_source"],
            "mutate_sources": compile_projects["go"].get("mutate_sources", []),
            "mixed_leaf_sources": compile_projects["go"].get("mixed_leaf_sources", []),
            "mixed_groups": compile_projects["go"].get("mixed_groups", []),
        },
    }


def apply_incremental_source_change(lang: str, source: Path, marker: str) -> None:
    if not source.exists():
        raise RuntimeError(f"Missing source to mutate: {source}")
    line = f"\n// incremental bench mutation {marker}\n"
    with source.open("a", encoding="utf-8") as f:
        f.write(line)


def apply_incremental_source_changes(lang: str, sources: List[Path], marker: str) -> None:
    for idx, source in enumerate(sources):
        apply_incremental_source_change(lang, source, f"{marker}_file_{idx:02d}")


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise RuntimeError(f"Expected mutation hook not found in {path}: {old}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def apply_mixed_invalidation_changes(lang: str, job: Dict, marker: str) -> None:
    leaf_sources = [Path(p) for p in job.get("mixed_leaf_sources", [])]
    apply_incremental_source_changes(lang, leaf_sources, f"{marker}_leaf")

    for idx, group in enumerate(job.get("mixed_groups", [])):
        group_name = group["group_name"]
        salt = int(group["call_salt"])
        extra = 5000 + group["group_index"] * 13 + idx

        if lang == "apex":
            replace_once(
                Path(group["surface_files"][0]),
                f"function {group_name}_bridge(x: Integer, salt: Integer): Integer {{",
                f"function {group_name}_bridge(x: Integer, salt: Integer, extra: Integer): Integer {{",
            )
            replace_once(
                Path(group["surface_files"][0]),
                f"    return core_fold(x, salt, {salt});",
                f"    return core_fold(x, salt + extra, {salt});",
            )
            old_call = f"    y = {group_name}_bridge(y, {salt}); // MUTATION_CALL_{group_name.upper()}"
            new_call = f"    y = {group_name}_bridge(y, {salt}, {extra}); // MUTATION_CALL_{group_name.upper()}"
        elif lang == "rust":
            replace_once(
                Path(group["surface_files"][0]),
                f"pub fn {group_name}_bridge(x: i64, salt: i64) -> i64 {{",
                f"pub fn {group_name}_bridge(x: i64, salt: i64, extra: i64) -> i64 {{",
            )
            replace_once(
                Path(group["surface_files"][0]),
                f"    crate::core::fold(x, salt, {salt})",
                f"    crate::core::fold(x, salt + extra, {salt})",
            )
            old_call = f"    y = crate::{group_name}::{group_name}_bridge(y, {salt}); // MUTATION_CALL_{group_name.upper()}"
            new_call = f"    y = crate::{group_name}::{group_name}_bridge(y, {salt}, {extra}); // MUTATION_CALL_{group_name.upper()}"
        elif lang == "go":
            replace_once(
                Path(group["surface_files"][0]),
                f"func {group_name}_bridge(x int64, salt int64) int64 {{",
                f"func {group_name}_bridge(x int64, salt int64, extra int64) int64 {{",
            )
            replace_once(
                Path(group["surface_files"][0]),
                f"    return coreFold(x, salt, {salt})",
                f"    return coreFold(x, salt+extra, {salt})",
            )
            old_call = f"    y = {group_name}_bridge(y, {salt}) // MUTATION_CALL_{group_name.upper()}"
            new_call = f"    y = {group_name}_bridge(y, {salt}, {extra}) // MUTATION_CALL_{group_name.upper()}"
        else:
            raise RuntimeError(f"Unsupported language for mixed invalidation: {lang}")

        for caller in group.get("caller_files", []):
            replace_once(Path(caller), old_call, new_call)


def timed_run(binary: Path, cwd: Path) -> (float, int):
    start = time.perf_counter()
    proc = run_cmd([str(binary)], cwd)
    elapsed = time.perf_counter() - start
    if proc.returncode != 0:
        raise RuntimeError(
            f"Benchmark execution failed: {binary}\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    checksum = parse_checksum(proc.stdout)
    return elapsed, checksum


def compute_stats(values: List[float]) -> Dict[str, float]:
    return {
        "min_s": min(values),
        "mean_s": statistics.mean(values),
        "median_s": statistics.median(values),
        "max_s": max(values),
        "stddev_s": statistics.pstdev(values) if len(values) > 1 else 0.0,
    }


def format_seconds(x: float) -> str:
    return f"{x:.6f}"


def clean_compile_artifacts(lang: str, job: Dict) -> None:
    binary = Path(job["binary"])
    if binary.exists():
        binary.unlink()

    if lang == "apex":
        apex_cache_dir = Path(job["cwd"]) / ".apexcache"
        if apex_cache_dir.exists():
            shutil.rmtree(apex_cache_dir)

    if lang == "go":
        go_cache_dir = job.get("go_cache_dir")
        if go_cache_dir is not None:
            cache_path = Path(go_cache_dir)
            if cache_path.exists():
                shutil.rmtree(cache_path)
            cache_path.mkdir(parents=True, exist_ok=True)
        else:
            proc = run_cmd(["go", "clean", "-cache"], job["cwd"], env=job.get("env"))
            if proc.returncode != 0:
                raise RuntimeError(f"Failed to clean Go cache:\n{proc.stderr}")


def build_markdown(result: Dict) -> str:
    lines: List[str] = []
    lines.append("# Benchmark Report")
    lines.append("")
    lines.append(f"- Generated: `{result['generated_at']}`")
    lines.append(f"- Repeats: `{result['repeats']}`")
    lines.append(f"- Warmup runs: `{result['warmup']}`")
    lines.append(f"- Apex opt level: `{result.get('apex_opt_level', 'n/a')}`")
    lines.append(f"- Apex target: `{result.get('apex_target') or 'native/default'}`")
    lines.append(f"- Apex phase timings: `{'enabled' if result.get('apex_timings') else 'disabled'}`")
    lines.append(f"- Compile mode: `{result.get('compile_mode', 'n/a')}`")
    lines.append("")

    for bench in result["benchmarks"]:
        lines.append(f"## {bench['name']}")
        lines.append("")
        lines.append(f"{bench['description']}")
        if bench.get("compile_mode"):
            lines.append("")
            lines.append(f"- compile mode: `{bench['compile_mode']}`")
        lines.append("")
        if bench.get("kind") in ("incremental", "incremental_batch", "incremental_batch_synthetic_mega_graph", "incremental_mixed_synthetic_mega_graph"):
            phase_one_label = bench.get("phase_one_label", "first mean (s)")
            phase_two_label = bench.get("phase_two_label", "second mean (s)")
            ratio_label = bench.get("ratio_label", "second/first")
            lines.append(
                f"| Language | Checksum | {phase_one_label} | {phase_two_label} | {ratio_label} |"
            )
            lines.append("|---|---:|---:|---:|---:|")
            for lang in LANGUAGES:
                entry = bench["languages"][lang]
                first_mean = entry["first_stats"]["mean_s"]
                second_mean = entry["second_stats"]["mean_s"]
                ratio = second_mean / first_mean if first_mean > 0 else float("inf")
                lines.append(
                    f"| {lang} | {entry['checksum']} | {format_seconds(first_mean)} | "
                    f"{format_seconds(second_mean)} | {ratio:.3f}x |"
                )
        else:
            lines.append("| Language | Checksum | min (s) | mean (s) | median (s) | stddev (s) | max (s) |")
            lines.append("|---|---:|---:|---:|---:|---:|---:|")
            for lang in LANGUAGES:
                entry = bench["languages"][lang]
                stats = entry["stats"]
                lines.append(
                    f"| {lang} | {entry['checksum']} | {format_seconds(stats['min_s'])} | "
                    f"{format_seconds(stats['mean_s'])} | {format_seconds(stats['median_s'])} | "
                    f"{format_seconds(stats['stddev_s'])} | {format_seconds(stats['max_s'])} |"
                )
        lines.append("")
        lines.append("| Relative to Apex (mean) | Value |")
        lines.append("|---|---:|")
        for lang in LANGUAGES:
            if lang == "apex":
                continue
            lines.append(
                f"| {lang.capitalize()} speedup | {bench['speedup_vs_apex'][lang]:.3f}x |"
            )
        lines.append("")

        for section in bench.get("apex_phase_timing_sections") or []:
            lines.append(f"{section['label']} (`--timings`, mean of measured runs):")
            lines.append("")
            lines.append("| Phase | Mean (ms) | Last counters |")
            lines.append("|---|---:|---|")
            for phase in section.get("phases", []):
                counters = ", ".join(
                    f"{key}={value}" for key, value in phase.get("counters", {}).items()
                )
                lines.append(
                    f"| {phase['label']} | {phase['mean_ms']:.3f} | {counters or '-'} |"
                )
            lines.append("")

    return "\n".join(lines) + "\n"


def detect_llvm_prefix() -> str:
    from_env = os.environ.get("LLVM_SYS_211_PREFIX", "").strip()
    if from_env:
        return from_env

    llvm_config = shutil.which("llvm-config")
    if not llvm_config:
        raise RuntimeError(
            "LLVM prefix not found. Set LLVM_SYS_211_PREFIX or install llvm-config."
        )

    proc = run_cmd([llvm_config, "--prefix"], Path.cwd())
    if proc.returncode != 0:
        raise RuntimeError(
            f"Failed to detect LLVM prefix via llvm-config:\n{proc.stderr}"
        )
    prefix = proc.stdout.strip()
    if not prefix:
        raise RuntimeError("llvm-config --prefix returned empty output")
    return prefix


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Apex vs Rust vs Go benchmarks")
    parser.add_argument("--repeats", type=int, default=5, help="Timed runs per benchmark/language")
    parser.add_argument("--warmup", type=int, default=1, help="Warmup runs per benchmark/language")
    parser.add_argument(
        "--apex-opt-level",
        choices=["0", "1", "2", "3", "s", "z", "fast"],
        default="3",
        help="Optimization level passed to `apex compile --opt-level`",
    )
    parser.add_argument(
        "--apex-target",
        default=None,
        help="Optional target triple passed to `apex compile --target`",
    )
    parser.add_argument(
        "--bench",
        choices=[b.name for b in BENCHMARKS],
        default=None,
        help="Run only one benchmark",
    )
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="Skip building apex compiler with cargo build --release",
    )
    parser.add_argument(
        "--compile-mode",
        choices=["hot", "cold"],
        default="hot",
        help="Compile benchmark mode: hot keeps caches/artifacts; cold clears artifacts between timed runs.",
    )
    parser.add_argument(
        "--include-extreme",
        action="store_true",
        help="Include opt-in heavy runtime/compile benchmarks in the default suite.",
    )
    parser.add_argument(
        "--apex-timings",
        action="store_true",
        help="Pass --timings to Apex project builds and record per-phase timing breakdowns in reports.",
    )
    args = parser.parse_args()

    if args.repeats < 1:
        raise RuntimeError("--repeats must be >= 1")
    if args.warmup < 0:
        raise RuntimeError("--warmup must be >= 0")

    root = Path(__file__).resolve().parents[1]
    bench_dir = root / "benchmark"
    bin_dir = bench_dir / "bin"
    out_dir = bench_dir / "results"
    bin_dir.mkdir(parents=True, exist_ok=True)
    out_dir.mkdir(parents=True, exist_ok=True)

    ensure_tool("python3")
    ensure_tool("rustc")
    ensure_tool("go")
    ensure_tool("cargo")

    llvm_prefix = detect_llvm_prefix()
    build_env = {"LLVM_SYS_211_PREFIX": llvm_prefix}

    if not args.no_build:
        proc = run_cmd(["cargo", "build", "--release"], root, env=build_env)
        if proc.returncode != 0:
            raise RuntimeError(f"Failed to build Apex compiler:\n{proc.stderr}")

    selected = [
        b
        for b in BENCHMARKS
        if (args.bench is None or b.name == args.bench)
        and (args.bench is not None or args.include_extreme or b.default_enabled)
    ]

    # Default run includes compile hot+cold and incremental view together.
    if args.bench is None:
        expanded: List[BenchmarkSpec] = []
        for spec in selected:
            if spec.kind == "compile":
                expanded.append(
                    BenchmarkSpec(
                        f"{spec.name}_hot",
                        f"{spec.description} (hot cache mode)",
                        kind="compile",
                    )
                )
                expanded.append(
                    BenchmarkSpec(
                        f"{spec.name}_cold",
                        f"{spec.description} (cold cache mode)",
                        kind="compile",
                    )
                )
            else:
                expanded.append(spec)
        selected = expanded

    report = {
        "generated_at": time.strftime("%Y-%m-%d %H:%M:%S %Z"),
        "repeats": args.repeats,
        "warmup": args.warmup,
        "apex_opt_level": args.apex_opt_level,
        "apex_target": args.apex_target,
        "apex_timings": args.apex_timings,
        "compile_mode": "mixed" if args.bench is None else args.compile_mode,
        "benchmarks": [],
    }

    for spec in selected:
        print(f"\n=== {spec.name} ===")
        lang_data: Dict[str, Dict] = {}
        reference_checksum = None

        if spec.kind == "runtime":
            binaries = {
                "apex": exe_path(bin_dir / f"{spec.name}_apex"),
                "rust": exe_path(bin_dir / f"{spec.name}_rust"),
                "go": exe_path(bin_dir / f"{spec.name}_go"),
            }

            compile_apex(
                root,
                spec.name,
                binaries["apex"],
                build_env,
                args.apex_opt_level,
                args.apex_target,
            )
            compile_rust(root, spec.name, binaries["rust"])
            compile_go(root, spec.name, binaries["go"])

            for lang in LANGUAGES:
                print(f"Running {lang}...")
                binary = binaries[lang]

                for _ in range(args.warmup):
                    timed_run(binary, root)

                samples: List[float] = []
                checksums: List[int] = []
                for _ in range(args.repeats):
                    elapsed, checksum = timed_run(binary, root)
                    samples.append(elapsed)
                    checksums.append(checksum)

                if len(set(checksums)) != 1:
                    raise RuntimeError(f"Non-deterministic checksum in {lang}/{spec.name}: {checksums}")

                checksum = checksums[0]
                if reference_checksum is None:
                    reference_checksum = checksum
                elif checksum != reference_checksum:
                    raise RuntimeError(
                        f"Checksum mismatch for {spec.name}: {lang}={checksum}, expected={reference_checksum}"
                    )

                stats = compute_stats(samples)
                lang_data[lang] = {
                    "checksum": checksum,
                    "samples_s": samples,
                    "stats": stats,
                    "metric": "runtime",
                }
        elif spec.kind == "compile":
            compile_mode = args.compile_mode
            base_name = spec.name
            if spec.name.endswith("_hot"):
                compile_mode = "hot"
                base_name = spec.name[: -len("_hot")]
            elif spec.name.endswith("_cold"):
                compile_mode = "cold"
                base_name = spec.name[: -len("_cold")]

            if "synthetic_mega_graph" in base_name or "extreme_graph" in base_name:
                compile_projects = generate_compile_project_synthetic_graph(
                    root, base_name, select_synthetic_graph_config(base_name)
                )
            else:
                compile_projects = generate_compile_project_10_files(root, base_name)
            compile_jobs = make_compile_jobs(root, compile_projects, build_env, args.apex_timings)

            for lang in LANGUAGES:
                print(f"Compiling {lang}...")
                job = compile_jobs[lang]
                apex_timing_samples: List[Dict[str, Dict]] = []

                for _ in range(args.warmup):
                    if compile_mode == "cold":
                        clean_compile_artifacts(lang, job)
                    timed_compile_with_retry(lang, job)

                samples: List[float] = []
                for _ in range(args.repeats):
                    if compile_mode == "cold":
                        clean_compile_artifacts(lang, job)
                    compile_result = timed_compile_with_retry(lang, job)
                    samples.append(compile_result.elapsed_s)
                    if lang == "apex" and args.apex_timings:
                        apex_timing_samples.append(parse_build_timings(compile_result.stdout))

                # Defensive guard for rare transient linker artifact misses in Apex cold mode.
                if not Path(job["binary"]).exists():
                    timed_compile_with_retry(lang, job, retries=2)
                checksum = run_checksum(job["binary"], job["cwd"])
                if reference_checksum is None:
                    reference_checksum = checksum
                elif checksum != reference_checksum:
                    raise RuntimeError(
                        f"Checksum mismatch for {spec.name}: {lang}={checksum}, expected={reference_checksum}"
                    )

                stats = compute_stats(samples)
                lang_data[lang] = {
                    "checksum": checksum,
                    "samples_s": samples,
                    "stats": stats,
                    "metric": "compile",
                }
                if lang == "apex" and args.apex_timings:
                    lang_data[lang]["phase_timings"] = summarize_apex_phase_timings(
                        apex_timing_samples
                    )
            benchmark_compile_mode = compile_mode
        elif spec.kind == "incremental":
            benchmark_compile_mode = args.compile_mode
            mutation_profile = "central" if "central" in spec.name else "leaf"
            for lang in LANGUAGES:
                print(f"Incremental compile {lang}...")
                first_samples: List[float] = []
                second_samples: List[float] = []
                checksums: List[int] = []
                apex_first_phase_samples: List[Dict[str, Dict]] = []
                apex_second_phase_samples: List[Dict[str, Dict]] = []

                cycles = args.warmup + args.repeats
                for i in range(cycles):
                    cycle_projects = generate_compile_project_10_files(
                        root, spec.name, mutation_profile=mutation_profile
                    )
                    cycle_jobs = make_compile_jobs(
                        root, cycle_projects, build_env, args.apex_timings
                    )
                    job = cycle_jobs[lang]

                    if benchmark_compile_mode == "cold":
                        clean_compile_artifacts(lang, job)
                    first_result = timed_compile_with_retry(lang, job)

                    apply_incremental_source_change(lang, Path(job["mutate_source"]), f"{i}")
                    second_result = timed_compile_with_retry(lang, job)

                    if not Path(job["binary"]).exists():
                        timed_compile_with_retry(lang, job, retries=2)
                    checksum = run_checksum(job["binary"], job["cwd"])

                    if i >= args.warmup:
                        first_samples.append(first_result.elapsed_s)
                        second_samples.append(second_result.elapsed_s)
                        checksums.append(checksum)
                        if lang == "apex" and args.apex_timings:
                            apex_first_phase_samples.append(
                                parse_build_timings(first_result.stdout)
                            )
                            apex_second_phase_samples.append(
                                parse_build_timings(second_result.stdout)
                            )

                if len(set(checksums)) != 1:
                    raise RuntimeError(
                        f"Non-deterministic checksum in incremental {lang}/{spec.name}: {checksums}"
                    )

                checksum = checksums[0]
                if reference_checksum is None:
                    reference_checksum = checksum
                elif checksum != reference_checksum:
                    raise RuntimeError(
                        f"Checksum mismatch for {spec.name}: {lang}={checksum}, expected={reference_checksum}"
                    )

                first_stats = compute_stats(first_samples)
                second_stats = compute_stats(second_samples)
                lang_data[lang] = {
                    "checksum": checksum,
                    "first_samples_s": first_samples,
                    "second_samples_s": second_samples,
                    "first_stats": first_stats,
                    "second_stats": second_stats,
                    "stats": second_stats,
                    "metric": "incremental_compile_second",
                }
                if lang == "apex" and args.apex_timings:
                    lang_data[lang]["phase_timings_first"] = summarize_apex_phase_timings(
                        apex_first_phase_samples
                    )
                    lang_data[lang]["phase_timings_second"] = summarize_apex_phase_timings(
                        apex_second_phase_samples
                    )
            phase_one_label = "full compile mean (s)"
            phase_two_label = "rebuild mean (s)"
            ratio_label = "rebuild/full"
        elif spec.kind == "incremental_batch":
            benchmark_compile_mode = "cold_then_hot_batch_edit"
            for lang in LANGUAGES:
                print(f"Incremental batch compile {lang}...")
                first_samples: List[float] = []
                second_samples: List[float] = []
                checksums: List[int] = []
                apex_first_phase_samples: List[Dict[str, Dict]] = []
                apex_second_phase_samples: List[Dict[str, Dict]] = []

                cycles = args.warmup + args.repeats
                for i in range(cycles):
                    cycle_projects = generate_incremental_rebuild_mega_project_10_files(
                        root, spec.name
                    )
                    cycle_jobs = make_compile_jobs(
                        root, cycle_projects, build_env, args.apex_timings
                    )
                    job = cycle_jobs[lang]

                    clean_compile_artifacts(lang, job)
                    first_result = timed_compile_with_retry(lang, job)

                    mutate_sources = [Path(p) for p in job.get("mutate_sources", [])]
                    apply_incremental_source_changes(lang, mutate_sources, f"{i}")
                    second_result = timed_compile_with_retry(lang, job)

                    if not Path(job["binary"]).exists():
                        timed_compile_with_retry(lang, job, retries=2)
                    checksum = run_checksum(job["binary"], job["cwd"])

                    if i >= args.warmup:
                        first_samples.append(first_result.elapsed_s)
                        second_samples.append(second_result.elapsed_s)
                        checksums.append(checksum)
                        if lang == "apex" and args.apex_timings:
                            apex_first_phase_samples.append(
                                parse_build_timings(first_result.stdout)
                            )
                            apex_second_phase_samples.append(
                                parse_build_timings(second_result.stdout)
                            )

                if len(set(checksums)) != 1:
                    raise RuntimeError(
                        f"Non-deterministic checksum in incremental batch {lang}/{spec.name}: {checksums}"
                    )

                checksum = checksums[0]
                if reference_checksum is None:
                    reference_checksum = checksum
                elif checksum != reference_checksum:
                    raise RuntimeError(
                        f"Checksum mismatch for {spec.name}: {lang}={checksum}, expected={reference_checksum}"
                    )

                first_stats = compute_stats(first_samples)
                second_stats = compute_stats(second_samples)
                lang_data[lang] = {
                    "checksum": checksum,
                    "first_samples_s": first_samples,
                    "second_samples_s": second_samples,
                    "first_stats": first_stats,
                    "second_stats": second_stats,
                    "stats": second_stats,
                    "metric": "incremental_compile_second",
                }
                if lang == "apex" and args.apex_timings:
                    lang_data[lang]["phase_timings_first"] = summarize_apex_phase_timings(
                        apex_first_phase_samples
                    )
                    lang_data[lang]["phase_timings_second"] = summarize_apex_phase_timings(
                        apex_second_phase_samples
                    )
            phase_one_label = "cold full build mean (s)"
            phase_two_label = "hot rebuild after 10 edits mean (s)"
            ratio_label = "hot/cold"
        elif spec.kind in ("incremental_batch_synthetic_mega_graph", "incremental_batch_extreme_graph"):
            benchmark_compile_mode = "cold_then_hot_batch_edit"
            graph_config = select_synthetic_graph_config(spec.name)
            for lang in LANGUAGES:
                print(f"Synthetic graph incremental compile {lang}...")
                first_samples: List[float] = []
                second_samples: List[float] = []
                checksums: List[int] = []
                apex_first_phase_samples: List[Dict[str, Dict]] = []
                apex_second_phase_samples: List[Dict[str, Dict]] = []

                cycles = args.warmup + args.repeats
                for i in range(cycles):
                    cycle_projects = generate_compile_project_synthetic_graph(
                        root, spec.name, graph_config, mutate_count=graph_config.mutate_count
                    )
                    cycle_jobs = make_compile_jobs(
                        root, cycle_projects, build_env, args.apex_timings
                    )
                    job = cycle_jobs[lang]

                    clean_compile_artifacts(lang, job)
                    first_result = timed_compile_with_retry(lang, job)

                    mutate_sources = [Path(p) for p in job.get("mutate_sources", [])]
                    apply_incremental_source_changes(lang, mutate_sources, f"{i}")
                    second_result = timed_compile_with_retry(lang, job)

                    if not Path(job["binary"]).exists():
                        timed_compile_with_retry(lang, job, retries=2)
                    checksum = run_checksum(job["binary"], job["cwd"])

                    if i >= args.warmup:
                        first_samples.append(first_result.elapsed_s)
                        second_samples.append(second_result.elapsed_s)
                        checksums.append(checksum)
                        if lang == "apex" and args.apex_timings:
                            apex_first_phase_samples.append(
                                parse_build_timings(first_result.stdout)
                            )
                            apex_second_phase_samples.append(
                                parse_build_timings(second_result.stdout)
                            )

                if len(set(checksums)) != 1:
                    raise RuntimeError(
                        f"Non-deterministic checksum in synthetic mega-graph incremental {lang}/{spec.name}: {checksums}"
                    )

                checksum = checksums[0]
                if reference_checksum is None:
                    reference_checksum = checksum
                elif checksum != reference_checksum:
                    raise RuntimeError(
                        f"Checksum mismatch for {spec.name}: {lang}={checksum}, expected={reference_checksum}"
                    )

                first_stats = compute_stats(first_samples)
                second_stats = compute_stats(second_samples)
                lang_data[lang] = {
                    "checksum": checksum,
                    "first_samples_s": first_samples,
                    "second_samples_s": second_samples,
                    "first_stats": first_stats,
                    "second_stats": second_stats,
                    "stats": second_stats,
                    "metric": "incremental_compile_second",
                }
                if lang == "apex" and args.apex_timings:
                    lang_data[lang]["phase_timings_first"] = summarize_apex_phase_timings(
                        apex_first_phase_samples
                    )
                    lang_data[lang]["phase_timings_second"] = summarize_apex_phase_timings(
                        apex_second_phase_samples
                    )
            phase_one_label = "cold full build mean (s)"
            phase_two_label = f"hot rebuild after {graph_config.mutate_count} edits mean (s)"
            ratio_label = "hot/cold"
        elif spec.kind in ("incremental_mixed_synthetic_mega_graph", "incremental_mixed_extreme_graph"):
            benchmark_compile_mode = "cold_then_hot_mixed_invalidation"
            graph_config = select_synthetic_graph_config(spec.name)
            for lang in LANGUAGES:
                print(f"Synthetic graph mixed invalidation compile {lang}...")
                first_samples: List[float] = []
                second_samples: List[float] = []
                checksums: List[int] = []
                apex_first_phase_samples: List[Dict[str, Dict]] = []
                apex_second_phase_samples: List[Dict[str, Dict]] = []

                cycles = args.warmup + args.repeats
                for i in range(cycles):
                    cycle_projects = generate_compile_project_synthetic_graph(
                        root, spec.name, graph_config, mutate_count=graph_config.mutate_count
                    )
                    cycle_jobs = make_compile_jobs(
                        root, cycle_projects, build_env, args.apex_timings
                    )
                    job = cycle_jobs[lang]

                    clean_compile_artifacts(lang, job)
                    first_result = timed_compile_with_retry(lang, job)

                    apply_mixed_invalidation_changes(lang, job, f"{i}")
                    second_result = timed_compile_with_retry(lang, job)

                    if not Path(job["binary"]).exists():
                        timed_compile_with_retry(lang, job, retries=2)
                    checksum = run_checksum(job["binary"], job["cwd"])

                    if i >= args.warmup:
                        first_samples.append(first_result.elapsed_s)
                        second_samples.append(second_result.elapsed_s)
                        checksums.append(checksum)
                        if lang == "apex" and args.apex_timings:
                            apex_first_phase_samples.append(
                                parse_build_timings(first_result.stdout)
                            )
                            apex_second_phase_samples.append(
                                parse_build_timings(second_result.stdout)
                            )

                if len(set(checksums)) != 1:
                    raise RuntimeError(
                        f"Non-deterministic checksum in mixed synthetic mega-graph invalidation {lang}/{spec.name}: {checksums}"
                    )

                checksum = checksums[0]
                if reference_checksum is None:
                    reference_checksum = checksum
                elif checksum != reference_checksum:
                    raise RuntimeError(
                        f"Checksum mismatch for {spec.name}: {lang}={checksum}, expected={reference_checksum}"
                    )

                first_stats = compute_stats(first_samples)
                second_stats = compute_stats(second_samples)
                lang_data[lang] = {
                    "checksum": checksum,
                    "first_samples_s": first_samples,
                    "second_samples_s": second_samples,
                    "first_stats": first_stats,
                    "second_stats": second_stats,
                    "stats": second_stats,
                    "metric": "incremental_compile_second",
                }
                if lang == "apex" and args.apex_timings:
                    lang_data[lang]["phase_timings_first"] = summarize_apex_phase_timings(
                        apex_first_phase_samples
                    )
                    lang_data[lang]["phase_timings_second"] = summarize_apex_phase_timings(
                        apex_second_phase_samples
                    )
            phase_one_label = "cold full build mean (s)"
            phase_two_label = (
                f"mixed rebuild mean (leaf {graph_config.mixed_leaf_edits} + "
                f"{graph_config.mixed_group_edits} API groups)"
            )
            ratio_label = "mixed/cold"
        else:
            raise RuntimeError(f"Unsupported benchmark kind: {spec.kind}")

        apex_mean = lang_data["apex"]["stats"]["mean_s"]
        speedups = {
            lang: apex_mean / lang_data[lang]["stats"]["mean_s"]
            for lang in LANGUAGES
            if lang != "apex"
        }
        apex_phase_timing_sections: List[Dict] = []
        apex_compile_phase_timings = lang_data["apex"].get("phase_timings")
        if apex_compile_phase_timings:
            apex_phase_timing_sections.append(
                {"label": "Apex build phase timings", "phases": apex_compile_phase_timings}
            )
        apex_first_phase_timings = lang_data["apex"].get("phase_timings_first")
        if apex_first_phase_timings:
            apex_phase_timing_sections.append(
                {"label": f"Apex {phase_one_label}", "phases": apex_first_phase_timings}
            )
        apex_second_phase_timings = lang_data["apex"].get("phase_timings_second")
        if apex_second_phase_timings:
            apex_phase_timing_sections.append(
                {"label": f"Apex {phase_two_label}", "phases": apex_second_phase_timings}
            )

        report["benchmarks"].append(
            {
                "name": spec.name,
                "description": spec.description,
                "kind": spec.kind,
                "compile_mode": benchmark_compile_mode if spec.kind in ("compile", "incremental", "incremental_batch", "incremental_batch_synthetic_mega_graph", "incremental_mixed_synthetic_mega_graph", "incremental_batch_extreme_graph", "incremental_mixed_extreme_graph") else None,
                "phase_one_label": phase_one_label if spec.kind in ("incremental", "incremental_batch", "incremental_batch_synthetic_mega_graph", "incremental_mixed_synthetic_mega_graph", "incremental_batch_extreme_graph", "incremental_mixed_extreme_graph") else None,
                "phase_two_label": phase_two_label if spec.kind in ("incremental", "incremental_batch", "incremental_batch_synthetic_mega_graph", "incremental_mixed_synthetic_mega_graph", "incremental_batch_extreme_graph", "incremental_mixed_extreme_graph") else None,
                "ratio_label": ratio_label if spec.kind in ("incremental", "incremental_batch", "incremental_batch_synthetic_mega_graph", "incremental_mixed_synthetic_mega_graph", "incremental_batch_extreme_graph", "incremental_mixed_extreme_graph") else None,
                "languages": lang_data,
                "speedup_vs_apex": speedups,
                "apex_phase_timing_sections": apex_phase_timing_sections,
            }
        )

    json_out = out_dir / "latest.json"
    md_out = out_dir / "latest.md"
    json_out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    md_out.write_text(build_markdown(report), encoding="utf-8")

    print(f"\nWrote: {json_out}")
    print(f"Wrote: {md_out}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
