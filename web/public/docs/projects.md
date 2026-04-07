# Projects

Project support is documented in detail in:

- [Multi-File Projects](features/projects.md)

Quick summary:

- Arden projects are configured with `arden.toml`.
- Source files are listed explicitly in `files`.
- Project `opt_level` controls final binary optimization (`0/1/2/3/s/z/fast`, default `3`).
- Single-file compile/run defaults to maximum-performance optimization.
- Cross-file usage is validated by the import checker.
- Top-level symbols are deterministically mangled during project build.
