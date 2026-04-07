//! Arden Code Generator - LLVM IR generation
//!
//! Module structure:
//! - core: Core codegen struct and main compilation logic
//! - types: Built-in type implementations (Option, Result, List, Map, etc.)
//! - util: Utility functions and external declarations

pub mod core;
pub mod types;
pub mod util;

pub use core::Codegen;
