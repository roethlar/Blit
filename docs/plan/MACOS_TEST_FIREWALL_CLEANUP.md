# macOS test firewall cleanup

**Status**: Active
**Created**: 2026-07-23
**Supersedes**: nothing
**Decision ref**: D-2026-07-23-6

## Goal

Make every macOS hardware test that temporarily admits an unsigned
`blit-daemon` own the complete Application Firewall rule lifecycle. A test must
record the exact rule it creates, remove that exact rule before deleting the
binary or detaching its volume, verify the path is absent from the real
firewall inventory, and fail cleanup visibly if any of those steps cannot be
proved.

## Evidence and defect

The ETL Thunderbolt record retained
`raw/firewall-cleanup.txt` containing only
`firewall_entry_after_cleanup=absent`. It retained neither the removal command
and exit status nor before/after `socketfilterfw --listapps` inventories.
Current read-only inventory contains the same supposedly removed path:

`/Volumes/BLIT_ETL_BUILD/etl3-review/target/release/blit-daemon`

It also contains a second ephemeral Thunderbolt-test path:

`/private/tmp/blit-tb-candidate.Ki87md/unpacked/blit-aarch64-apple-darwin/blit-daemon`

The record cannot distinguish a failed removal, a faulty check, or a later
re-add. Therefore the historical assertion is not cleanup proof. Repository
target paths and every non-ephemeral firewall entry are outside this plan.

## Non-goals

- No daemon installation, LaunchAgent, LaunchDaemon, login item, startup
  service, stable production path, Apple signing, notarization, or package
  change.
- No Blit product, transfer, protocol, policy, CLI, daemon, filesystem, or
  telemetry behavior change.
- No firewall disablement, global firewall preference change, broad port rule,
  persistent benchmark allow rule, or reuse of a stale rule.
- No hardware transfer, benchmark payload, SSD payload write, release, tag,
  publication, or push.
- No automatic removal of developer, product, system, or third-party entries.

## Constraints

- Implement one repository-owned macOS test helper under `scripts/macos/`.
  Hardware procedures must invoke it rather than issuing ad hoc
  `socketfilterfw --add`, `--unblockapp`, or `--remove` calls.
- The helper accepts one absolute daemon path and a command to run. It refuses
  a missing/non-regular daemon, a non-absolute path, an already-listed path,
  an unresolved owned-rule ledger, or unavailable administrator authorization
  before starting the command.
- Administrator authorization is explicit and bounded to the test session.
  The helper may validate and keep the existing `sudo` ticket alive only for
  its own add, unblock, and remove operations; the daemon and test command run
  as the invoking user, never as root.
- Write a small durable owned-rule ledger before adding the rule. Use a
  per-user cache/state location supplied by the helper contract, not the repo
  or a RAM disk, so interruption, process death, or reboot cannot erase the
  recovery record while leaving the firewall entry.
- The ledger records the exact lexical path, canonical path when resolvable,
  creation timestamp, helper PID, and test/session identifier. Atomic replace
  and a restrictive user-only mode are required.
- Before add, retain the complete firewall inventory and prove the exact path
  is absent. After add/unblock, retain command results, prove the exact path
  appears exactly once, and prove it is permitted.
- Install cleanup handlers before the add. Normal return, command failure,
  `EXIT`, `HUP`, `INT`, and `TERM` all stop the authorization keeper, remove
  only the exact owned path, retain the complete post-remove inventory, and
  prove zero exact matches before clearing the ledger.
- Cleanup occurs while the executable path and backing volume still exist.
  The caller may delete scratch or detach the volume only after the helper
  reports verified cleanup.
- A cleanup failure overrides a successful test result, leaves the ledger and
  executable/volume available for recovery, prints the exact unresolved path,
  and never emits a success marker.
- `SIGKILL`, host crash, and power loss cannot run a trap. The next invocation
  detects the durable ledger, refuses new work, and offers only exact-path
  recovery. It clears the ledger only after verified firewall absence.
- Parsing must use complete `socketfilterfw --listapps` output and exact path
  equality. A hand-authored `absent` line, substring grep, detached-volume
  state, missing executable, or successful `--remove` exit alone is not proof.
- Tests replace `sudo` and `socketfilterfw` with controlled fakes. They never
  modify the host firewall or require administrator authorization.

## Acceptance criteria

- [ ] A new macOS test helper owns exactly one temporary application rule from
      absent preflight through add, unblock, test command, remove, and verified
      absence.
