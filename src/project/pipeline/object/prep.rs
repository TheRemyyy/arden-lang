use crate::cache::{
    object_cache_paths, BuildTimings, ObjectCachePaths, ParsedProjectUnit, RewrittenProjectUnit,
};
use crate::dependency::{precompute_all_transitive_dependencies, PrecomputedDependencyClosures};
use crate::symbol_lookup::CodegenReferenceMetadata;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(crate) struct ObjectPrepOutputs {
    pub(crate) rewritten_file_indices: HashMap<PathBuf, usize>,
    pub(crate) object_cache_paths_by_file: HashMap<PathBuf, ObjectCachePaths>,
    pub(crate) codegen_reference_metadata: HashMap<PathBuf, CodegenReferenceMetadata>,
    pub(crate) precomputed_dependency_closures: PrecomputedDependencyClosures,
}

pub(crate) fn run_object_prep_step(
    build_timings: &mut BuildTimings,
    parsed_files: &[ParsedProjectUnit],
    rewritten_files: &[RewrittenProjectUnit],
    project_root: &Path,
    file_dependency_graph: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> ObjectPrepOutputs {
    build_timings.measure_step("object prep", || {
        let rewritten_file_indices: HashMap<PathBuf, usize> = rewritten_files
            .iter()
            .enumerate()
            .map(|(index, unit)| (unit.file.clone(), index))
            .collect();

        let object_cache_paths_by_file: HashMap<PathBuf, ObjectCachePaths> = rewritten_files
            .iter()
            .map(|unit| {
                (
                    unit.file.clone(),
                    object_cache_paths(project_root, &unit.file),
                )
            })
            .collect();

        let codegen_reference_metadata: HashMap<PathBuf, CodegenReferenceMetadata> = parsed_files
            .iter()
            .map(|unit| {
                (
                    unit.file.clone(),
                    CodegenReferenceMetadata {
                        imports: unit.imports.clone(),
                        referenced_symbols: unit.referenced_symbols.clone(),
                        qualified_symbol_refs: unit.qualified_symbol_refs.clone(),
                        api_referenced_symbols: unit.api_referenced_symbols.clone(),
                    },
                )
            })
            .collect();

        let precomputed_dependency_closures =
            precompute_all_transitive_dependencies(file_dependency_graph);

        ObjectPrepOutputs {
            rewritten_file_indices,
            object_cache_paths_by_file,
            codegen_reference_metadata,
            precomputed_dependency_closures,
        }
    })
}
