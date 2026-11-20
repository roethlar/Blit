# Utility verbs: behave like the local commands, run on the daemon

**Status**: Draft (r1, 2026-11-20 — the owner's requirement stated in chat lands here with the audit it asked for; the owner interview has not begun)
**Created**: 2026-11-20
**Supersedes**: nothing yet. `docs/plan/BLIT_UTILS_PLAN.md` is Historical and is today the only written design intent for these verbs; whether this plan replaces it is settled at activation.
**Decision ref**: pending (activation needs a `docs/DECISIONS.md` entry)

## Goal

The utility verbs — `find`, `rm` and the others the owner names — behave like the local commands of the same name, with the work done by the daemon on the machine that holds the files, so that an operation over thousands of files does not pay a network filesystem's per-file cost. This paragraph is the drafting agent's wording of the owner's statement below; how far "like the local command" reaches is the open questions.

## Owner requirements (verbatim)

- 2026-11-20, asked whether `blit find` should follow the case rule ruled for transfers: "find should work identically to the unix find command. I don't know what check is."
- 2026-11-20: "the point to all of the utility commands that blit supports is to avoid the speed issues with running checks over nfs/smb. find, rm, etc. used to be part of blit-utils, and they need to behave like the local versions, just run directly from the daemon. that way deleting thousands of files doesn't pay the network fs tax. find should likewise work the same way. look at the old plans for blit-utils to make sure its understood and we're building the correct thing."

## What the old plans say (audit, 2026-11-20)

Read: `docs/plan/BLIT_UTILS_PLAN.md` (Historical), `docs/plan/greenfield_plan_v6.md` §"Discovery & Admin Utilities" and §"Remote Services", `docs/plan/WORKFLOW_PHASE_3.md`, the first committed text of `greenfield_plan_v4.md` and `greenfield_plan_v5.md`, and the retired `blit-utils` manual page as first committed (`4e6f5886`).

- **What they specify.** "Operator tooling for discovery, inspection, and maintenance of remote Blit daemons": a command matrix of blit's own design — `find <remote> [--pattern <glob>]`, `rm <remote> [--yes]`, `du`, `df`, `ls`, `list-modules`, `scan`, `completions`, `profile` — each backed by a daemon RPC, with five UX principles (confirmation before a destructive command, tabular output with `--json`, exit codes, shared endpoint parsing, authentication hooks). v4 and v5 name the crate and nothing more.
- **What they do not say, anywhere.** Neither half of the owner's statement is written down: not that the purpose is to avoid a network filesystem's per-file cost, and not that a verb must behave like the local command of its name. A search of `docs/`, `README.md`, `TODO.md` and `DEVLOG.md` for a line pairing a network filesystem with one of these verbs found none.
- **So the verbs were built to the written matrix, and the written matrix is not the owner's intent.** The half that was met without being written is the architecture: each verb is one request answered by the daemon's own walk of its own disk. The half that was not met is the behavior.

## What ships today, beside the local command

From each verb's `--help` and, where noted, its source; blit 1.0.0 built at `863dfcf0`. The local-command column is the standard behavior of the Unix command, measured today only where the Measured section says so.

| Verb | Ships today | The local command |
|---|---|---|
| `find` | `find <remote> [--pattern GLOB] [--files] [--dirs] [--case-insensitive] [--limit N] [--json]`; the pattern is tried against the relative path and the base name; `*` does not cross `/` | an expression: `-name`, `-iname`, `-path`, `-type`, `-size`, `-mtime`, `-maxdepth`, `-delete`, …; `-name` reads the base name only |
| `rm` | `rm <remote> [-y] [--json]`; ONE target per command, sent as one path to the daemon's purge; asks "Delete …? [y/N]" unless `-y`; refuses a whole module; remote paths only (`crates/blit-cli/src/rm.rs`) | several operands; `-r` required for a directory; `-f`; asks nothing unless `-i` |
| `ls` | `ls <local or remote> [--json]` | `-l`, `-a`, `-R`, sorting and the rest |
| `du` | `du <remote> [--max-depth N] [--json]` | `-h`, `-s`, `-a`, `-d N`, one line per directory |
| `df` | `df <remote module> [--json]` | `-h` and the rest |

`check` is not in the old matrix. It arrived 2026-05-01 (`13e1e546`), compares two trees by size and time or by hash, and reads both trees from the machine the command runs on (`crates/blit-cli/src/check.rs` handles paths only) — so with one tree on a network mount it pays exactly the cost the owner describes. The owner on 2026-11-20: "I don't know what check is."

## Measured (2026-11-20)

`blit find` against a loopback daemon and GNU find 4.11.0, over the same five files (`Cache.txt`, `ÜBUNG.txt`, `übung.txt`, `a.dat`, the composed `é.dat`):

| Search | GNU find, UTF-8 locale | GNU find, `C` locale | `blit find` |
|---|---|---|---|
| ignore case, `cache*` | `Cache.txt` | `Cache.txt` | `Cache.txt` |
| ignore case, `übung*` | `übung.txt`, `ÜBUNG.txt` | `übung.txt` | `übung.txt` |
| `?.dat` | `a.dat`, `é.dat` | `a.dat` | `a.dat` |
| `??.dat` | `é.dat` | `é.dat` | `é.dat` |

Unix find depends on the locale; `blit find` equals it in the `C` locale and not in a UTF-8 one. Its help says `?` "matches one non-`/` character" where it matches one byte.

## Non-goals

- None ruled yet.

## Constraints

- FAST, SIMPLE, RELIABLE (`.agents/repo-guidance.md` Product Principles).
- Three platforms: a daemon runs on Linux, macOS and Windows, and "the local command" is not the same program on each (Q1).
- These verbs and their options shipped in the 0.1.x releases (Q7).

## Acceptance criteria

- [ ] To be written from the interview; none is ruled yet.

## Design

Not begun. No code until `Active`.

## Slices

One coherent, testable change per slice — sized for the `.review/` loop.

1. To be cut after the interview.

## Open questions

- **Q1 — which local command is the reference**: GNU findutils and coreutils, the BSD versions macOS ships, or POSIX's common subset; in which locale; and what a Windows daemon is held to. — owner
- **Q2 — how much of each command**: the command line itself (`blit find host:/module/path -name '*.jpg' -type f`), or blit's own options with the local command's matching behavior underneath. — owner
- **Q3 — which verbs**: the owner named `find` and `rm` and said "etc."; `ls`, `du` and `df` share the old matrix. Whether anything is to be added. — owner
- **Q4 — `rm`**: the local command asks nothing, takes many operands and needs `-r` for a directory; `blit rm` asks unless `-y`, takes one target, and refuses a whole module, under the old plan's "Safety First". "Deleting thousands of files" is one directory today, never a set of matches. Which behavior wins. — owner
- **Q5 — output**: the local command's own output, or the old plan's table with `--json`. — owner
- **Q6 — `check`**: whether it is one of these utilities — and so should run on the daemon — or something else. `docs/plan/EXCLUDE_CASE_MATCHING.md` Q11 separately asks whether it takes the transfer verbs' pattern options. — owner
- **Q7 — the options that shipped** (`--pattern`, `--case-insensitive`, `--files`, `--dirs`, `--limit`, `--max-depth`, `--json`): kept, hidden or removed. — owner
- **Q8 — before or after the 1.0 tag** (D-2026-10-19-2 blocks 1.0 on every known imperfection). — owner
