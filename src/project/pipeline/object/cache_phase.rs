use crate::cache::{
    load_object_cache_hit, load_object_shard_cache_hit, BuildTimings, ObjectCachePaths,
    RewrittenProjectUnit, OBJECT_CACHE_META_TIMING_TOTALS,
};
use crate::cli::output::{cli_error, format_cli_path};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

#[derive(Debug)]
enum ObjectCacheProbeError {
    CacheProbe(String),
    ProbeResult(String),
}

impl fmt::Display for ObjectCacheProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CacheProbe(message) | Self::ProbeResult(message) => write!(f, "{message}"),
        }
    }
}

impl From<ObjectCacheProbeError> for String {
    fn from(value: ObjectCacheProbeError) -> Self {
        value.to_string()
    }
}

impl From<String> for ObjectCacheProbeError {
    fn from(value: String) -> Self {
        Self::ProbeResult(value)
    }
}

pub(crate) struct ObjectCacheProbeInputs<'a> {
    pub(crate) object_build_fingerprint: &'a str,
    pub(crate) rewritten_files: &'a [RewrittenProjectUnit],
    pub(crate) object_cache_paths_by_file: &'a HashMap<PathBuf, ObjectCachePaths>,
    pub(crate) object_shards: &'a [crate::ObjectCodegenShard],
    pub(crate) object_candidate_count: usize,
    pub(crate) object_shard_size: usize,
    pub(crate) object_shard_threshold: usize,
}

pub(crate) struct ObjectCacheProbeOutputs {
    pub(crate) object_paths: Vec<Option<PathBuf>>,
    pub(crate) object_cache_hits: usize,
    pub(crate) cache_misses: Vec<crate::ObjectCodegenShard>,
}

type ProbeResult = Result<(Vec<usize>, Option<PathBuf>), ObjectCacheProbeError>;

pub(crate) fn run_object_cache_probe(
    build_timings: &mut BuildTimings,
    inputs: ObjectCacheProbeInputs<'_>,
) -> Result<ObjectCacheProbeOutputs, String> {
    run_object_cache_probe_impl(build_timings, inputs).map_err(Into::into)
}

fn run_object_cache_probe_impl(
    build_timings: &mut BuildTimings,
    inputs: ObjectCacheProbeInputs<'_>,
) -> Result<ObjectCacheProbeOutputs, ObjectCacheProbeError> {
    let cache_probe_results: Vec<ProbeResult> = build_timings
        .measure("object cache probe", || {
            Ok::<_, String>(
                inputs
                    .object_shards
                    .par_iter()
                    .map(|shard| {
                        let cached_obj = if let Some(cache_paths) = &shard.cache_paths {
                            load_object_shard_cache_hit(
                                cache_paths,
                                &shard.member_fingerprints,
                                inputs.object_build_fingerprint,
                            )
                            .map_err(ObjectCacheProbeError::ProbeResult)?
                        } else {
                            let index = shard.member_indices[0];
                            let unit = &inputs.rewritten_files[index];
                            let cache_paths = inputs
                                .object_cache_paths_by_file
                                .get(&unit.file)
                                .ok_or_else(|| {
                                    ObjectCacheProbeError::ProbeResult(format!(
                                        "{}: missing object cache paths for rewritten unit '{}'",
                                        cli_error("error"),
                                        format_cli_path(&unit.file)
                                    ))
                                })?;
                            load_object_cache_hit(
                                cache_paths,
                                &unit.semantic_fingerprint,
                                &unit.rewrite_context_fingerprint,
                                inputs.object_build_fingerprint,
                            )
                            .map_err(ObjectCacheProbeError::ProbeResult)?
                        };
                        Ok((shard.member_indices.clone(), cached_obj))
                    })
                    .collect(),
            )
        })
        .map_err(ObjectCacheProbeError::CacheProbe)?;

    let mut object_paths: Vec<Option<PathBuf>> = vec![None; inputs.rewritten_files.len()];
    let mut object_cache_hits: usize = 0;
    let mut cache_misses: Vec<crate::ObjectCodegenShard> = Vec::new();
    for (shard, result) in inputs.object_shards.iter().zip(cache_probe_results) {
        let (member_indices, cached_obj) = result?;
        if let Some(cached_obj) = cached_obj {
            for index in member_indices {
                object_paths[index] = Some(cached_obj.clone());
                object_cache_hits += 1;
            }
        } else {
            cache_misses.push(shard.clone());
        }
    }

    build_timings.record_counts(
        "object cache probe",
        &[
            ("candidates", inputs.object_candidate_count),
            ("reused", object_cache_hits),
            (
                "missed",
                inputs
                    .object_candidate_count
                    .saturating_sub(object_cache_hits),
            ),
            ("shard_size", inputs.object_shard_size),
            ("shard_threshold", inputs.object_shard_threshold),
            ("shards", inputs.object_shards.len()),
            ("missed_shards", cache_misses.len()),
        ],
    );
    build_timings.record_duration_ns(
        "object cache meta/load",
        OBJECT_CACHE_META_TIMING_TOTALS
            .load_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_counts(
        "object cache meta/read",
        &[
            (
                "loads",
                OBJECT_CACHE_META_TIMING_TOTALS
                    .load_count
                    .load(Ordering::Relaxed),
            ),
            (
                "bytes_read",
                OBJECT_CACHE_META_TIMING_TOTALS
                    .bytes_read
                    .load(Ordering::Relaxed) as usize,
            ),
        ],
    );

    Ok(ObjectCacheProbeOutputs {
        object_paths,
        object_cache_hits,
        cache_misses,
    })
}
