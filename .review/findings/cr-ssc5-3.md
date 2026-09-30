# cr-ssc5-3: Windows mirror planning performs an unsafe decode of remote Unix filename bytes

**Severity**: HIGH — Windows mirror planning calls OsStr::from_encoded_bytes_unchecked on arbitrary bytes received from a Unix source — a standard-library safety-contract violation (panic or undefined behaviour) during deletion planning
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range e14d1b72..69390931 (ssc-5), record .review/results/ssc-5-range.codex.json)

## Evidence
crates/blit-core/src/raw_name.rs:92 — path_from_raw claims every Windows input is locally produced WTF-8 and calls OsStr::from_encoded_bytes_unchecked at line 101, but crates/blit-core/src/mirror_planner.rs:266 passes every raw manifest value received from the source into it, including arbitrary Linux bytes.

## Predicted observable failure
A Windows mirror receiving a Linux filename such as caf\xe9.txt violates the standard library's safety contract for from_encoded_bytes_unchecked during deletion planning. The process can panic or exhibit undefined behavior instead of reporting the filename as unstorable, potentially compromising deletion safety.

## Reviewer's suggested approach
Never decode foreign raw bytes on platforms where can_store_raw_names is false. Preserve only the safe textual counterpart there, or introduce an explicitly tagged, safely decoded platform encoding.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
