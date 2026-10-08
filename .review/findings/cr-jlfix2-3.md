# cr-jlfix2-3: the `raw:` label still collides with a UTF-8 name that begins with it

**Severity**: LOW (reviewer: LOW)
**Status**: Admitted — open
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
