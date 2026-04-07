from .types import BenchmarkSpec, SyntheticGraphConfig

LANGUAGES = ("arden", "rust", "go")

SYNTHETIC_MEGA_GRAPH_CONFIG = SyntheticGraphConfig(
    file_count=1400,
    funcs_per_file=96,
    mutate_count=40,
    max_deps=6,
    group_size=50,
    mixed_leaf_edits=24,
    mixed_group_edits=8,
)

EXTREME_GRAPH_CONFIG = SyntheticGraphConfig(
    file_count=2200,
    funcs_per_file=112,
    mutate_count=64,
    max_deps=8,
    group_size=44,
    mixed_leaf_edits=40,
    mixed_group_edits=12,
)

BENCHMARKS = [
    BenchmarkSpec("sum_loop", "Integer-heavy pseudo-random accumulation loop"),
    BenchmarkSpec("prime_count", "Prime counting via sieve"),
    BenchmarkSpec("matrix_mul", "Dense matrix multiplication (100x100)"),
    BenchmarkSpec(
        "matrix_mul_heavy",
        "Dense integer matrix multiplication (220x220) for a heavier CPU-bound runtime pass",
        default_enabled=False,
    ),
    BenchmarkSpec(
        "compile_project_starter_graph",
        "Compile stress test on a generated starter project graph per language",
        kind="compile",
        aliases=("compile_project_10_files",),
    ),
    BenchmarkSpec(
        "compile_project_mega_graph",
        "Compile stress test on a generated 1400-file mega-graph project per language",
        kind="compile",
        aliases=("compile_project_synthetic_mega_graph",),
    ),
    BenchmarkSpec(
        "compile_project_extreme_graph",
        "Compile stress test on a generated 2200-file extreme dependency graph per language",
        kind="compile",
        default_enabled=False,
    ),
    BenchmarkSpec(
        "incremental_rebuild_single_file",
        "Compile a starter project graph, mutate one leaf file, then rebuild",
        kind="incremental",
        aliases=("incremental_rebuild_1_file",),
    ),
    BenchmarkSpec(
        "incremental_rebuild_shared_core",
        "Compile a starter project graph with a shared core dependency, mutate the shared core, then rebuild",
        kind="incremental",
        aliases=("incremental_rebuild_central_file",),
    ),
    BenchmarkSpec(
        "incremental_rebuild_large_project_batch",
        "Compile a generated large project graph, apply syntax-only edits to 10 files, then rebuild",
        kind="incremental_batch",
        aliases=("incremental_rebuild_mega_project_10_files",),
    ),
    BenchmarkSpec(
        "incremental_rebuild_mega_graph_batch",
        "Compile a generated mega-graph project, apply syntax-only edits to many files, then rebuild",
        kind="incremental_batch_synthetic_mega_graph",
        aliases=("incremental_rebuild_synthetic_mega_graph",),
    ),
    BenchmarkSpec(
        "incremental_rebuild_mega_graph_mixed",
        "Compile a generated mega-graph project, then rebuild after mixed leaf edits and API-surface invalidation",
        kind="incremental_mixed_synthetic_mega_graph",
        aliases=("incremental_rebuild_synthetic_mega_graph_mixed_invalidation",),
    ),
    BenchmarkSpec(
        "incremental_rebuild_extreme_graph_batch",
        "Compile a generated 2200-file extreme dependency graph, apply syntax-only edits to many files, then rebuild",
        kind="incremental_batch_extreme_graph",
        default_enabled=False,
        aliases=("incremental_rebuild_extreme_graph",),
    ),
    BenchmarkSpec(
        "incremental_rebuild_extreme_graph_mixed",
        "Compile a generated 2200-file extreme dependency graph, then rebuild after leaf edits plus shared API invalidation",
        kind="incremental_mixed_extreme_graph",
        default_enabled=False,
        aliases=("incremental_rebuild_extreme_graph_mixed_invalidation",),
    ),
]


def all_benchmark_names() -> list[str]:
    names: list[str] = []
    for spec in BENCHMARKS:
        names.append(spec.name)
        names.extend(spec.aliases)
    return names


def resolve_benchmark_name(name: str | None) -> str | None:
    if name is None:
        return None
    for spec in BENCHMARKS:
        if name == spec.name or name in spec.aliases:
            return spec.name
    raise RuntimeError(f"Unknown benchmark: {name}")


def select_synthetic_graph_config(bench_name: str) -> SyntheticGraphConfig:
    if "extreme_graph" in bench_name:
        return EXTREME_GRAPH_CONFIG
    return SYNTHETIC_MEGA_GRAPH_CONFIG


def select_benchmarks(requested_name: str | None, include_extreme: bool) -> list[BenchmarkSpec]:
    resolved_name = resolve_benchmark_name(requested_name)
    return [
        spec
        for spec in BENCHMARKS
        if (resolved_name is None or spec.name == resolved_name)
        and (resolved_name is not None or include_extreme or spec.default_enabled)
    ]


def expand_default_suite(selected: list[BenchmarkSpec]) -> list[BenchmarkSpec]:
    expanded: list[BenchmarkSpec] = []
    for spec in selected:
        if spec.kind == "compile":
            expanded.append(
                BenchmarkSpec(
                    f"{spec.name}_hot",
                    f"{spec.description} (hot cache mode)",
                    kind="compile",
                )
            )
            expanded.append(
                BenchmarkSpec(
                    f"{spec.name}_cold",
                    f"{spec.description} (cold cache mode)",
                    kind="compile",
                )
            )
            continue
        expanded.append(spec)
    return expanded
