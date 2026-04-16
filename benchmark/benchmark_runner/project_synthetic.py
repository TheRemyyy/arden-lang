from pathlib import Path

from .project_small import pick_mutation_targets, reset_generated_root
from .synthetic_arden import (
    write_arden_core,
    write_arden_group_files,
    write_arden_main_and_config,
    write_arden_part_files,
)
from .synthetic_go import write_go_core, write_go_group_files, write_go_main, write_go_part_files
from .synthetic_rust import write_rust_core, write_rust_group_files, write_rust_main, write_rust_part_files
from .synthetic_shared import spread_group_plans
from .types import SyntheticGraphConfig


def generate_compile_project_synthetic_graph(
    root: Path,
    bench_name: str,
    config: SyntheticGraphConfig,
    mutate_count: int | None = None,
) -> dict[str, dict[str, Path]]:
    effective_mutate_count = mutate_count if mutate_count is not None else config.mutate_count
    generated_root = root / "benchmark" / "generated" / bench_name
    reset_generated_root(generated_root)

    part_names = [f"part_{index:04d}" for index in range(config.file_count)]
    group_count = (config.file_count + config.group_size - 1) // config.group_size
    group_names = [f"group_{index:02d}" for index in range(group_count)]

    arden_dir = generated_root / "arden"
    arden_src = arden_dir / "src"
    arden_src.mkdir(parents=True, exist_ok=True)
    rust_dir = generated_root / "rust"
    rust_dir.mkdir(parents=True, exist_ok=True)
    go_dir = generated_root / "go"
    go_dir.mkdir(parents=True, exist_ok=True)

    write_arden_core(arden_src)
    arden_files, arden_group_plans = write_arden_group_files(arden_src, group_names)
    arden_part_files = write_arden_part_files(
        arden_src,
        part_names,
        group_names,
        config.funcs_per_file,
        config.max_deps,
        config.group_size,
        config.topology,
        arden_files,
        arden_group_plans,
    )
    write_arden_main_and_config(arden_dir, arden_src, bench_name, part_names, arden_files)

    rust_bin_name = f"{bench_name}_rust"
    (rust_dir / "Cargo.toml").write_text(
        "\n".join(
            [
                "[package]",
                f'name = "{rust_bin_name}"',
                'version = "0.1.0"',
                'edition = "2021"',
                "",
                "[[bin]]",
                f'name = "{rust_bin_name}"',
                'path = "main.rs"',
                "",
            ]
        ),
        encoding="utf-8",
    )
    write_rust_core(rust_dir)
    rust_group_plans = write_rust_group_files(rust_dir, group_names)
    rust_part_files = write_rust_part_files(
        rust_dir,
        part_names,
        group_names,
        config.funcs_per_file,
        config.max_deps,
        config.group_size,
        config.topology,
        rust_group_plans,
    )
    write_rust_main(rust_dir, part_names, group_names)

    write_go_core(go_dir)
    go_group_plans = write_go_group_files(go_dir, group_names)
    go_part_files = write_go_part_files(
        go_dir,
        part_names,
        group_names,
        config.funcs_per_file,
        config.max_deps,
        config.group_size,
        config.topology,
        go_group_plans,
    )
    write_go_main(go_dir, part_names)

    arden_mutate_sources = pick_mutation_targets(arden_part_files, effective_mutate_count, "batch_spread")
    rust_mutate_sources = pick_mutation_targets(rust_part_files, effective_mutate_count, "batch_spread")
    go_mutate_sources = pick_mutation_targets(go_part_files, effective_mutate_count, "batch_spread")

    return {
        "arden": {
            "project_dir": arden_dir,
            "binary": arden_dir / bench_name,
            "mutate_source": arden_mutate_sources[0],
            "mutate_sources": arden_mutate_sources,
            "mixed_leaf_sources": pick_mutation_targets(arden_part_files, config.mixed_leaf_edits, "batch_spread"),
            "mixed_groups": spread_group_plans(arden_group_plans, config.mixed_group_edits),
        },
        "rust": {
            "project_dir": rust_dir,
            "binary": rust_dir / "target" / "release" / rust_bin_name,
            "mutate_source": rust_mutate_sources[0],
            "mutate_sources": rust_mutate_sources,
            "mixed_leaf_sources": pick_mutation_targets(rust_part_files, config.mixed_leaf_edits, "batch_spread"),
            "mixed_groups": spread_group_plans(rust_group_plans, config.mixed_group_edits),
        },
        "go": {
            "project_dir": go_dir,
            "binary": go_dir / f"{bench_name}_go",
            "mutate_source": go_mutate_sources[0],
            "mutate_sources": go_mutate_sources,
            "mixed_leaf_sources": pick_mutation_targets(go_part_files, config.mixed_leaf_edits, "batch_spread"),
            "mixed_groups": spread_group_plans(go_group_plans, config.mixed_group_edits),
        },
    }
