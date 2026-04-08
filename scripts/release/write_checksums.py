#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Write SHA256 checksums for release archives.")
    parser.add_argument("directory", type=Path)
    return parser.parse_args()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    args = parse_args()
    archive_paths = sorted(
        path
        for path in args.directory.iterdir()
        if path.is_file() and path.name != "SHA256SUMS.txt"
    )
    lines = [f"{sha256_file(path)}  {path.name}" for path in archive_paths]
    (args.directory / "SHA256SUMS.txt").write_text("\n".join(lines) + "\n", encoding="utf8")


if __name__ == "__main__":
    main()
