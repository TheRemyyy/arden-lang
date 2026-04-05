#!/usr/bin/env python3
"""Fix visibility and circular dependency issues in extracted modules."""

import re
import sys

def read_lines(path):
    with open(path, 'r') as f:
        return f.readlines()

def write_file(path, content):
    with open(path, 'w') as f:
        f.write(content)

def add_visibility_to_module(lines):
    """
    Add pub(crate) to:
    - Top-level fn/struct/enum/type/const/static (no leading whitespace, not already pub)
    - Struct fields (4-space indent, identifier: Type pattern, inside a struct block)
    - Impl methods (4-space indent, fn declarations, inside an impl block)
    """
    result = []
    brace_depth = 0
    context_stack = []  # stack of ('struct'|'impl'|'fn'|'other', brace_depth_when_entered)
    current_context = None  # 'struct' or 'impl' or 'fn' or 'other'

    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.rstrip('\n')
        content = stripped.lstrip()
        indent = len(stripped) - len(content)

        # Count braces in this line to track depth
        open_count = stripped.count('{')
        close_count = stripped.count('}')

        # Before processing braces, decide what to do with this line
        new_line = line

        if indent == 0 and content:
            # Top-level line
            is_pub = content.startswith('pub ')
            is_attr = content.startswith('#[')
            is_doc = content.startswith('///')

            if not is_pub and not is_attr and not is_doc:
                for prefix in ('fn ', 'struct ', 'enum ', 'type ', 'const ', 'static '):
                    if content.startswith(prefix):
                        new_line = 'pub(crate) ' + line
                        break

            # Track context for next level
            if open_count > close_count:
                if content.startswith('struct ') or content.startswith('pub(crate) struct ') or \
                   (is_pub and content[4:].lstrip().startswith('struct ')):
                    ctx = 'struct'
                elif content.startswith('impl ') or content.startswith('pub(crate) impl ') or \
                     (is_pub and content[4:].lstrip().startswith('impl ')):
                    ctx = 'impl'
                elif content.startswith('enum ') or content.startswith('pub(crate) enum ') or \
                     (is_pub and content[4:].lstrip().startswith('enum ')):
                    ctx = 'enum'
                else:
                    ctx = 'other'
                context_stack.append((ctx, brace_depth))

        elif indent == 4 and content:
            # Possibly struct field or impl method
            top_ctx = context_stack[-1][0] if context_stack else 'other'
            top_depth = context_stack[-1][1] if context_stack else -1

            if top_ctx == 'struct' and brace_depth == top_depth + 1:
                # Inside a struct body at depth 1 - these are fields
                is_pub = content.startswith('pub ')
                is_attr = content.startswith('#[')
                is_doc = content.startswith('///')
                is_vis = content.startswith('pub(')
                # Field pattern: identifier: type (possibly with trailing comma)
                # Not a function body line (no semicolon at end of `let` etc.)
                if not is_pub and not is_attr and not is_doc and not is_vis:
                    # Check if it looks like a field: `name: type` pattern
                    # Fields don't start with keywords like let, if, for, etc.
                    keywords = ('let ', 'if ', 'for ', 'while ', 'match ', 'return ',
                                'loop ', 'break ', 'continue ', '//', 'fn ')
                    if not any(content.startswith(kw) for kw in keywords):
                        # Use regex to detect field pattern: identifier followed by colon
                        if re.match(r'^[a-zA-Z_][a-zA-Z0-9_]*\s*:', content):
                            new_line = '    pub(crate) ' + line[4:]

            elif top_ctx == 'impl' and brace_depth == top_depth + 1:
                # Inside an impl body at depth 1 - these are methods
                is_pub = content.startswith('pub ')
                is_vis = content.startswith('pub(')
                is_attr = content.startswith('#[')

                if not is_pub and not is_vis and content.startswith('fn '):
                    new_line = '    pub(crate) ' + line[4:]

        # Now handle any nested struct/impl contexts
        if open_count > close_count and indent > 0:
            # We're entering a sub-context
            if indent == 4:
                top_ctx = context_stack[-1][0] if context_stack else 'other'
                if top_ctx == 'impl':
                    context_stack.append(('fn', brace_depth))
                elif top_ctx != 'struct':
                    context_stack.append(('other', brace_depth))

        result.append(new_line)

        # Update brace depth after adding the line
        brace_depth += open_count - close_count

        # Pop contexts that have closed
        while context_stack and brace_depth <= context_stack[-1][1]:
            context_stack.pop()

        i += 1

    return result


