# cr-fix2-1: DECLINED — contract bump for ManifestComplete.scan_failures

Reviewer (codex, fix batch 2 range 44200834..be0b9fda) predicted that a pre-change contract-7 receiver in a rolling upgrade would ignore `ManifestComplete.scan_failures` and let a retry clear a failure it never observed.

Declined: no released build carries contract 7 (v0.1.3, the last release, is contract 6), so no such peer exists; the plan's Constraints rule this case explicitly — every wire change before the next release lands under 7, and the contract bumps again only if a release ships between slices. Two unreleased builds of different commits interoperating is not a supported mode (D-2026-07-05-2: no version compatibility obligation; D-2026-08-18-2: protocol number is the key). Recorded here so the release that ships contract 7 is the first with these fields; if a release were cut between now and the next wire change, the plan already requires a bump.
