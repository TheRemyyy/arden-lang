#[path = "validation/collision_checks.rs"]
mod collision_checks;
#[path = "build/compile_dispatch_phase.rs"]
mod compile_dispatch_phase;
#[path = "validation/entry_validation_phase.rs"]
mod entry_validation_phase;
#[path = "build/finish_phase.rs"]
mod finish_phase;
#[path = "build/full_codegen_phase.rs"]
mod full_codegen_phase;
#[path = "validation/import_check.rs"]
mod import_check;
#[path = "build/link_phase.rs"]
mod link_phase;
#[path = "object/cache_phase.rs"]
mod object_cache_phase;
#[path = "object/codegen_phase.rs"]
mod object_codegen_phase;
#[path = "object/pipeline.rs"]
mod object_pipeline;
#[path = "object/prep.rs"]
mod object_prep;
#[path = "object/sharding.rs"]
mod object_sharding;
#[path = "parse/index_metrics.rs"]
mod parse_index_metrics;
#[path = "parse/index_phase.rs"]
mod parse_index_phase;
#[path = "parse/index_types.rs"]
mod parse_index_types;
#[path = "build/postcheck_phase.rs"]
mod postcheck_phase;
#[path = "rewrite/context_phase.rs"]
mod rewrite_context_phase;
#[path = "rewrite/phase.rs"]
mod rewrite_phase;
#[path = "rewrite/pipeline.rs"]
mod rewrite_pipeline;
#[path = "rewrite/prep.rs"]
mod rewrite_prep;
#[path = "semantic/gate.rs"]
mod semantic_gate;
#[path = "semantic/phase.rs"]
mod semantic_phase;

pub(crate) use collision_checks::validate_symbol_collisions;
pub(crate) use compile_dispatch_phase::{
    run_compile_dispatch_phase, CompileDispatchInputs, CompileDispatchOutcome,
};
pub(crate) use entry_validation_phase::run_entry_validation_phase;
pub(crate) use finish_phase::{
    finalize_completed_build, finish_full_program_build, FinalizeBuildInputs,
    FullProgramFinishInputs,
};
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
pub(crate) use postcheck_phase::{run_postcheck_phase, PostcheckInputs, PostcheckOutcome};
pub(crate) use rewrite_context_phase::{
    build_rewrite_fingerprint_context, run_rewrite_prep_phase, RewriteContextInputs,
    RewritePrepInputs,
};
pub(crate) use rewrite_phase::{run_rewrite_phase, RewritePhaseInputs};
pub(crate) use rewrite_pipeline::{run_rewrite_pipeline, RewritePipelineInputs};
pub(crate) use rewrite_prep::{prepare_rewrite_inputs, RewritePreparation};
pub(crate) use semantic_gate::{
    compute_project_change_impact, evaluate_semantic_cache_gate, SemanticGateInputs,
};
pub(crate) use semantic_phase::{run_semantic_phase, SemanticPhaseInputs};
