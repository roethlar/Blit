# cr-ssc2-2: DECLINED — public blit-core API changed under 0.1.3

Reviewer (codex, ssc-2 range) predicted downstream compile failures for 0.1.x consumers because `PreparedPayload::TarShard` gained `skipped` and `build_tar_shard` returns `TarShardBuild`.

Declined as a defect: the crate version is set at release time, not per commit; contract 7 already makes this release wire-incompatible by design (D-2026-07-05-2, D-2026-08-18-2), and blit-core's public surface is expected to move before 1.0 (RELEASE_1_0.md). Recorded instead as a release requirement carried by ssc-5: the next release bumps at least the minor version (0.2.0 or 1.0.0) and CHANGELOG Unreleased names the blit-core API changes.
