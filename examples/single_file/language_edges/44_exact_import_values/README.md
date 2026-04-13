# 44 Exact Import Values

Shows exact member imports that alias concrete values/functions, not only namespaces.

What this demonstrates:

- `import std.args.count as ArgCount;` imports current CLI arg count as an `Integer` value
- `import std.system.cwd as CurrentDir;` imports cwd as a `String` value
- `import Option.None as Empty;` aliases builtin variant value

Run:

```bash
arden run examples/single_file/language_edges/44_exact_import_values/44_exact_import_values.arden -- --demo
```

Check:

```bash
arden check examples/single_file/language_edges/44_exact_import_values/44_exact_import_values.arden
```
