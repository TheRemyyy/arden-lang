# Modules

## Why This Matters

Modules create explicit symbol boundaries and prevent a single global namespace mess.
For beginners: module is a named container for related functions/types.

## Minimal Module Example

```arden
import std.io.*;

module MathUtil {
    function twice(x: Integer): Integer {
        return x * 2;
    }
}

function main(): None {
    value: Integer = MathUtil.twice(10);
    println("value={value}");
    return None;
}
```

## Runnable Example With Output

```arden
import std.io.*;

module TextUtil {
    function shout(s: String): String {
        return s + "!";
    }
}

function main(): None {
    println(TextUtil.shout("hello"));
    return None;
}
```

## Packages and Imports (Project-Scale)

In multi-file projects, you often combine modules with package namespaces and imports.

```arden
package app;
import utils.math.factorial;
import utils.strings as str;
```

Import patterns supported in practice:

- exact symbol import
- exact member import (value/function/variant alias)
- wildcard import (`...*`)
- alias import (`... as alias`)

Example:

```arden
import std.system.cwd as CurrentDir;
import std.args.count as ArgCount;
```

## Why Modules Matter In Real Projects

- keep related code together (`auth`, `math`, `parsing`, ...)
- avoid accidental name collisions
- make imports and dependencies explicit

## Common Mistakes

- dumping unrelated functions into one utility module
- deep module nesting without clear payoff
- relying on implicit access instead of explicit imports in multi-file projects

## Decision Rule

If you need namespacing and organization, use a module.
If you also need per-instance state and methods, use a class.

## Related

- [Projects](projects.md)
- [Packages and Imports](packages_imports.md)
- Examples:
  - [`08_modules`](../../examples/single_file/language_core/08_modules/08_modules.arden)
  - [`38_import_aliases`](../../examples/single_file/language_edges/38_import_aliases/38_import_aliases.arden)
  - [`44_exact_import_values`](../../examples/single_file/language_edges/44_exact_import_values/44_exact_import_values.arden)
