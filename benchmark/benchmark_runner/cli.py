import argparse
import json
from pathlib import Path

from .execution import run_selected_benchmarks
from .reporting import build_markdown
from .specs import BENCHMARKS, expand_default_suite, select_benchmarks
from .system import current_timestamp, detect_llvm_prefix, ensure_tool, run_cmd


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run Arden vs Rust vs Go benchmarks")
    parser.add_argument("--repeats", type=int, default=5, help="Timed runs per benchmark/language")
    parser.add_argument("--warmup", type=int, default=1, help="Warmup runs per benchmark/language")
    parser.add_argument(
        "--arden-opt-level",
        choices=["0", "1", "2", "3", "s", "z", "fast"],
        default="3",
        help="Optimization level passed to `arden compile --opt-level`",
    )
    parser.add_argument(
        "--arden-target",
        default=None,
        help="Optional target triple passed to `arden compile --target`",
    )
    parser.add_argument(
        "--bench",
        default=None,
        metavar="BENCHMARK",
        help=(
            "Run only one benchmark. Canonical names: "
            + ", ".join(spec.name for spec in BENCHMARKS)
            + ". Legacy aliases still work."
        ),
    )
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="Skip building arden compiler with cargo build --release",
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
        "--arden-timings",
        action="store_true",
        help="Pass --timings to Arden project builds and record per-phase timing breakdowns in reports.",
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.repeats < 1:
        raise RuntimeError("--repeats must be >= 1")
    if args.warmup < 0:
        raise RuntimeError("--warmup must be >= 0")

    root = Path(__file__).resolve().parents[2]
    bench_dir = root / "benchmark"
    bin_dir = bench_dir / "bin"
    out_dir = bench_dir / "results"
    bin_dir.mkdir(parents=True, exist_ok=True)
    out_dir.mkdir(parents=True, exist_ok=True)

    for tool in ("python3", "rustc", "go", "cargo"):
        ensure_tool(tool)

    build_env = {"LLVM_SYS_211_PREFIX": detect_llvm_prefix()}
    print(f"Benchmark root: {bench_dir}", flush=True)
    print(f"Results dir: {out_dir}", flush=True)
    print(
        f"Config: repeats={args.repeats}, warmup={args.warmup}, compile_mode={args.compile_mode}, "
        f"arden_opt_level={args.arden_opt_level}, no_build={args.no_build}",
        flush=True,
    )
    if not args.no_build:
        print("Building target/release/arden...", flush=True)
        proc = run_cmd(["cargo", "build", "--release"], root, env=build_env)
        if proc.returncode != 0:
            raise RuntimeError(f"Failed to build Arden:\n{proc.stderr}")
        print("Built target/release/arden", flush=True)

    selected = select_benchmarks(args.bench, args.include_extreme)
    if args.bench is None:
        selected = expand_default_suite(selected)
    print(
        "Selected benchmarks: " + ", ".join(spec.name for spec in selected),
        flush=True,
    )

    report = {
        "generated_at": current_timestamp(),
        "repeats": args.repeats,
        "warmup": args.warmup,
        "arden_opt_level": args.arden_opt_level,
        "arden_target": args.arden_target,
        "arden_timings": args.arden_timings,
        "compile_mode": "mixed" if args.bench is None else args.compile_mode,
        "benchmarks": run_selected_benchmarks(
            selected,
            root,
            bin_dir,
            build_env,
            args.arden_opt_level,
            args.arden_target,
            args.compile_mode,
            args.warmup,
            args.repeats,
            args.arden_timings,
        ),
    }

    json_out = out_dir / "latest.json"
    md_out = out_dir / "latest.md"
    json_out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    md_out.write_text(build_markdown(report), encoding="utf-8")

    print(f"\nWrote: {json_out}")
    print(f"Wrote: {md_out}")
    return 0
