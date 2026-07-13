Reading additional input from stdin...
OpenAI Codex v0.144.3
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: ultra
reasoning summaries: none
session id: 019f5dac-7a70-77b0-9f3c-d5783d29abb6
--------
user
Review the diff of commit 4402987 (run: git show 4402987). It adds a LOCAL benchmark harness (scripts/bench_win_local_ab.sh + scripts/windows/local-ab-run.ps1) and its evidence (docs/bench/win-local-ab-2026-07-13/), comparing blit against robocopy for a local D: -> E: copy on a Windows box (two separate identical NVMe drives). No network is involved.

Headline result (RUNS=8 medians, transfer + destination flush):
  large  1 x 1 GiB           blit  539ms  robocopy  541ms  ratio 0.996 (parity)
  mixed  512MiB + 5000 files blit  934ms  robocopy  501ms  ratio 1.863
  small  10,000 x 4 KiB      blit 1388ms  robocopy  697ms  ratio 1.991

THE MOST IMPORTANT QUESTION — answer it first, with code evidence:

**Is this an apples-to-apples comparison, or is blit simply DOING MORE WORK than robocopy?** A 2x cost is not a defect if blit is doing strictly more. Read blit's actual local copy path in the tree (it rides the unified transfer_session since otp-11 — see crates/blit-core/src/transfer_session/, the planner, and the local apply carrier / FsTransferSink) and determine what work a `blit copy <src> <dst> --yes` performs per file that `robocopy <src> <dst> /E /MT:8 /R:2 /W:2` does NOT — and vice versa. Consider specifically: content hashing/checksums or verification passes; manifest/need negotiation; metadata (mtime/permissions/ACLs/alternate data streams) preservation; per-file fsync or flush; directory pre-creation; enumeration/stat passes (one vs several); thread/parallelism defaults (robocopy /MT:8 vs whatever blit chooses); and whether blit's local path still frames records through a data-plane/tar-shard encoder rather than doing a direct file copy. Name the concrete asymmetries with file:line. If blit is doing materially more work, say so plainly — the finding as written would then be misleading.

THEN check the harness itself for defects that could manufacture or inflate the gap:
1. scripts/windows/local-ab-run.ps1 — the timed window. Is anything inside the Stopwatch that should be outside it (or vice versa) for one arm but not the other? Note blit is invoked as `& $BlitExe copy $Src $DestRoot` (it nests the source basename under the destination) while robocopy is pointed at `$DestRoot\<leaf>` directly (it copies contents). Do BOTH arms therefore land an identical tree and do identical work? Is the container precreation fair to both?
2. Durability: `Write-VolumeCache -DriveLetter E` is self-timed and added to the wall time for BOTH arms. Is that keyed by DESTINATION (correct) and symmetric? Could one tool leave more dirty data for the flush to absorb, and is that legitimate to charge it?
3. Cold-cache and drain logic: standby purge before every run; drain waits for PhysicalDisk(_Total) Disk Write Bytes/sec < 1MB/s for 3 consecutive 2s samples. `_Total` spans ALL disks — is that adequate, or should it be per-disk? Could an undrained queue from the previous run land on the next?
4. Validity gates: robocopy exit <8 = success, blit exit 0; landed file count verified per run; failed flush voids. Anything a broken run could slip through?
5. The recorded cold-allocation outlier: blit's warm-up (2253ms) AND first timed run (2175ms) in the `small` cell are slow, then it settles at 1386-1430; robocopy shows the same far more weakly (742 -> ~690). The README claims blit is "disproportionately sensitive to cold NTFS destination-allocation state". Is that supported, or is there a harness explanation?
6. The README's claim that this is NOT an otp-11 regression rests on docs/bench/otp11-local-2026-07-11/ (old 1684ms -> new 1750ms, +3.9%, on the MAC). Is that inference sound, given the local numbers here are from Windows/NTFS and that gate was macOS/APFS? The README labels the Windows old-vs-new as UNMEASURED — is that labelled honestly and is any other claim leaning on the unmeasured part?

