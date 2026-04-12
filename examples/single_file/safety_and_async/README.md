# Safety and Async Examples

Ownership, borrowing, errors, async, and pattern matching in practical flows.

Recommended order:

1. `10_ownership`
2. `13_error_handling`
3. `14_async`
4. `16_pattern_matching`
5. `40_borrow_scope_recovery`

Fast validation:

```bash
for f in examples/single_file/safety_and_async/*/*.arden; do arden check "$f"; done
```
