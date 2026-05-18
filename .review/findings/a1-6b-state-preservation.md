# a1-6b-state-preservation: preserve per-pane state across F-key navigation

**Severity**: Medium (follow-up split from `a1-6-screen-router`)
**Status**: Open
**Branch**: `phase5/a1`

## What

a1-6 landed in-app F-key routing but each navigation
re-enters the destination pane's event loop from scratch.
That means:

- **F1**: mDNS discovery task restarts. The first scan can
  block up to 1.5s; the operator sees `scanning...` every
  time they revisit F1 from F2/F3/F4.
- **F2**: Subscribe stream is reopened and GetState is
  re-fired. Roughly 2× control-plane RTT every visit.
- **F3**: Browse path is forgotten. Operator's mid-tree
  position is gone; they restart at the module list.
- **F4**: `perf_local.jsonl` is re-read from disk.

This makes the router feel sluggish even though the
mechanics work. State preservation is the natural follow-up.

## Scope

1. New `AppState` struct holding all four panes' states
   AND their background-channel handles
   (mDNS discovery rx, Subscribe rx, browse fetch rx,
   profile fetch rx).
2. Replace the four `run_fN_event_loop` functions with a
   single `run_app_event_loop` that owns `AppState` and
   selects! across all background channels plus the
   keystroke channel.
3. Per-pane keystroke handlers extracted as
   `handle_f1_key(&mut app, action)` etc.
4. The render dispatch reads `app.current_screen` and
   routes to the appropriate `render_into`.

## Why split out

- a1-6 landed the routing primitive (`LoopOutcome::Navigate`
  + F-key recognition + tab strip) cleanly.
- The state-preservation refactor consolidates four event
  loops into one with a unified select!, which is a
  substantial code movement (hundreds of lines).
- Splitting keeps each PR easy to review: a1-6 is "did the
  F-keys work"; a1-6b is "did the consolidation preserve
  semantics."

## Reviewer comments

(empty — pending implementation)
