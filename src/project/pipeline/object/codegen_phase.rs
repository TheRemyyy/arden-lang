use crate::cache::{
    elapsed_nanos_u64, save_object_cache_meta, save_object_shard_cache_meta, BuildTimings,
    DeclarationClosureTimingTotals, ObjectCachePaths, ObjectCodegenTimingTotals,
    ObjectEmitTimingTotals, ProjectSymbolLookup, RewrittenProjectUnit,
    OBJECT_CACHE_META_TIMING_TOTALS,
};
use crate::cli::output::{cli_error, format_cli_path};
use crate::linker::LinkConfig;
use crate::specialization::{codegen_program_for_units, combined_program_for_files};
use crate::symbol_lookup::{
    closure_body_symbols_for_files, declaration_symbols_for_unit, CodegenReferenceMetadata,
    DeclarationClosureRequest, GlobalSymbolMaps,
};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
enum ObjectCodegenPhaseError {
    ShardCompile(String),
}

impl fmt::Display for ObjectCodegenPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShardCompile(message) => write!(f, "{message}"),
        }
    }
}

impl From<ObjectCodegenPhaseError> for String {
    fn from(value: ObjectCodegenPhaseError) -> Self {
        value.to_string()
    }
}

impl From<String> for ObjectCodegenPhaseError {
    fn from(value: String) -> Self {
        Self::ShardCompile(value)
    }
}

pub(crate) struct ObjectCodegenPhaseInputs<'a, 'b> {
    pub(crate) cache_misses: &'a [crate::ObjectCodegenShard],
    pub(crate) rewritten_files: &'a [RewrittenProjectUnit],
    pub(crate) object_cache_paths_by_file: &'a HashMap<PathBuf, ObjectCachePaths>,
    pub(crate) object_build_fingerprint: &'a str,
    pub(crate) link: &'a LinkConfig<'b>,
    pub(crate) rewritten_file_indices: &'a HashMap<PathBuf, usize>,
    pub(crate) codegen_reference_metadata: &'a HashMap<PathBuf, CodegenReferenceMetadata>,
    pub(crate) precomputed_dependency_closures:
        &'a crate::dependency::PrecomputedDependencyClosures,
    pub(crate) entry_namespace: &'a str,
    pub(crate) project_symbol_lookup: &'a ProjectSymbolLookup,
    pub(crate) global_maps: GlobalSymbolMaps<'a>,
    pub(crate) object_candidate_count: usize,
    pub(crate) object_cache_hits: usize,
    pub(crate) object_shard_size: usize,
    pub(crate) object_shard_threshold: usize,
}

pub(crate) fn run_object_codegen_phase(
    build_timings: &mut BuildTimings,
    inputs: ObjectCodegenPhaseInputs<'_, '_>,
) -> Result<Vec<(usize, PathBuf)>, String> {
    run_object_codegen_phase_impl(build_timings, inputs).map_err(Into::into)
}

