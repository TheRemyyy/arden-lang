#!/usr/bin/env python3

from __future__ import annotations

import argparse
import shutil
import stat
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
    return parser.parse_args()


def ensure_clean_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def copy_tree(source: Path, destination: Path) -> None:
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(source, destination, symlinks=True)


def copy_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


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
    extra_lib = (
        '"${ROOT}/toolchain/llvm/lib:${ROOT}/toolchain/lld/lib:${%s:-}"' % library_var
        if platform_name == "macos"
        else '"${ROOT}/toolchain/llvm/lib:${%s:-}"' % library_var
    )
    return f"""#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
export PATH={extra_path}
export {library_var}={extra_lib}
exec "${{ROOT}}/bin/arden-real" "$@"
"""


def build_windows_wrapper() -> str:
    return """@echo off
setlocal
set "ROOT=%~dp0"
set "PATH=%ROOT%toolchain\\llvm\\bin;%PATH%"
"%ROOT%bin\\arden-real.exe" %*
"""


def build_unix_install_script(platform_name: str) -> str:
    library_var = "DYLD_LIBRARY_PATH" if platform_name == "macos" else "LD_LIBRARY_PATH"
    extra_path = (
        '${ROOT}/toolchain/llvm/bin:${ROOT}/toolchain/extra/bin:${PATH}'
        if platform_name == "linux"
        else '${ROOT}/toolchain/llvm/bin:${ROOT}/toolchain/lld/bin:${PATH}'
    )
    extra_lib = (
        f'${{ROOT}}/toolchain/llvm/lib:${{ROOT}}/toolchain/lld/lib:${{{library_var}:-}}'
        if platform_name == "macos"
        else f'${{ROOT}}/toolchain/llvm/lib:${{{library_var}:-}}'
    )
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
export {library_var}="{extra_lib}"
exec "${{ROOT}}/bin/arden-real" "$@"
EOF
chmod +x "${{TARGET}}"

printf 'Installed Arden launcher to %s\n' "${TARGET}"
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


def build_readme(platform_name: str, asset_name: str) -> str:
    entrypoint = "arden.cmd" if platform_name == "windows" else "./arden"
    install_step = (
        "- Optional: run `install.ps1` in PowerShell to add Arden to your user PATH"
        if platform_name == "windows"
        else "- Optional: run `./install.sh` to install an Arden launcher into ~/.local/bin"
    )
    return f"""Arden portable bundle
=====================

Asset: {asset_name}

What is included:
- Arden compiler binary
- Bundled LLVM/Clang toolchain files needed by Arden
- Bundled linker helper binaries required by Arden on this platform

How to run:
- Extract the archive
- Launch `{entrypoint}`
{install_step}

Notes:
- This bundle is intended to avoid manual LLVM/linker setup for normal compiler use.
- Linux bundles still depend on the host kernel and compatible glibc baseline.
- Portable bundles are designed so Arden can run directly from the extracted folder.
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


def package_release() -> None:
    args = parse_args()
    bundle_dir = args.bundle_root / args.asset_name
    ensure_clean_dir(bundle_dir)

    real_binary_name = "arden-real.exe" if args.platform == "windows" else "arden-real"
    copy_file(args.binary, bundle_dir / "bin" / real_binary_name)
    copy_tree(args.llvm_prefix, bundle_dir / "toolchain" / "llvm")

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
        copy_file(extra_bin, bundle_dir / "toolchain" / "extra" / "bin" / extra_bin.name)

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