Finally: does the README overclaim anywhere, or under-report a gap? It explicitly disclaims P1 relevance (no initiator axis locally) and P2 relevance (old blit has the cost too). Check both disclaimers against docs/plan/OTP12_PERF_FINDINGS.md.

Output a concise markdown findings list — each finding with file:line, severity, and rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll review this read-only, starting with the actual `blit copy` local data path, then audit the timing/validity logic and benchmark claims against the recorded evidence and OTP12 findings. I’ll cite concrete lines from commit `4402987`.
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
codex
The core asymmetry is now clear: default blit does not hash or fsync each file, but it does run every entry through source manifest/readability checks, destination need-diff bookkeeping, and—in these small-file cells—an in-memory tar encode/validate/decode path. I’m checking whether that extra machinery represents stronger output semantics or merely implementation overhead, and whether robocopy is actually preserving more Windows metadata.
web search: 
web search: site:learn.microsoft.com robocopy /MT default 8 /COPY:DAT /DCOPY:DA alternate data streams ...
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
2026-07-13T22:57:38.502051Z ERROR codex_core::tools::router: error=timeout_ms must be at least 10000
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
collab: Wait
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
codex
**Answer first:** This is apples-to-apples only as a user-facing wall-clock test of these plain-file fixtures—not as equal work. Blit performs materially more internal per-file work, but it does not produce a strictly stronger or more durable copy. The ~2× result is therefore real blit overhead, not a checksum/durability premium.

- **High — The README under-reports major execution asymmetries.** Default blit uses size+mtime, with BLAKE3 only when `--checksum` is selected ([local.rs:175](/Users/michael/Dev/blit_v2/crates/blit-cli/src/transfers/local.rs:175), [mod.rs:1331](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1331)). Nevertheless, every file is statted/opened for readability, cloned into a mutex-protected manifest map and sent over an in-process channel, canonical-containment-checked/statted at the destination, then reopened for availability ([source.rs:137](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/source.rs:137), [mod.rs:1349](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1349), [mod.rs:3541](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3541), [source.rs:236](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/source.rs:236)). This is substantial bookkeeping/safety work, but not post-copy verification.

- **High — Small-file execution is radically different, including parallelism.** These files are encoded into in-memory tar shards, reparsed and copied into new buffers, then written with a `create_dir_all` check per member ([payload.rs:254](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/payload.rs:254), [tar_safety.rs:104](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/tar_safety.rs:104), [sink.rs:542](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:542)). Normal local blit uses one payload worker; only writes within the current shard use Rayon ([local.rs:602](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/local.rs:602), [sink.rs:592](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:592)). Robocopy is explicitly given `/MT:8` ([local-ab-run.ps1:60](/Users/michael/Dev/blit_v2/scripts/windows/local-ab-run.ps1:60)); that is selected eight-thread execution, not equal default parallelism. This and tar processing are plausible causes, not proven attribution.

- **High — Blit is not a semantic superset of robocopy.** Blit’s Windows tar path carries only mtime—not Windows permissions/attributes, ACLs, or ADS—and the manifest omits directory entries, so empty directories and directory attributes are lost ([source.rs:223](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/source.rs:223), [sink.rs:583](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:583), [source.rs:137](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/source.rs:137)). Robocopy defaults to `/COPY:DAT` and `/DCOPY:DA`, copies ADS unless `X` is requested, and `/E` includes empty directories; neither command requests ACLs. [Microsoft’s robocopy reference](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/robocopy). Large blit files do go directly through `CopyFileExW` ([windows.rs:340](/Users/michael/Dev/blit_v2/crates/blit-core/src/copy/windows.rs:340)), which preserves attributes and ADS. [Microsoft’s CopyFileEx reference](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-copyfileexa).

