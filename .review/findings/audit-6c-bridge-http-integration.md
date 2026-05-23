# audit-6c-bridge-http-integration: end-to-end HTTP test for the /metrics server

**Severity**: Test Gap
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `02c7a9c`
**Parent finding**: `audit-6-test-gaps` (item 3).

## What

The Prometheus bridge had unit tests for its components (`scrape_body`,
`write_all_within`, `request_target`, `http_response`, `build_listener`,
the read timeout) but no test that drives a real HTTP request/response
over a socket through `handle_conn` (finding item 3).

## Approach (no production change)

Added a loopback integration harness `http_roundtrip(remote_raw, request)`
— binds the bridge listener on `127.0.0.1:0`, accepts one connection and
runs `handle_conn` against it, while a real client `TcpStream` writes the
request and reads the full response — plus two tests:

- `metrics_endpoint_serves_http_200_with_down_metrics_when_daemon_unreachable`:
  the scrape target is `127.0.0.1:1` (ECONNREFUSED → fast fail), so the
  client GET `/metrics` gets `HTTP/1.1 200 OK` with the prometheus
  content-type, a `Content-Length`, and the down-metrics body
  (`blit_daemon_up 0`). Exercises accept → read head → route → scrape →
  format → bounded write end-to-end, deterministically and without
  standing up a daemon.
- `unknown_path_serves_http_404`: GET `/favicon.ico` → `404 Not Found`
  with the hint body (route decided before any scrape).

## Files changed

- `crates/blit-prometheus-bridge/src/server.rs`: `http_roundtrip` helper
  + 2 tests in the existing `tests` module. No production change.

## Scope

One sub-item of audit-6. Remaining: 6a (blit-app inline tests), 6b (TUI
render), 6e (pull-move/push-move). 6d/6f/6g verified.

## Reviewer comments

(empty — pending review)
