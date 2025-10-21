# macGPT Suggestion — Small-file perf gaps

- The 100 k × 4 KiB bench still tilts toward rsync (blit avg 12.3 s vs rsync 11.2 s). Pretty sure we’re paying per-file overhead rather than payload costs.
- Ideas to claw it back:
  1. **Small-file batching on the control plane** — let `mirror` ship manifests/files in bundles so we’re not doing a gRPC send/ack for every 4 KiB chunk.
  2. **Reuse enumerator metadata** — skip redundant `stat` calls; only touch the filesystem when attributes truly differ.
  3. **Adaptive worker cap** — drop the worker count for sub-page workloads (e.g., average file < 8 KiB ⇒ limit to 2–4 workers) to reduce APFS thrash.
  4. **Kick deletes early** — stream delete ops as soon as we detect them instead of waiting for the summary, so empty mutation passes don’t spin in the transfer loop.
- Happy to prototype the worker cap heuristic or the batching once we settle on the plan.
