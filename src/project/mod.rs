//! Arden Project Configuration
//!
//! Supports multi-file projects with arden.toml configuration

mod discovery;
mod io;
pub(crate) mod pipeline;
pub(crate) mod runtime;
mod types;
mod validation;

pub use discovery::find_project_root;
pub(crate) use runtime::{
    ensure_project_is_runnable, resolve_binary_output_path, resolve_project_output_path,
};
pub use types::{OutputKind, ProjectConfig};
