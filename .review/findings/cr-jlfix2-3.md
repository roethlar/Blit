# cr-jlfix2-3: the `raw:` label still collides with a UTF-8 name that begins with it

**Severity**: LOW (reviewer: LOW)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jlfix2-r1 over `d06061ba..ff27f035` (record `.review/results/jlfix2-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:1053 — the strengthened test expects both the raw-byte filename and the valid UTF-8 filename beginning with raw: to render as the identical string raw:caf\\xe9.

## Predicted observable failure
jobs watch can count both failures but print indistinguishable path labels, so the user cannot determine which reason belongs to which actual file.

## Reviewer's suggested approach
Use a collision-free representation, such as tagging and escaping both UTF-8 and raw names, or escaping valid names that begin with reserved raw:/utf8: prefixes.

## Intake
Admitted. Labelling only raw names leaves a UTF-8 file literally named `raw:…` reading the same. Fix: an injective rendering — a raw name is `raw:` + its escaped bytes, a UTF-8 name that itself begins with `raw:` or `utf8:` gets `utf8:` in front, every other name shows as itself — so no two files ever read the same.

## What
`job_log::shown_name` is injective:
- a raw-byte name shows as `raw:` + its escaped bytes;
- a UTF-8 name that itself begins with `raw:` or `utf8:` gets `utf8:` in front;
- every other name shows as itself.

Read back, `raw:` means raw bytes follow and `utf8:` means the rest is the name as written. So no two files read the same in the text log or in `jobs watch`'s list, and ordinary names are untouched.

## Guard proof
- `events_read_as_text` pins `utf8:raw:caf\\xe9`, `utf8:utf8:x`, `raw:caf\\xe9` and `plain`.
- `failed_names_are_told_apart_by_identity` shows the label look-alike as `utf8:raw:caf\\xe9`.
- Mutation — drop the `utf8:` prefix — makes the text test fail; restored, green.
