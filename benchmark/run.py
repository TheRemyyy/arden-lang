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
]

LANGUAGES = ("apex", "c", "rust", "go")


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
    cmd = [c_compiler, "-O3", "-march=native", "-std=c11", str(src), "-o", str(out)]
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


def generate_compile_project_10_files(root: Path) -> Dict[str, Dict[str, Path]]:
    generated_root = root / "benchmark" / "generated" / "compile_project_10_files"
    if generated_root.exists():
        shutil.rmtree(generated_root)
    generated_root.mkdir(parents=True, exist_ok=True)

    file_count = 10
    funcs_per_file = 180

    apex_dir = generated_root / "apex"
    apex_src = apex_dir / "src"
    apex_src.mkdir(parents=True, exist_ok=True)
    apex_files = []
    for i in range(file_count):
        part = f"part_{i:02d}"
        apex_files.append(f"src/{part}.apex")
        lines: List[str] = ["import std.io.*;", ""]
        for j in range(funcs_per_file):
            lines.append(f"function {part}_f{j:03d}(x: Integer): Integer {{ return x + {i + j + 1}; }}")
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
        'name = "compile_project_10_files"',
        'version = "0.1.0"',
        'entry = "src/main.apex"',
        "files = [",
    ]
    toml_lines.extend([f'    "{f}",' for f in apex_files])
    toml_lines.extend(["]", 'output = "compile_project_10_files"', 'opt_level = "3"'])
    (apex_dir / "apex.toml").write_text("\n".join(toml_lines) + "\n", encoding="utf-8")

    c_dir = generated_root / "c"
    c_dir.mkdir(parents=True, exist_ok=True)
    for i in range(file_count):
        part = f"part_{i:02d}"
        lines = ["#include <stdint.h>", f"int64_t {part}_apply(int64_t x) {{", "    int64_t y = x;"]
        for j in range(funcs_per_file):
            lines.append(f"    y = y + {i + j + 1};")
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
    rust_main = []
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
            lines.append(f"    y += {i + j + 1};")
        lines.extend(["    y", "}"])
        (rust_dir / f"{part}.rs").write_text("\n".join(lines) + "\n", encoding="utf-8")

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
            lines.append(f"    y += {i + j + 1}")
        lines.extend(["    return y", "}"])
        (go_dir / f"{part}.go").write_text("\n".join(lines) + "\n", encoding="utf-8")

    return {
        "apex": {
            "project_dir": apex_dir,
            "binary": apex_dir / "compile_project_10_files",
        },
        "c": {
            "project_dir": c_dir,
            "binary": c_dir / "compile_project_10_files_c",
        },
        "rust": {
            "project_dir": rust_dir,
            "binary": rust_dir / "compile_project_10_files_rust",
        },
        "go": {
            "project_dir": go_dir,
            "binary": go_dir / "compile_project_10_files_go",
        },
    }


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
    lines.append(f"- Compile mode: `{result.get('compile_mode', 'hot')}`")
    lines.append("")

    for bench in result["benchmarks"]:
        lines.append(f"## {bench['name']}")
        lines.append("")
        lines.append(f"{bench['description']}")
        lines.append("")
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
    ensure_tool("clang")
    ensure_tool("rustc")
    ensure_tool("go")
    ensure_tool("cargo")
    c_compiler = "clang"

    llvm_prefix = detect_llvm_prefix()
    build_env = {"LLVM_SYS_211_PREFIX": llvm_prefix}

    if not args.no_build:
        proc = run_cmd(["cargo", "build", "--release"], root, env=build_env)
        if proc.returncode != 0:
            raise RuntimeError(f"Failed to build Apex compiler:\n{proc.stderr}")

    selected = [b for b in BENCHMARKS if args.bench is None or b.name == args.bench]

    report = {
        "generated_at": time.strftime("%Y-%m-%d %H:%M:%S %Z"),
        "repeats": args.repeats,
        "warmup": args.warmup,
        "apex_opt_level": args.apex_opt_level,
        "apex_target": args.apex_target,
        "c_compiler": c_compiler,
        "compile_mode": args.compile_mode,
        "benchmarks": [],
    }

    compile_projects = None
    if any(spec.kind == "compile" for spec in selected):
        compile_projects = generate_compile_project_10_files(root)

    for spec in selected:
        print(f"\n=== {spec.name} ===")
        lang_data: Dict[str, Dict] = {}
        reference_checksum = None

        if spec.kind == "runtime":
            binaries = {
                "apex": bin_dir / f"{spec.name}_apex",
                "c": bin_dir / f"{spec.name}_c",
                "rust": bin_dir / f"{spec.name}_rust",
                "go": bin_dir / f"{spec.name}_go",
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
            if compile_projects is None:
                raise RuntimeError("Internal error: compile projects were not generated")

            compiler = root / "target" / "release" / "apex-compiler"
            compile_jobs = {
                "apex": {
                    "cmd": [str(compiler), "build"],
                    "cwd": compile_projects["apex"]["project_dir"],
                    "env": build_env,
                    "binary": compile_projects["apex"]["binary"],
                },
                "c": {
                    "cmd": [
                        c_compiler,
                        "-O3",
                        "-march=native",
                        "-std=c11",
                        *[str(compile_projects["c"]["project_dir"] / f"part_{i:02d}.c") for i in range(10)],
                        str(compile_projects["c"]["project_dir"] / "main.c"),
                        "-o",
                        str(compile_projects["c"]["binary"]),
                    ],
                    "cwd": compile_projects["c"]["project_dir"],
                    "env": None,
                    "binary": compile_projects["c"]["binary"],
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
                    "binary": compile_projects["rust"]["binary"],
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
                        "GOCACHE": str(
                            compile_projects["go"]["project_dir"] / ".gocache"
                        ),
                    },
                    "binary": compile_projects["go"]["binary"],
                    "go_cache_dir": compile_projects["go"]["project_dir"] / ".gocache",
                },
            }

            for lang in LANGUAGES:
                print(f"Compiling {lang}...")
                job = compile_jobs[lang]

                for _ in range(args.warmup):
                    if args.compile_mode == "cold":
                        clean_compile_artifacts(lang, job)
                    timed_compile_with_retry(lang, job)

                samples: List[float] = []
                for _ in range(args.repeats):
                    if args.compile_mode == "cold":
                        clean_compile_artifacts(lang, job)
                    samples.append(timed_compile_with_retry(lang, job))

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
