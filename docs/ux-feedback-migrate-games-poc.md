# UX feedback from the `migrate_games.sh` PoC

**Context:** Using `blit copy` as the transfer primitive for a Steam library migration script (`~/dev/Move-SteamGame.ps1/migrate_games.sh` + `.ps1`). The script does exactly two things per selected game — copy one `.acf` file (~1 KB), then copy the game's data directory (~100 GiB). Run on Linux, local-to-local, across different disks.

This document is **UX observations**, not bugs — the two bugs we hit (rsync-semantics, single-file no-op) are documented separately in `docs/bugs/`. Bugs can be fixed in isolation; the items below are patterns that made those bugs harder to notice and that will make future bugs harder to notice if left alone. Ordered roughly by impact.

---

## 1. "Copy complete: 0 files, 0 B" should not print as success

**What happened:** Both bugs we hit presented identical user-visible output:

```
Copy complete: 0 files, 0 B in 88.37µs
• Throughput: 0 B/s | Workers used: 32
```

Exit code 0. No stderr. Indistinguishable from a legitimate up-to-date skip.

**Why it matters:** "Copy complete" is the single most important line of output from a file-transfer tool. When it lies, every downstream user (eyeball-scanning logs, scripts checking `$?`, CI pipelines) is blind. Both of the bugs we fixed would have been caught in seconds if this line had been trustworthy.

**Suggestion — distinguish the three legitimate zero-files cases from the error case:**

| Condition | Suggested line |
|---|---|
| `skip_unchanged` + journal probe said no changes | `Up to date: 0 files changed (journal probe).` |
| `skip_unchanged` + planner compared and found nothing to do | `Up to date: N files examined, 0 changed.` |
| Source is empty directory | `Source is empty: 0 files copied.` |
| **Fast-path yielded no entries at all (unexpected)** | `Warning: no entries enumerated from source.` *(stderr, non-zero exit, or at minimum a visible warning)* |

The fourth case is the one that bit us. It currently looks identical to the first three. `FastPathDecision::NoWork` should carry a reason, and the summary should print it.

This one change catches an entire class of future "silent no-op" bugs before anyone opens a ticket.

---

## 2. "Copy complete" phrasing assumes work was done

When zero bytes moved, "Copy complete" is technically true but rhetorically wrong. Either:

- Split the verb: `Up to date: …` vs `Copy complete: N files, X in T`.
- Or always lead with the numbers: `Copied 0 files (up to date) in 88µs`.

Rsync's `sent 0 bytes  received 35 bytes  1.23 bytes/sec` is ugly but it makes the empty case visually distinct. Blit currently does not.

---

## 3. Throughput / worker-count fields are noise on small transfers

From the test run:

```
Copy complete: 1 files, 12.00 B in 64.93ms
• Throughput: 184.00 B/s | Workers used: 32
```

184 B/s because startup dominated. A user scanning the log sees "184 B/s" on their fast NVMe and worries. "Workers used: 32" for a 12-byte copy is also absurd.

**Suggestion:** suppress the `• Throughput / Workers used` line when `total_bytes < some_threshold` (a megabyte? a second of wall time?) or when `files_copied <= 1`. Keep it for bulk transfers where it's meaningful. Alternatively, only show workers under `-v`.

This is minor on its own, but it's part of a pattern: the current output is optimized for the "large bulk transfer" case and reads as noise on the "one small file" case. Both cases are legitimate and frequent.

---

## 4. No progress output by default on long transfers

The TEKKEN 8 copy:

```
  -> data: common/TEKKEN 8
blit v0.1.0: starting copy .../common/TEKKEN 8 -> .../common/TEKKEN 8
                                                                       ← 77 seconds of nothing
Copy complete: 490 files, 126.39 GiB in 77.02s
```

For 77 seconds the user has no idea if blit is alive, stuck, reading, writing, or hung. `--progress` exists but is opt-in.

Rsync's default (`rsync -a` with no flags) emits per-file lines on stdout. Most modern rsync-likes (rclone, restic, borg) emit *some* progress by default. Blit defaulting fully silent is surprising and looks broken to a user running it for the first time interactively.

**Suggestions, pick one:**

