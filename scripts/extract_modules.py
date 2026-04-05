#!/usr/bin/env python3
"""Extract modules from src/main.rs into separate module files."""

import re
import sys

def read_lines(path):
    with open(path, 'r') as f:
        return f.readlines()

def write_file(path, content):
    with open(path, 'w') as f:
        f.write(content)

def add_pub_crate(lines):
    """Add pub(crate) to top-level items that don't already have visibility."""
    result = []
    for line in lines:
        # Only modify lines with no leading whitespace
        if line and not line[0].isspace():
            stripped = line.rstrip('\n')
            for prefix in ('fn ', 'struct ', 'enum ', 'type ', 'const ', 'static '):
                if stripped.startswith(prefix):
                    line = 'pub(crate) ' + line
                    break
        result.append(line)
    return result

def extract(all_lines, start, end):
    """Extract lines start..=end (1-indexed, inclusive)."""
    return all_lines[start - 1:end]

def main():
    src = 'src/main.rs'
    all_lines = read_lines(src)
    total = len(all_lines)
    print(f"Total lines in main.rs: {total}")

    # Extract ranges (1-indexed, inclusive)
    cache_p1  = extract(all_lines, 280, 1007)
    spec      = extract(all_lines, 1008, 1748)
    symlookup = extract(all_lines, 1749, 2419)
    cache_p2  = extract(all_lines, 2420, 2778)
    dep       = extract(all_lines, 2779, 4302)
    diag      = extract(all_lines, 4303, 4321)
    cache_p3  = extract(all_lines, 4322, 5138)
    linker    = extract(all_lines, 8949, 9346)

    # --- cache/mod.rs ---
    cache_header = """\
use colored::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use std::time::UNIX_EPOCH;
use twox_hash::XxHash64;
use crate::ast::{ImportDecl, Program};
use crate::formatter;
use crate::project::ProjectConfig;
use crate::typeck::{ClassMethodEffectsSummary, FunctionEffectsSummary};
"""
    cache_lines = add_pub_crate(cache_p1 + cache_p2 + cache_p3)
    write_file('src/cache/mod.rs', cache_header + ''.join(cache_lines))
    print("Wrote src/cache/mod.rs")

    # --- specialization/mod.rs ---
    spec_header = """\
use std::collections::HashSet;
use std::path::PathBuf;
use crate::ast::{Block, Decl, Expr, ImportDecl, Pattern, Program, Spanned, Stmt, Type};
use crate::cache::*;
use crate::parser::parse_type_source;
use crate::formatter;
"""
    spec_lines = add_pub_crate(spec)
    write_file('src/specialization/mod.rs', spec_header + ''.join(spec_lines))
    print("Wrote src/specialization/mod.rs")

    # --- symbol_lookup/mod.rs ---
    sym_header = """\
use colored::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::ast::{ImportDecl, Program};
use crate::cache::*;
use crate::dependency::*;
use crate::specialization::*;
"""
    sym_lines = add_pub_crate(symlookup)
    write_file('src/symbol_lookup/mod.rs', sym_header + ''.join(sym_lines))
    print("Wrote src/symbol_lookup/mod.rs")

    # --- dependency/mod.rs ---
    dep_header = """\
use colored::*;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use crate::ast::{ImportDecl, Program};
use crate::cache::*;
use crate::typeck::{ClassMethodEffectsSummary, FunctionEffectsSummary, TypeChecker};
"""
    dep_lines = add_pub_crate(dep)
    write_file('src/dependency/mod.rs', dep_header + ''.join(dep_lines))
    print("Wrote src/dependency/mod.rs")

    # --- diagnostics/mod.rs ---
    diag_header = """\
use crate::typeck;
use crate::borrowck;
"""
    diag_lines = add_pub_crate(diag)
    write_file('src/diagnostics/mod.rs', diag_header + ''.join(diag_lines))
    print("Wrote src/diagnostics/mod.rs")

    # --- linker/mod.rs ---
    linker_header = """\
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::cache::*;
use crate::project::OutputKind;
"""
    linker_lines = add_pub_crate(linker)
    write_file('src/linker/mod.rs', linker_header + ''.join(linker_lines))
    print("Wrote src/linker/mod.rs")

    # --- Rewrite main.rs ---
    # Keep: lines 1..=279, then module decls, then lines 5139..=8948, then lines 9347..=10673
    keep_head   = all_lines[0:279]          # lines 1-279
    keep_middle = all_lines[5138:8948]      # lines 5139-8948
    keep_tail   = all_lines[9346:total]     # lines 9347-end

    module_decls = """\

mod cache;
mod dependency;
mod symbol_lookup;
mod specialization;
mod diagnostics;
mod linker;

use crate::cache::*;
use crate::dependency::*;
use crate::symbol_lookup::*;
use crate::specialization::*;
use crate::diagnostics::*;
use crate::linker::*;

"""

    new_main = ''.join(keep_head) + module_decls + ''.join(keep_middle) + ''.join(keep_tail)
    write_file('src/main.rs', new_main)
    print("Wrote src/main.rs")
    print(f"New main.rs lines: {new_main.count(chr(10))}")

if __name__ == '__main__':
    main()
