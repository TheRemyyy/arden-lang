use crate::cache::{
    object_codegen_shard_size, object_codegen_shard_threshold, object_shard_cache_paths,
    ObjectShardMemberFingerprint, RewrittenProjectUnit,
};
use std::path::Path;

pub(crate) struct ObjectShardingPlan {
    pub(crate) object_candidate_count: usize,
    pub(crate) object_shard_size: usize,
    pub(crate) object_shard_threshold: usize,
    pub(crate) object_shards: Vec<crate::ObjectCodegenShard>,
}

pub(crate) fn build_object_sharding_plan(
    rewritten_files: &[RewrittenProjectUnit],
    project_root: &Path,
) -> ObjectShardingPlan {
    let object_candidate_count = rewritten_files
        .iter()
        .filter(|unit| !unit.active_symbols.is_empty())
        .count();
    let active_indices = rewritten_files
        .iter()
        .enumerate()
        .filter_map(|(index, unit)| (!unit.active_symbols.is_empty()).then_some(index))
        .collect::<Vec<_>>();

    let object_shard_size = object_codegen_shard_size();
    let object_shard_threshold = object_codegen_shard_threshold();
    let use_object_shards = object_shard_size > 1 && active_indices.len() >= object_shard_threshold;

    let object_shards = if use_object_shards {
        active_indices
            .chunks(object_shard_size)
            .map(|chunk| {
                let member_indices = chunk.to_vec();
                let member_files = member_indices
                    .iter()
                    .map(|index| rewritten_files[*index].file.clone())
                    .collect::<Vec<_>>();
                let member_fingerprints = member_indices
                    .iter()
                    .map(|index| {
                        let unit = &rewritten_files[*index];
                        ObjectShardMemberFingerprint {
                            file: unit.file.clone(),
                            semantic_fingerprint: unit.semantic_fingerprint.clone(),
                            rewrite_context_fingerprint: unit.rewrite_context_fingerprint.clone(),
                        }
                    })
                    .collect::<Vec<_>>();
                let cache_paths = Some(object_shard_cache_paths(project_root, &member_files));
                crate::ObjectCodegenShard {
                    member_indices,
                    member_files,
                    member_fingerprints,
                    cache_paths,
                }
            })
            .collect::<Vec<_>>()
    } else {
        active_indices
            .iter()
            .map(|index| {
                let unit = &rewritten_files[*index];
                crate::ObjectCodegenShard {
                    member_indices: vec![*index],
                    member_files: vec![unit.file.clone()],
                    member_fingerprints: vec![ObjectShardMemberFingerprint {
                        file: unit.file.clone(),
                        semantic_fingerprint: unit.semantic_fingerprint.clone(),
                        rewrite_context_fingerprint: unit.rewrite_context_fingerprint.clone(),
                    }],
                    cache_paths: None,
                }
            })
            .collect::<Vec<_>>()
    };

    ObjectShardingPlan {
        object_candidate_count,
        object_shard_size,
        object_shard_threshold,
        object_shards,
    }
}
