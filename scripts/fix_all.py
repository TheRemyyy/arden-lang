#!/usr/bin/env python3
"""Comprehensive fix for extracted module files."""
import re, sys

def read_file(p):
    with open(p) as f: return f.read()
def write_file(p, c):
    with open(p, 'w') as f: f.write(c)

def make_pub_crate(content):
    """Add pub(crate) to top-level items, struct fields, and impl methods."""
    lines = content.split('\n')
    result = []
    
    # State machine tracking
    depth = 0
    # context stack: each entry is (type, depth_at_entry)
    # type: 'struct', 'enum', 'impl', 'fn', 'other'
    ctx_stack = []
    
    def cur_ctx():
        return ctx_stack[-1] if ctx_stack else ('top', 0)
    
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        indent = len(line) - len(line.lstrip())
        
        new_line = line
        
        # Get open/close brace counts (rough - ignoring strings)
        n_open = line.count('{')
        n_close = line.count('}')
        
        ctx_type, ctx_depth = cur_ctx()
        
        if stripped:
            already_pub = stripped.startswith('pub ') or stripped.startswith('pub(')
            is_attr = stripped.startswith('#[')
            is_doc = stripped.startswith('///')
            is_comment = stripped.startswith('//')
            is_use = stripped.startswith('use ')
            
            if indent == 0 and not already_pub and not is_attr and not is_doc and not is_comment and not is_use:
                for prefix in ('fn ', 'struct ', 'enum ', 'type ', 'const ', 'static '):
                    if stripped.startswith(prefix):
                        new_line = 'pub(crate) ' + line
                        stripped = 'pub(crate) ' + stripped
                        break
            
            elif indent == 4 and ctx_type == 'struct' and depth == ctx_depth + 1:
                # Inside struct body - field
                if not already_pub and not is_attr and not is_doc and not is_comment:
                    # Field pattern: identifier followed by colon (not => or ::)
                    m = re.match(r'^([a-zA-Z_][a-zA-Z0-9_]*)\s*:', stripped)
                    if m and '::' not in stripped[:stripped.index(':')]:
                        new_line = '    pub(crate) ' + line[4:]
            
            elif indent == 4 and ctx_type == 'impl' and depth == ctx_depth + 1:
                # Inside impl body - method
                if not already_pub and not is_attr and not is_doc and not is_comment:
                    if stripped.startswith('fn ') or stripped.startswith('async fn '):
                        new_line = '    pub(crate) ' + line[4:]
        
        result.append(new_line)
        
        # Update depth
        prev_depth = depth
        depth += n_open - n_close
        
        # Push new context if we opened a block
        if n_open > n_close and stripped:
            # Determine what kind of block this opened
            # Use the original stripped (before pub(crate) addition for context tracking)
            orig = lines[i].strip()
            entry_d = prev_depth  # depth before this line
            
            if indent == 0:
                if re.search(r'\bstruct\b', orig) and '{' in orig and 'fn ' not in orig:
                    ctx_stack.append(('struct', entry_d))
                elif re.search(r'\benum\b', orig) and '{' in orig and 'fn ' not in orig:
                    ctx_stack.append(('enum', entry_d))
                elif re.search(r'\bimpl\b', orig) and '{' in orig:
                    ctx_stack.append(('impl', entry_d))
                else:
                    ctx_stack.append(('other', entry_d))
            elif indent == 4 and ctx_type == 'impl':
                ctx_stack.append(('fn', entry_d))
            else:
                ctx_stack.append(('other', entry_d))
        
        # Pop closed contexts
        while ctx_stack and depth <= ctx_stack[-1][1]:
            ctx_stack.pop()
        
        i += 1
    
    return '\n'.join(result)


def fix_trailing_orphan(content, pattern):
    """Remove trailing lines matching pattern."""
    lines = content.rstrip('\n').split('\n')
    while lines and re.match(pattern, lines[-1].strip()):
        lines.pop()
    return '\n'.join(lines) + '\n'


