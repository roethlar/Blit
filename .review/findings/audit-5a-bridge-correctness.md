# audit-5a-bridge-correctness: one-shot scrape timeout + `\r` label escaping

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `f6d2d2d`
**Parent finding**: `audit-5-bridge-robustness` (part 1 of 2). audit-5a is
the two self-contained correctness fixes; audit-5b will cover the
HTTP-server hardening (graceful shutdown, connection-concurrency
semaphore, response write timeout, SO_REUSEADDR).

## What

Two correctness fixes from audit-5 in `blit-prometheus-bridge`:

1. **One-shot scrape timeout** (`main.rs`). The one-shot path (`--remote`
   without `--listen`) called `jobs::query(...).await` with no timeout,
   inheriting the OS TCP connect timeout (60-127s) against a dead host —
   so `blit-prometheus-bridge --remote dead:9031` hung for minutes,
   problematic for cron / node_exporter textfile-collector usage. The
   server path already bounds the identical call with `SCRAPE_TIMEOUT`
   (8s); the one-shot path didn't.

2. **`\r` label escaping** (`metrics.rs`). `escape_label` escaped `\`,
   `"`, `\n` but not `\r`, producing non-compliant exposition output for
   any label value containing a carriage return. Latent today (current
   labels — e.g. version from `CARGO_PKG_VERSION` — can't contain CR) but
   a correctness gap as labels grow.

## Approach

1. Route the one-shot query through a new generic, testable helper
   `query_within(query: F, timeout) -> Result<DaemonState>` that wraps
   the future in `tokio::time::timeout` and maps elapsed → error (caller
   adds endpoint context). `ONESHOT_TIMEOUT = 8s`, matching the server's
   `SCRAPE_TIMEOUT`. **Semantics differ deliberately from the server
   path**: the server emits `down_metrics` on timeout (a scrape always
   "succeeds" with a down indicator — correct for a live exporter); the
   one-shot path keeps **fail-loudly** semantics (timeout → hard error,
   non-zero exit), which a cron wrapper can detect. Generic over the
   future so it's unit-testable with `std::future::pending()` — the same
   approach `server::scrape_body` uses.

2. Add `'\r' => "\\r"` to `escape_label` + doc note.

## Files changed

- `crates/blit-prometheus-bridge/src/main.rs`: `ONESHOT_TIMEOUT`,
  `query_within` helper, route the one-shot path through it; tests.
- `crates/blit-prometheus-bridge/src/metrics.rs`: `\r` escape + doc; test.

## Tests

`blit-prometheus-bridge` 15 (was 12; +3):

- `escape_label_escapes_all_metacharacters` — `\`, `"`, `\n`, `\r`, and
  `\r\n`.
- `query_within_times_out_on_a_stalled_query` — a `pending()` future
  under a short timeout surfaces a "timed out" error (deterministic: a
  never-resolving future can only fire the timeout).
- `query_within_passes_through_a_prompt_result` — `Ok` passes through and
  a prompt `Err` propagates (timeout wrapper doesn't swallow errors).

## Scope / next

audit-5b: the HTTP-server hardening from audit-5 — graceful
`ctrl_c` shutdown, a `Semaphore` bound on concurrent scrape handlers, a
response-write timeout, and `SO_REUSEADDR` on the listener. Larger and
server-loop-shaped, so it ships separately.

## Reviewer comments

(empty — pending review)
