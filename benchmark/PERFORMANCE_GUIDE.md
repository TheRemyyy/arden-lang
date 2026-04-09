# Arden Performance Measurement Guide

> **This guide has been merged into [benchmark/README.md](README.md).**
>
> All documentation for running benchmarks, choosing presets, understanding output files,
> using `--timings` and `--capture-profile`, methodology caveats, and debugging is now in
> `benchmark/README.md` — the single source of truth for the benchmark system.

## Quick Links

- [Choose Your Workflow](README.md#choose-your-workflow) — command map table (smoke / quick / full / article-grade)
- [Entrypoints](README.md#entrypoints) — `run.py` vs `full_campaign.py`
- [Benchmark Groups](README.md#benchmark-groups) — runtime / compile / incremental
- [Arden-Specific Instrumentation](README.md#arden-specific-instrumentation) — `--arden-timings`, `--capture-profile`
- [Output Files](README.md#output-files) — `latest.*` vs `campaign_<ts>/`
- [Publication-Grade Run](README.md#publication-grade-run)
- [Methodology Caveats](README.md#methodology-caveats)
