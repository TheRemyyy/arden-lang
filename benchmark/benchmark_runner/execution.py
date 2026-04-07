from pathlib import Path

from .jobs import (
    clean_compile_artifacts,
    compile_arden,
    compile_go,
    compile_rust,
    make_compile_jobs,
    run_checksum,
    timed_compile_with_retry,
    timed_run,
)
from .mutations import (
    apply_incremental_source_change,
    apply_incremental_source_changes,
    apply_mixed_invalidation_changes,
)
from .project_small import (
    generate_compile_project_starter_graph,
    generate_incremental_rebuild_large_project_batch,
)
from .project_synthetic import generate_compile_project_synthetic_graph
from .specs import LANGUAGES, select_synthetic_graph_config
from .system import compute_stats, exe_path
from .timings import parse_build_timings, summarize_arden_phase_timings

INCREMENTAL_KINDS = {
    "incremental",
    "incremental_batch",
    "incremental_batch_synthetic_mega_graph",
    "incremental_mixed_synthetic_mega_graph",
    "incremental_batch_extreme_graph",
    "incremental_mixed_extreme_graph",
}


def _append_phase_timings(lang_data: dict, phase_one_label: str | None, phase_two_label: str | None) -> list[dict]:
    sections: list[dict] = []
    compile_phase_timings = lang_data["arden"].get("phase_timings")
    if compile_phase_timings:
        sections.append({"label": "Arden build phase timings", "phases": compile_phase_timings})
    first_phase_timings = lang_data["arden"].get("phase_timings_first")
    if first_phase_timings and phase_one_label:
        sections.append({"label": f"Arden {phase_one_label}", "phases": first_phase_timings})
    second_phase_timings = lang_data["arden"].get("phase_timings_second")
    if second_phase_timings and phase_two_label:
        sections.append({"label": f"Arden {phase_two_label}", "phases": second_phase_timings})
    return sections


def _finalize_benchmark(spec, lang_data: dict, benchmark_compile_mode: str | None, phase_one_label: str | None, phase_two_label: str | None, ratio_label: str | None) -> dict:
    arden_mean = lang_data["arden"]["stats"]["mean_s"]
    speedups = {
        lang: arden_mean / lang_data[lang]["stats"]["mean_s"]
        for lang in LANGUAGES
        if lang != "arden"
    }
    return {
        "name": spec.name,
        "description": spec.description,
        "kind": spec.kind,
        "compile_mode": benchmark_compile_mode if spec.kind in INCREMENTAL_KINDS | {"compile"} else None,
        "phase_one_label": phase_one_label if spec.kind in INCREMENTAL_KINDS else None,
        "phase_two_label": phase_two_label if spec.kind in INCREMENTAL_KINDS else None,
        "ratio_label": ratio_label if spec.kind in INCREMENTAL_KINDS else None,
        "languages": lang_data,
        "speedup_vs_arden": speedups,
        "arden_phase_timing_sections": _append_phase_timings(lang_data, phase_one_label, phase_two_label),
    }


def run_runtime_benchmark(spec, root: Path, bin_dir: Path, build_env: dict[str, str], opt_level: str, target: str | None, warmup: int, repeats: int) -> dict:
    lang_data: dict[str, dict] = {}
    reference_checksum = None
    binaries = {
        "arden": exe_path(bin_dir / f"{spec.name}_arden"),
        "rust": exe_path(bin_dir / f"{spec.name}_rust"),
        "go": exe_path(bin_dir / f"{spec.name}_go"),
    }
    compile_arden(root, spec.name, binaries["arden"], build_env, opt_level, target)
    compile_rust(root, spec.name, binaries["rust"])
    compile_go(root, spec.name, binaries["go"])

    for lang in LANGUAGES:
        print(f"Running {lang}...")
        for _ in range(warmup):
            timed_run(binaries[lang], root)
        samples: list[float] = []
        checksums: list[int] = []
        for _ in range(repeats):
            elapsed, checksum = timed_run(binaries[lang], root)
            samples.append(elapsed)
            checksums.append(checksum)
        if len(set(checksums)) != 1:
            raise RuntimeError(f"Non-deterministic checksum in {lang}/{spec.name}: {checksums}")
        checksum = checksums[0]
        if reference_checksum is None:
            reference_checksum = checksum
        elif checksum != reference_checksum:
            raise RuntimeError(f"Checksum mismatch for {spec.name}: {lang}={checksum}, expected={reference_checksum}")
        lang_data[lang] = {"checksum": checksum, "samples_s": samples, "stats": compute_stats(samples), "metric": "runtime"}

    return _finalize_benchmark(spec, lang_data, None, None, None, None)


