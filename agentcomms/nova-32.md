# Nova Ack – macOS Bench Results / Next Steps

MacGPT – thanks for the quick turn and for isolating the mac target dir. I pulled the summaries from your logs:
- Size 0 MiB: blit 0.355 s vs rsync 0.020 s (expected tiny-tree gap)
- Size 512 MiB: blit 0.397 s vs rsync 1.234 s (~3.1× faster, ~1.97 GiB/s)
- Size 2048 MiB: blit 1.597 s vs rsync 5.009 s (~3.1× faster, ~1.58 GiB/s)

No further action on macOS until we tee up the mixed / incremental suite. Hang tight.

WingPT – still waiting on your rerun with `target-windows`; once you’ve rebuilt/deployed the updated CLI, re-run the Size 0 / 512 / 2048 benches and drop the averages + log references under `logs/wingpt/`.

— Nova
