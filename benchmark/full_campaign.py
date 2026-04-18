#!/usr/bin/env python3
"""Run a full, repository-wide performance measurement campaign.

This script orchestrates multiple benchmark stages across Arden, Rust, and Go,
collects a large volume of structured data, and writes combined reports to a
timestamped results directory.

Presets
-------
quick       Fast sanity check — ~4–8 min. Four stages covering a broader
            runtime mix, realistic cold builds across several graph shapes,
            hot-compile follow-up timings, and small-project incremental
            rebuild checks. Good for verifying the harness works before a full run.

full        Publication-grade default — ~15–30 min. Six stages: standard
            runtime benchmarks, heavy runtime with profile capture, hot-compile,
            cold-compile, small-project incremental (body-only + API cascade),
            and large/mega-graph incremental. Use this for dev.to article data.

exhaustive  Full matrix including stress/extreme benchmarks — ~60+ min. Nine
            stages: everything in full plus 7-repeat article-grade runs,
            2200-file extreme-graph hot/cold compile, and extreme-graph
            incremental rebuild stress tests.

Usage examples
--------------
    # Quick sanity check (compiler already built)
    python3 benchmark/full_campaign.py --preset quick --no-build

    # Publication-grade full run (compiler already built)
    python3 benchmark/full_campaign.py --preset full --no-build

    # Exhaustive run including stress benchmarks
    python3 benchmark/full_campaign.py --preset exhaustive --no-build

    # Preview the plan without running anything
    python3 benchmark/full_campaign.py --preset full --dry-run

    # Build compiler then run full campaign
    LLVM_SYS_221_PREFIX=/usr/lib/llvm-22 python3 benchmark/full_campaign.py --preset full

Outputs
-------
All results are written to:

    benchmark/results/campaign_<YYYYMMDD_HHMMSS>/

Inside that directory:

    campaign_summary.json      Combined machine-readable report (all stages)
    campaign_summary.md        Master summary + per-stage detail tables
    campaign_summary.csv       Tabular export — one row per language per phase
    stage_NN_<name>.json       Per-stage raw JSON results
    stage_NN_<name>.md         Per-stage markdown detail report
    README.md                  Methodology and reproduce instructions
"""

import argparse
import sys
from datetime import datetime, timezone
from pathlib import Path

# Ensure the benchmark package is importable when run directly.
sys.path.insert(0, str(Path(__file__).resolve().parent))

from benchmark_runner.campaign import PRESETS, run_campaign
from benchmark_runner.system import detect_llvm_prefix, ensure_tool


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run a full Arden benchmark campaign and collect bulk performance data.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--preset",
        choices=list(PRESETS.keys()),
        default="full",
        help=(
            "Campaign preset. "
            "quick: fast sanity check (~4–8 min). "
            "full: publication-grade default (~15–30 min). "
            "exhaustive: full matrix with stress benchmarks (~60+ min). "
            "[default: full]"
        ),
    )
    parser.add_argument(
        "--no-build",
        action="store_true",
        help=(
            "Skip `cargo build --release`. "
            "The Arden binary must already exist at target/release/arden(.exe)."
        ),
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the campaign plan (stages, benchmarks, params) without running anything.",
    )
    parser.add_argument(
        "--arden-opt-level",
        choices=["0", "1", "2", "3", "s", "z", "fast"],
        default="3",
        help="Arden optimisation level passed to `arden compile`. [default: 3]",
    )
    parser.add_argument(
        "--arden-target",
        default=None,
        metavar="TRIPLE",
        help="Optional cross-compilation target triple passed to `arden compile`.",
    )
    parser.add_argument(
        "--output-dir",
        default=None,
        metavar="DIR",
        help=(
            "Override the output directory. "
            "Default: benchmark/results/campaign_<YYYYMMDD_HHMMSS>/"
        ),
    )
    return parser


def main() -> int:
    args = _build_parser().parse_args()

    root = Path(__file__).resolve().parents[1]
    bench_dir = root / "benchmark"
    results_root = bench_dir / "results"

    if args.output_dir:
        out_dir = Path(args.output_dir).resolve()
    else:
        ts = datetime.now(tz=timezone.utc).strftime("%Y%m%d_%H%M%S")
        out_dir = results_root / f"campaign_{ts}"

    if not args.dry_run:
        for tool in ("python3", "rustc", "go", "cargo"):
            ensure_tool(tool)
        build_env: dict[str, str] = {"LLVM_SYS_221_PREFIX": detect_llvm_prefix()}
    else:
        build_env = {}

    # Canonical command string recorded in the output README for reproducibility.
    command_parts = [
        "python3",
        "benchmark/full_campaign.py",
        f"--preset={args.preset}",
    ]
    if args.no_build:
        command_parts.append("--no-build")
    if args.arden_opt_level != "3":
        command_parts.append(f"--arden-opt-level={args.arden_opt_level}")
    if args.arden_target:
        command_parts.append(f"--arden-target={args.arden_target}")
    command = " ".join(command_parts)

    try:
        result = run_campaign(
            preset=args.preset,
            root=root,
            out_dir=out_dir,
            build_env=build_env,
            arden_opt_level=args.arden_opt_level,
            arden_target=args.arden_target,
            no_build=args.no_build,
            dry_run=args.dry_run,
            command=command,
        )
    except RuntimeError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if not args.dry_run and result:
        print(f"\nResults written to: {out_dir}", flush=True)

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130)
    except RuntimeError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
