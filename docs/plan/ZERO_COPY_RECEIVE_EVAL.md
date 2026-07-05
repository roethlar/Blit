# zero_copy.rs FAST evaluation — splice in the receive pipeline

**Status**: Active (verdict ratified — outcome is deletion, executed in w8-1)
**Created**: 2026-06-12
**Supersedes**: nothing
**Decision ref**: D-2026-06-12-1 (owner agreed 2026-06-12: delete)

## Goal

The w8-1b slice deliverable: an evidence-based answer to "delete
`zero_copy.rs`, or keep it and wire `splice` into the receive pipeline for
a FAST win?" — so the w8-1 dead-code sweep (which excluded `zero_copy.rs`
pending this evaluation) can close out the module either way.

**Recommendation: delete.** Rationale below; the deletion itself should
fold into w8-1 (it touches `lib.rs`, which has a pending sentinel). The
owner's review verdict on this doc is the decision gate.

## What exists (all dead)

`crates/blit-core/src/zero_copy.rs`, 219 lines, zero callers (only
reference is the `pub mod` in `lib.rs`). `cfg(all(unix, not(macos)))` —
empty module on macOS (the dev platform) and Windows.

- `splice_from_socket_to_file` / `splice_chunk_blocking`: socket → kernel
  pipe → file, 8 MiB chunks, run under `spawn_blocking`.
- `sendfile_chunk`: file → socket helper (send side, same dead class).
- `Pipe` RAII wrapper (pipe2 + O_CLOEXEC).

## Where it *could* wire (what makes this tempting)

The receive byte path is structurally splice-friendly:

- The data plane is **raw TCP** (token handshake, no TLS, no compression),
  and `execute_receive_pipeline` parses tag-framed records where
  `DATA_PLANE_RECORD_FILE` is followed by an **exact `file_size`-byte run**
  of payload (`pipeline.rs:216-238`, via `(&mut *socket).take(file_size)`).
  Splice could replace exactly that run.
- `receive_stream_double_buffered` (`data_plane.rs:569`) does **no inline
  hashing** of streamed payload bytes — it is a pure userspace
  socket→buffer→file relay (double-buffered, read/write overlapped via
  `tokio::join!`). No checksum dependency blocks the swap on this path.

## Why deletion still wins

1. **The existing code is not shippable.** The socket fd inside a tokio
   `TcpStream` is nonblocking; `splice_chunk_blocking` handles the
   resulting EAGAIN with a `thread::sleep(10ms)` retry loop inside
   `spawn_blocking` (`zero_copy.rs:96-102`) — a busy-wait that would
   throttle exactly the high-rate transfers it is meant to accelerate. A
   real implementation needs an `AsyncFd` readiness loop; this module
   would be rewritten, not reused. So "keep it because it's already
   written" buys nothing.
2. **The abstraction fights it.** `execute_receive_pipeline` is generic
   over `R: AsyncRead` on purpose — the same code drives the gRPC
   fallback path, wrapped readers (`Take`), and tests' in-memory readers.
   Splice needs the raw fd, so wiring it means a downcast/raw-fd side
   channel plus a permanently-maintained fallback for every non-raw-TCP
   caller (gRPC fallback can never splice).
3. **The win is speculative and narrow.** The double-buffered relay
   already overlaps read and write; the userspace copy at 10 GbE
   (~1.25 GB/s) costs a fraction of one core against ~10-30 GB/s/core
   memcpy bandwidth, and the receive side is typically wire- or
   disk-bound first. The benefit exists (CPU headroom on multi-stream
   10+ GbE receive, Linux only, raw-file payloads only — tar shards
   must be un-tarred in userspace regardless), but it is unmeasured:
   the 10+ GbE bench rig (`BENCHMARK_10GBE_PLAN.md`) hasn't run.
   Carrying 219 lines of dead unsafe libc code (the only `unsafe`
   network/file I/O in blit-core) against an unmeasured win fails the
   SIMPLE test the design review applied everywhere else.
4. **Surface cost is real.** Progress cadence (`byte_progress` per
   chunk), StallGuard semantics, dry-run wire-drain, resume/block
   paths, and the platform matrix (macOS/Windows excluded by cfg) all
   need parallel splice-path handling and tests.

## If FAST evidence appears later

**Gate declared MET by the owner, 2026-07-05 (D-2026-07-05-3)**: a
UniFi UNAS 8 Pro daemon target is CPU-bound below 10 GbE even from SSD
cache. The design below is now the input to the unparked work, which
lands as a runtime-selected write strategy inside ONE_TRANSFER_PATH's
unified receive sink, sequenced after its cutover slice. The dead
module's deletion (w8-1) still stands — this is a rewrite against the
unified sink, not a revival.

Revisit only after the 10 GbE benchmark plan runs and shows receive-side
CPU saturation with the buffered relay. The design then should be: an
`AsyncFd`-readiness splice loop owned by `data_plane.rs` (next to
`receive_stream_double_buffered`, same progress contract), selected at
runtime when the reader is a raw `TcpStream` and the payload is
`DATA_PLANE_RECORD_FILE`, with the buffered relay as the universal
fallback. Git history preserves today's module for reference; nothing
else needs to survive.

## Non-goals

- Implementing splice now (no measurement justifies it).
- Touching the send side (`sendfile_chunk` dies with the module; a send
  side sendfile evaluation would follow the same revisit gate).

## Acceptance criteria

- [ ] Owner verdict on this doc: ratify **delete** (fold into w8-1) or
      direct a FAST implementation plan instead.
- [ ] On delete: w8-1 removes `zero_copy.rs` + the `lib.rs` export and
      this doc's Status flips to Shipped with the deletion commit noted.

## Slices

1. (conditional, inside w8-1) delete `zero_copy.rs` + `pub mod zero_copy;`
   — blocked until the w5-1 sentinel (lib.rs) is graded.

## Open questions

- None beyond the owner verdict itself.
