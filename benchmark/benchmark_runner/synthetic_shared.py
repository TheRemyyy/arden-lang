from pathlib import Path


def _unique_dependency_candidates(candidates: list[int], index: int, max_deps: int) -> list[int]:
    deps: list[int] = []
    for candidate in candidates:
        if 0 <= candidate < index and candidate not in deps:
            deps.append(candidate)
        if len(deps) == max_deps:
            break
    return deps


def synthetic_graph_dependency_indices(
    index: int,
    max_deps: int,
    topology: str,
    group_size: int,
) -> list[int]:
    if index <= 0:
        return []

    if topology == "flat":
        return []

    if topology == "layered":
        layer_width = max(8, group_size)
        layer_index = index // layer_width
        if layer_index == 0:
            return []
        prev_layer_start = (layer_index - 1) * layer_width
        prev_layer_end = min(index, layer_index * layer_width)
        prev_layer = list(range(prev_layer_start, prev_layer_end))
        if not prev_layer:
            return []
        if len(prev_layer) <= max_deps:
            return prev_layer
        step = (len(prev_layer) - 1) / (max_deps - 1)
        sampled = [prev_layer[round(step * dep_index)] for dep_index in range(max_deps)]
        return _unique_dependency_candidates(sampled, index, max_deps)

    if topology == "dense":
        return list(range(max(0, index - max_deps), index))

    if topology == "worst_case":
        return _unique_dependency_candidates(
            [
                0,
                1,
                index - 1,
                index - 2,
                index - 3,
                index - 5,
                index - 8,
                index // 2,
                (index * 3) // 4,
                (index * 7) // 8,
            ],
            index,
            max_deps,
        )

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
    return _unique_dependency_candidates(candidates, index, max_deps)


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
