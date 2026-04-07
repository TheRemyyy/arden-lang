from pathlib import Path


def synthetic_graph_dependency_indices(index: int, max_deps: int) -> list[int]:
    if index <= 0:
        return []

    candidates = [
        index - 1,
        index - 3,
        index - 7,
        index - 15,
        index // 2,
        index - 32,
        index - 64,
        (index * 7) // 11,
        (index * 5) // 8,
        (index * 3) // 5,
    ]
    deps: list[int] = []
    for candidate in candidates:
        if 0 <= candidate < index and candidate not in deps:
            deps.append(candidate)
        if len(deps) == max_deps:
            break
    return deps


def spread_group_plans(plans: list[dict], count: int) -> list[dict]:
    if not plans:
        return []
    count = max(1, min(count, len(plans)))
    if count == 1:
        return [plans[-1]]
    last_index = len(plans) - 1
    indices = {round((last_index * index) / (count - 1)) for index in range(count)}
    return [plans[index] for index in sorted(indices)]


def create_group_plan(group_name: str, group_index: int, group_salt: int, surface_file: Path) -> dict:
    return {
        "group_name": group_name,
        "group_index": group_index,
        "call_salt": group_salt,
        "surface_files": [surface_file],
        "caller_files": [],
    }


def build_main_lines(part_names: list[str], template: str) -> list[str]:
    lines = []
    for part_name in part_names:
        lines.append(template.format(part_name=part_name))
    return lines