def add_visibility_simple(lines):
    """
    Simpler approach: use brace depth tracking to determine context.
    Add pub(crate) to struct fields and impl methods.
    """
    result = []
    depth = 0
    # Stack entries: (context_type, entry_depth)
    # context_type: 'struct', 'impl', 'fn', 'other'
    ctx_stack = []

    def current_ctx():
        return ctx_stack[-1][0] if ctx_stack else 'top'

    def current_entry_depth():
        return ctx_stack[-1][1] if ctx_stack else 0

    for raw_line in lines:
        line_content = raw_line.rstrip('\n')
        stripped = line_content.lstrip()
        indent = len(line_content) - len(stripped)

        new_line = raw_line

        # Determine what to do with this line before counting braces
        ctx = current_ctx()
        entry_depth = current_entry_depth()

        if indent == 0 and stripped:
            # Top-level item
            is_already_pub = stripped.startswith('pub ') or stripped.startswith('pub(')
            is_attr = stripped.startswith('#[')
            is_doc = stripped.startswith('///')
            is_comment = stripped.startswith('//')

            if not is_already_pub and not is_attr and not is_doc and not is_comment:
                for prefix in ('fn ', 'struct ', 'enum ', 'type ', 'const ', 'static '):
                    if stripped.startswith(prefix):
                        new_line = 'pub(crate) ' + raw_line
                        stripped = 'pub(crate) ' + stripped
                        break

        elif indent == 4 and stripped and ctx in ('struct', 'impl'):
            if ctx == 'struct' and depth == entry_depth + 1:
                # Inside struct body - field declaration
                is_already_pub = stripped.startswith('pub ') or stripped.startswith('pub(')
                is_attr = stripped.startswith('#[')
                is_doc = stripped.startswith('///')
                is_comment = stripped.startswith('//')
                keywords = ('let ', 'if ', 'for ', 'while ', 'match ', 'return ',
                            'loop ', 'break ', 'continue ', 'fn ', 'use ')
                is_kw = any(stripped.startswith(kw) for kw in keywords)

                if not is_already_pub and not is_attr and not is_doc and not is_comment and not is_kw:
                    if re.match(r'^[a-zA-Z_][a-zA-Z0-9_]*\s*:', stripped):
                        new_line = '    pub(crate) ' + raw_line[4:]
                        stripped = 'pub(crate) ' + stripped

            elif ctx == 'impl' and depth == entry_depth + 1:
                # Inside impl body - method declaration
                is_already_pub = stripped.startswith('pub ') or stripped.startswith('pub(')
                is_attr = stripped.startswith('#[')

                if not is_already_pub and stripped.startswith('fn '):
                    new_line = '    pub(crate) ' + raw_line[4:]
                    stripped = 'pub(crate) ' + stripped

        result.append(new_line)

        # Count braces to track depth changes
        in_str = False
        in_char = False
        escape = False
        for ch in line_content:
            if escape:
                escape = False
                continue
            if ch == '\\':
                escape = True
                continue
            if in_str:
                if ch == '"':
                    in_str = False
            elif in_char:
                if ch == '\'':
                    in_char = False
            else:
                if ch == '"':
                    in_str = True
                elif ch == '\'':
                    in_char = True
                elif ch == '{':
                    depth += 1
                elif ch == '}':
                    depth -= 1

        # Pop completed contexts
        while ctx_stack and depth <= ctx_stack[-1][1]:
            ctx_stack.pop()

        # Push new context if this line opened a new block
        # We look at the UPDATED depth after processing
        open_count = line_content.count('{')
        close_count = line_content.count('}')
        if open_count > close_count:
            # Determine context based on stripped content (before we modified it)
            orig_stripped = raw_line.rstrip('\n').lstrip()
            entry_d = depth - (open_count - close_count)

            if indent == 0:
                if re.match(r'(pub(\(crate\))?\s+)?struct\s+', orig_stripped):
                    ctx_stack.append(('struct', entry_d))
                elif re.match(r'(pub(\(crate\))?\s+)?impl(\s+|<)', orig_stripped):
                    ctx_stack.append(('impl', entry_d))
                elif re.match(r'(pub(\(crate\))?\s+)?(async\s+)?fn\s+', orig_stripped):
                    ctx_stack.append(('fn', entry_d))
                else:
                    ctx_stack.append(('other', entry_d))
            elif indent == 4 and ctx_stack and ctx_stack[-1][0] == 'impl':
                if re.match(r'(pub(\(crate\))?\s+)?(async\s+)?fn\s+', orig_stripped):
                    ctx_stack.append(('fn', entry_d))
                else:
                    ctx_stack.append(('other', entry_d))
            else:
                ctx_stack.append(('other', entry_d))

    return result