- **Medium — “No carrier” and the H7 reproduction claim are inaccurate.** Local payload bytes bypass TCP/protobuf, but manifest control frames still traverse bounded in-process channels, and small data still takes the tar path ([transport.rs:61](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/transport.rs:61), [transfer_session/local.rs:4](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/local.rs:4)). More importantly, local apply sends no per-file `NeedBatch`; it plans/applies directly and sends only `NeedComplete` ([mod.rs:3353](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3353)). Thus [README.md:91](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/README.md:91) exercises H7’s manifest-map insertion, but not its cited per-need channel hop. It is a supplemental lead, not an H7 reproduction or pf-1 substitute.

- **Medium — Blit has an unpinned, timed side effect.** Performance history defaults enabled and is appended before the process returns; crossing its 1 MB cap triggers a read/rewrite rotation ([context.rs:8](/Users/michael/Dev/blit_v2/crates/blit-cli/src/context.rs:8), [transfer_session/local.rs:694](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/local.rs:694), [perf_history.rs:443](/Users/michael/Dev/blit_v2/crates/blit-core/src/perf_history.rs:443)). The harness neither disables nor records this blit-only work inside the Stopwatch. Large-file parity makes it unlikely to explain the shape-dependent 2× gap, but it is a real confound.

- **Medium — Validity proves counts, not bytes or tree equivalence.** Source preflight and destination validation check only file counts ([bench_win_local_ab.sh:70](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:70), [local-ab-run.ps1:78](/Users/michael/Dev/blit_v2/scripts/windows/local-ab-run.ps1:78)). Correct-count zero-length, truncated, wrong-content, or structurally different trees can pass, so [README.md:43](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/README.md:43) should say throughput parity, not “byte-for-byte.” Robocopy mismatch codes below 8 also pass, and an incomplete matrix merely logs then exits successfully ([bench_win_local_ab.sh:119](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:119), [bench_win_local_ab.sh:164](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:164)). Recorded runs were all robocopy rc=1 and n=8, so no committed run is shown broken.

- **Medium — The cold-NTFS attribution is unsupported.** Warm-up is always blit, and the first timed arm is again blit; robocopy never observes the same cell-start state ([bench_win_local_ab.sh:135](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:135), [bench_win_local_ab.sh:143](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:143), [bench_win_local_ab.sh:148](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:148)). This confounds tool with order/allocation/cleanup state. Robocopy run 8 is also 743 ms versus run 1’s 742 ms ([runs.csv:19](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/runs.csv:19), [runs.csv:33](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/runs.csv:33)). The safe conclusion is “unexplained first-blit outlier”; the RUNS=8 median remains robust.

- **High — “Not an otp-11 regression” is too categorical.** The cited gate was RUNS=3 on macOS/APFS, same-volume, old-vs-new blit—not Windows/NTFS or blit-vs-robocopy ([otp11 README.md:6](/Users/michael/Dev/blit_v2/docs/bench/otp11-local-2026-07-11/README.md:6), [otp11 README.md:11](/Users/michael/Dev/blit_v2/docs/bench/otp11-local-2026-07-11/README.md:11), [otp11 README.md:22](/Users/michael/Dev/blit_v2/docs/bench/otp11-local-2026-07-11/README.md:22)). [Windows README.md:66](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/README.md:66) honestly admits Windows old-vs-new is unmeasured, contradicting the categorical wording at lines 52–57. The evidence proves only “no material otp-11 regression on the Mac gate.” The local cell is indeed not P2 because P2 is old-vs-new TCP push ([OTP12_PERF_FINDINGS.md:241](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:241)); “old blit carries this Windows cost” is not established.

No timed-window/tree/flush defect was found: both arms target the same precreated `$DestRoot\$leaf`, and E: flush is symmetric and properly charged ([local-ab-run.ps1:44](/Users/michael/Dev/blit_v2/scripts/windows/local-ab-run.ps1:44), [resolution.rs:141](/Users/michael/Dev/blit_v2/crates/blit-app/src/transfers/resolution.rs:141), [local-ab-run.ps1:68](/Users/michael/Dev/blit_v2/scripts/windows/local-ab-run.ps1:68)). Removing flush actually enlarges the mixed/small ratios to about 2.37×/2.35×. `_Total` is conservative—unrelated disks can cause false timeouts, not conceal E: writing above the threshold—though a per-E-disk queue/rate check would be more specific. The P1 disclaimer is correct.

