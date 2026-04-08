import shutil
import time
from pathlib import Path

from .system import exe_path, parse_checksum, run_cmd
from .types import TimedCompileResult


def compile_arden(
    root: Path,
    bench: str,
    out: Path,
    build_env: dict[str, str],
    opt_level: str,
    target: str | None,
) -> None:
    compiler = root / "target" / "release" / "arden"
    if not compiler.exists():
        raise RuntimeError(
            f"Arden missing at {compiler}. Build it first or run without --no-build."
        )

    src = root / "benchmark" / "arden" / f"{bench}.arden"
    print(f"  [build] arden {bench}", flush=True)
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
        raise RuntimeError(f"Failed to compile Arden benchmark {bench}:\n{proc.stderr}")


def compile_rust(root: Path, bench: str, out: Path) -> None:
    src = root / "benchmark" / "rust" / f"{bench}.rs"
    print(f"  [build] rust {bench}", flush=True)
    proc = run_cmd(
        ["rustc", "-C", "opt-level=3", "-C", "target-cpu=native", str(src), "-o", str(out)],
        root,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"Failed to compile Rust benchmark {bench}:\n{proc.stderr}")


def compile_go(root: Path, bench: str, out: Path) -> None:
    src = root / "benchmark" / "go" / f"{bench}.go"
    print(f"  [build] go {bench}", flush=True)
    proc = run_cmd(
        ["go", "build", "-trimpath", "-ldflags", "-s -w", "-o", str(out), str(src)],
        root,
        env={"GO111MODULE": "off"},
    )
    if proc.returncode != 0:
        raise RuntimeError(f"Failed to compile Go benchmark {bench}:\n{proc.stderr}")


def timed_compile(
    cmd: list[str], cwd: Path, env: dict[str, str] | None = None
) -> TimedCompileResult:
    start = time.perf_counter()
    proc = run_cmd(cmd, cwd, env=env)
    elapsed = time.perf_counter() - start
    if proc.returncode != 0:
        raise RuntimeError(
            f"Compile failed: {' '.join(cmd)}\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return TimedCompileResult(elapsed_s=elapsed, stdout=proc.stdout, stderr=proc.stderr)


def timed_compile_with_retry(lang: str, job: dict, retries: int = 1) -> TimedCompileResult:
    for attempt in range(retries + 1):
        try:
            return timed_compile(job["cmd"], job["cwd"], env=job["env"])
        except RuntimeError as exc:
            transient_ll_missing = (
                lang == "arden"
                and ".ll" in str(exc)
                and "no such file or directory" in str(exc).lower()
            )
            if transient_ll_missing and attempt < retries:
                time.sleep(0.05)
                continue
            raise
    raise RuntimeError("unreachable")


def make_compile_jobs(
    root: Path,
    compile_projects: dict[str, dict[str, Path]],
    build_env: dict[str, str],
    arden_timings: bool,
) -> dict[str, dict]:
    compiler = root / "target" / "release" / "arden"
    arden_cmd = [str(compiler), "build", "--no-check"]
    if arden_timings:
        arden_cmd.append("--timings")
    return {
        "arden": {
            "cmd": arden_cmd,
            "cwd": compile_projects["arden"]["project_dir"],
            "env": build_env,
            "binary": exe_path(compile_projects["arden"]["binary"]),
            "mutate_source": compile_projects["arden"]["mutate_source"],
            "mutate_sources": compile_projects["arden"].get("mutate_sources", []),
            "mixed_leaf_sources": compile_projects["arden"].get("mixed_leaf_sources", []),
            "mixed_groups": compile_projects["arden"].get("mixed_groups", []),
            "api_core_file": compile_projects["arden"].get("api_core_file"),
            "api_part_files": compile_projects["arden"].get("api_part_files", []),
            "api_main_file": compile_projects["arden"].get("api_main_file"),
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
            "api_core_file": compile_projects["rust"].get("api_core_file"),
            "api_part_files": compile_projects["rust"].get("api_part_files", []),
            "api_main_file": compile_projects["rust"].get("api_main_file"),
        },
        "go": {
            "cmd": ["go", "build", "-trimpath", "-o", str(compile_projects["go"]["binary"]), "."],
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
            "api_core_file": compile_projects["go"].get("api_core_file"),
            "api_part_files": compile_projects["go"].get("api_part_files", []),
            "api_main_file": compile_projects["go"].get("api_main_file"),
        },
    }


def run_checksum(binary: Path, cwd: Path) -> int:
    proc = run_cmd([str(binary)], cwd)
    if proc.returncode != 0:
        raise RuntimeError(
            f"Binary execution failed: {binary}\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return parse_checksum(proc.stdout)


def timed_run(binary: Path, cwd: Path) -> tuple[float, int]:
    start = time.perf_counter()
    proc = run_cmd([str(binary)], cwd)
    elapsed = time.perf_counter() - start
    if proc.returncode != 0:
        raise RuntimeError(
            f"Benchmark execution failed: {binary}\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return elapsed, parse_checksum(proc.stdout)


def clean_compile_artifacts(lang: str, job: dict) -> None:
    binary = Path(job["binary"])
    if binary.exists():
        binary.unlink()

    if lang == "arden":
        cache_dir = Path(job["cwd"]) / ".ardencache"
        if cache_dir.exists():
            shutil.rmtree(cache_dir)

    if lang == "go":
        cache_dir = job.get("go_cache_dir")
        if cache_dir is not None:
            cache_path = Path(cache_dir)
            if cache_path.exists():
                shutil.rmtree(cache_path)
            cache_path.mkdir(parents=True, exist_ok=True)
            return
        proc = run_cmd(["go", "clean", "-cache"], job["cwd"], env=job.get("env"))
        if proc.returncode != 0:
            raise RuntimeError(f"Failed to clean Go cache:\n{proc.stderr}")
