# ldt-4 rig-W sustained controller supplement

Status: **REVIEW_REQUIRED**

Validated 4 sustained arms in two physical byte directions.
Parent fixed-matrix evidence: `docs/bench/ldt4-rigw-2026-07-21` (713cb4624e6f64a3863b67101fb9a3f3df288306d3e6f418c19501428711990b).
Every arm must accept an ADD above the four-stream floor; accepted transition sequences must match within each initiator-layout pair.
Reason-only trailing sample differences are exported separately and do not override matching accepted membership transitions.

| Cell | source-init ms | destination-init ms | source operations | destination operations | verdict |
|---|---:|---:|---|---|---|
| q_to_windows_sustained | 6412 | 4375 | `[]` | `[]` | TRANSITIONS_MATCH |
| windows_to_q_sustained | 20670 | 4616 | `[]` | `[]` | TRANSITIONS_MATCH |
