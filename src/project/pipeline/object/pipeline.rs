use super::{
    build_object_sharding_plan, run_final_link_phase, run_object_cache_probe,
    run_object_codegen_phase, run_object_prep_step, FinalLinkInputs, ObjectCacheProbeInputs,
    ObjectCacheProbeOutputs, ObjectCodegenPhaseInputs, ObjectPrepOutputs, ObjectShardingPlan,
};
use crate::cache::{
    compute_object_build_fingerprint, load_link_manifest_cache, BuildTimings, ParsedProjectUnit,
    ProjectSymbolLookup, RewrittenProjectUnit,
};
use crate::cli::output::{print_cli_cache, print_cli_step};
use crate::linker::LinkConfig;
use crate::symbol_lookup::GlobalSymbolMaps;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum ObjectPipelineError {
    LinkManifestLoad(String),
    CacheProbe(String),
    Codegen(String),
    FinalLink(String),
}

impl fmt::Display for ObjectPipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LinkManifestLoad(message)
            | Self::CacheProbe(message)
            | Self::Codegen(message)
            | Self::FinalLink(message) => write!(f, "{message}"),
        }
    }
}

impl From<ObjectPipelineError> for String {
    fn from(value: ObjectPipelineError) -> Self {
        value.to_string()
    }
}

pub(crate) struct ObjectPipelineInputs<'a, 'b> {
    pub(crate) project_root: &'a Path,
    pub(crate) output_path: &'a Path,
    pub(crate) parsed_files: &'a [ParsedProjectUnit],
    pub(crate) rewritten_files: &'a [RewrittenProjectUnit],
    pub(crate) file_dependency_graph: &'a HashMap<PathBuf, HashSet<PathBuf>>,
    pub(crate) link: &'a LinkConfig<'b>,
    pub(crate) entry_namespace: &'a str,
    pub(crate) project_symbol_lookup: &'a ProjectSymbolLookup,
    pub(crate) global_maps: GlobalSymbolMaps<'a>,
}

pub(crate) fn run_object_pipeline(
    build_timings: &mut BuildTimings,
    inputs: ObjectPipelineInputs<'_, '_>,
) -> Result<(), String> {
    run_object_pipeline_impl(build_timings, inputs).map_err(Into::into)
}

fn run_object_pipeline_impl(
    build_timings: &mut BuildTimings,
    inputs: ObjectPipelineInputs<'_, '_>,
) -> Result<(), ObjectPipelineError> {
    print_cli_step("Compiling objects");
    let object_build_fingerprint = compute_object_build_fingerprint(inputs.link);
    let previous_link_manifest = build_timings
        .measure("link manifest load", || {
            load_link_manifest_cache(inputs.project_root)
        })
        .map_err(ObjectPipelineError::LinkManifestLoad)?;

    let ObjectPrepOutputs {
        rewritten_file_indices,
        object_cache_paths_by_file,
        codegen_reference_metadata,
        precomputed_dependency_closures,
    } = run_object_prep_step(
        build_timings,
        inputs.parsed_files,
        inputs.rewritten_files,
        inputs.project_root,
        inputs.file_dependency_graph,
    );

    let ObjectShardingPlan {
        object_candidate_count,
        object_shard_size,
        object_shard_threshold,
        object_shards,
    } = build_object_sharding_plan(inputs.rewritten_files, inputs.project_root);

    let ObjectCacheProbeOutputs {
        mut object_paths,
        object_cache_hits,
        cache_misses,
    } = run_object_cache_probe(
        build_timings,
        ObjectCacheProbeInputs {
            object_build_fingerprint: &object_build_fingerprint,
            rewritten_files: inputs.rewritten_files,
            object_cache_paths_by_file: &object_cache_paths_by_file,
            object_shards: &object_shards,
            object_candidate_count,
            object_shard_size,
            object_shard_threshold,
        },
    )
    .map_err(ObjectPipelineError::CacheProbe)?;

    let compiled_results: Vec<(usize, PathBuf)> = run_object_codegen_phase(
        build_timings,
        ObjectCodegenPhaseInputs {
            cache_misses: &cache_misses,
            rewritten_files: inputs.rewritten_files,
            object_cache_paths_by_file: &object_cache_paths_by_file,
            object_build_fingerprint: &object_build_fingerprint,
            link: inputs.link,
            rewritten_file_indices: &rewritten_file_indices,
            codegen_reference_metadata: &codegen_reference_metadata,
            precomputed_dependency_closures: &precomputed_dependency_closures,
            entry_namespace: inputs.entry_namespace,
            project_symbol_lookup: inputs.project_symbol_lookup,
            global_maps: inputs.global_maps,
            object_candidate_count,
            object_cache_hits,
            object_shard_size,
            object_shard_threshold,
        },
    )
    .map_err(ObjectPipelineError::Codegen)?;

    for (index, obj_path) in compiled_results {
        object_paths[index] = Some(obj_path);
    }

    if object_cache_hits > 0 {
        print_cli_cache(format!(
            "Reused object cache for {}/{} files",
            object_cache_hits, object_candidate_count
        ));
    }

    run_final_link_phase(
        build_timings,
        FinalLinkInputs {
            previous_link_manifest: previous_link_manifest.as_ref(),
            output_path: inputs.output_path,
            link: inputs.link,
            project_root: inputs.project_root,
            object_paths,
            cache_miss_count: cache_misses.len(),
        },
    )
    .map_err(ObjectPipelineError::FinalLink)?;

    Ok(())
}