def get_lines_between(lines, start_pattern, end_pattern=None, end_line=None):
    """Extract lines matching a pattern."""
    start = None
    for i, line in enumerate(lines):
        if re.match(start_pattern, line.strip()):
            start = i
            break
    if start is None:
        return None, None
    return start, lines[start:]


def main():
    print("=== Fixing visibility in all module files ===")

    # Fix each module file
    module_files = [
        'src/cache/mod.rs',
        'src/specialization/mod.rs',
        'src/symbol_lookup/mod.rs',
        'src/dependency/mod.rs',
        'src/diagnostics/mod.rs',
        'src/linker/mod.rs',
    ]

    for path in module_files:
        print(f"Processing {path}...")
        lines = read_lines(path)
        fixed = add_visibility_simple(lines)
        write_file(path, ''.join(fixed))
        print(f"  Done ({len(lines)} lines)")

    print("\n=== Moving circular-dependency items from dependency to cache ===")

    # Read dependency module
    dep_lines = read_lines('src/dependency/mod.rs')

    # Find the range of items to move to cache
    # Items: hash_imports, hash_filtered_namespace_map, hash_filtered_global_map,
    #        compute_namespace_api_fingerprints, collect_known_namespace_paths_for_units,
    #        hash_namespace_api_fingerprints, hash_file_api_fingerprint,
    #        RewriteFingerprintContext, DependencyResolutionContext
    # These are at the start of the dependency module (after header)

    # Find start of items to move (first occurrence of hash_imports)
    move_start = None
    for i, line in enumerate(dep_lines):
        if re.match(r'pub\(crate\) fn hash_imports|fn hash_imports', line):
            move_start = i
            break

    if move_start is None:
        print("  WARNING: Could not find hash_imports in dependency module")
    else:
        # Find end of items to move: end of DependencyResolutionContext struct
        # Then the next item 'import_lookup_key' or 'qualified_symbol_path'
        # We need to find the first function that should STAY in dependency
        move_end = None
        # After DependencyResolutionContext (which ends with }) comes import_lookup_key
        # or qualified_symbol_path
        in_dep_resolution = False
        dep_brace_depth = 0
        looking_for_end = False

        # Find the end by looking for the function after DependencyResolutionContext
        # which is import_lookup_key or qualified_symbol_path
        for i in range(move_start, len(dep_lines)):
            line = dep_lines[i]
            stripped = line.strip()

            if re.match(r'(pub\(crate\) )?fn import_lookup_key|'
                        r'(pub\(crate\) )?fn qualified_symbol_path', stripped):
                move_end = i
                break
            # Also stop at resolve_ functions or build_ functions
            if re.match(r'(pub\(crate\) )?fn (resolve_|build_|collect_|namespace_)', stripped):
                # Check if this is NOT one of the items we want to move
                if not re.match(r'(pub\(crate\) )?fn (collect_known_namespace)', stripped):
                    move_end = i
                    break

        if move_end is None:
            print("  WARNING: Could not find end of items to move")
        else:
            items_to_move = dep_lines[move_start:move_end]
            remaining_dep = dep_lines[:move_start] + dep_lines[move_end:]

            print(f"  Moving lines {move_start+1}-{move_end} from dependency to cache")

            # Add items to cache module
            cache_lines = read_lines('src/cache/mod.rs')
            # Find a good insertion point in cache - before the end (or after stable_hasher)
            # Look for the end of the cache module content

            # Insert before the last function or at end
            # Find the insertion point - we want to insert the moved items near the end of cache
            # but before any functions that reference them
            # The simplest approach: append to end of cache module
            cache_content = ''.join(cache_lines)
            items_content = ''.join(items_to_move)
            write_file('src/cache/mod.rs', cache_content + '\n' + items_content)
            print(f"  Added {len(items_to_move)} lines to cache/mod.rs")

            # Update dependency module
            write_file('src/dependency/mod.rs', ''.join(remaining_dep))
            print(f"  Removed items from dependency/mod.rs")

    print("\n=== Fixing trailing attribute/comment issues ===")

    # Fix specialization/mod.rs - remove trailing #[allow(clippy::too_many_arguments)]
    spec_lines = read_lines('src/specialization/mod.rs')
    while spec_lines and spec_lines[-1].strip() in ('', '#[allow(clippy::too_many_arguments)]'):
        if spec_lines[-1].strip() == '#[allow(clippy::too_many_arguments)]':
            spec_lines.pop()
            break
        spec_lines.pop()
    # Re-add the newline at end
    write_file('src/specialization/mod.rs', ''.join(spec_lines))
    print("Fixed specialization/mod.rs trailing attribute")

    # Fix linker/mod.rs - remove trailing /// Check a single file
    linker_lines = read_lines('src/linker/mod.rs')
    while linker_lines and linker_lines[-1].strip() in ('', '/// Check a single file'):
        if linker_lines[-1].strip() == '/// Check a single file':
            linker_lines.pop()
            break
        linker_lines.pop()
    write_file('src/linker/mod.rs', ''.join(linker_lines))
    print("Fixed linker/mod.rs trailing doc comment")

    print("\n=== Updating cache module imports ===")
    # The cache module now has items that use Decl, Spanned, Arc, etc.
    # Need to add those imports to cache/mod.rs
    cache_lines = read_lines('src/cache/mod.rs')
    cache_header_end = 0
    for i, line in enumerate(cache_lines):
        if not line.startswith('use ') and line.strip() and not line.startswith('//'):
            cache_header_end = i
            break

    # Check what's missing
    cache_content = ''.join(cache_lines)
    new_imports = []
    if 'use crate::ast::' in cache_content:
        # Replace the existing ast import with a more complete one
        pass

    # Add Arc import for RewriteFingerprintContext
    if 'use std::sync::Arc' not in cache_content:
        new_imports.append('use std::sync::Arc;\n')
    if 'Decl' not in cache_content and 'use crate::ast::' in cache_content:
        new_imports.append('use crate::ast::{Decl, Spanned};\n')

    if new_imports:
        # Insert after existing use statements
        for imp in new_imports:
            # Find insertion point
            for i, line in enumerate(cache_lines):
                if line.startswith('use crate::ast::'):
                    # Update the existing import
                    break
        # Insert at top after existing imports
        insert_pos = 0
        for i, line in enumerate(cache_lines):
            if line.startswith('use ') or line.strip() == '':
                insert_pos = i + 1
            elif line.strip():
                break
        cache_lines[insert_pos:insert_pos] = new_imports
        write_file('src/cache/mod.rs', ''.join(cache_lines))
        print(f"  Added {len(new_imports)} new imports to cache/mod.rs")

    print("\nDone! Run cargo check to verify.")


if __name__ == '__main__':
    main()
