from pathlib import Path

from .synthetic_shared import create_group_plan, synthetic_graph_dependency_indices


def write_go_core(go_dir: Path) -> None:
    (go_dir / "go.mod").write_text("module compile10\n\ngo 1.22\n", encoding="utf-8")
    (go_dir / "core.go").write_text(
        "\n".join(
            [
                "package main",
                "",
                "func coreMix(x int64, k int64) int64 {",
                "    return x + k",
                "}",
                "",
                "func coreFold(a int64, b int64, salt int64) int64 {",
                "    return a + b + salt",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )


def write_go_group_files(go_dir: Path, group_names: list[str]) -> list[dict]:
    group_plans: list[dict] = []
    for group_index, group_name in enumerate(group_names):
        group_salt = 1000 + group_index * 37
        group_file = go_dir / f"{group_name}.go"
        group_file.write_text(
            "\n".join(
                [
                    "package main",
                    "",
                    f"// MUTATION_SURFACE_{group_name.upper()}",
                    f"func {group_name}_bridge(x int64, salt int64) int64 {{",
                    f"    return coreFold(x, salt, {group_salt})",
                    "}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        group_plans.append(create_group_plan(group_name, group_index, group_salt, group_file))
    return group_plans


def write_go_part_files(
    go_dir: Path,
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
        part_file = go_dir / f"unit_{index:04d}.go"
        part_files.append(part_file)
        group_plans[group_index]["caller_files"].append(part_file)

        lines = ["package main", ""]
        for func_index in range(funcs_per_file):
            lines.append(f"func {part_name}_f{func_index:03d}(x int64) int64 {{ return coreMix(x, {index + func_index + 1}) }}")
        lines.extend(["", f"func {part_name}_apply(x int64) int64 {{", "    y := x"])
        for func_index in range(funcs_per_file):
            lines.append(f"    y = {part_name}_f{func_index:03d}(y)")
        lines.append(f"    y = {group_name}_bridge(y, {group_salt}) // MUTATION_CALL_{group_name.upper()}")
        lines.extend(["    return y", "}"])
        lines.extend(["", f"func {part_name}_chain(x int64) int64 {{", f"    y := {part_name}_apply(x)"])
        for func_index in range(0, funcs_per_file, 3):
            lines.append(f"    y = {part_name}_f{func_index:03d}(y)")
        lines.extend(["    return y", "}"])
        lines.extend(["", f"func {part_name}_wire(x int64) int64 {{", f"    y := {part_name}_chain(x)"])
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = coreFold(y, {dep_part}_apply(x), {index + dep + 1})")
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = coreFold(y, {dep_part}_wire(x), {index + dep + 33})")
        lines.extend(["    return y", "}"])
        lines.extend(["", f"func {part_name}_fanout(x int64) int64 {{", f"    y := {part_name}_wire(x)"])
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = coreFold(y, {dep_part}_chain(x), {index + dep + 65})")
        lines.extend(["    return y", "}"])
        lines.extend(["", f"func {part_name}_signature(seed int64) int64 {{", f"    y := {part_name}_fanout(seed)"])
        for dep in deps:
            dep_part = part_names[dep]
            lines.append(f"    y = coreFold(y, {dep_part}_apply(seed), {index + dep + 97})")
        lines.extend(["    return y", "}"])
        part_file.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return part_files


def write_go_main(go_dir: Path, part_names: list[str]) -> None:
    lines = ['package main', "", 'import "fmt"', "", "func main() {", "    var acc int64 = 0"]
    for part_name in part_names:
        lines.append(f"    acc = {part_name}_apply(acc)")
    lines.extend(["    fmt.Println(acc)", "}"])
    (go_dir / "main.go").write_text("\n".join(lines) + "\n", encoding="utf-8")