fn run_object_codegen_phase_impl(
    build_timings: &mut BuildTimings,
    inputs: ObjectCodegenPhaseInputs<'_, '_>,
) -> Result<Vec<(usize, PathBuf)>, ObjectCodegenPhaseError> {
    let object_codegen_timing_totals = Arc::new(ObjectCodegenTimingTotals::default());
    let declaration_closure_timing_totals = Arc::new(DeclarationClosureTimingTotals::default());
    let object_emit_timing_totals = Arc::new(ObjectEmitTimingTotals::default());

    crate::codegen::core::reset_codegen_phase_timings();
    crate::codegen::util::reset_object_write_timings();

    let compiled_results: Vec<(usize, PathBuf)> =
        build_timings.measure("object codegen", || {
            inputs
                .cache_misses
                .par_iter()
                .map(|shard| {
                    let obj_path = if let Some(cache_paths) = &shard.cache_paths {
                        cache_paths.object_path.clone()
                    } else {
                        let unit = &inputs.rewritten_files[shard.member_indices[0]];
                        inputs
                            .object_cache_paths_by_file
                            .get(&unit.file)
                            .ok_or_else(|| {
                                ObjectCodegenPhaseError::ShardCompile(format!(
                                    "{}: missing object cache paths for rewritten unit '{}'",
                                    cli_error("error"),
                                    format_cli_path(&unit.file)
                                ))
                            })?
                            .object_path
                            .clone()
                    };

                    let declaration_closure_started_at = Instant::now();
                    let mut batch_active_symbols = HashSet::new();
                    let mut batch_declaration_symbols = HashSet::new();
                    let mut batch_closure_files = HashSet::new();
                    for index in &shard.member_indices {
                        let unit = &inputs.rewritten_files[*index];
                        let declaration_closure = declaration_symbols_for_unit(
                            DeclarationClosureRequest {
                                root_file: &unit.file,
                                root_active_symbols: &unit.active_symbols,
                                precomputed_dependency_closures: inputs
                                    .precomputed_dependency_closures,
                                reference_metadata: inputs.codegen_reference_metadata,
                                entry_namespace: inputs.entry_namespace,
                                symbol_lookup: inputs.project_symbol_lookup,
                                timings: Some(declaration_closure_timing_totals.as_ref()),
                            },
                            &inputs.global_maps,
                        );
                        batch_active_symbols.extend(unit.active_symbols.iter().cloned());
                        batch_declaration_symbols.extend(declaration_closure.symbols);
                        batch_closure_files.extend(declaration_closure.files);
                    }
                    object_codegen_timing_totals
                        .declaration_closure_ns
                        .fetch_add(
                            elapsed_nanos_u64(declaration_closure_started_at),
                            Ordering::Relaxed,
                        );

                    let codegen_program_started_at = Instant::now();
                    let codegen_program = if shard
                        .member_indices
                        .iter()
                        .any(|index| inputs.rewritten_files[*index].has_specialization_demand)
                    {
                        combined_program_for_files(inputs.rewritten_files)
                    } else {
                        codegen_program_for_units(
                            inputs.rewritten_files,
                            inputs.rewritten_file_indices,
                            &shard.member_files,
                            Some(&batch_closure_files),
                            Some(&batch_declaration_symbols),
                        )
                    };
                    object_codegen_timing_totals.codegen_program_ns.fetch_add(
                        elapsed_nanos_u64(codegen_program_started_at),
                        Ordering::Relaxed,
                    );

                    let closure_body_symbols_started_at = Instant::now();
                    let mut codegen_active_symbols = batch_active_symbols;
                    let shard_member_files =
                        shard.member_files.iter().cloned().collect::<HashSet<_>>();
                    codegen_active_symbols.extend(closure_body_symbols_for_files(
                        &shard_member_files,
                        &batch_declaration_symbols,
                        inputs.global_maps.function_file_map,
                        inputs.global_maps.class_file_map,
                        inputs.global_maps.module_file_map,
                    ));
                    object_codegen_timing_totals
                        .closure_body_symbols_ns
                        .fetch_add(
                            elapsed_nanos_u64(closure_body_symbols_started_at),
                            Ordering::Relaxed,
                        );

                    let llvm_emit_started_at = Instant::now();
                    crate::compile_program_ast_to_object_filtered(
                        &codegen_program,
                        &shard.member_files[0],
                        &obj_path,
                        inputs.link,
                        &codegen_active_symbols,
                        &batch_declaration_symbols,
                        Some(object_emit_timing_totals.as_ref()),
                    )?;
                    object_codegen_timing_totals
                        .llvm_emit_ns
                        .fetch_add(elapsed_nanos_u64(llvm_emit_started_at), Ordering::Relaxed);

                    let cache_save_started_at = Instant::now();
                    if let Some(cache_paths) = &shard.cache_paths {
                        save_object_shard_cache_meta(
                            cache_paths,
                            &shard.member_fingerprints,
                            inputs.object_build_fingerprint,
                        )?;
                    } else {
                        let unit = &inputs.rewritten_files[shard.member_indices[0]];
                        let cache_paths = inputs
                            .object_cache_paths_by_file
                            .get(&unit.file)
                            .ok_or_else(|| {
                                format!(
                                    "{}: missing object cache paths for rewritten unit '{}'",
                                    cli_error("error"),
                                    format_cli_path(&unit.file)
                                )
                            })?;
                        save_object_cache_meta(
                            cache_paths,
                            &unit.semantic_fingerprint,
                            &unit.rewrite_context_fingerprint,
                            inputs.object_build_fingerprint,
                        )?;
                    }
                    object_codegen_timing_totals
                        .cache_save_ns
                        .fetch_add(elapsed_nanos_u64(cache_save_started_at), Ordering::Relaxed);

                    Ok::<Vec<(usize, PathBuf)>, ObjectCodegenPhaseError>(
                        shard
                            .member_indices
                            .iter()
                            .map(|index| (*index, obj_path.clone()))
                            .collect(),
                    )
                })
                .collect::<Result<Vec<_>, ObjectCodegenPhaseError>>()
                .map(|results| results.into_iter().flatten().collect())
        })?;

    build_timings.record_counts(
        "object codegen",
        &[
            ("candidates", inputs.object_candidate_count),
            ("reused", inputs.object_cache_hits),
            (
                "rebuilt",
                inputs
                    .object_candidate_count
                    .saturating_sub(inputs.object_cache_hits),
            ),
            ("shard_size", inputs.object_shard_size),
            ("shard_threshold", inputs.object_shard_threshold),
            ("rebuilt_shards", inputs.cache_misses.len()),
        ],
    );
    build_timings.record_duration_ns(
        "object codegen/declaration closure",
        object_codegen_timing_totals
            .declaration_closure_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/program projection",
        object_codegen_timing_totals
            .codegen_program_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/closure body symbols",
        object_codegen_timing_totals
            .closure_body_symbols_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/llvm emit",
        object_codegen_timing_totals
            .llvm_emit_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/cache save",
        object_codegen_timing_totals
            .cache_save_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object cache meta/save",
        OBJECT_CACHE_META_TIMING_TOTALS
            .save_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_counts(
        "object cache meta/write",
        &[
            (
                "saves",
                OBJECT_CACHE_META_TIMING_TOTALS
                    .save_count
                    .load(Ordering::Relaxed),
            ),
            (
                "bytes_written",
                OBJECT_CACHE_META_TIMING_TOTALS
                    .bytes_written
                    .load(Ordering::Relaxed) as usize,
            ),
        ],
    );

    build_timings.record_duration_ns(
        "object codegen/emit context create",
        object_emit_timing_totals
            .context_create_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/emit codegen new",
        object_emit_timing_totals
            .codegen_new_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/emit compile filtered",
        object_emit_timing_totals
            .compile_filtered_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/emit object dir setup",
        object_emit_timing_totals
            .object_dir_setup_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/emit write object",
        object_emit_timing_totals
            .write_object_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_counts(
        "object codegen/emit details",
        &[
            (
                "active_symbols",
                object_emit_timing_totals
                    .active_symbol_count
                    .load(Ordering::Relaxed),
            ),
            (
                "decl_symbols",
                object_emit_timing_totals
                    .declaration_symbol_count
                    .load(Ordering::Relaxed),
            ),
            (
                "program_decls",
                object_emit_timing_totals
                    .program_decl_count
                    .load(Ordering::Relaxed),
            ),
        ],
    );

    let codegen_phase_timings = crate::codegen::core::snapshot_codegen_phase_timings();
    build_timings.record_duration_ns(
        "object codegen/core generic class check",
        codegen_phase_timings.program_has_generic_classes_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core specialize classes 1",
        codegen_phase_timings.specialize_generic_classes_initial_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core explicit generic check",
        codegen_phase_timings.program_has_explicit_generic_calls_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core specialize explicit",
        codegen_phase_timings.specialize_explicit_generic_calls_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core specialize classes 2",
        codegen_phase_timings.specialize_generic_classes_final_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core collect spec symbols",
        codegen_phase_timings.collect_generated_spec_symbols_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core specialize active",
        codegen_phase_timings.specialized_active_symbols_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core import aliases",
        codegen_phase_timings.import_alias_collection_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core enum pass",
        codegen_phase_timings.enum_declare_pass_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core enum filters",
        codegen_phase_timings.enum_declare_decl_filter_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core enum work",
        codegen_phase_timings.enum_declare_work_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core decl pass",
        codegen_phase_timings.decl_pass_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core decl filters",
        codegen_phase_timings.decl_pass_decl_filter_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core decl class work",
        codegen_phase_timings.decl_pass_class_work_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core decl fn work",
        codegen_phase_timings.decl_pass_function_work_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core decl module work",
        codegen_phase_timings.decl_pass_module_work_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core body pass",
        codegen_phase_timings.body_pass_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core body filters",
        codegen_phase_timings.body_pass_decl_filter_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core body fn work",
        codegen_phase_timings.body_pass_function_work_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core body fn setup",
        codegen_phase_timings.body_function_setup_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core body fn param alloc",
        codegen_phase_timings.body_function_param_alloc_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core body fn stmt loop",
        codegen_phase_timings.body_function_stmt_loop_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core stmt let",
        codegen_phase_timings.body_stmt_let_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core stmt assign",
        codegen_phase_timings.body_stmt_assign_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core stmt expr",
        codegen_phase_timings.body_stmt_expr_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core stmt return",
        codegen_phase_timings.body_stmt_return_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core body fn implicit return",
        codegen_phase_timings.body_function_implicit_return_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core expr literal",
        codegen_phase_timings.expr_literal_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core expr ident",
        codegen_phase_timings.expr_ident_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core expr binary",
        codegen_phase_timings.expr_binary_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core expr call",
        codegen_phase_timings.expr_call_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core body class work",
        codegen_phase_timings.body_pass_class_work_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/core body module work",
        codegen_phase_timings.body_pass_module_work_ns,
    );
    build_timings.record_counts(
        "object codegen/core counts",
        &[
            ("decls", codegen_phase_timings.total_decls_count),
            ("import_aliases", codegen_phase_timings.import_alias_count),
            ("active_symbols", codegen_phase_timings.active_symbols_count),
            (
                "decl_symbols",
                codegen_phase_timings.declaration_symbols_count,
            ),
            (
                "spec_owners",
                codegen_phase_timings.generated_spec_owners_count,
            ),
            ("declared_enums", codegen_phase_timings.declared_enum_count),
            (
                "declared_classes",
                codegen_phase_timings.declared_class_count,
            ),
            (
                "declared_functions",
                codegen_phase_timings.declared_function_count,
            ),
            (
                "declared_modules",
                codegen_phase_timings.declared_module_count,
            ),
            (
                "compiled_functions",
                codegen_phase_timings.compiled_function_count,
            ),
            (
                "compiled_classes",
                codegen_phase_timings.compiled_class_count,
            ),
            (
                "compiled_modules",
                codegen_phase_timings.compiled_module_count,
            ),
        ],
    );

    build_timings.record_duration_ns(
        "object codegen/decl closure seed",
        declaration_closure_timing_totals
            .closure_seed_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/decl metadata lookup",
        declaration_closure_timing_totals
            .metadata_lookup_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/decl wildcard imports",
        declaration_closure_timing_totals
            .wildcard_imports_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/decl exact imports",
        declaration_closure_timing_totals
            .exact_imports_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/decl qualified refs",
        declaration_closure_timing_totals
            .qualified_refs_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_duration_ns(
        "object codegen/decl reference symbols",
        declaration_closure_timing_totals
            .reference_symbols_ns
            .load(Ordering::Relaxed),
    );
    build_timings.record_counts(
        "object codegen/decl details",
        &[
            (
                "visited_files",
                declaration_closure_timing_totals
                    .visited_file_count
                    .load(Ordering::Relaxed),
            ),
            (
                "wildcard_imports",
                declaration_closure_timing_totals
                    .wildcard_import_count
                    .load(Ordering::Relaxed),
            ),
            (
                "exact_imports",
                declaration_closure_timing_totals
                    .exact_import_count
                    .load(Ordering::Relaxed),
            ),
            (
                "qualified_refs",
                declaration_closure_timing_totals
                    .qualified_ref_count
                    .load(Ordering::Relaxed),
            ),
            (
                "reference_symbols",
                declaration_closure_timing_totals
                    .reference_symbol_count
                    .load(Ordering::Relaxed),
            ),
        ],
    );

    let object_write_timings = crate::codegen::util::snapshot_object_write_timings();
    build_timings.record_duration_ns(
        "object codegen/write object total",
        object_write_timings.emit_object_bytes_ns + object_write_timings.filesystem_write_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object with TM",
        object_write_timings.with_target_machine_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object TM config",
        object_write_timings.target_machine_config_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object TM init",
        object_write_timings.ensure_targets_initialized_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object triple",
        object_write_timings.target_triple_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object host cpu",
        object_write_timings.host_cpu_query_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object opt resolve",
        object_write_timings.opt_level_resolve_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object target from triple",
        object_write_timings.target_from_triple_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object TM create",
        object_write_timings.target_machine_create_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object target setup",
        object_write_timings.target_machine_setup_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object set triple",
        object_write_timings.module_set_triple_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object set layout",
        object_write_timings.module_set_data_layout_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object memory buffer",
        object_write_timings.write_to_memory_buffer_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object to vec",
        object_write_timings.memory_buffer_to_vec_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object direct file emit",
        object_write_timings.direct_write_to_file_ns,
    );
    build_timings.record_duration_ns(
        "object codegen/write object fs write",
        object_write_timings.filesystem_write_ns,
    );
    build_timings.record_counts(
        "object codegen/write object counts",
        &[
            (
                "tm_cache_hits",
                object_write_timings.target_machine_cache_hit_count,
            ),
            (
                "tm_cache_misses",
                object_write_timings.target_machine_cache_miss_count,
            ),
            ("emit_calls", object_write_timings.emit_object_call_count),
            ("write_calls", object_write_timings.write_object_call_count),
        ],
    );

    Ok(compiled_results)
}
