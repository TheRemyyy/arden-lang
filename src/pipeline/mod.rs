mod collision_checks;
mod import_check;
mod rewrite_prep;
mod semantic_gate;

pub(crate) use collision_checks::validate_symbol_collisions;
pub(crate) use import_check::{run_import_check_phase, ImportCheckInputs};
pub(crate) use rewrite_prep::{prepare_rewrite_inputs, RewritePreparation};
pub(crate) use semantic_gate::{
    compute_project_change_impact, evaluate_semantic_cache_gate, SemanticGateInputs,
};
