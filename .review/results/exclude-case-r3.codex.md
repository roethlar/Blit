I would give each transfer one case-matching rule and use it for both the source walk and the destination’s deletion scope. The operator should know before confirming a mirror whether `*cache` protects `aCache`. The change makes the right core choice: exact matching by default, with `--ignore-case` compiling Unicode case alternatives into the pattern. Matching the original filename bytes preserves the meaning of `?` and bracket classes. Reading pattern files once and saving their contents also makes jobs reproducible.

**Verdict** `acceptable_with_changes`

1. **High — Version the delegated request.** I ran the head CLI against two daemons built from the base. A direct transfer refused the version mismatch. A delegated `mirror --ignore-case --exclude '*cache'` exited 0, copied `aCache/source.txt`, and deleted `bCACHE/saved.txt`. The two daemons negotiated contract 8 with each other, and the CLI still sent delegated spec version 3. Bump that spec version so an older destination refuses the request before starting work. [operation_spec.rs:49](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/operation_spec.rs:49)

2. **High — Reject unsupported `move` filters before saving a job.** `move -y --exclude-from skip --save sample src/ dst/` reported an error but saved `sample`. Running `jobs run sample` then moved the entire tree, including the file named by the exclusion. Reject both pattern-file options before creating any replayable record. [main.rs:264](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/main.rs:264)

3. **High — Give includes a portable wire spelling.** A Windows include of `docs\report.txt` names a nested file. The wire preserves the backslash verbatim. I verified that the Linux receiver with `--ignore-case` rejected `DOCS/REPORT.TXT` and accepted `DOCSREPORT.TXT`. That puts an unrelated destination-only file in deletion scope. Encode includes in the initiating platform’s dialect before transmission, as the code already does for excludes. [filter.rs:180](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:180)

4. **Medium — Refresh compiled rules on revalidation.** Through the public `FileFilter` API, I validated an exclusion of `old`, replaced it with `*.cfg`, and validated again before the first match. The filter let `a.cfg` through. A fresh clone excluded it. Revalidation must install matchers for the current patterns and case mode. [fs_enum.rs:922](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/fs_enum.rs:922)

5. **Medium — Derive the warning from the effective matching rules.** `*[a-z]` and `*ß` produced no case warning, although exact matching deleted `TEST` and `CAPẞ` respectively and `--ignore-case` preserved them. Conversely, `copy` described a `*.cfg` exclusion while `--files-from` caused the listed `picked.cfg` to be copied. Use the matcher’s case variants and class rules to choose examples, suppress the note when the literal list overrides the patterns, and escape line breaks so a typed multiline pattern produces one line. [case_fold.rs:90](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:90) · [transfers/mod.rs:209](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/mod.rs:209)

**Verification.** Linux formatting and native Clippy passed. Windows GNU cross-target Clippy passed. The Linux workspace suite passed on rerun with 1,662 tests passed and two ignored. I reproduced one **medium-severity test flake** twice with a fresh configuration: the new replay test read the shared machine-ID file before another process finished writing it. Give each prompt test its own configuration directory. Windows and macOS runtime behavior remains unverified. [case_note.rs:38](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/tests/case_note.rs:38)

I read `AGENTS.md` and resolved the requested head SHA. For the guard, forcing the decoded peer filter to exact matching made `a_peers_case_mode_reaches_the_filter` fail. Restoring the pinned file made the same test pass. I removed the disposable copy.

```text
base: 3bc3ee47612a8d485bafa18101946882b02e85a0
head: 467773b43cd14a504f86b0b858d71f0121480d75
capability_ok: true
guard_confirmed: true
```