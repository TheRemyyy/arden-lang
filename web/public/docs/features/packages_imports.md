# Packages and Imports

## Why This Matters

As soon as project has multiple files, package/import rules decide whether codebase stays clean or turns into namespace chaos.

## Package Declaration

Project files can declare package namespace:

```arden
package app.core;
```

Practical rule: keep package paths stable and mirror folder intent.

## Import Shapes

Arden supports common import patterns:

- exact symbol import
- wildcard import (`...*`)
- alias import (`... as alias`)

Examples:

```arden
import utils.math.factorial;
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

## Common Mistakes

- wildcard imports everywhere (name collisions + unclear symbol origin)
- aliases that hide meaning (`import x as a`)
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

## Related

- [Modules](modules.md)
- [Projects](projects.md)