def run_compile_benchmark(spec, root: Path, build_env: dict[str, str], arden_timings: bool, compile_mode: str, warmup: int, repeats: int) -> dict:
    base_name = spec.name
    effective_mode = compile_mode
    if spec.name.endswith("_hot"):
        effective_mode = "hot"
        base_name = spec.name[: -len("_hot")]
    elif spec.name.endswith("_cold"):
        effective_mode = "cold"
        base_name = spec.name[: -len("_cold")]

    if "mega_graph" in base_name or "extreme_graph" in base_name:
        compile_projects = generate_compile_project_synthetic_graph(root, base_name, select_synthetic_graph_config(base_name))
    else:
        compile_projects = generate_compile_project_starter_graph(root, base_name)
    compile_jobs = make_compile_jobs(root, compile_projects, build_env, arden_timings)

    lang_data: dict[str, dict] = {}
    reference_checksum = None
    for lang in LANGUAGES:
        print(f"Compiling {lang}...")
        job = compile_jobs[lang]
        arden_timing_samples: list[dict[str, dict]] = []
        for _ in range(warmup):
            if effective_mode == "cold":
                clean_compile_artifacts(lang, job)
            timed_compile_with_retry(lang, job)
        samples: list[float] = []
        for _ in range(repeats):
            if effective_mode == "cold":
                clean_compile_artifacts(lang, job)
            compile_result = timed_compile_with_retry(lang, job)
            samples.append(compile_result.elapsed_s)
            if lang == "arden" and arden_timings:
                arden_timing_samples.append(parse_build_timings(compile_result.stdout))
        if not Path(job["binary"]).exists():
            timed_compile_with_retry(lang, job, retries=2)
        checksum = run_checksum(job["binary"], job["cwd"])
        if reference_checksum is None:
            reference_checksum = checksum
        elif checksum != reference_checksum:
            raise RuntimeError(f"Checksum mismatch for {spec.name}: {lang}={checksum}, expected={reference_checksum}")
        lang_data[lang] = {"checksum": checksum, "samples_s": samples, "stats": compute_stats(samples), "metric": "compile"}
        if lang == "arden" and arden_timings:
            lang_data[lang]["phase_timings"] = summarize_arden_phase_timings(arden_timing_samples)

    return _finalize_benchmark(spec, lang_data, effective_mode, None, None, None)


def _run_two_phase_incremental(spec, root: Path, build_env: dict[str, str], arden_timings: bool, warmup: int, repeats: int, project_factory, mutate_job, benchmark_compile_mode: str, phase_one_label: str, phase_two_label: str, ratio_label: str) -> dict:
    lang_data: dict[str, dict] = {}
    reference_checksum = None

    for lang in LANGUAGES:
        print(f"{phase_two_label.split(' mean')[0].capitalize()} {lang}...")
        first_samples: list[float] = []
        second_samples: list[float] = []
        checksums: list[int] = []
        arden_first_phase_samples: list[dict[str, dict]] = []
        arden_second_phase_samples: list[dict[str, dict]] = []

        for cycle in range(warmup + repeats):
            cycle_projects = project_factory()
            cycle_jobs = make_compile_jobs(root, cycle_projects, build_env, arden_timings)
            job = cycle_jobs[lang]
            clean_compile_artifacts(lang, job)
            first_result = timed_compile_with_retry(lang, job)
            mutate_job(lang, job, cycle)
            second_result = timed_compile_with_retry(lang, job)

            if not Path(job["binary"]).exists():
                timed_compile_with_retry(lang, job, retries=2)
            checksum = run_checksum(job["binary"], job["cwd"])

            if cycle >= warmup:
                first_samples.append(first_result.elapsed_s)
                second_samples.append(second_result.elapsed_s)
                checksums.append(checksum)
                if lang == "arden" and arden_timings:
                    arden_first_phase_samples.append(parse_build_timings(first_result.stdout))
                    arden_second_phase_samples.append(parse_build_timings(second_result.stdout))

        if len(set(checksums)) != 1:
            raise RuntimeError(f"Non-deterministic checksum in incremental {lang}/{spec.name}: {checksums}")

        checksum = checksums[0]
        if reference_checksum is None:
            reference_checksum = checksum
        elif checksum != reference_checksum:
            raise RuntimeError(f"Checksum mismatch for {spec.name}: {lang}={checksum}, expected={reference_checksum}")

        lang_data[lang] = {
            "checksum": checksum,
            "first_samples_s": first_samples,
            "second_samples_s": second_samples,
            "first_stats": compute_stats(first_samples),
            "second_stats": compute_stats(second_samples),
            "stats": compute_stats(second_samples),
            "metric": "incremental_compile_second",
        }
        if lang == "arden" and arden_timings:
            lang_data[lang]["phase_timings_first"] = summarize_arden_phase_timings(arden_first_phase_samples)
            lang_data[lang]["phase_timings_second"] = summarize_arden_phase_timings(arden_second_phase_samples)

    return _finalize_benchmark(spec, lang_data, benchmark_compile_mode, phase_one_label, phase_two_label, ratio_label)


