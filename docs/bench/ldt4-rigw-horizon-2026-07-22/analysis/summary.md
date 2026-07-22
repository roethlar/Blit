# ldt-4 rig-W admission-horizon controller supplement

Status: **REVIEW_REQUIRED**

Validated 4 horizon arms in two physical byte directions.
Parent fixed-matrix evidence: `docs/bench/ldt4-rigw-2026-07-21` (713cb4624e6f64a3863b67101fb9a3f3df288306d3e6f418c19501428711990b).
Predecessor sustained evidence: `docs/bench/ldt4-rigw-sustained-2026-07-22` (17348aaa261b936e04c104553d7b5c4bbcf008968306a29c4dea922535110eef).
Every arm must accept an ADD above the four-stream floor; accepted transition sequences must match within each initiator-layout pair.
Reason-only trailing sample differences are exported separately and do not override matching accepted membership transitions.

| Cell | source-init ms | destination-init ms | source operations | destination operations | verdict |
|---|---:|---:|---|---|---|
| q_to_windows_horizon | 47661 | 47740 | `[[1,"REMOVE",3],[2,"REMOVE",2],[3,"REMOVE",1]]` | `[[1,"REMOVE",3],[2,"REMOVE",2],[3,"REMOVE",1]]` | TRANSITIONS_MATCH |
| windows_to_q_horizon | 34710 | 45282 | `[[1,"REMOVE",3],[2,"REMOVE",2],[3,"REMOVE",1]]` | `[[1,"ADD",5],[2,"ADD",6],[3,"ADD",7],[4,"ADD",8],[5,"ADD",9],[6,"ADD",10]]` | REVIEW_REQUIRED |