**VERDICT:** Valid user-level plain-file shipping comparison, but not equal work. Blit does more internal bookkeeping/safety/tar work while robocopy uses more copy threads and preserves more Windows tree metadata. The ~2× median stands as genuine blit overhead; the README must narrow its H7, cold-allocation, and Windows otp-11/P2 claims.
tokens used
229,090
**Answer first:** This is apples-to-apples only as a user-facing wall-clock test of these plain-file fixtures—not as equal work. Blit performs materially more internal per-file work, but it does not produce a strictly stronger or more durable copy. The ~2× result is therefore real blit overhead, not a checksum/durability premium.

- **High — The README under-reports major execution asymmetries.** Default blit uses size+mtime, with BLAKE3 only when `--checksum` is selected ([local.rs:175](/Users/michael/Dev/blit_v2/crates/blit-cli/src/transfers/local.rs:175), [mod.rs:1331](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1331)). Nevertheless, every file is statted/opened for readability, cloned into a mutex-protected manifest map and sent over an in-process channel, canonical-containment-checked/statted at the destination, then reopened for availability ([source.rs:137](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/source.rs:137), [mod.rs:1349](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1349), [mod.rs:3541](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3541), [source.rs:236](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/source.rs:236)). This is substantial bookkeeping/safety work, but not post-copy verification.

- **High — Small-file execution is radically different, including parallelism.** These files are encoded into in-memory tar shards, reparsed and copied into new buffers, then written with a `create_dir_all` check per member ([payload.rs:254](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/payload.rs:254), [tar_safety.rs:104](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/tar_safety.rs:104), [sink.rs:542](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:542)). Normal local blit uses one payload worker; only writes within the current shard use Rayon ([local.rs:602](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/local.rs:602), [sink.rs:592](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:592)). Robocopy is explicitly given `/MT:8` ([local-ab-run.ps1:60](/Users/michael/Dev/blit_v2/scripts/windows/local-ab-run.ps1:60)); that is selected eight-thread execution, not equal default parallelism. This and tar processing are plausible causes, not proven attribution.

- **High — Blit is not a semantic superset of robocopy.** Blit’s Windows tar path carries only mtime—not Windows permissions/attributes, ACLs, or ADS—and the manifest omits directory entries, so empty directories and directory attributes are lost ([source.rs:223](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/source.rs:223), [sink.rs:583](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:583), [source.rs:137](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/source.rs:137)). Robocopy defaults to `/COPY:DAT` and `/DCOPY:DA`, copies ADS unless `X` is requested, and `/E` includes empty directories; neither command requests ACLs. [Microsoft’s robocopy reference](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/robocopy). Large blit files do go directly through `CopyFileExW` ([windows.rs:340](/Users/michael/Dev/blit_v2/crates/blit-core/src/copy/windows.rs:340)), which preserves attributes and ADS. [Microsoft’s CopyFileEx reference](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-copyfileexa).

- **Medium — “No carrier” and the H7 reproduction claim are inaccurate.** Local payload bytes bypass TCP/protobuf, but manifest control frames still traverse bounded in-process channels, and small data still takes the tar path ([transport.rs:61](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/transport.rs:61), [transfer_session/local.rs:4](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/local.rs:4)). More importantly, local apply sends no per-file `NeedBatch`; it plans/applies directly and sends only `NeedComplete` ([mod.rs:3353](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3353)). Thus [README.md:91](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/README.md:91) exercises H7’s manifest-map insertion, but not its cited per-need channel hop. It is a supplemental lead, not an H7 reproduction or pf-1 substitute.

