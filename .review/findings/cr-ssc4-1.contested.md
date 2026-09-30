# cr-ssc4-1: DECLINED (duplicate) — skipped local mirror file exposes its destination subtree

Reviewer (codex, ssc-4 range b70ef03a..261912bb) reported that mirror deletion keeps a failed path's ancestors but not its descendants.

Declined as a duplicate of cr-ssc1-1 (plan A19), which was admitted from the ssc-1 review and closed AFTER this review's head by `8c1b8dad` + `5d557004` (fix batch ed4bc773..b342d636): `merge_failures` carries the exact failed set and `plan_session_deletions` skips every entry that is or descends from a failed path, with guards on both remote carriers and the local route (the local guard uses a source file that becomes unavailable before apply against a populated destination directory).
