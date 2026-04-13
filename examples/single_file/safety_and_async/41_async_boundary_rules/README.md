# 41 Async Boundary Rules

Demonstrates current async boundary constraint: async functions/blocks must not carry borrowed-reference-containing values across the async boundary.

What this shows:

- synchronous borrowed read is valid (`inspect_now(&msg)`)
- async call uses owned value move (`background_len(msg)`)
- commented invalid patterns document rejected forms (`async fn` parameter `&T`, async block capturing `&T`)

Run:

```bash
arden run examples/single_file/safety_and_async/41_async_boundary_rules/41_async_boundary_rules.arden
```

Check:

```bash
arden check examples/single_file/safety_and_async/41_async_boundary_rules/41_async_boundary_rules.arden
```
