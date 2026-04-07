# Editor Setup

While official plugins are currently in development, you can still have a good development experience.

## VS Code

We recommend using the standard **Rust** or general **C/C++** extensions for basic syntax highlighting if you configure file associations, as the syntax is C-family.

Alternatively, extensions that support generic **TextMate** grammars can be customized for Arden.

### Recommended Settings

Add this to your `settings.json` to associate `.arden` files with Rust syntax highlighting as a temporary measure (due to similarity):

```json
"files.associations": {
    "*.arden": "rust"
}
```

*Note: This is an approximation. Keywords like `function` vs `fn` will differ, but it provides basic highlighting.*
