from pathlib import Path

from .synthetic_shared import create_group_plan, synthetic_graph_dependency_indices


def write_rust_core(rust_dir: Path) -> None:
    (rust_dir / "core.rs").write_text(
        "\n".join(
            [
                "pub fn mix(x: i64, k: i64) -> i64 {",
                "    x + k",
                "}",
                "",
                "pub fn fold(a: i64, b: i64, salt: i64) -> i64 {",
                "    a + b + salt",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )


def write_rust_group_files(rust_dir: Path, group_names: list[str]) -> list[dict]:
    group_plans: list[dict] = []
    for group_index, group_name in enumerate(group_names):
        group_salt = 1000 + group_index * 37
        group_file = rust_dir / f"{group_name}.rs"
        group_file.write_text(
            "\n".join(
                [
                    f"// MUTATION_SURFACE_{group_name.upper()}",
                    f"pub fn {group_name}_bridge(x: i64, salt: i64) -> i64 {{",
                    f"    crate::core::fold(x, salt, {group_salt})",
                    "}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        group_plans.append(create_group_plan(group_name, group_index, group_salt, group_file))
    return group_plans


def write_rust_part_files(
    rust_dir: Path,
    part_names: list[str],
    group_names: list[str],
    funcs_per_file: int,
    max_deps: int,
    group_size: int,
    group_plans: list[dict],
) -> list[Path]:
    part_files: list[Path] = []
    for index, part_name in enumerate(part_names):
        deps = synthetic_graph_dependency_indices(index, max_deps)
        group_index = index // group_size
        group_name = group_names[group_index]
        group_salt = 1000 + group_index * 37
        part_file = rust_dir / f"{part_name}.rs"
        part_files.append(part_file)
        group_plans[group_index]["caller_files"].append(part_file)

        lines = [
            *[
                f"pub fn f{func_index:03d}(x: i64) -> i64 {{ crate::core::mix(x, {index + func_index + 1}) }}"
                for func_index in range(funcs_per_file)
            ],
            "",
            "pub fn apply(x: i64) -> i64 {",
            "    let mut y = x;",
        ]
        for func_index in range(funcs_per_file):
            lines.append(f"    y = f{func_index:03d}(y);")
        lines.append(f"    y = crate::{group_name}::{group_name}_bridge(y, {group_salt}); // MUTATION_CALL_{group_name.upper()}")
        lines.extend(["    y", "}"])
        lines.extend(["", "pub fn chain(x: i64) -> i64 {", "    let mut y = apply(x);"])
        for func_index in range(0, funcs_per_file, 3):
            lines.append(f"    y = f{func_index:03d}(y);")
        lines.extend(["    y", "}"])
        lines.extend(["", "pub fn wire(x: i64) -> i64 {", "    let mut y = chain(x);"])
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = crate::core::fold(y, crate::{dep_part}::apply(x), {index + dep + 1});")
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = crate::core::fold(y, crate::{dep_part}::wire(x), {index + dep + 33});")
        lines.extend(["    y", "}"])
        lines.extend(["", "pub fn fanout(x: i64) -> i64 {", "    let mut y = wire(x);"])
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = crate::core::fold(y, crate::{dep_part}::chain(x), {index + dep + 65});")
        lines.extend(["    y", "}"])
        lines.extend(["", "pub fn signature(seed: i64) -> i64 {", "    let mut y = fanout(seed);"])
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = crate::core::fold(y, crate::{dep_part}::apply(seed), {index + dep + 97});")
        lines.extend(["    y", "}"])
        part_file.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return part_files


def write_rust_main(rust_dir: Path, part_names: list[str], group_names: list[str]) -> None:
    lines = ["mod core;"]
    for group_name in group_names:
        lines.append(f"mod {group_name};")
    for part_name in part_names:
        lines.append(f"mod {part_name};")
    lines.extend(["", "fn main() {", "    let mut acc: i64 = 0;"])
    for part_name in part_names:
        lines.append(f"    acc = {part_name}::apply(acc);")
    lines.extend(['    println!("{acc}");', "}"])
    (rust_dir / "main.rs").write_text("\n".join(lines) + "\n", encoding="utf-8")