- [ ] The helper never installs or starts Blit automatically. The daemon and
      wrapped command run as the invoking user and only for that invocation.
- [ ] Exact inventories and command outcomes are retained before add, after
      admission, and after removal. Evidence includes the path, session ID,
      timestamps, exit codes, and whether cleanup superseded the command result.
- [ ] A command exit zero, nonzero exit, `INT`, and `TERM` all remove the exact
      owned rule and clear the ledger only after zero exact inventory matches.
- [ ] Add failure, unblock failure, malformed inventory, duplicate exact path,
      removal failure, lost authorization, and post-remove persistence all
      fail closed. Removal failure retains the ledger and blocks scratch
      deletion.
- [ ] An existing firewall entry is never adopted or removed. An unresolved
      owned ledger blocks a new rule until exact recovery succeeds.
- [ ] Deterministic fake-backed tests cover paths containing spaces, exact
      rather than substring matching, unrelated-entry preservation, duplicate
      entries, stale ledgers with present and absent rules, signal cleanup, and
      cleanup failure overriding command success.
- [ ] Each new behavioral guard is mutation-proved by changing production
      helper behavior, observing the targeted test fail, restoring it, and
      observing it pass.
- [ ] Relevant existing macOS hardware procedures point to the helper and
      explicitly forbid deleting the binary or detaching its volume before the
      helper's verified cleanup marker.
- [ ] The docs gate, shell syntax/static checks available in the repo, focused
      helper suite, repository verification entry point, and `git diff --check`
      pass.
- [ ] After implementation acceptance, the two exact proven ephemeral test
      paths listed above are presented to the owner for one-time removal.
      No live firewall mutation occurs without that separate exact approval;
      complete before/after inventories are retained.

## Design

### Owned lifecycle

The helper receives:

- an absolute daemon path;
- a unique test/session ID;
- an evidence output directory;
- the wrapped command and arguments.

It validates inputs and the real inventory, acquires administrator
authorization, writes the durable ledger, installs cleanup handlers, then adds
and unblocks the exact path. Only after exact admission proof does it run the
wrapped command as the original user.

On every catchable exit it removes the exact path and verifies the complete
inventory contains zero exact matches. It preserves the wrapped command's exit
status only when cleanup succeeds. Any cleanup failure returns a distinct
nonzero status, retains recovery state, and forbids the caller from destroying
the path.

### Durable recovery

The ledger is recovery state, not evidence of success. A later helper
invocation first reconciles it against the live inventory:

- ledger path present: require administrator-authorized exact removal and
  verified absence;
- ledger path absent: retain a recovery record explaining that no removal was
  needed, then clear the ledger;
- unreadable or malformed inventory: stop without clearing anything.

The helper never scans for or removes paths that are not in its one owned
ledger. The one-time historical cleanup is a separately approved exact target
list, not automatic discovery.

### Evidence

The helper writes raw, timestamped command output and a small machine-readable
summary derived from that output. The summary may state cleanup success only
when add ownership, removal, and zero exact post-remove matches are all proved.
The historical single-line assertion is retained unchanged as evidence of the
old defect; it is not used as a template.

## Affected files

- `scripts/macos/with-temporary-firewall-rule.sh` — owned lifecycle, recovery,
  signal handling, evidence, and exact verification.
- `scripts/macos/test-with-temporary-firewall-rule.sh` — fake-backed
  deterministic behavior and mutation guards.
- Relevant macOS hardware plan/procedure guidance — require the helper and its
  cleanup marker before scratch deletion or volume detach.
- `docs/plan/MACOS_TEST_FIREWALL_CLEANUP.md`, `docs/DECISIONS.md`,
  `docs/STATE.md`, `.agents/machines.md`, and `DEVLOG.md` — durable scope,
  evidence, state, and closure.

## Slices

1. **mtfc-1 — helper and deterministic guards.** Implement the exact owned-rule
   lifecycle and durable recovery state, add fake-backed tests, mutation-prove
   the failure paths, and commit without touching the host firewall.
2. **mtfc-2 — integration and verification.** Route relevant future macOS
   hardware guidance through the helper, run the complete local gates, record
   accepted behavior, and close implementation without a hardware transfer.
3. **mtfc-3 — optional one-time residue cleanup.** Only after separate exact
   owner approval, remove the two proven ephemeral test entries, retain full
   before/after inventories, update machine state, and commit the evidence.

## Open questions

- None. Live cleanup remains separately gated after implementation.
