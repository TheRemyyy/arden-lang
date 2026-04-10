pub(crate) use crate::{
    api_program_fingerprint, build_file_dependency_graph_incremental, build_project,
    build_project_symbol_lookup, build_reverse_dependency_graph, can_reuse_safe_rewrite_cache,
    check_command, check_file, codegen_program_for_unit, compile_file, compile_source,
    component_fingerprint, compute_link_fingerprint, compute_namespace_api_fingerprints,
    compute_rewrite_context_fingerprint_for_unit, dedupe_link_inputs, escape_response_file_arg,
    fix_target, format_targets, lex_file, lint_target, load_cached_fingerprint,
    load_link_manifest_cache, load_object_shard_cache_hit, load_semantic_cached_fingerprint,
    new_project, object_shard_cache_key, object_shard_cache_paths, parse_file, parse_project_unit,
    precompute_all_transitive_dependencies, read_cache_blob, reusable_component_fingerprints,
    run_project, run_tests, save_object_shard_cache_meta, semantic_program_fingerprint,
    should_skip_final_link, show_project_info, transitive_dependencies_from_precomputed,
    transitive_dependents, typecheck_summary_cache_from_state, typecheck_summary_cache_matches,
    DependencyGraphCache, DependencyGraphFileEntry, DependencyResolutionContext, LinkConfig,
    LinkManifestCache, ObjectShardMemberFingerprint, OutputKind, ParsedFileCacheEntry,
    ParsedProjectUnit, RewriteFingerprintContext, RewrittenProjectUnit,
    DEPENDENCY_GRAPH_CACHE_SCHEMA, LINK_MANIFEST_CACHE_SCHEMA,
};
pub(crate) use helpers::{
    assert_frontend_pipeline_ok, cli_test_lock, collect_project_symbol_maps, fingerprint_for,
    make_temp_project_root, normalize_nested_cargo_linker_env, normalize_output, parse_program,
    rewrite_fingerprint_for_test_unit, with_current_dir, write_test_project_config,
};

mod bindgen;
mod cli;
mod cli_output;
mod cli_test_discovery;
mod compile_source;
mod helpers;
mod lexer;
mod project;
mod project_config;
mod typeck_frontend;
