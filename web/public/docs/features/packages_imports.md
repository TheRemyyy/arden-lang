# Packages and Imports

## Why This Matters

As soon as project has multiple files, package/import rules decide whether codebase stays clean or turns into namespace chaos.

## Package Declaration

Project files can declare package namespace:

```arden
package app.core;
```

Practical rule: keep package paths stable and mirror folder intent.

Compiler placement rule:

- `package ...;` must be the first declaration in file (header position)

## First 3 Minutes Mental Model

- single-file code: usually no imports between your own declarations needed
- multi-file project: each file participates in namespace graph via `package` + `import`
- `arden.toml` `files` list is part of import resolution contract

If a symbol exists but import still fails, check project file graph first.

## Import Shapes

Arden supports common import patterns:

- exact symbol import
- exact member import (function/value/variant)
- wildcard import (`...*`)
- alias import (`... as alias`)

Examples:

```arden
import utils.math.factorial;
import std.system.cwd as CurrentDir;
import Option.None as Empty;
import utils.strings.*;
import std.math as math;
```

## Runnable Alias Example

```arden
import std.io.*;
import std.math as math;

function main(): None {
    value: Integer = math.abs(-42);
    println("value={value}");
    return None;
}
```

## Exact Member Imports (Functions and Values)

Arden can import callable symbols and constant-like values directly.
This is useful when you want very explicit dependencies and short call sites.

```arden
import std.io.*;
import std.args.count as ArgCount;
import std.system.cwd as CurrentDir;
import Option.None as Empty;

function main(): None {
    argc: Integer = ArgCount;
    cwd: String = CurrentDir;
    empty: Option<Integer> = Empty;
    println("argc={argc}, cwd_len={cwd.length()}, empty={empty.is_none()}");
    return None;
}
```

Practical interpretation:

- `ArgCount` behaves as imported `Integer` value
- `CurrentDir` behaves as imported `String` value
- `Empty` aliases enum-like `Option.None` variant value

This syntax is intentionally precise and works well in strict code reviews
because imported usage is obvious at declaration site.

## Parser Constraints

- wildcard import cannot be combined with alias
- alias must be a valid identifier
- import path cannot start with `.`
- import path cannot contain empty segment (`..`)

For `package` declarations:

- package path cannot start with `.`
- package path cannot contain empty segment (`..`)
- package path cannot end with `.`

## Project Graph Interaction

Imports are validated against project file graph in `arden.toml`.
If file is not in `files`, import resolution can fail even when path looks correct.

Quick diagnosis checklist:

1. does target file declare expected `package ...;`?
2. is target file listed in `arden.toml` `files`?
3. does import path match package + symbol/module shape?

## Common Mistakes

- wildcard imports everywhere (name collisions + unclear symbol origin)
- aliases that hide meaning (`import x as a`)
- forgetting exact member imports can target values, not only functions
- moving files without updating package/import paths
- forgetting to list new file in project `files`

## Recommended Team Rules

- exact imports by default
- wildcard only in narrow local contexts
- aliases only when they improve clarity (`std.math as math`)
- package path reviews in PR when moving modules

## Examples

- [nested_package_project](../../examples/nested_package_project/README.md)
- [`38_import_aliases`](../../examples/single_file/language_edges/38_import_aliases/38_import_aliases.arden)
- [`44_exact_import_values`](../../examples/single_file/language_edges/44_exact_import_values/44_exact_import_values.arden)

## Related

- [Modules](modules.md)
- [Projects](projects.md)
