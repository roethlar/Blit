Reading additional input from stdin...
OpenAI Codex v0.142.5
--------
workdir: /home/michael/dev/Blit
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f3567-bef3-7041-b56d-0348969d7488
--------
user
Review the diff of commit e1aafcc (run: git show e1aafcc). It is the fix commit addressing two codex findings on otp-4b-1 (the TCP data plane on the unified transfer_session in crates/blit-core/src/transfer_session/). The two fixes:

F1: the data-plane completion check was a weak count proxy. Replaced with a shared 'outstanding' need set (Arc<Mutex<HashSet<String>>>, type OutstandingNeeds in data_plane.rs) that BOTH carriers claim from: the destination control loop in mod.rs (diff_chunk_and_send_needs) inserts each granted path into the set BEFORE sending its NeedBatch frame; the in-stream FileBegin/TarShardHeader arms claim (remove) inline; and a new NeedListSink decorator (data_plane.rs) claims on the data-plane receive path, rejecting off-need-list paths, duplicate deliveries, and resume block records (non-resume session). Completion at SourceDone is outstanding.is_empty() for both carriers.

F2: wrapped each accepted data-plane socket in StallGuard::new(socket, TRANSFER_STALL_TIMEOUT) before execute_receive_pipeline.

Focus your review on the FIX itself: (1) The insert-before-send ordering claim — is it actually race-free? The control loop (which inserts) and the data-plane receive task (which claims via NeedListSink) run concurrently; verify no claim can execute before its insert, and that no insert can happen after SourceDone is processed. (2) Lock discipline / deadlock: the outstanding mutex is locked in diff_chunk_and_send_needs (held across the filter/map building the NeedBatch, released before the await), in the in-stream arms, in NeedListSink::claim, and at SourceDone; any lock-across-await or poisoning hazard? (3) Does joining the data-plane receive task at SourceDone before checking outstanding.is_empty() correctly drain all claims? (4) Correctness of NeedListSink for tar shards (multiple paths per record) and 0-byte files. (5) StallGuard placement — correct wrapping, no double-guard, matches old push. (6) Any regression to the in-stream carrier path from the shared-set refactor. Also confirm the test count did not drop (1509 -> 1512).

Output a concise markdown findings list — each with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
