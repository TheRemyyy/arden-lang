use crate::cache::{ParsedProjectUnit, ProjectSymbolLookup};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub(crate) struct ParseIndexOutputs {
    pub(crate) parsed_files: Vec<ParsedProjectUnit>,
    pub(crate) global_function_map: HashMap<String, String>,
    pub(crate) global_function_file_map: HashMap<String, PathBuf>,
    pub(crate) global_class_map: HashMap<String, String>,
    pub(crate) global_class_file_map: HashMap<String, PathBuf>,
    pub(crate) global_interface_map: HashMap<String, String>,
    pub(crate) global_interface_file_map: HashMap<String, PathBuf>,
    pub(crate) global_enum_map: HashMap<String, String>,
    pub(crate) global_enum_file_map: HashMap<String, PathBuf>,
    pub(crate) global_module_map: HashMap<String, String>,
    pub(crate) global_module_file_map: HashMap<String, PathBuf>,
    pub(crate) namespace_class_map: HashMap<String, HashSet<String>>,
    pub(crate) namespace_interface_map: HashMap<String, HashSet<String>>,
    pub(crate) namespace_enum_map: HashMap<String, HashSet<String>>,
    pub(crate) namespace_module_map: HashMap<String, HashSet<String>>,
    pub(crate) function_collisions: Vec<(String, String, String)>,
    pub(crate) class_collisions: Vec<(String, String, String)>,
    pub(crate) interface_collisions: Vec<(String, String, String)>,
    pub(crate) enum_collisions: Vec<(String, String, String)>,
    pub(crate) module_collisions: Vec<(String, String, String)>,
    pub(crate) project_symbol_lookup: ProjectSymbolLookup,
    pub(crate) total_module_names: usize,
}
