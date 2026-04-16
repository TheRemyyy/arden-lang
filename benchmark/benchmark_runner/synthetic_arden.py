from pathlib import Path

from .synthetic_shared import create_group_plan, synthetic_graph_dependency_indices


def write_arden_core(arden_src: Path) -> None:
    (arden_src / "core.arden").write_text(
        "\n".join(
            [
                "function core_mix(x: Integer, k: Integer): Integer {",
                "    return x + k;",
                "}",
                "",
                "function core_fold(a: Integer, b: Integer, salt: Integer): Integer {",
                "    return a + b + salt;",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )


def write_arden_group_files(arden_src: Path, group_names: list[str]) -> tuple[list[str], list[dict]]:
    arden_files = ["src/core.arden"]
    group_plans: list[dict] = []
    for group_index, group_name in enumerate(group_names):
        group_salt = 1000 + group_index * 37
        group_file = arden_src / f"{group_name}.arden"
        group_file.write_text(
            "\n".join(
                [
                    f"// MUTATION_SURFACE_{group_name.upper()}",
                    f"function {group_name}_bridge(x: Integer, salt: Integer): Integer {{",
                    f"    return core_fold(x, salt, {group_salt});",
                    "}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        arden_files.append(f"src/{group_name}.arden")
        group_plans.append(create_group_plan(group_name, group_index, group_salt, group_file))
    return arden_files, group_plans


def write_arden_part_files(
    arden_src: Path,
    part_names: list[str],
    group_names: list[str],
    funcs_per_file: int,
    max_deps: int,
    group_size: int,
    topology: str,
    arden_files: list[str],
    group_plans: list[dict],
) -> list[Path]:
    part_files: list[Path] = []
    for index, part_name in enumerate(part_names):
        deps = synthetic_graph_dependency_indices(index, max_deps, topology, group_size)
        group_index = index // group_size
        group_name = group_names[group_index]
        group_salt = 1000 + group_index * 37
        part_file = arden_src / f"{part_name}.arden"
        part_files.append(part_file)
        arden_files.append(f"src/{part_name}.arden")
        group_plans[group_index]["caller_files"].append(part_file)

        lines: list[str] = []
        for func_index in range(funcs_per_file):
            lines.append(
                f"function {part_name}_f{func_index:03d}(x: Integer): Integer {{ return core_mix(x, {index + func_index + 1}); }}"
            )
        lines.extend(["", f"function {part_name}_apply(x: Integer): Integer {{", "    mut y: Integer = x;"])
        for func_index in range(funcs_per_file):
            lines.append(f"    y = {part_name}_f{func_index:03d}(y);")
        lines.append(f"    y = {group_name}_bridge(y, {group_salt}); // MUTATION_CALL_{group_name.upper()}")
        lines.extend(["    return y;", "}"])
        lines.extend(["", f"function {part_name}_chain(x: Integer): Integer {{", f"    mut y: Integer = {part_name}_apply(x);"])
        for func_index in range(0, funcs_per_file, 3):
            lines.append(f"    y = {part_name}_f{func_index:03d}(y);")
        lines.extend(["    return y;", "}"])
        lines.extend(["", f"function {part_name}_wire(x: Integer): Integer {{", f"    mut y: Integer = {part_name}_chain(x);"])
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = core_fold(y, {dep_part}_apply(x), {index + dep + 1});")
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = core_fold(y, {dep_part}_wire(x), {index + dep + 33});")
        lines.extend(["    return y;", "}"])
        lines.extend(["", f"function {part_name}_fanout(x: Integer): Integer {{", f"    mut y: Integer = {part_name}_wire(x);"])
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = core_fold(y, {dep_part}_chain(x), {index + dep + 65});")
        lines.extend(["    return y;", "}"])
        lines.extend(["", f"function {part_name}_signature(seed: Integer): Integer {{", f"    mut y: Integer = {part_name}_fanout(seed);"])
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = core_fold(y, {dep_part}_apply(seed), {index + dep + 97});")
        lines.extend(["    return y;", "}"])
        part_file.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return part_files


def write_arden_main_and_config(arden_dir: Path, arden_src: Path, bench_name: str, part_names: list[str], arden_files: list[str]) -> None:
    main_lines = ["import std.io.*;", "", "function main(): None {", "    mut acc: Integer = 0;"]
    for part_name in part_names:
        main_lines.append(f"    acc = {part_name}_apply(acc);")
    main_lines.extend(['    println(to_string(acc));', "    return None;", "}"])
    (arden_src / "main.arden").write_text("\n".join(main_lines) + "\n", encoding="utf-8")
    arden_files.append("src/main.arden")

    toml_lines = [f'name = "{bench_name}"', 'version = "0.1.0"', 'entry = "src/main.arden"', "files = ["]
    toml_lines.extend([f'    "{value}",' for value in arden_files])
    toml_lines.extend(["]", f'output = "{bench_name}"', 'opt_level = "3"'])
    (arden_dir / "arden.toml").write_text("\n".join(toml_lines) + "\n", encoding="utf-8")