def run_incremental_benchmark(spec, root: Path, build_env: dict[str, str], arden_timings: bool, warmup: int, repeats: int) -> dict:
    mutation_profile = "central" if "shared_core" in spec.name else "leaf"
    return _run_two_phase_incremental(
        spec,
        root,
        build_env,
        arden_timings,
        warmup,
        repeats,
        lambda: generate_compile_project_starter_graph(root, spec.name, mutation_profile=mutation_profile),
        lambda _lang, job, cycle: apply_incremental_source_change(Path(job["mutate_source"]), f"{cycle}"),
        "cold_then_hot_single_edit",
        "full compile mean (s)",
        "rebuild mean (s)",
        "rebuild/full",
    )


def run_incremental_batch_benchmark(spec, root: Path, build_env: dict[str, str], arden_timings: bool, warmup: int, repeats: int) -> dict:
    if "mega_graph" in spec.name or "extreme_graph" in spec.name:
        graph_config = select_synthetic_graph_config(spec.name)
        project_factory = lambda: generate_compile_project_synthetic_graph(root, spec.name, graph_config, mutate_count=graph_config.mutate_count)
        phase_two_label = f"hot rebuild after {graph_config.mutate_count} edits mean (s)"
    else:
        graph_config = None
        project_factory = lambda: generate_incremental_rebuild_large_project_batch(root, spec.name)
        phase_two_label = "hot rebuild after 10 edits mean (s)"

    return _run_two_phase_incremental(
        spec,
        root,
        build_env,
        arden_timings,
        warmup,
        repeats,
        project_factory,
        lambda _lang, job, cycle: apply_incremental_source_changes([Path(path) for path in job.get("mutate_sources", [])], f"{cycle}"),
        "cold_then_hot_batch_edit",
        "cold full build mean (s)",
        phase_two_label,
        "hot/cold",
    )


def run_incremental_mixed_benchmark(spec, root: Path, build_env: dict[str, str], arden_timings: bool, warmup: int, repeats: int) -> dict:
    graph_config = select_synthetic_graph_config(spec.name)
    return _run_two_phase_incremental(
        spec,
        root,
        build_env,
        arden_timings,
        warmup,
        repeats,
        lambda: generate_compile_project_synthetic_graph(root, spec.name, graph_config, mutate_count=graph_config.mutate_count),
        lambda lang, job, cycle: apply_mixed_invalidation_changes(lang, job, f"{cycle}"),
        "cold_then_hot_mixed_invalidation",
        "cold full build mean (s)",
        f"mixed rebuild mean (leaf {graph_config.mixed_leaf_edits} + {graph_config.mixed_group_edits} API groups)",
        "mixed/cold",
    )


def run_selected_benchmarks(selected: list, root: Path, bin_dir: Path, build_env: dict[str, str], opt_level: str, target: str | None, compile_mode: str, warmup: int, repeats: int, arden_timings: bool) -> list[dict]:
    results: list[dict] = []
    for spec in selected:
        print(f"\n=== {spec.name} ===")
        if spec.kind == "runtime":
            results.append(run_runtime_benchmark(spec, root, bin_dir, build_env, opt_level, target, warmup, repeats))
            continue
        if spec.kind == "compile":
            results.append(run_compile_benchmark(spec, root, build_env, arden_timings, compile_mode, warmup, repeats))
            continue
        if spec.kind == "incremental":
            results.append(run_incremental_benchmark(spec, root, build_env, arden_timings, warmup, repeats))
            continue
        if spec.kind in {"incremental_batch", "incremental_batch_synthetic_mega_graph", "incremental_batch_extreme_graph"}:
            results.append(run_incremental_batch_benchmark(spec, root, build_env, arden_timings, warmup, repeats))
            continue
        if spec.kind in {"incremental_mixed_synthetic_mega_graph", "incremental_mixed_extreme_graph"}:
            results.append(run_incremental_mixed_benchmark(spec, root, build_env, arden_timings, warmup, repeats))
            continue
        raise RuntimeError(f"Unsupported benchmark kind: {spec.kind}")
    return results
