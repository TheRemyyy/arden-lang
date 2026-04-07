import shutil
import time
from pathlib import Path


def pick_mutation_targets(
    part_files: list[Path], mutate_count: int, profile: str
) -> list[Path]:
    if not part_files:
        return []

    mutate_count = max(1, min(mutate_count, len(part_files)))
    if profile == "central":
        return [part_files[len(part_files) // 2]]
    if profile == "batch_spread":
        if mutate_count == 1:
            return [part_files[-1]]
        last_index = len(part_files) - 1
        indices = {
            round((last_index * index) / (mutate_count - 1))
            for index in range(mutate_count)
        }
        return [part_files[index] for index in sorted(indices)]
    return part_files[-mutate_count:]


def reset_generated_root(generated_root: Path) -> None:
    if generated_root.exists():
        for _ in range(3):
            shutil.rmtree(generated_root, ignore_errors=True)
            if not generated_root.exists():
                break
            time.sleep(0.05)
        if generated_root.exists():
            raise RuntimeError(f"Failed to reset generated benchmark directory: {generated_root}")
    generated_root.mkdir(parents=True, exist_ok=True)


def generate_compile_project_starter_graph(
    root: Path,
    bench_name: str,
    mutation_profile: str = "leaf",
    file_count: int = 10,
    funcs_per_file: int = 180,
    mutate_count: int = 1,
) -> dict[str, dict[str, Path]]:
    generated_root = root / "benchmark" / "generated" / bench_name
    reset_generated_root(generated_root)

    arden_dir = generated_root / "arden"
    arden_src = arden_dir / "src"
    arden_src.mkdir(parents=True, exist_ok=True)
    arden_files = ["src/core.arden"]
    arden_part_files: list[Path] = []
    arden_core = arden_src / "core.arden"
    arden_core.write_text(
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
    for index in range(file_count):
        part_name = f"part_{index:02d}"
        part_file = arden_src / f"{part_name}.arden"
        arden_files.append(f"src/{part_name}.arden")
        arden_part_files.append(part_file)
        lines = ["import std.io.*;", ""]
        for func_index in range(funcs_per_file):
            lines.append(
                f"function {part_name}_f{func_index:03d}(x: Integer): Integer {{ return core_mix(x, {index + func_index + 1}); }}"
            )
        lines.extend(["", f"function {part_name}_apply(x: Integer): Integer {{", "    mut y: Integer = x;"])
        for func_index in range(funcs_per_file):
            lines.append(f"    y = {part_name}_f{func_index:03d}(y);")
        lines.extend(["    return y;", "}"])
        part_file.write_text("\n".join(lines) + "\n", encoding="utf-8")

    main_lines = ["import std.io.*;", "", "function main(): None {", "    mut acc: Integer = 0;"]
    for index in range(file_count):
        main_lines.append(f"    acc = part_{index:02d}_apply(acc);")
    main_lines.extend(['    println(to_string(acc));', "    return None;", "}"])
    (arden_src / "main.arden").write_text("\n".join(main_lines) + "\n", encoding="utf-8")
    arden_files.append("src/main.arden")
    toml_lines = [
        f'name = "{bench_name}"',
        'version = "0.1.0"',
        'entry = "src/main.arden"',
        "files = [",
    ]
    toml_lines.extend([f'    "{value}",' for value in arden_files])
    toml_lines.extend(["]", f'output = "{bench_name}"', 'opt_level = "3"'])
    (arden_dir / "arden.toml").write_text("\n".join(toml_lines) + "\n", encoding="utf-8")

    rust_dir = generated_root / "rust"
    rust_dir.mkdir(parents=True, exist_ok=True)
    rust_part_files: list[Path] = []
    rust_main = ["mod core;"]
    for index in range(file_count):
        rust_main.append(f"mod part_{index:02d};")
    rust_main.extend(["", "fn main() {", "    let mut acc: i64 = 0;"])
    for index in range(file_count):
        rust_main.append(f"    acc = part_{index:02d}::apply(acc);")
    rust_main.extend(['    println!("{acc}");', "}"])
    (rust_dir / "main.rs").write_text("\n".join(rust_main) + "\n", encoding="utf-8")
    (rust_dir / "core.rs").write_text(
        "\n".join(["pub fn mix(x: i64, k: i64) -> i64 {", "    x + k", "}", ""]),
        encoding="utf-8",
    )
    for index in range(file_count):
        part_name = f"part_{index:02d}"
        part_file = rust_dir / f"{part_name}.rs"
        rust_part_files.append(part_file)
        lines = ["pub fn apply(x: i64) -> i64 {", "    let mut y = x;"]
        for func_index in range(funcs_per_file):
            lines.append(f"    y = crate::core::mix(y, {index + func_index + 1});")
        lines.extend(["    y", "}"])
        part_file.write_text("\n".join(lines) + "\n", encoding="utf-8")

    go_dir = generated_root / "go"
    go_dir.mkdir(parents=True, exist_ok=True)
    go_part_files: list[Path] = []
    (go_dir / "go.mod").write_text("module compile10\n\ngo 1.22\n", encoding="utf-8")
    (go_dir / "core.go").write_text(
        "\n".join(
            ["package main", "", "func coreMix(x int64, k int64) int64 {", "    return x + k", "}", ""]
        ),
        encoding="utf-8",
    )
    go_main = ['package main', "", 'import "fmt"', "", "func main() {", "    var acc int64 = 0"]
    for index in range(file_count):
        go_main.append(f"    acc = part_{index:02d}_apply(acc)")
    go_main.extend(["    fmt.Println(acc)", "}"])
    (go_dir / "main.go").write_text("\n".join(go_main) + "\n", encoding="utf-8")
    for index in range(file_count):
        part_name = f"part_{index:02d}"
        part_file = go_dir / f"unit_{index:04d}.go"
        go_part_files.append(part_file)
        lines = ["package main", "", f"func {part_name}_apply(x int64) int64 {{", "    y := x"]
        for func_index in range(funcs_per_file):
            lines.append(f"    y = coreMix(y, {index + func_index + 1})")
        lines.extend(["    return y", "}"])
        part_file.write_text("\n".join(lines) + "\n", encoding="utf-8")

    arden_mutate_sources = pick_mutation_targets(arden_part_files, mutate_count, mutation_profile)
    rust_mutate_sources = pick_mutation_targets(rust_part_files, mutate_count, mutation_profile)
    go_mutate_sources = pick_mutation_targets(go_part_files, mutate_count, mutation_profile)
    if mutation_profile == "central":
        arden_mutate_sources = [arden_core]
        rust_mutate_sources = [rust_dir / "core.rs"]
        go_mutate_sources = [go_dir / "core.go"]

    return {
        "arden": {
            "project_dir": arden_dir,
            "binary": arden_dir / bench_name,
            "mutate_source": arden_mutate_sources[0],
            "mutate_sources": arden_mutate_sources,
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


def generate_incremental_rebuild_large_project_batch(
    root: Path, bench_name: str
) -> dict[str, dict[str, Path]]:
    return generate_compile_project_starter_graph(
        root,
        bench_name,
        mutation_profile="batch_spread",
        file_count=120,
        funcs_per_file=320,
        mutate_count=10,
    )
