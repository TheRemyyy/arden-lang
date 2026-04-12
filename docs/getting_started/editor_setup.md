# Editor Setup

## Why This Matters

Good editor integration shortens feedback loops from minutes to seconds.

## Recommended Setup

Use an editor that supports language-server integration and run the Arden LSP:

```bash
arden lsp
```

## Baseline Workflow

- keep `arden check` running frequently
- run `arden fmt` before commits
- use `arden lint` / `arden fix` for static hygiene

## Practical Loop

```bash
arden check
arden test
arden fmt
arden lint
```

## Troubleshooting

If completion/diagnostics do not appear:

- ensure your editor is connected to the LSP process
- ensure workspace root is the project root (`arden.toml` present for project mode)
- run `arden check` in terminal to confirm compiler diagnostics directly
