from dataclasses import dataclass


@dataclass(frozen=True)
class BenchmarkSpec:
    name: str
    description: str
    kind: str = "runtime"
    default_enabled: bool = True
    aliases: tuple[str, ...] = ()


@dataclass(frozen=True)
class SyntheticGraphConfig:
    file_count: int
    funcs_per_file: int
    mutate_count: int
    max_deps: int
    group_size: int
    mixed_leaf_edits: int
    mixed_group_edits: int


@dataclass(frozen=True)
class TimedCompileResult:
    elapsed_s: float
    stdout: str
    stderr: str
