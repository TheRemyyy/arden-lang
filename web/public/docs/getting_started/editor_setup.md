# Editor Setup

## Why This Matters

Good editor integration shortens feedback loops from minutes to seconds.

## Core Setup

Run Arden language server:

```bash
arden lsp
```

Use an editor/client that can connect to a stdio language server.

## Recommended Daily Loop

```bash
arden check
arden test
arden fmt
arden lint
```

## Project Root Rule

Open the project at directory containing `arden.toml` when in project mode.
That keeps diagnostics/import resolution consistent with CLI behavior.

## Practical LSP Sanity Check

If editor diagnostics look suspicious, compare with terminal truth:

```bash
arden check
```

If terminal and editor disagree, LSP wiring/workspace root is usually the cause.

## Troubleshooting

- no completions/diagnostics: ensure editor actually starts `arden lsp`
- stale diagnostics: restart LSP process and rerun `arden check`
- wrong imports in diagnostics: verify opened workspace root
- formatting mismatch: run `arden fmt` and compare with editor formatter output

## Related

- [Installation](installation.md)
- [CLI Reference](../compiler/cli.md)
