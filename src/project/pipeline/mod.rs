mod collision_checks;
mod import_check;
mod rewrite_phase;
mod rewrite_prep;
mod semantic_gate;
mod semantic_phase;

pub(crate) use collision_checks::validate_symbol_collisions;
pub(crate) use import_check::{run_import_check_phase, ImportCheckInputs};
pub(crate) use rewrite_phase::{run_rewrite_phase, RewritePhaseInputs};
pub(crate) use rewrite_prep::{prepare_rewrite_inputs, RewritePreparation};
pub(crate) use semantic_gate::{
    compute_project_change_impact, evaluate_semantic_cache_gate, SemanticGateInputs,
};
pub(crate) use semantic_phase::{run_semantic_phase, SemanticPhaseInputs};
