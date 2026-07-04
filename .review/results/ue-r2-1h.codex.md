# codex raw review output — ue-r2-1h

Invocation: `codex exec -s read-only -c 'plugins."superpowers@openai-curated".enabled=false' "<review prompt over commits 2a13f53 + 48c5a11 + 9f37a7a>"`
(rerun with stdin closed after the first invocation hung on
`Reading additional input from stdin...`). This codex build streams its
full session transcript (~694 KB) to stdout; per the established
record format (cf. ue-r2-1a..1g codex.md files) only the final
findings message is preserved below, verbatim. tokens used: 214,431.

---

- `crates/blit-daemon/src/service/delegated_pull.rs:208` - **High** - `metadata_only` is accepted by delegated pull validation and then forwarded unchanged at `:362`. A client can set it on a real delegated transfer; the source sends headers+summary only, while the destination receive loop creates/finalizes empty files as success. This violates the "PullSync-only" field contract and byte-identical invariant.

- `crates/blit-core/src/remote/pull.rs:1174` - **Medium** - `RemoteFileStream` polls `Streaming<ServerPullMessage>` directly and recursively skips frames at `:1182`/`:1215`, so `open_remote_file` is outside the `recv_fallback_message`/future StallGuard chokepoint and can stack-grow on long ready runs of skipped or empty frames. This is a RELIABLE regression surface for stalled or malformed peers.

- `docs/API.md:14` - **Low** - Current API docs still advertise removed `rpc Pull(PullRequest) returns (stream PullChunk)` and define `PullRequest`/`PullChunk` later in the same file. The live proto/code deletion is complete, but public docs are stale.

VERDICT: **NEEDS FIXES** for the committed stack.
