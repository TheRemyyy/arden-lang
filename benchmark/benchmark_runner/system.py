import os
import shutil
import statistics
import subprocess
import time
from pathlib import Path


def is_windows() -> bool:
    return os.name == "nt"


def exe_path(path: Path) -> Path:
    if is_windows() and path.suffix.lower() != ".exe":
        return path.with_suffix(".exe")
    return path


def arden_binary_path(root: Path) -> Path:
    return exe_path(root / "target" / "release" / "arden")


def run_cmd(cmd: list[str], cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
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


def compute_stats(values: list[float]) -> dict[str, float]:
    return {
        "min_s": min(values),
        "mean_s": statistics.mean(values),
        "median_s": statistics.median(values),
        "max_s": max(values),
        "stddev_s": statistics.pstdev(values) if len(values) > 1 else 0.0,
    }


def format_seconds(value: float) -> str:
    return f"{value:.6f}"


def detect_llvm_prefix() -> str:
    from_env = os.environ.get("LLVM_SYS_221_PREFIX", "").strip()
    if from_env:
        return from_env

    from_env = os.environ.get("LLVM_SYS_211_PREFIX", "").strip()
    if from_env:
        return from_env

    llvm_config = shutil.which("llvm-config")
    if not llvm_config:
        raise RuntimeError(
            "LLVM prefix not found. Set LLVM_SYS_221_PREFIX or install llvm-config."
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


def current_timestamp() -> str:
    return time.strftime("%Y-%m-%d %H:%M:%S %Z")
