I would keep block-level resume and carry one verified destination handle from acquisition through completion. The goal is to reuse matching bytes, protect the source’s permissions, and preserve the previous destination when acquisition fails. When reuse is unsafe, the receiver should fully prepare a private replacement in the resolved destination directory before renaming it into place.

**Verdict — `acceptable_with_changes`.** The short staging name, pre-rename sizing, and release of held files after a data-bearing failure all serve that goal. I would require six corrections.

| Severity | Required change |
|---|---|
| High | **Verify the staged file before replacing the destination.** The wire path renames the file before obtaining its identity. With the repository’s identity fault injected, I observed the original `0123456789` replaced by eleven zero bytes before any payload arrived. Obtain the identity on the staged handle and finish preparation before the rename. [sink.rs:2767](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:2767) |
| High | **Propagate early acquisition errors.** On Unix, with no recorded hash baseline, failure of the first metadata read becomes an empty handle. Completion then reopens the path. Injecting the fault at that first read produced one file reported written and zero failures. Return the acquisition error and require a verified handle for completion. Windows also discards a supplied baseline here, leaving its documented zero-block completion issue open. [sink.rs:2751](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:2751) |
| High | **Check containment at the resolved parent.** When a whole-file fallback encounters a dangling link chain, the containment check can accept the original link’s parent. `fresh_partial` then follows the chain and can create an unfinished file outside the selected tree. Validate and hold the final parent before creating the sibling there. [path_safety.rs:359](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/path_safety.rs:359), [resume.rs:181](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/resume.rs:181) |
| High | **Include ownership in the reuse decision.** A root-run restore of a private source over another user’s `0600` file passes the group-and-other-bit check. That user still owns the inode and can read incoming bytes. Require compatible ownership on both local and wire paths before reusing it. [resume.rs:117](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/resume.rs:117), [transfer_session/mod.rs:6730](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:6730) |
| Medium | **Decode mode presence independently of Windows metadata.** `--drop-windows-metadata` produces a Windows header with zero permissions and no metadata. The receiver interprets it as Unix mode `0000`. A simulated Windows source and Linux receiver transferred all 131,072 bytes of an identical `0644` file. Using the full wire mode’s presence restored the expected zero-byte transfer. [transfer_session/mod.rs:6119](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:6119), [wire_metadata.rs:29](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/wire_metadata.rs:29) |
| Medium | **Send zero-block failures through the sink’s close callback.** When the source failed before sending its first block, the control lane reported the failure directly. In both initiator roles I measured one saved baseline still present at sink destruction. Route this branch through `fail_resume_record` too. [transfer_session/mod.rs:5228](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:5228) |

The range also records an open Windows `find -type … -delete` issue. If a selected link is replaced by a regular file, removal can delete the replacement. Carrying the selected entry type through removal and checking it on native Windows remains pending. [deletion.rs:426](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/deletion.rs:426)

**Verification.** On Linux/Btrfs, formatting and strict workspace Clippy passed. The restored workspace suite passed **1,992 tests, with 2 ignored**. Windows cross-target Clippy passed. Native Windows and macOS behavior were unverified, and GitHub listed no CI run for this head.

For the guard, I changed symlink resolution to follow one link. The two-link test failed because the middle link was replaced. It passed after I restored the exact Git blob. I removed the disposable export.

```text
base: 24d4064f691286b7756adf4872f91e65ed07a537
head: c89b1eb136a6afe84b963c803ea246d3533d8e3d
capability_ok: true
guard_confirmed: true
```