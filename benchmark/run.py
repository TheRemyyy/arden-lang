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


BENCHMARKS: List[BenchmarkSpec] = [
    BenchmarkSpec("sum_loop", "Integer-heavy pseudo-random accumulation loop"),
    BenchmarkSpec("prime_count", "Prime counting via sieve"),
    BenchmarkSpec("matrix_mul", "Dense matrix multiplication (100x100)"),
    BenchmarkSpec(
        "compile_project_10_files",
        "Compile stress test on generated 10-file project per language",
        kind="compile",
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
]

LANGUAGES = ("apex", "c", "rust", "go")


def is_windows() -> bool:
    return os.name == "nt"


def exe_path(path: Path) -> Path:
    if is_windows() and path.suffix.lower() != ".exe":
        return path.with_suffix(".exe")
    return path


def pick_c_compiler() -> str:
    cc_env = os.environ.get("CC", "").strip()
    if cc_env:
        return cc_env
    if shutil.which("clang"):
        return "clang"
    if shutil.which("gcc"):
        return "gcc"
    raise RuntimeError("C compiler not found. Install clang/gcc or set CC.")


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


def compile_c(root: Path, bench: str, out: Path, c_compiler: str) -> None:
    src = root / "benchmark" / "c" / f"{bench}.c"
    cmd = [c_compiler, "-O3", "-std=c11", str(src), "-o", str(out)]
    if c_compiler != "cl":
        cmd.insert(2, "-march=native")
    proc = run_cmd(cmd, root)
    if proc.returncode != 0:
        raise RuntimeError(f"Failed to compile C benchmark {bench}:\n{proc.stderr}")


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


def timed_compile(cmd: List[str], cwd: Path, env: Dict[str, str] | None = None) -> float:
    start = time.perf_counter()
    proc = run_cmd(cmd, cwd, env=env)
    elapsed = time.perf_counter() - start
    if proc.returncode != 0:
        raise RuntimeError(
            f"Compile failed: {' '.join(cmd)}\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return elapsed


def timed_compile_with_retry(lang: str, job: Dict, retries: int = 1) -> float:
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


def run_checksum(binary: Path, cwd: Path) -> int:
    proc = run_cmd([str(binary)], cwd)
    if proc.returncode != 0:
        raise RuntimeError(
            f"Binary execution failed: {binary}\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return parse_checksum(proc.stdout)


def generate_compile_project_10_files(
    root: Path, bench_name: str, mutation_profile: str = "leaf"
) -> Dict[str, Dict[str, Path]]:
    generated_root = root / "benchmark" / "generated" / bench_name
    if generated_root.exists():
        shutil.rmtree(generated_root)
    generated_root.mkdir(parents=True, exist_ok=True)

    file_count = 10
    funcs_per_file = 180

    apex_dir = generated_root / "apex"
    apex_src = apex_dir / "src"
    apex_src.mkdir(parents=True, exist_ok=True)
    apex_files = []
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

    c_dir = generated_root / "c"
    c_dir.mkdir(parents=True, exist_ok=True)
    (c_dir / "core.h").write_text(
        "\n".join(
            [
                "#ifndef CORE_H",
                "#define CORE_H",
                "#include <stdint.h>",
                "int64_t core_mix(int64_t x, int64_t k);",
                "#endif",
                "",
            ]
        ),
        encoding="utf-8",
    )
    (c_dir / "core.c").write_text(
        "\n".join(
            [
                "#include <stdint.h>",
                "int64_t core_mix(int64_t x, int64_t k) {",
                "    return x + k;",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    for i in range(file_count):
        part = f"part_{i:02d}"
        lines = [
            "#include <stdint.h>",
            '#include "core.h"',
            f"int64_t {part}_apply(int64_t x) {{",
            "    int64_t y = x;",
        ]
        for j in range(funcs_per_file):
            lines.append(f"    y = core_mix(y, {i + j + 1});")
        lines.extend(["    return y;", "}"])
        (c_dir / f"{part}.c").write_text("\n".join(lines) + "\n", encoding="utf-8")
    main_c = ["#include <stdint.h>", "#include <stdio.h>"]
    for i in range(file_count):
        main_c.append(f"int64_t part_{i:02d}_apply(int64_t x);")
    main_c.extend(["int main(void) {", "    int64_t acc = 0;"])
    for i in range(file_count):
        main_c.append(f"    acc = part_{i:02d}_apply(acc);")
    main_c.extend(['    printf("%lld\\n", (long long)acc);', "    return 0;", "}"])
    (c_dir / "main.c").write_text("\n".join(main_c) + "\n", encoding="utf-8")

    rust_dir = generated_root / "rust"
    rust_dir.mkdir(parents=True, exist_ok=True)
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
    (go_dir / "go.mod").write_text("module compile10\n\ngo 1.22\n", encoding="utf-8")
    go_main = ['package main', "", 'import "fmt"', "", "func main() {", "    var acc int64 = 0"]
    for i in range(file_count):
        go_main.append(f"    acc = part_{i:02d}_apply(acc)")
    go_main.extend(["    fmt.Println(acc)", "}"])
    (go_dir / "main.go").write_text("\n".join(go_main) + "\n", encoding="utf-8")
    for i in range(file_count):
        part = f"part_{i:02d}"
        lines = ["package main", "", "func " + part + "_apply(x int64) int64 {", "    y := x"]
        for j in range(funcs_per_file):
            lines.append(f"    y = coreMix(y, {i + j + 1})")
        lines.extend(["    return y", "}"])
        (go_dir / f"{part}.go").write_text("\n".join(lines) + "\n", encoding="utf-8")
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

    apex_mutate = apex_src / "part_09.apex"
    c_mutate = c_dir / "part_09.c"
    rust_mutate = rust_dir / "part_09.rs"
    go_mutate = go_dir / "part_09.go"
    if mutation_profile == "central":
        apex_mutate = apex_core
        c_mutate = c_dir / "core.c"
        rust_mutate = rust_dir / "core.rs"
        go_mutate = go_dir / "core.go"

    return {
        "apex": {
            "project_dir": apex_dir,
            "binary": apex_dir / bench_name,
            "mutate_source": apex_mutate,
        },
        "c": {
            "project_dir": c_dir,
            "binary": c_dir / f"{bench_name}_c",
            "mutate_source": c_mutate,
        },
        "rust": {
            "project_dir": rust_dir,
            "binary": rust_dir / f"{bench_name}_rust",
            "mutate_source": rust_mutate,
        },
        "go": {
            "project_dir": go_dir,
            "binary": go_dir / f"{bench_name}_go",
            "mutate_source": go_mutate,
        },
    }


def make_compile_jobs(
    root: Path,
    compile_projects: Dict[str, Dict[str, Path]],
    build_env: Dict[str, str],
    c_compiler: str,
) -> Dict[str, Dict]:
    compiler = root / "target" / "release" / "apex-compiler"
    return {
        "apex": {
            "cmd": [str(compiler), "build", "--no-check"],
            "cwd": compile_projects["apex"]["project_dir"],
            "env": build_env,
            "binary": exe_path(compile_projects["apex"]["binary"]),
            "mutate_source": compile_projects["apex"]["mutate_source"],
        },
        "c": {
            "cmd": [
                c_compiler,
                "-O3",
                "-march=native",
                "-std=c11",
                *[
                    str(path)
                    for path in sorted(compile_projects["c"]["project_dir"].glob("*.c"))
                ],
                "-o",
                str(compile_projects["c"]["binary"]),
            ],
            "cwd": compile_projects["c"]["project_dir"],
            "env": None,
            "binary": exe_path(compile_projects["c"]["binary"]),
            "mutate_source": compile_projects["c"]["mutate_source"],
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
        },
    }


def apply_incremental_source_change(lang: str, source: Path, marker: str) -> None:
    if not source.exists():
        raise RuntimeError(f"Missing source to mutate: {source}")
    prefix = "//" if lang in ("apex", "rust", "go") else "/*"
    suffix = "" if lang in ("apex", "rust", "go") else " */"
    line = f"\n{prefix} incremental bench mutation {marker}{suffix}\n"
    with source.open("a", encoding="utf-8") as f:
        f.write(line)


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
    lines.append(f"- C compiler: `{result.get('c_compiler', 'n/a')}`")
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
        if bench.get("kind") == "incremental":
            lines.append(
                "| Language | Checksum | first mean (s) | second mean (s) | second/first |"
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
    parser = argparse.ArgumentParser(description="Run Apex vs C vs Rust benchmarks")
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
    c_compiler = pick_c_compiler()
    ensure_tool(c_compiler)
    ensure_tool("rustc")
    ensure_tool("go")
    ensure_tool("cargo")

    llvm_prefix = detect_llvm_prefix()
    build_env = {"LLVM_SYS_211_PREFIX": llvm_prefix}

    if not args.no_build:
        proc = run_cmd(["cargo", "build", "--release"], root, env=build_env)
        if proc.returncode != 0:
            raise RuntimeError(f"Failed to build Apex compiler:\n{proc.stderr}")

    selected = [b for b in BENCHMARKS if args.bench is None or b.name == args.bench]

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
        "c_compiler": c_compiler,
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
                "c": exe_path(bin_dir / f"{spec.name}_c"),
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
            compile_c(root, spec.name, binaries["c"], c_compiler)
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

            compile_projects = generate_compile_project_10_files(root, base_name)
            compile_jobs = make_compile_jobs(root, compile_projects, build_env, c_compiler)

            for lang in LANGUAGES:
                print(f"Compiling {lang}...")
                job = compile_jobs[lang]

                for _ in range(args.warmup):
                    if compile_mode == "cold":
                        clean_compile_artifacts(lang, job)
                    timed_compile_with_retry(lang, job)

                samples: List[float] = []
                for _ in range(args.repeats):
                    if compile_mode == "cold":
                        clean_compile_artifacts(lang, job)
                    samples.append(timed_compile_with_retry(lang, job))

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
            benchmark_compile_mode = compile_mode
        elif spec.kind == "incremental":
            benchmark_compile_mode = args.compile_mode
            mutation_profile = "central" if "central" in spec.name else "leaf"
            for lang in LANGUAGES:
                print(f"Incremental compile {lang}...")
                first_samples: List[float] = []
                second_samples: List[float] = []
                checksums: List[int] = []

                cycles = args.warmup + args.repeats
                for i in range(cycles):
                    cycle_projects = generate_compile_project_10_files(
                        root, spec.name, mutation_profile=mutation_profile
                    )
                    cycle_jobs = make_compile_jobs(root, cycle_projects, build_env, c_compiler)
                    job = cycle_jobs[lang]

                    if benchmark_compile_mode == "cold":
                        clean_compile_artifacts(lang, job)
                    first_elapsed = timed_compile_with_retry(lang, job)

                    apply_incremental_source_change(lang, Path(job["mutate_source"]), f"{i}")
                    second_elapsed = timed_compile_with_retry(lang, job)

                    if not Path(job["binary"]).exists():
                        timed_compile_with_retry(lang, job, retries=2)
                    checksum = run_checksum(job["binary"], job["cwd"])

                    if i >= args.warmup:
                        first_samples.append(first_elapsed)
                        second_samples.append(second_elapsed)
                        checksums.append(checksum)

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
        else:
            raise RuntimeError(f"Unsupported benchmark kind: {spec.kind}")

        apex_mean = lang_data["apex"]["stats"]["mean_s"]
        speedups = {
            lang: apex_mean / lang_data[lang]["stats"]["mean_s"]
            for lang in LANGUAGES
            if lang != "apex"
        }

        report["benchmarks"].append(
            {
                "name": spec.name,
                "description": spec.description,
                "kind": spec.kind,
                "compile_mode": benchmark_compile_mode if spec.kind in ("compile", "incremental") else None,
                "languages": lang_data,
                "speedup_vs_apex": speedups,
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
