#!/usr/bin/env python3

from __future__ import annotations

import argparse
import shutil
import stat
import subprocess
import tarfile
import zipfile
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Package a portable Arden release bundle.")
    parser.add_argument("--platform", choices=["linux", "macos", "windows"], required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--asset-name", required=True)
    parser.add_argument("--bundle-root", type=Path, required=True)
    parser.add_argument("--llvm-prefix", type=Path, required=True)
    parser.add_argument("--extra-prefix", action="append", default=[])
    parser.add_argument("--extra-bin", action="append", default=[])
    parser.add_argument("--extra-lib-dir", action="append", default=[])
    return parser.parse_args()


def ensure_clean_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def copy_path(source: Path, destination: Path, visited_dirs: set[Path]) -> None:
    if source.is_symlink():
        resolved = source.resolve()
        if resolved.is_dir():
            if resolved in visited_dirs:
                return
            visited_dirs.add(resolved)
            destination.mkdir(parents=True, exist_ok=True)
            for child in resolved.iterdir():
                copy_path(child, destination / child.name, visited_dirs)
            return
        copy_symlinked_file(resolved, destination)
        return

    if source.is_dir():
        real_dir = source.resolve()
        if real_dir in visited_dirs:
            return
        visited_dirs.add(real_dir)
        destination.mkdir(parents=True, exist_ok=True)
        for child in source.iterdir():
            copy_path(child, destination / child.name, visited_dirs)
        return

    copy_file(source, destination)


def copy_tree(source: Path, destination: Path) -> None:
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True, exist_ok=True)
    if not source.is_dir():
        raise NotADirectoryError(f"source path is not a directory: {source}")
    visited_dirs: set[Path] = set()
    for child in source.iterdir():
        copy_path(child, destination / child.name, visited_dirs)


def copy_selected_paths(source_root: Path, destination_root: Path, relative_paths: list[Path]) -> None:
    visited_dirs: set[Path] = set()
    for relative_path in relative_paths:
        source_path = source_root / relative_path
        if not (source_path.exists() or source_path.is_symlink()):
            raise FileNotFoundError(
                f"required packaging path not found: {relative_path} (resolved to {source_path})"
            )
        copy_path(source_path, destination_root / relative_path, visited_dirs)


def copy_linux_llvm_runtime(source_root: Path, destination_root: Path) -> None:
    destination_root.mkdir(parents=True, exist_ok=True)


def copy_llvm_prefix(platform_name: str, source_root: Path, destination_root: Path) -> None:
    if platform_name == "linux":
        copy_linux_llvm_runtime(source_root, destination_root)
        return
    copy_tree(source_root, destination_root)


def copy_symlinked_file(resolved_source: Path, destination: Path) -> None:
    copy_file(resolved_source, destination)
    if destination.name != resolved_source.name:
        copy_file(resolved_source, destination.with_name(resolved_source.name))


def copy_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists() or destination.is_symlink():
        destination.chmod(stat.S_IWUSR | stat.S_IRUSR | stat.S_IXUSR)
        destination.unlink()
    shutil.copy2(source, destination)


def collect_linux_runtime_libraries(bundle_dir: Path) -> None:
    runtime_prefixes = ("libLLVM", "libLTO", "libRemarks", "libPolly")
    bundle_root = bundle_dir.resolve()
    inspect_queue: list[Path] = []
    bundled_binary = bundle_dir / "bin" / "arden-real"
    if bundled_binary.exists():
        inspect_queue.append(bundled_binary)
    extra_bin_dir = bundle_dir / "toolchain" / "extra" / "bin"
    if extra_bin_dir.exists():
        inspect_queue.extend(path for path in extra_bin_dir.iterdir() if path.is_file())

    inspected_paths: set[Path] = set()
    while inspect_queue:
        inspect_path = inspect_queue.pop()
        resolved_inspect_path = inspect_path.resolve()
        if resolved_inspect_path in inspected_paths:
            continue
        inspected_paths.add(resolved_inspect_path)

        for dependency_path in parse_linux_dependencies(resolved_inspect_path):
            dependency_name = dependency_path.name
            if not dependency_name.startswith(runtime_prefixes):
                continue
            if bundle_root in dependency_path.resolve().parents:
                continue
            bundled_dependency = bundle_dir / "toolchain" / "llvm" / "lib" / dependency_name
            copy_file(dependency_path, bundled_dependency)
            inspect_queue.append(dependency_path)


