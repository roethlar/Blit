I would keep one version-checked request per daemon and module. The goal is for `rm`, `find`, `du`, and `df` to behave like their GNU counterparts while doing the filesystem work on the daemon’s disk, avoiding a network round trip per entry. The client should preserve the operand’s bytes and spelling. The daemon should resolve it inside the export, perform the operation, and report what happened. For `move`, copying and cleanup must refer to the same source. [Utility plan](</home/michael/dev/gitea_repos/blit_v2/docs/plan/UTILITY_VERBS_LOCAL_PARITY.md:8>)

**Verdict: `acceptable_with_changes`.** The shared operand representation, Unix descriptor-relative deletion, and bounded failure stream with an explicit `RmDone` are worth keeping. In a loopback test, I passed a non-UTF-8 path printed by `find HOST://` directly to `du` and `rm`. Both used the correct file, and its UTF-8 lookalike survived.

### Required changes

1. **High — A remote `move` can report success while leaving the source intact.** I moved a regular file spelled `single.txt/`. Blit copied its contents, exited 0, and left both files in place. The transfer drops the trailing slash, while cleanup preserves it and suppresses the resulting directory error under `force`. Validate the directory requirement before copying and use the same validated source for both remote cleanup routes. [Transfer source conversion](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/session_client.rs:662) · [Move cleanup](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/admin/rm.rs:139)

2. **High — Windows deletion still acts on paths after checking containment.** From source inspection, a concurrent change to an ancestor can redirect attribute clearing or removal outside the checked directory. A locked child can also leave removable siblings behind, with one error attributed to the whole operand. Use handles anchored at the module root for traversal, attribute changes, and deletion. Report each failed entry and count successful removals directly. [Containment check](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/deletion.rs:650) · [Recursive removal](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/deletion.rs:697)

3. **High — Closing the `rm` response stream can leave deletion running.** In a focused test I closed the receiver before the worker started. The worker deleted the queued file and discovered the closure when it tried to send `done`. Check cancellation before each operand and during successful traversal. On the client, a transport failure before response headers also leaves the outcome unknown. Report it as unfinished and reserve `Refused` for a confirmed rejection. [Daemon cancellation argument](/home/michael/dev/gitea_repos/blit_v2/crates/blit-daemon/src/service/admin.rs:204) · [Client error classification](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/admin/rm.rs:96)

4. **Medium — Unix `rm` requires read permission on directories it only needs to traverse.** In a directory with mode `0300`, GNU `rm` removed a known file. Blit returned `Permission denied` and left its file behind. Use search-capable handles for ancestors and request read access when enumerating a directory. [Ancestor open flags](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/deletion.rs:816)

5. **Medium — Some displayed names still identify the wrong operand or cannot be reused.** Blit printed the valid Unix filename `quarter\notes.pst`, then classified that exact remote operand as local. For a name containing byte `0xff`, plain `df` output and an `rm` diagnostic both displayed the spelling of a distinct file containing U+FFFD. Parse the endpoint prefix independently of the filename bytes. Preserve those bytes in path output and use reversible quoting in prompts and diagnostics. [Operand parser](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/admin/operand.rs:117) · [Display conversion](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/admin/operand.rs:168) · [Removal diagnostic](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/rm.rs:139)

6. **Medium — Two `find` predicates return different selections from GNU `find`.** `find HOST:/module/. -maxdepth 0 -name .` omitted the start, while `-name module` selected it. Use the explicitly typed final `.` for that predicate. Separately, `[[:print:]]` selected a filename consisting of U+0378, an unassigned code point that GNU `find` omitted under `C.UTF-8`. Pin character-class membership to the reference data and include unassigned code points in the comparison. [Start-name selection](/home/michael/dev/gitea_repos/blit_v2/crates/blit-daemon/src/service/core.rs:1227) · [Character classes](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/find_name.rs:128) · [Comparison guard](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/find_name.rs:674)

### Verification

Linux formatting and strict workspace Clippy passed. Workspace tests recorded **1,798 passed, 0 failed, 2 ignored**. Windows GNU cross-target Clippy passed. Native Windows and macOS runtime behavior remains unverified.

For the independent guard, I restricted `[:blank:]` to ASCII space and tab. The existing test failed on U+2003 EM SPACE, then passed after I restored the file from the pinned export. I verified the restored source against Git and removed the disposable copy.

```text
base: 4841b2d4ce9dc439fd8e98f5fea5e60d50c8374a
head: 3cd49e40e81900ab5bfe95210e931bf44583d6fa
capability_ok: true
guard_confirmed: true
```