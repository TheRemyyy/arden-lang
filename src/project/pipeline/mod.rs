mod collision_checks;
mod dependency_graph_metrics;
mod dependency_graph_phase;
mod dependency_graph_types;
mod entry_validation_phase;
mod full_codegen_phase;
mod import_check;
mod link_phase;
mod object_cache_phase;
mod object_codegen_phase;
mod object_pipeline;
mod object_prep;
mod object_sharding;
mod parse_index_metrics;
mod parse_index_phase;
mod parse_index_types;
mod rewrite_context_phase;
mod rewrite_phase;
mod rewrite_prep;
mod semantic_gate;
mod semantic_phase;

pub(crate) use collision_checks::validate_symbol_collisions;
pub(crate) use dependency_graph_phase::{run_dependency_graph_phase, DependencyGraphInputs};
pub(crate) use dependency_graph_types::DependencyGraphOutputs;
pub(crate) use entry_validation_phase::run_entry_validation_phase;
pub(crate) use full_codegen_phase::{run_full_codegen_phase, FullCodegenInputs, FullCodegenRoute};
pub(crate) use import_check::{run_import_check_phase, ImportCheckInputs};
pub(crate) use link_phase::{run_final_link_phase, FinalLinkInputs};
pub(crate) use object_cache_phase::{
    run_object_cache_probe, ObjectCacheProbeInputs, ObjectCacheProbeOutputs,
};
pub(crate) use object_codegen_phase::{run_object_codegen_phase, ObjectCodegenPhaseInputs};
pub(crate) use object_pipeline::{run_object_pipeline, ObjectPipelineInputs};
pub(crate) use object_prep::{run_object_prep_step, ObjectPrepOutputs};
pub(crate) use object_sharding::{build_object_sharding_plan, ObjectShardingPlan};
pub(crate) use parse_index_phase::run_parse_index_phase;
pub(crate) use parse_index_types::ParseIndexOutputs;
pub(crate) use rewrite_context_phase::{
    build_rewrite_fingerprint_context, run_rewrite_prep_phase, RewriteContextInputs,
    RewritePrepInputs,
};
pub(crate) use rewrite_phase::{run_rewrite_phase, RewritePhaseInputs};
pub(crate) use rewrite_prep::{prepare_rewrite_inputs, RewritePreparation};
pub(crate) use semantic_gate::{
    compute_project_change_impact, evaluate_semantic_cache_gate, SemanticGateInputs,
};
pub(crate) use semantic_phase::{run_semantic_phase, SemanticPhaseInputs};
