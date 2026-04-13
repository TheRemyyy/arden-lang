# Language Edges Examples

Behavior that matters for larger projects and API boundaries.

Includes:

- visibility enforcement
- inheritance via `extends`
- interface contracts
- import aliases
- compound assignment
- exact member imports (function/value aliases)

Run all edge samples:

```bash
for f in examples/single_file/language_edges/*/*.arden; do arden run "$f"; done
```

Spotlight:

- [`44_exact_import_values`](./44_exact_import_values/44_exact_import_values.arden)
