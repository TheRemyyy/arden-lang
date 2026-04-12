pub(crate) use crate::{
    api_program_fingerprint, build_file_dependency_graph_incremental, build_project,
    build_reverse_dependency_graph, can_reuse_safe_rewrite_cache, check_command, check_file,
    compile_file, compile_source, component_fingerprint, compute_link_fingerprint,
    compute_namespace_api_fingerprints, dedupe_link_inputs, escape_response_file_arg, fix_target,
    format_targets, lex_file, lint_target, load_cached_fingerprint, load_link_manifest_cache,
    load_object_shard_cache_hit, load_semantic_cached_fingerprint, new_project,
    object_shard_cache_key, object_shard_cache_paths, parse_file, parse_project_unit,
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
    assert_frontend_pipeline_ok, build_project_symbol_lookup, cli_test_lock,
    codegen_program_for_unit, collect_project_symbol_maps,
    compute_rewrite_context_fingerprint_for_unit, fingerprint_for, make_temp_project_root,
    normalize_nested_cargo_linker_env, normalize_output, parse_program,
    rewrite_fingerprint_for_test_unit, with_current_dir, write_test_project_config,
    ProjectSymbolLookupMaps, TestExpectErrExt, TestExpectExt,
};
pub(crate) use project_rewrite_dependency_graph::{
    empty_global_interface_file_map, empty_global_interface_map,
};

mod bindgen;
mod borrowck;
mod cli;
mod cli_output;
mod cli_test_discovery;
mod compile_source_assignment_runtime;
mod compile_source_async_borrow_runtime;
mod compile_source_borrowed_mutation_runtime;
mod compile_source_builtin_function_values;
mod compile_source_call_result_runtime;
mod compile_source_codegen_tail;
mod compile_source_conversion_diagnostics;
mod compile_source_direct_chains_runtime;
mod compile_source_entry_namespace_main;
mod compile_source_enum_alias_match_runtime;
mod compile_source_enum_ctor_alias_runtime;
mod compile_source_function_value_alias_runtime;
mod compile_source_function_value_builtin_runtime;
mod compile_source_function_value_callees;
mod compile_source_function_value_diagnostics;
mod compile_source_function_value_shapes;
mod compile_source_function_values_math_system;
mod compile_source_generic_constructor_runtime;
mod compile_source_generic_nested_runtime;
mod compile_source_imported_type_alias_runtime;
mod compile_source_interface_runtime;
mod compile_source_iteration_numeric_edges;
mod compile_source_map_set_runtime;
mod compile_source_member_diagnostics;
mod compile_source_mixed_numeric_runtime;
mod compile_source_mutable_assignment_runtime;
mod compile_source_no_check_builtin_arg_diagnostics;
mod compile_source_no_check_builtin_string_diagnostics;
mod compile_source_no_check_call_shape_diagnostics;
mod compile_source_no_check_codegen_guards;
mod compile_source_no_check_constant_index_diagnostics;
mod compile_source_no_check_constructor_paths;
mod compile_source_no_check_incompatible_types;
mod compile_source_no_check_nominal_types;
mod compile_source_no_check_operator_condition_diagnostics;
mod compile_source_no_check_primary_diagnostics;
mod compile_source_no_check_user_facing_types;
mod compile_source_package_qualified_builtins;
mod compile_source_primary_diagnostics;
mod compile_source_print_runtime;
mod compile_source_receiver_runtime;
mod compile_source_root_alias_patterns;
mod compile_source_runtime_tail_checks;
mod compile_source_stdlib_alias_calls;
mod compile_source_stdlib_alias_values;
mod compile_source_string_conversion_runtime;
mod compile_source_string_interpolation_runtime;
mod compile_source_tagged_static_runtime;
mod compile_source_typed_runtime_identity;
mod compile_source_unicode_system_runtime;
mod compile_source_user_facing_runtime;
mod diagnostics;
mod formatter;
mod helpers;
mod import_check;
mod lexer;
mod linker;
mod lint;
mod lsp;
mod parser;
mod project;
mod project_config;
mod project_diagnostics_imports;
mod project_import_aliases;
mod project_option_none_aliases;
mod project_rewrite_dependency_graph;
mod project_runtime_recovery;
mod project_zero_arg_exact_imports;
mod typeck_frontend;
mod typeck_frontend_alias_builtins;
mod typeck_frontend_fingerprint;
mod typeck_frontend_matches;
mod typeck_frontend_repeated_update;
mod typeck_frontend_source_driven;
mod typeck_frontend_tagged_basics;
