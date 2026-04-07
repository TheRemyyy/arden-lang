# Editor Setup

Arden does not yet ship a polished first-party editor extension, but you can still get a workable setup today.

The main thing is to separate what already exists from what is still manual:

- the compiler exposes `arden lsp`
- `.arden` files can be associated with an existing grammar as a stopgap
- most of the productive workflow still comes from terminal commands such as `arden check`, `arden fmt`, and `arden test`

## Current State

- the compiler exposes `arden lsp`
- basic syntax highlighting can be approximated with existing C-family or Rust grammars
- `.arden` file association is usually enough to make editing much less painful

## VS Code

Temporary file association:

```json
{
  "files.associations": {
    "*.arden": "rust"
  }
}
```

This is only a stopgap, but it gives you:

- comments
- strings
- braces / indentation support
- basic code coloration

## LSP

The CLI exposes:

```bash
arden lsp
```

If you are wiring your own editor integration or experimenting with an LSP client, that is the entrypoint to use.

Practical rule: make sure `arden --help` and `arden run hello.arden` work first. Editor integration is much easier once the compiler itself is confirmed working.

## Useful Terminal Pairing

Right now the best editing experience usually comes from combining a basic editor setup with a nearby terminal:

```bash
arden check
arden fmt
arden test
```

That loop already covers most of what a contributor needs while editor support is still maturing.

## Practical Recommendation

Right now the best experience is usually:

1. file association for `.arden`
2. external terminal running `arden check`, `arden fmt`, and `arden test`
3. optional manual LSP integration if you want to experiment

## If You Are Setting Up A Team Editor Workflow

Prefer the conservative setup first:

- standardize on `.arden` file association
- standardize on formatter/test/check terminal commands
- treat custom LSP integration as optional until it is stable enough for everyone

That keeps the workflow reproducible even if editor-specific integration varies between machines.

## Related Docs

- [Compiler CLI](../compiler/cli.md)
- [Quick Start](quick_start.md)
- [Installation](installation.md)
