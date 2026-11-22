The goal is to make `--resume` recover interrupted copies safely across local and wire transfers. I would resolve the destination once, hold its parent directory, and reuse an existing file only after verifying it. When a replacement is needed, I would prepare it privately and acquire every handle needed for copying, metadata updates, and cleanup before publishing it. Each resume grant should release its state when it completes, fails, or is skipped.

**Verdict `acceptable_with_changes`.** I would keep the new Unix destination resolver. It handles exact-file destinations and directories the user can write and search without listing. The remaining problems are in paths that still create, modify, or publish a file before acquisition is complete, and in failure paths that bypass record cleanup.

| Severity | Required change and evidence |
|---|---|
| High | **Remove the handle duplication after publication.** Injecting `EMFILE` at the clone used for metadata updates left nine zero bytes in place of an eight-byte destination before any source bytes were written. Carry the acquired handle through copying and metadata updates. [sink.rs:3477](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:3477) |
| High | **Use the held parent when a wire destination has disappeared.** A whole-file grant creates the missing file by pathname and checks its identity afterward. An injected identity failure left a visible nine-byte file. A concurrent parent change can also redirect that creation. Prepare the file in the held directory and publish it with exclusive-create semantics. [sink.rs:2847](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:2847) |
| High | **Keep a file-local hash read error from aborting the session.** An injected `EIO` on the second destination block propagated out of hash computation. The grant sender then returns early, abandoning unrelated files. Discard the incomplete hashes and request the whole file when the error is confined to that file. [Hash read](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:6751) · [Grant sender](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:6111) |
| Medium | **Preserve the destination when local acquisition fails.** Refusing the cleanup handle changed a reusable file from `0644` to `0600`. With an absent destination, the same refusal left an empty file. Acquire the handle before changing an existing file’s mode, and stage a new file until acquisition succeeds. [resume.rs:129](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/resume.rs:129) · [resume.rs:184](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/resume.rs:184) |
| Medium | **Apply the unreadable-file fallback locally.** `blit copy --resume` from a `0600` source to an owned `0004` destination in a writable directory exited 2. The wire path already selects a fresh copy for this case. Make that decision locally before requiring read access to the old file. [resume.rs:140](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/resume.rs:140) |
| Medium | **Release the baseline on a source-side skip.** When the source skipped a granted file before its first block, I measured zero open grants and one retained baseline. Route skips on both carriers through sink cleanup. [Control receiver](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:5159) · [Data-plane receiver](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:2390) |

Linux formatting and strict workspace Clippy passed. The workspace suite had **2,010 passed, 0 failed, and 2 ignored**. Windows GNU cross-Clippy passed. Runtime verification covered Linux only.

For the independent guard, I removed the exact-file exception and observed `a_destination_that_is_one_file_takes_a_fresh_partial` fail. I restored the pinned Git blob and observed the same test pass. I removed the disposable copy after verification.

```text
base: 19025839f231d4a95afc9d3c35deda59966a6ec4
head: 05ff50d7303a80bd40438bdea9000cad64ac4d0b
capability_ok: true
guard_confirmed: true
```