def parse_linux_dependencies(binary_path: Path) -> list[Path]:
    try:
        result = subprocess.run(
            ["ldd", str(binary_path)],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return []

    dependencies: list[Path] = []
    for line in result.stdout.splitlines():
        stripped_line = line.strip()
        if "=>" in stripped_line:
            candidate = stripped_line.split("=>", 1)[1].strip().split(" ", 1)[0]
        elif stripped_line.startswith("/"):
            candidate = stripped_line.split(" ", 1)[0]
        else:
            continue
        if candidate.startswith("/") and Path(candidate).exists():
            dependencies.append(Path(candidate))
    return dependencies


def collect_macos_runtime_libraries(bundle_dir: Path) -> None:
    bundle_root = bundle_dir.resolve()
    inspect_queue: list[tuple[Path, Path]] = []
    bundled_binary = bundle_dir / "bin" / "arden-real"
    if bundled_binary.exists():
        inspect_queue.append((bundled_binary, bundle_dir / "toolchain" / "llvm" / "lib"))

    llvm_bin_dir = bundle_dir / "toolchain" / "llvm" / "bin"
    if llvm_bin_dir.exists():
        inspect_queue.extend(
            (path, bundle_dir / "toolchain" / "llvm" / "lib")
            for path in llvm_bin_dir.iterdir()
            if path.is_file()
        )

    lld_bin_dir = bundle_dir / "toolchain" / "lld" / "bin"
    if lld_bin_dir.exists():
        inspect_queue.extend(
            (path, bundle_dir / "toolchain" / "lld" / "lib")
            for path in lld_bin_dir.iterdir()
            if path.is_file()
        )

    inspected_paths: set[Path] = set()
    while inspect_queue:
        inspect_path, destination_dir = inspect_queue.pop()
        resolved_inspect_path = inspect_path.resolve()
        if resolved_inspect_path in inspected_paths:
            continue
        inspected_paths.add(resolved_inspect_path)

        for dependency_path in parse_macos_dependencies(resolved_inspect_path):
            resolved_dependency_path = dependency_path.resolve()
            if bundle_root == resolved_dependency_path or bundle_root in resolved_dependency_path.parents:
                continue
            bundled_dependency = destination_dir / dependency_path.name
            copy_file(resolved_dependency_path, bundled_dependency)
            inspect_queue.append((resolved_dependency_path, destination_dir))


def parse_macos_dependencies(binary_path: Path) -> list[Path]:
    try:
        result = subprocess.run(
            ["otool", "-L", str(binary_path)],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return []

    dependencies: list[Path] = []
    for line in result.stdout.splitlines()[1:]:
        stripped_line = line.strip()
        if not stripped_line:
            continue
        candidate = stripped_line.split(" (compatibility version", 1)[0].strip()
        if not candidate.startswith("/"):
            continue
        dependency_path = Path(candidate)
        if not dependency_path.exists():
            continue
        if candidate.startswith("/System/") or candidate.startswith("/usr/lib/"):
            continue
        dependencies.append(dependency_path)
    return dependencies


def write_text_file(path: Path, contents: str, executable: bool = False) -> None:
    path.write_text(contents, encoding="utf8")
    if executable:
        current_mode = path.stat().st_mode
        path.chmod(current_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def build_unix_wrapper(platform_name: str) -> str:
    library_var = "DYLD_LIBRARY_PATH" if platform_name == "macos" else "LD_LIBRARY_PATH"
    extra_path = (
        '"${ROOT}/toolchain/llvm/bin:${ROOT}/toolchain/extra/bin:${PATH}"'
        if platform_name == "linux"
        else '"${ROOT}/toolchain/llvm/bin:${ROOT}/toolchain/lld/bin:${PATH}"'
    )
    library_setup = build_unix_library_setup(platform_name, library_var)
    sdk_setup = ""
    if platform_name == "macos":
        sdk_setup = """
if [[ -z "${SDKROOT:-}" ]] && command -v xcrun >/dev/null 2>&1; then
  SDKROOT="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null || true)"
  if [[ -n "${SDKROOT}" ]]; then
    export SDKROOT
  fi
fi
if [[ -z "${SDKROOT:-}" || ! -d "${SDKROOT}" ]]; then
  if command -v xcode-select >/dev/null 2>&1; then
    xcode-select --install >/dev/null 2>&1 || true
  fi
  printf '%s\n' 'error: macOS SDK not found. Arden can ship LLVM/lld, but Apple SDK files must come from Command Line Tools or Xcode.' >&2
  printf '%s\n' 'Run `xcode-select --install`, then retry.' >&2
  exit 1
fi
"""
    return f"""#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
export PATH={extra_path}
{library_setup}
{sdk_setup}\
exec "${{ROOT}}/bin/arden-real" "$@"
"""


def build_windows_wrapper() -> str:
    return """@echo off
setlocal
set "ROOT=%~dp0"
set "PATH=%ROOT%toolchain\\llvm\\bin;%PATH%"
set "LIB=%ROOT%toolchain\\windows-libs\\vc;%ROOT%toolchain\\windows-libs\\ucrt;%ROOT%toolchain\\windows-libs\\um;%LIB%"
set "LIBPATH=%ROOT%toolchain\\windows-libs\\vc;%ROOT%toolchain\\windows-libs\\ucrt;%ROOT%toolchain\\windows-libs\\um;%LIBPATH%"
"%ROOT%bin\\arden-real.exe" %*
"""


def build_unix_install_script(platform_name: str) -> str:
    library_var = "DYLD_LIBRARY_PATH" if platform_name == "macos" else "LD_LIBRARY_PATH"
    extra_path = (
        '${ROOT}/toolchain/llvm/bin:${ROOT}/toolchain/extra/bin:\\${PATH}'
        if platform_name == "linux"
        else '${ROOT}/toolchain/llvm/bin:${ROOT}/toolchain/lld/bin:\\${PATH}'
    )
    library_setup = build_unix_library_setup(platform_name, library_var, escaped=True)
    sdk_setup = ""
    if platform_name == "macos":
        sdk_setup = """
if [[ -z "\\${SDKROOT:-}" ]] && command -v xcrun >/dev/null 2>&1; then
  SDKROOT="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null || true)"
  if [[ -n "\\${SDKROOT}" ]]; then
    export SDKROOT
  fi
fi
if [[ -z "\\${SDKROOT:-}" || ! -d "\\${SDKROOT}" ]]; then
  if command -v xcode-select >/dev/null 2>&1; then
    xcode-select --install >/dev/null 2>&1 || true
  fi
  printf '%s\n' 'error: macOS SDK not found. Arden can ship LLVM/lld, but Apple SDK files must come from Command Line Tools or Xcode.' >&2
  printf '%s\n' 'Run `xcode-select --install`, then retry.' >&2
  exit 1
fi
"""
    return f"""#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
BIN_DIR="${{HOME}}/.local/bin"
TARGET="${{BIN_DIR}}/arden"

mkdir -p "${{BIN_DIR}}"
cat > "${{TARGET}}" <<EOF
#!/usr/bin/env bash
set -euo pipefail
ROOT="${{ROOT}}"
export PATH="{extra_path}"
{library_setup}
{sdk_setup}\
exec "${{ROOT}}/bin/arden-real" "\\$@"
EOF
chmod +x "${{TARGET}}"

printf 'Installed Arden launcher to %s\n' "${{TARGET}}"
printf 'If ~/.local/bin is not on your PATH yet, add this line to your shell config:\n'
printf '  export PATH="$HOME/.local/bin:$PATH"\n'
printf 'Then run: arden --version\n'
"""


def build_windows_install_script() -> str:
    return """$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$UserBin = Join-Path $env:USERPROFILE "AppData\\Local\\Arden\\bin"
$Target = Join-Path $UserBin "arden.cmd"

New-Item -ItemType Directory -Path $UserBin -Force | Out-Null
$Launcher = @"
@echo off
setlocal
set "ROOT=$Root"
set "PATH=%ROOT%\\toolchain\\llvm\\bin;%PATH%"
set "LIB=%ROOT%\\toolchain\\windows-libs\\vc;%ROOT%\\toolchain\\windows-libs\\ucrt;%ROOT%\\toolchain\\windows-libs\\um;%LIB%"
set "LIBPATH=%ROOT%\\toolchain\\windows-libs\\vc;%ROOT%\\toolchain\\windows-libs\\ucrt;%ROOT%\\toolchain\\windows-libs\\um;%LIBPATH%"
"%ROOT%\\bin\\arden-real.exe" %*
"@
Set-Content -Path $Target -Value $Launcher -Encoding ASCII

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$NeedsPath = -not (($UserPath -split ";") | Where-Object { $_ -eq $UserBin })
if ($NeedsPath) {
    $UpdatedPath = if ([string]::IsNullOrWhiteSpace($UserPath)) { $UserBin } else { "$UserPath;$UserBin" }
    [Environment]::SetEnvironmentVariable("Path", $UpdatedPath, "User")
}

Write-Host "Installed Arden launcher to $Target"
Write-Host "Open a new terminal and run: arden --version"
"""


def build_unix_library_setup(platform_name: str, library_var: str, escaped: bool = False) -> str:
    candidate_dirs = [
        "${ROOT}/toolchain/llvm/lib",
        "${ROOT}/toolchain/llvm/lib64",
    ]
    if platform_name == "macos":
        candidate_dirs.append("${ROOT}/toolchain/lld/lib")

    candidate_literals = " ".join(f'"{directory}"' for directory in candidate_dirs)
    block = f"""RUNTIME_LIB_DIRS=""
for candidate in {candidate_literals}; do
  if [[ -d "$candidate" ]]; then
    RUNTIME_LIB_DIRS="${{RUNTIME_LIB_DIRS:+${{RUNTIME_LIB_DIRS}}:}}$candidate"
    while IFS= read -r nested_dir; do
      RUNTIME_LIB_DIRS="${{RUNTIME_LIB_DIRS}}:$nested_dir"
    done < <(find "$candidate" -mindepth 1 -maxdepth 2 -type d \\( -name 'lib' -o -name 'lib64' -o -name '*linux-gnu' \\) | sort)
  fi
done
export {library_var}="${{RUNTIME_LIB_DIRS:+${{RUNTIME_LIB_DIRS}}:}}${{{library_var}:-}}"
"""
    if escaped:
        return block.replace("$", "\\$")
    return block


def build_readme(platform_name: str, asset_name: str) -> str:
    entrypoint = "arden.cmd" if platform_name == "windows" else "./arden"
    install_step = (
        "- Optional: run `install.ps1` in PowerShell to add Arden to your user PATH"
        if platform_name == "windows"
        else "- Optional: run `./install.sh` to install an Arden launcher into ~/.local/bin"
    )
    platform_notes = {
        "windows": (
            "- Windows bundles are intended to run directly after extraction without extra LLVM setup.\n"
            "- Windows bundles now include the MSVC/UCRT/Windows SDK import libraries Arden needs for linking on clean machines."
        ),
        "linux": (
            "- Linux bundles still depend on the host kernel and compatible glibc baseline.\n"
            "- Linux bundles are designed so Arden can run directly from the extracted folder."
        ),
        "macos": (
            "- macOS bundles include Arden, LLVM, and lld, but Apple SDK files still come from Command Line Tools or Xcode.\n"
            "- On machines missing the Apple SDK, the launcher now triggers the native `xcode-select --install` prompt and exits with a clear instruction."
        ),
    }[platform_name]
    return f"""Arden portable bundle
=====================

Asset: {asset_name}

What is included:
- Arden compiler binary
- Bundled LLVM runtime files needed by Arden
- Bundled linker helper binaries required by Arden on this platform

How to run:
- Extract the archive
- Launch `{entrypoint}`
{install_step}

Notes:
- This bundle is intended to avoid manual LLVM/linker setup for normal compiler use.
{platform_notes}
"""


def make_archive(platform_name: str, source_dir: Path, archive_path: Path) -> None:
    if archive_path.exists():
        archive_path.unlink()

    if platform_name == "windows":
        with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for file_path in source_dir.rglob("*"):
                archive.write(file_path, file_path.relative_to(source_dir.parent))
        return

    with tarfile.open(archive_path, "w:gz") as archive:
        archive.add(source_dir, arcname=source_dir.name)


def parse_named_windows_lib_dir(raw_value: str) -> tuple[str, Path]:
    if "=" not in raw_value:
        raise ValueError("expected NAME=PATH")
    name, raw_path = raw_value.split("=", 1)
    normalized_name = name.strip().lower()
    if normalized_name not in {"vc", "ucrt", "um"}:
        raise ValueError("name must be one of: vc, ucrt, um")
    lib_dir = Path(raw_path.strip())
    if not lib_dir.is_dir():
        raise ValueError(f"directory does not exist: {lib_dir}")
    return normalized_name, lib_dir


def package_release() -> None:
    args = parse_args()
    bundle_dir = args.bundle_root / args.asset_name
    ensure_clean_dir(bundle_dir)

    real_binary_name = "arden-real.exe" if args.platform == "windows" else "arden-real"
    copy_file(args.binary, bundle_dir / "bin" / real_binary_name)
    copy_llvm_prefix(args.platform, args.llvm_prefix, bundle_dir / "toolchain" / "llvm")

    for extra_prefix_raw in args.extra_prefix:
        extra_prefix = Path(extra_prefix_raw)
        if not extra_prefix.exists():
            continue
        destination_name = "lld" if args.platform == "macos" else extra_prefix.name
        copy_tree(extra_prefix, bundle_dir / "toolchain" / destination_name)

    for extra_bin_raw in args.extra_bin:
        extra_bin = Path(extra_bin_raw)
        if not extra_bin.exists():
            continue
        extra_bin_destination = bundle_dir / "toolchain" / "extra" / "bin" / extra_bin.name
        copy_file(extra_bin, extra_bin_destination)
        if args.platform == "linux":
            if extra_bin.name == "mold":
                copy_file(extra_bin, bundle_dir / "toolchain" / "extra" / "bin" / "ld.mold")
            elif extra_bin.name == "ld.mold":
                copy_file(extra_bin, bundle_dir / "toolchain" / "extra" / "bin" / "mold")

    if args.platform == "linux":
        collect_linux_runtime_libraries(bundle_dir)
    elif args.platform == "macos":
        collect_macos_runtime_libraries(bundle_dir)

    for extra_lib_dir_raw in args.extra_lib_dir:
        lib_name, lib_dir = parse_named_windows_lib_dir(extra_lib_dir_raw)
        copy_tree(lib_dir, bundle_dir / "toolchain" / "windows-libs" / lib_name)

    if args.platform == "windows":
        write_text_file(bundle_dir / "arden.cmd", build_windows_wrapper())
        write_text_file(bundle_dir / "install.ps1", build_windows_install_script())
    else:
        write_text_file(
            bundle_dir / "arden",
            build_unix_wrapper(args.platform),
            executable=True,
        )
        write_text_file(
            bundle_dir / "install.sh",
            build_unix_install_script(args.platform),
            executable=True,
        )

    write_text_file(
        bundle_dir / "README.txt",
        build_readme(args.platform, args.asset_name),
    )

    archive_extension = ".zip" if args.platform == "windows" else ".tar.gz"
    make_archive(args.platform, bundle_dir, args.bundle_root / f"{args.asset_name}{archive_extension}")


if __name__ == "__main__":
    package_release()
