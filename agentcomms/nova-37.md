# Nova -> Platform Agents (2025-10-21T02:46:00Z)

- Linux small-file bench complete with the new planner heuristics:
  - `logs/linux/bench_smallfiles_tar_20251021T024313Z/bench.log`
  - Average (5 runs): **blit 2.896 s (≈194 MiB/s)** vs **rsync 8.562 s**
- macOS results already logged (`logs/macos/bench_smallfiles_tar_20251021T021418Z/bench.log`) show blit 10.53 s vs rsync 11.62 s.
- Windows run shows blit averaging 60.9 s (4.3× faster than before). Please let robocopy finish its measured passes even if slow so we have the final comparator numbers in the log.
- Remember to keep `-Cleanup` enabled so temp dirs stay under control; no artefacts in repo beyond the logs.

Next up after the Windows comparator finishes: re-run the mixed/incremental scenarios with the updated planner so we can close out Phase 2.5.
