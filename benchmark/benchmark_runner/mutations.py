from pathlib import Path


def apply_incremental_source_change(source: Path, marker: str) -> None:
    if not source.exists():
        raise RuntimeError(f"Missing source to mutate: {source}")
    with source.open("a", encoding="utf-8") as handle:
        handle.write(f"\n// incremental bench mutation {marker}\n")


def apply_incremental_source_changes(sources: list[Path], marker: str) -> None:
    for index, source in enumerate(sources):
        apply_incremental_source_change(source, f"{marker}_file_{index:02d}")


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise RuntimeError(f"Expected mutation hook not found in {path}: {old}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def apply_api_surface_cascade_changes(lang: str, job: dict, _marker: str) -> None:
    """
    Simulate an API-surface change by adding an ignored extra parameter
    (`_api_extra`) to the shared `core_blend` / `blend` / `coreBlend`
    function and updating every call site across all dependent files.

    All languages add the same parameter so the output checksum is unchanged
    (the extra argument is always passed as 0 and never used in the body).
    This lets the cross-language checksum verification still pass after
    the mutation.
    """
    core_file = Path(job.get("api_core_file", ""))
    part_files = [Path(p) for p in job.get("api_part_files", [])]

    if lang == "arden":
        replace_once(
            core_file,
            "function core_blend(x: Integer, k: Integer): Integer {",
            "function core_blend(x: Integer, k: Integer, _api_extra: Integer): Integer {",
        )
        for index, part_file in enumerate(part_files):
            part_name = f"part_{index:02d}"
            replace_once(
                part_file,
                f"function {part_name}_blend_step(x: Integer): Integer {{ return core_blend(x, {index + 1}); }}",
                f"function {part_name}_blend_step(x: Integer): Integer {{ return core_blend(x, {index + 1}, 0); }}",
            )

    elif lang == "rust":
        replace_once(
            core_file,
            "pub fn blend(x: i64, k: i64) -> i64 { x * k + k }",
            "pub fn blend(x: i64, k: i64, _api_extra: i64) -> i64 { x * k + k }",
        )
        for index, part_file in enumerate(part_files):
            replace_once(
                part_file,
                f"fn blend_step_{index}(x: i64) -> i64 {{ crate::core::blend(x, {index + 1}) }}",
                f"fn blend_step_{index}(x: i64) -> i64 {{ crate::core::blend(x, {index + 1}, 0) }}",
            )

    elif lang == "go":
        replace_once(
            core_file,
            "func coreBlend(x int64, k int64) int64 { return x*k + k }",
            "func coreBlend(x int64, k int64, _ int64) int64 { return x*k + k }",
        )
        for index, part_file in enumerate(part_files):
            part_name = f"part_{index:02d}"
            replace_once(
                part_file,
                f"func {part_name}_blend(x int64) int64 {{ return coreBlend(x, {index + 1}) }}",
                f"func {part_name}_blend(x int64) int64 {{ return coreBlend(x, {index + 1}, 0) }}",
            )

    else:
        raise RuntimeError(f"Unsupported language for API surface cascade: {lang}")


def apply_mixed_invalidation_changes(lang: str, job: dict, marker: str) -> None:
    apply_incremental_source_changes(
        [Path(path) for path in job.get("mixed_leaf_sources", [])],
        f"{marker}_leaf",
    )

    for index, group in enumerate(job.get("mixed_groups", [])):
        group_name = group["group_name"]
        salt = int(group["call_salt"])
        extra = 5000 + group["group_index"] * 13 + index

        if lang == "arden":
            replace_once(
                Path(group["surface_files"][0]),
                f"function {group_name}_bridge(x: Integer, salt: Integer): Integer {{",
                f"function {group_name}_bridge(x: Integer, salt: Integer, extra: Integer): Integer {{",
            )
            replace_once(
                Path(group["surface_files"][0]),
                f"    return core_fold(x, salt, {salt});",
                f"    return core_fold(x, salt + extra, {salt});",
            )
            old_call = f"    y = {group_name}_bridge(y, {salt}); // MUTATION_CALL_{group_name.upper()}"
            new_call = f"    y = {group_name}_bridge(y, {salt}, {extra}); // MUTATION_CALL_{group_name.upper()}"
        elif lang == "rust":
            replace_once(
                Path(group["surface_files"][0]),
                f"pub fn {group_name}_bridge(x: i64, salt: i64) -> i64 {{",
                f"pub fn {group_name}_bridge(x: i64, salt: i64, extra: i64) -> i64 {{",
            )
            replace_once(
                Path(group["surface_files"][0]),
                f"    crate::core::fold(x, salt, {salt})",
                f"    crate::core::fold(x, salt + extra, {salt})",
            )
            old_call = f"    y = crate::{group_name}::{group_name}_bridge(y, {salt}); // MUTATION_CALL_{group_name.upper()}"
            new_call = f"    y = crate::{group_name}::{group_name}_bridge(y, {salt}, {extra}); // MUTATION_CALL_{group_name.upper()}"
        elif lang == "go":
            replace_once(
                Path(group["surface_files"][0]),
                f"func {group_name}_bridge(x int64, salt int64) int64 {{",
                f"func {group_name}_bridge(x int64, salt int64, extra int64) int64 {{",
            )
            replace_once(
                Path(group["surface_files"][0]),
                f"    return coreFold(x, salt, {salt})",
                f"    return coreFold(x, salt+extra, {salt})",
            )
            old_call = f"    y = {group_name}_bridge(y, {salt}) // MUTATION_CALL_{group_name.upper()}"
            new_call = f"    y = {group_name}_bridge(y, {salt}, {extra}) // MUTATION_CALL_{group_name.upper()}"
        else:
            raise RuntimeError(f"Unsupported language for mixed invalidation: {lang}")

        for caller in group.get("caller_files", []):
            replace_once(Path(caller), old_call, new_call)
