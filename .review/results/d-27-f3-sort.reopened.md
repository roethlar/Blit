# d-27-f3-sort reopened

Reviewed sha: `95d364bd9aac444e32a6b1fef44a6a19989cdf58`

Validation:
- `cargo fmt --all -- --check`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
- `cargo test --workspace`: passed

## Findings

1. Low — case-insensitive equal keys are still input-order dependent.

   `sort_rows` caches only `(sort_priority(&row.kind), row.name.to_lowercase())`. For names that differ only by case, such as `Foo` and `foo`, the sort keys are equal. Because `sort_by_cached_key` is stable, the output preserves whatever order came from the upstream fetch. That leaves the headline module-list problem partially open: rows sourced from a daemon `HashMap` can still appear in different orders across reconnects for case variants, despite `apply_modules_sort_is_deterministic_regardless_of_input_order` claiming input-order independence.

   Relevant lines:
   - `crates/blit-tui/src/browse.rs:528` — sort key omits a deterministic tie-breaker after the lowercase name.
   - `crates/blit-tui/src/browse.rs:1194` — deterministic-regardless-of-input-order test only covers distinct lowercase keys.

   Please add a deterministic tie-breaker, for example `(priority, lowercase_name, original_name)` or an equivalent comparator, and pin it with a case-variant regression test for both reversed input orders.
