# 45 Interface Inline Body Rules

Demonstrates current behavior for interface methods with inline bodies.

Current rule:

- interface methods may include inline bodies
- implementing classes still must explicitly implement required methods

So treat inline bodies in interfaces as contract-level declarations, not auto-inherited class defaults.

Run:

```bash
arden run examples/single_file/language_edges/45_interface_inline_body_rules/45_interface_inline_body_rules.arden
```

Check:

```bash
arden check examples/single_file/language_edges/45_interface_inline_body_rules/45_interface_inline_body_rules.arden
```
