# prose-resume-source: a resume's skip reason was the first message beginning "source:"

**Severity**: LOW — a breach of D-2026-11-24-1, misreporting a reason. A resume whose source cannot open its file skips it with a reason. `ResumeBlockDiff::skip_reason` took the first message in the error's chain whose text began "source:", meaning the file's own failure (size drift, metadata). Any other message beginning so was taken as that reason.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix prose-resume-source`)
**Reviewer**: none yet. Found by the working agent on 2026-11-24, scanning for decisions read from prose under D-2026-11-24-1.

## Evidence
`crates/blit-core/src/remote/transfer/resume_diff.rs` `skip_reason` did `if cause.to_string().starts_with("source:") { return text; }`. `FsTransferSource::open_file` refuses a non-regular file with `"{path} is not a regular file"`, where the path is the source root joined to the file. A relative source root named `source:dir` puts "source:" at the head of that refusal. The reason then became the bare refusal, not "source: cannot open: …".

## What
The reasons the diff makes for the file's own failure carry the type `resume_diff::SourceReason`. There are two:
- "cannot read metadata";
- "changed size during transfer".

`skip_reason` finds a `SourceReason` in the chain by its type. Anything else is reported as an open failure, as before. The messages are unchanged.

## Guard proof
- `resume_diff::tests::a_skip_reason_is_found_by_its_type_not_its_words`: an open refusal whose words begin "source:" is reported as an open failure, and a typed drift reason is reported as itself.
- Mutations, each restored byte for byte:
  - found by the words → red;
  - the typed reason not looked for → red.
- Checks: native clippy is clean, and the resume-diff tests pass.

## Files changed
- `crates/blit-core/src/remote/transfer/resume_diff.rs` — `SourceReason`; `open`'s two reasons; `skip_reason`; guard.

## Known gaps
- None known.
