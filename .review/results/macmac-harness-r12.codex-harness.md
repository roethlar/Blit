Reading additional input from stdin...
OpenAI Codex v0.144.4
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f62af-64cd-7ac0-980f-ac33e519d271
--------
user
Shell correctness review of one bash script: scripts/bench_otp12pf_mac.sh (read only that file, plus docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md for what it is supposed to do).

It is a benchmark harness on the owner's own two Macs. It runs file transfers between them, times them, and records the numbers. It runs entirely on hardware the owner controls, and no data has ever been taken with it. It uses ssh to the second machine, sudo to drop the page cache between runs, and pgrep/iostat/top to check the machines are idle before it measures anything. Those are the measurement mechanics; the review is about whether they are CORRECT, not about what they are.

The thing at stake: this harness has been reviewed eleven times and every round found a defect that would have produced a WRONG MEASUREMENT. Two defect classes keep coming back, and you should assume both are still present:

(a) THE FIX WAS APPLIED WHERE IT WAS SHOWN, NOT WHERE IT LIVED. A fail-open `pgrep` was fixed
    in one gate and left identical in a second. The drain was fixed by validating its VALUE
    and left failing by discarding its EXIT STATUS -- and then, one round later, three MORE
    substitutions in the same function were still discarding theirs. Find the next duplicate.

(b) A PROTECTION THAT NEVER EXECUTES, OR CANNOT FAIL. The equal pre-fsync settle had NEVER
    RUN FOR THREE REVISIONS (an awk quoting bug killed the sleep and its status was
    discarded) while the pre-registration asserted it. `bash -n` sees nothing. Writing THIS
    revision produced two more of the same class, both caught only by running it: the pinned
    topology literals were placed above the override check, so the harness refused EVERY run;
    and a new sentinel was `:`-delimited around a MAC address (which is all colons), so the
    gate read BLIND on a good link. Find the next protection that does not actually execute,
    that cannot fail, or that cannot pass.

WHAT CHANGED IN THIS REVISION (verify each is closed, then find the next instance of the same class):
- THE REGISTERED TOPOLOGY IS NOW ENFORCED (this was the round-11 BLOCKER: NIC/IP/MAC were env-overridable and the MTU and link speed were never checked at all, so the run could go over the 1GbE NIC or at MTU 1500). Now: the topology is a pinned literal and the harness refuses if any of those names is present in the environment; topology_gate checks the NIC's MTU, negotiated media and link status; mss_gate opens a real TCP connection to the peer's registered IP and checks the negotiated MSS (8948) AND the local address the kernel chose; the fabric is re-checked at the END of the session and voids it if it changed.
- the drain now checks the exit status of every producer, not just the value.
- resolve_disk now checks the df pipeline's status, and a FAILED grep no longer reads as "not APFS".
- both Time Machine gates no longer use `tr -cd '0-9'` (which manufactured a passing 0 out of any malformed output containing a zero, e.g. "0%").
- `ps` no longer maps every non-zero status onto GONE.
- gate_probe no longer runs gates inside an `||` context (which disabled errexit through their call tree).
- link_gate no longer discards its ARP/route substitution statuses.
- RUNS and PORT are pinned like the other registered constants.

Check specifically:
1. Every exit status that a following command can discard: pipelines (a pipeline's status is
   the LAST command's), command substitutions, `|| true`, `&&` chains, and any value that is
   validated without checking that the probe producing it succeeded. The NEW code (topology_gate,
   mss_read/mss_gate/end_mss_gate, the rewritten link_gate) is the most likely place for a new one.
2. Every gate that reads a number: can an error, an empty string, or a non-number read as a
   passing value? Every sentinel frame: can the value being framed contain the delimiter?
3. mss_gate and end_mss_gate share ONE reader (mss_read) but apply different policies (refuse
   vs void). Is the shared reader correct for both? Can end_mss_gate silently fail to void?
   Does the manifest's use of mss_read (inside a $() ) lose anything?
4. The slot-major scheduler (run_all_cells / run_one_pair): void/retry accounting, ABBA arm
   alternation, per-slot cell rotation, the CSV rows, and whether a cell that exhausts its
   attempts can corrupt another. macOS ships bash 3.2: no associative arrays, no mapfile.
5. `set -euo pipefail` interactions, subshell state loss (a function that sets a global, called
   inside `$(...)`, loses it -- this has bitten twice; gate_probe's new `set -e` inside its
   subshell is a fresh instance of that hazard), and the EXIT trap.
6. Does the harness do what the pre-registration says? Any claim in the doc the script does not
   implement, or behaviour it has that the doc does not register? (The per-run RTT gate was
   unregistered in round 11 -- check it is now registered, and that the new fabric gates are too.)

You may run the script's own self-test (`SELFTEST=1 bash scripts/bench_otp12pf_mac.sh`), which exercises the gates and takes no data, and you may read its output.

Output a concise markdown findings list (file:line, severity BLOCKER/HIGH/MEDIUM/LOW, rationale), then a final VERDICT line: READY or NOT READY. Be concise; do not invoke skills.
codex
I’ll inspect only the script and preregistration, then run the permitted self-test. I’ll trace producer statuses, sentinels, gate execution, scheduler state, and documented-vs-implemented behavior before giving only actionable findings and a verdict.
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