def main():
    print("Step 1: Apply pub(crate) visibility to all module files")
    modules = [
        'src/cache/mod.rs',
        'src/specialization/mod.rs', 
        'src/symbol_lookup/mod.rs',
        'src/dependency/mod.rs',
        'src/diagnostics/mod.rs',
        'src/linker/mod.rs',
    ]
    for path in modules:
        print(f"  Fixing {path}...")
        content = read_file(path)
        fixed = make_pub_crate(content)
        write_file(path, fixed)
    
    print("Step 2: Fix trailing orphan attribute in specialization/mod.rs")
    content = read_file('src/specialization/mod.rs')
    content = fix_trailing_orphan(content, r'#\[allow\(clippy::too_many_arguments\)\]')
    write_file('src/specialization/mod.rs', content)
    
    print("Step 3: Fix trailing doc comment in linker/mod.rs")
    content = read_file('src/linker/mod.rs')
    content = fix_trailing_orphan(content, r'///.*')
    write_file('src/linker/mod.rs', content)
    
    print("Step 4: Move hash/context items from dependency to cache")
    dep = read_file('src/dependency/mod.rs')
    dep_lines = dep.split('\n')
    
    # Find the range of items to move
    # Start: first line that is hash_imports  
    # End: first line that is qualified_symbol_path OR import_lookup_key (NOT in the move set)
    
    move_start = None
    move_end = None
    
    for i, line in enumerate(dep_lines):
        s = line.strip()
        if move_start is None:
            if re.match(r'(pub\(crate\) )?fn hash_imports\b', s):
                move_start = i
        else:
            # Look for end: function NOT in our move set
            if re.match(r'(pub\(crate\) )?fn (qualified_symbol_path|import_lookup_key|resolve_|build_)\b', s):
                move_end = i
                break
    
    if move_start is not None and move_end is not None:
        items_to_move = dep_lines[move_start:move_end]
        print(f"  Moving {len(items_to_move)} lines from dependency to cache")
        
        # Remove from dependency
        new_dep = dep_lines[:move_start] + dep_lines[move_end:]
        write_file('src/dependency/mod.rs', '\n'.join(new_dep))
        
        # Add to cache module
        cache = read_file('src/cache/mod.rs')
        # Add at the end
        # Make sure Arc is imported in cache
        if 'use std::sync::Arc' not in cache:
            cache = cache.replace('use std::time::UNIX_EPOCH;\n', 
                                  'use std::time::UNIX_EPOCH;\nuse std::sync::Arc;\n')
        write_file('src/cache/mod.rs', cache.rstrip('\n') + '\n\n' + '\n'.join(items_to_move) + '\n')
        print("  Done moving items")
    else:
        print(f"  WARNING: Could not find items to move (start={move_start}, end={move_end})")
    
    print("Step 5: Update cache module AST imports to include Decl, Spanned")
    cache = read_file('src/cache/mod.rs')
    # Check if Decl is needed but not imported
    if 'Decl' not in cache.split('use ')[0] or 'Decl' not in cache[:500]:
        if 'use crate::ast::' in cache:
            # Update existing import to add Decl, Spanned
            old_import = re.search(r'use crate::ast::\{[^}]+\}', cache)
            if old_import:
                old = old_import.group()
                # Parse existing items
                items_str = old[len('use crate::ast::{'):-1]
                items = [x.strip() for x in items_str.split(',')]
                for item in ['Decl', 'Spanned']:
                    if item not in items:
                        items.append(item)
                items.sort()
                new_import = 'use crate::ast::{' + ', '.join(items) + '}'
                cache = cache.replace(old, new_import)
                write_file('src/cache/mod.rs', cache)
                print(f"  Updated cache AST imports: {new_import}")
    
    print("\nAll done! Run cargo check to verify.")


if __name__ == '__main__':
    main()