- **Medium — Blit has an unpinned, timed side effect.** Performance history defaults enabled and is appended before the process returns; crossing its 1 MB cap triggers a read/rewrite rotation ([context.rs:8](/Users/michael/Dev/blit_v2/crates/blit-cli/src/context.rs:8), [transfer_session/local.rs:694](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/local.rs:694), [perf_history.rs:443](/Users/michael/Dev/blit_v2/crates/blit-core/src/perf_history.rs:443)). The harness neither disables nor records this blit-only work inside the Stopwatch. Large-file parity makes it unlikely to explain the shape-dependent 2× gap, but it is a real confound.

- **Medium — Validity proves counts, not bytes or tree equivalence.** Source preflight and destination validation check only file counts ([bench_win_local_ab.sh:70](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:70), [local-ab-run.ps1:78](/Users/michael/Dev/blit_v2/scripts/windows/local-ab-run.ps1:78)). Correct-count zero-length, truncated, wrong-content, or structurally different trees can pass, so [README.md:43](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/README.md:43) should say throughput parity, not “byte-for-byte.” Robocopy mismatch codes below 8 also pass, and an incomplete matrix merely logs then exits successfully ([bench_win_local_ab.sh:119](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:119), [bench_win_local_ab.sh:164](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:164)). Recorded runs were all robocopy rc=1 and n=8, so no committed run is shown broken.

- **Medium — The cold-NTFS attribution is unsupported.** Warm-up is always blit, and the first timed arm is again blit; robocopy never observes the same cell-start state ([bench_win_local_ab.sh:135](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:135), [bench_win_local_ab.sh:143](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:143), [bench_win_local_ab.sh:148](/Users/michael/Dev/blit_v2/scripts/bench_win_local_ab.sh:148)). This confounds tool with order/allocation/cleanup state. Robocopy run 8 is also 743 ms versus run 1’s 742 ms ([runs.csv:19](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/runs.csv:19), [runs.csv:33](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/runs.csv:33)). The safe conclusion is “unexplained first-blit outlier”; the RUNS=8 median remains robust.

- **High — “Not an otp-11 regression” is too categorical.** The cited gate was RUNS=3 on macOS/APFS, same-volume, old-vs-new blit—not Windows/NTFS or blit-vs-robocopy ([otp11 README.md:6](/Users/michael/Dev/blit_v2/docs/bench/otp11-local-2026-07-11/README.md:6), [otp11 README.md:11](/Users/michael/Dev/blit_v2/docs/bench/otp11-local-2026-07-11/README.md:11), [otp11 README.md:22](/Users/michael/Dev/blit_v2/docs/bench/otp11-local-2026-07-11/README.md:22)). [Windows README.md:66](/Users/michael/Dev/blit_v2/docs/bench/win-local-ab-2026-07-13/README.md:66) honestly admits Windows old-vs-new is unmeasured, contradicting the categorical wording at lines 52–57. The evidence proves only “no material otp-11 regression on the Mac gate.” The local cell is indeed not P2 because P2 is old-vs-new TCP push ([OTP12_PERF_FINDINGS.md:241](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:241)); “old blit carries this Windows cost” is not established.

No timed-window/tree/flush defect was found: both arms target the same precreated `$DestRoot\$leaf`, and E: flush is symmetric and properly charged ([local-ab-run.ps1:44](/Users/michael/Dev/blit_v2/scripts/windows/local-ab-run.ps1:44), [resolution.rs:141](/Users/michael/Dev/blit_v2/crates/blit-app/src/transfers/resolution.rs:141), [local-ab-run.ps1:68](/Users/michael/Dev/blit_v2/scripts/windows/local-ab-run.ps1:68)). Removing flush actually enlarges the mixed/small ratios to about 2.37×/2.35×. `_Total` is conservative—unrelated disks can cause false timeouts, not conceal E: writing above the threshold—though a per-E-disk queue/rate check would be more specific. The P1 disclaimer is correct.

**VERDICT:** Valid user-level plain-file shipping comparison, but not equal work. Blit does more internal bookkeeping/safety/tar work while robocopy uses more copy threads and preserves more Windows tree metadata. The ~2× median stands as genuine blit overhead; the README must narrow its H7, cold-allocation, and Windows otp-11/P2 claims.