- **Default to `--progress` when stdout is a TTY.** Keep silent when piped or redirected (so scripts aren't affected). This matches `cargo`, `ripgrep`, `fd`, etc.
- Or, at minimum, print a live elapsed-time heartbeat line every few seconds to prove liveness.
- Or, as a low-effort first step, print a single `Enumerating…` / `Transferring…` phase-change line so the user knows blit got past the startup.

---

## 5. Resolved destination is visible but not called out

The rsync-style destination resolution is working correctly now, but the `starting copy SRC -> DEST` line is the only signal that resolution happened, and the user has to know to compare it against what they typed.

Example from the session:

```
blit v0.1.0: starting copy /path/src/file.txt -> /path/dst/file.txt
```

The user typed `/path/dst/` and got `/path/dst/file.txt`. That's correct, but a new user may not realize blit did anything to their dest argument. For rsync users the behavior is familiar; for others it's magic.

**Suggestion:** keep the line as-is, but under `-v`, print a second line when resolution materially changed the path:

```
blit v0.1.0: starting copy /path/src/file.txt -> /path/dst/file.txt
  (destination resolved via rsync trailing-slash rule: dst/ is a container, appended basename)
```

Low priority, but it's the kind of line that makes the second-time user stop filing "why did blit move my file?" bugs.

---

## 6. Startup banner is noisy in batch / scripted use

Every invocation prints:

```
blit v0.1.0: starting copy <SRC> -> <DEST>
```

When our script copies 18 games, that's 36 of these lines interleaved with the actual output. The version number in particular is useful maybe once per session, not once per invocation.

**Suggestions:**

- Drop the version from per-invocation output; expose it via `blit --version` only.
- Or demote the `starting copy` line to stderr so stdout is reserved for actual results / summary.
- Or suppress under `--quiet`, which doesn't currently appear in `--help`. (Is there one? I couldn't find it.)

---

## 7. Path display shows doubled separators

In the real run:

```
blit v0.1.0: starting copy /run/media/michael/Games Two/SteamLibrary/steamapps//appmanifest_1778820.acf
  -> /home/michael/.steam/steam/steamapps//appmanifest_1778820.acf
```

The `//` comes from `$SRC` having a trailing slash and the script appending `/`. Filesystems ignore it; users stare at it and wonder.

**Suggestion:** normalize path separators in the display string only (collapse `//+` → `/`). Don't change actual path handling — leave the trailing-slash significance intact for rsync semantics.

---

## 8. `blit copy --help` doesn't mention rsync semantics

`docs/cli/blit.1.md` was updated — great. But first-time users tend to run `blit copy --help`, not `man blit`. The `--help` output lists every flag but says nothing about the source/destination rules that just caused two bugs.

**Suggestion:** add to the `copy` long-help a one-paragraph "PATHS" or "DESTINATION SEMANTICS" section summarizing the three rules, with a pointer to the man page for examples.

---

## 9. `--help` flag ordering buries the common case

Current order (from `blit copy --help`):

```
--config-dir, --dry-run, --checksum, --size-only, --ignore-times,
--ignore-existing, --force, --retries, --verbose, --progress, --yes,
--force-grpc, --resume, --null, --json
```

Common-case flags (`-p`, `-v`, `--dry-run`) are interleaved with niche performance/debug flags (`--null`, `--force-grpc`, `--retries`). A new user reading top-to-bottom sees `--null` and `--force-grpc` before they see `--progress`.

**Suggestion:** group and order by expected frequency. rsync groups as "common options" / "delete options" / "filter options" / "transport options". `clap` supports `#[arg(help_heading = "…")]` for this.

---

## 10. Missing a `diagnostics reproducer` or equivalent

When I had to write the two bug reports, I built the reproducers by hand (`mkdir`, `echo`, invoke, `ls`). `blit diagnostics` exists per the help output but its subcommands aren't discoverable from the top-level `--help`.

**Suggestion:** a `blit diagnostics dump` that captures:
- blit version + build info
- the full CLI invocation as received
- the parsed endpoints
- the resolved src + dest
- a one-line summary of the decision path taken (fast-path Tiny / Huge / streaming / journal-skip / NoWork-with-reason)
- filesystem info (fs type, free space, is-mounted, is-local)

... would turn "file a good bug report" from a 20-minute source-reading exercise into `blit copy X Y --diagnose > bug.log`. Worth its weight for both users and blit's own maintainers.

---

## Meta-observation

The two bugs we hit share a pattern: **blit's summary disagreed with reality and nobody noticed**.

- Rsync bug: summary said "490 files, 126 GiB" — true, but the 490 files landed in the wrong place.
- Single-file bug: summary said "0 files, 0 B" — true, but presented as success.

The summary is the contract between blit and the user. Strengthening it (items 1–3 above) pays for itself the next time something's broken. Everything else on this list is polish; the summary changes are the load-bearing recommendation.

---

## What we did *not* see problems with

To keep this balanced — things that worked well during the PoC:

- **Throughput.** 126 GiB in 77 s on local disk is genuinely impressive; the Steam game copy felt faster than `cp -r` would have been.
- **Exit code discipline on hard errors** (e.g. nonexistent source). Non-zero, useful error. Good.
- **The rsync-semantics fix itself.** Once the decision was made, the implementation was uniform across all four transport combinations and the tests pin it down. The fix was the right fix.
- **Destination-parent auto-creation.** Never had to `mkdir -p` ahead of time for a new leaf directory — nice quality-of-life.
- **JSON output mode exists.** Didn't use it here but it's the right answer for programmatic consumers and means the human-facing output can be optimized for humans without breaking scripts.

---

*Filed by the agent working on `migrate_games.sh` in `~/dev/Move-SteamGame.ps1/`. Happy to expand on any of these or provide more reproducers if useful.*
