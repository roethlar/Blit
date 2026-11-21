I would keep each utility as one request to the daemon that owns the filesystem, carry the operand’s exact bytes and directory requirement through that request, and use the set of entries successfully copied to decide what a move may remove. The goal is to give `find`, `du`, `df`, and `rm` the behavior of their GNU counterparts without paying a network round trip per entry. [Utility parity plan](/home/michael/dev/gitea_repos/blit_v2/docs/plan/UTILITY_VERBS_LOCAL_PARITY.md:8)

**Verdict — `acceptable_with_changes`.** I would keep the Unix search-only directory handles, streamed removal results, and generated `C.UTF-8` character classes. The most consequential remaining defect is a remote move that exits successfully after losing a source entry.

### Required changes

1. **High — tie move cleanup to what arrived.** I moved `incoming/.` containing a regular file and a symlink to it. At the base, cleanup failed and the source link survived. At the head, the move exited 0, removed the source tree, and left only the regular file at the destination. Cleanup issues a fresh recursive `rm`. Remove only entries accounted for at the destination, then remove empty directories. An untransferred entry must survive and make the move report incomplete. [Move cleanup](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/admin/rm.rs:175)

2. **High — version the delegated request.** I copied a regular file spelled `one.txt/` between two daemons. A head CLI with base daemons and a base CLI with head daemons both exited 0 and copied the file. With all three at head, the command refused it and created nothing. The daemon-to-daemon contract advanced to 19, but the CLI-to-daemon specification stayed at 7. Bump the delegated version and guard both mixed combinations. [Delegated specification](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/operation_spec.rs:435)

3. **High — preserve component boundaries on the receiving platform.** On Linux, `find` printed the module-root entries `\notes` and `C:receipt`, then `du` rejected both printed operands as Windows paths. On Windows, source inspection shows the converse risk. The newly accepted `quarter\notes.pst` becomes two native components, so `rm` can target `quarter/notes.pst`. Decode utility paths as `/`-separated components, preserve legal Unix names, and reject components Windows cannot represent before filesystem access. [Operand parser](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/admin/operand.rs:141) · [Native path conversion](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/path_safety.rs:91)

4. **High — finish the acknowledged Windows removal work.** Once a recursive Windows removal starts, disconnecting the client cannot stop it between entries. The path-based attribute pass also precedes `remove_dir_all`, and one locked child can leave later siblings untouched. Use handle-relative traversal with cancellation and failure reporting per entry, and exercise it on Windows. [Windows removal](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/deletion.rs:737)

5. **Medium — retain directory intent at a module root.** With a module backed by one regular file, `copy HOST:/single/. DEST` exited 0 and wrote that file. Preserve the distinction between the mandatory separator in `HOST:/single/` and an explicit final `.`. Validate the latter even when the normalized relative path is empty. [Source encoding](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/path_posix.rs:113) · [Daemon validation](/home/michael/dev/gitea_repos/blit_v2/crates/blit-daemon/src/service/transfer.rs:346)

6. **Medium — distinguish move intent from scan completeness.** A library caller requesting a complete scan for a copy of `link/` receives the move-specific refusal because the daemon interprets `require_complete_scan` as a promise to delete the source. Carry source-removal intent explicitly. [Session resolver](/home/michael/dev/gitea_repos/blit_v2/crates/blit-daemon/src/service/transfer.rs:394)

7. **Medium — preserve filename identity in JSON.** I observed `find --json` return the same path twice for two distinct names, one containing byte `FF` and the other containing the literal replacement character. `rm --json` likewise produced indistinguishable failure paths. Keep the display string and supply an exact byte representation when conversion loses information. [Find output](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/find.rs:154) · [Removal output](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/rm.rs:230)

8. **Medium — make the new tests portable.** The Unix round-trip test unconditionally creates `bad\xff.dat`, which APFS rejects before the backslash case runs. Separate the two fixtures and condition the raw-byte case on filesystem support. The Linux character-class test also calls a glibc-only API, preventing test compilation on musl. Scope that comparison to glibc and retain the portable table tests. [Filename fixture](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/tests/rm.rs:642) · [Locale comparison](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/find_name.rs:633)

### Verification

Formatting, strict workspace Clippy, and the Linux workspace suite passed with **1,824 passed and 2 ignored**. Two build-identity tests initially discovered the enclosing checkout and passed on rerun with Git discovery bounded at the fixture directory. Windows GNU cross-Clippy passed. Runtime evidence is Linux-only. Regenerating the character tables reproduced the pinned file byte for byte.

For the independent guard, I reinstated the old final-dot condition. The daemon’s start-name test failed because it returned `m` for `.`. I restored the file, verified its Git blob against the head, and reran the same test successfully. I removed the disposable copy.

```text
base: eb0cc4d8f9acfe567f5e59c75583497c7bf7db5d
head: dd0a3179bfe20da195c356982f6e3d0f1c3f0f5a
capability_ok: true
guard_confirmed: true
```