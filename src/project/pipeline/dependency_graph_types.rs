use crate::cache::DependencyGraphCache;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub(crate) struct DependencyGraphOutputs {
    pub(crate) previous_dependency_graph: Option<DependencyGraphCache>,
    pub(crate) file_dependency_graph: HashMap<PathBuf, HashSet<PathBuf>>,
    pub(crate) reverse_file_dependency_graph: HashMap<PathBuf, HashSet<PathBuf>>,
    pub(crate) current_dependency_graph_cache: DependencyGraphCache,
}
