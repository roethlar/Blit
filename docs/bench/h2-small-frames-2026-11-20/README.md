# Small HTTP/2 frames close the connection — evidence, 2026-11-20

**Status:** Complete. Evidence for the fix recorded in
`.review/findings/h2-small-frames.md`.

## Bottom line

On the tree as it stood (`585d3599`), a push of 200,000 empty files to a
daemon on the same machine failed after 35 ms with 768 files written, and
`blit find` into a reader that paused for two seconds failed three times in
three. With one HTTP/2 window setting changed on both ends, the push, a
no-op mirror, the pull back and the paused listing all complete, and the
speed of a large file is unchanged.

## Cause, read from the library

`h2` 0.4.20 (`src/proto/streams/counts.rs`, `record_data_frame`; in this
workspace's lock since `3f15f835`, 2026-10-17, taken for RUSTSEC-2026-0258):
a received DATA frame under 256 bytes uses `256 - length` of a budget, given
back when the application takes the frame; when the budget is gone the
connection is closed with `ENHANCE_YOUR_CALM`, debug data
`too_many_data_frames`. The budget is half the connection's target window,
never under 25,600 (`src/proto/connection.rs`, `DataFrameBudget::resolve`).
hyper 1.11 and tonic 0.14.6 pass no setting for the budget; they do pass the
window. Their default connection windows are 5 MiB for a client and 1 MiB
for a server, so about 10,000 unread small frames end a client's connection
and about 2,000 a server's.

The error as the client's library reported it (one diagnostic build that
printed the whole status): `Status { code: ResourceExhausted, message: "h2
protocol error: error reading a body from connection", source:
Some(hyper::Error(Body, Error { kind: GoAway(b"too_many_data_frames",
ENHANCE_YOUR_CALM, Library) })) }`.

Whether a real tree produces small frames depends on timing: tonic puts into
one frame every message that is ready when it looks, so a sender that is
polled slower than it produces sends large frames and is safe. That is why
the suite never met this (debug builds, small trees) and why the old `du`
succeeded on one run and failed on the next.

## Method

Linux host `continuum`, tmpfs, loopback, release builds, each build's own
daemon. "before" is `585d3599`; "after" is the same tree with the fix.
The tree: 2,000 directories of 100 empty files.

- `probe_transfer_small.py` — push the tree, mirror it again, pull it back.
- `probe_slow_reader.py` — `blit find` with nothing read for two seconds.
- `probe_big_file.py` — one 2,048 MiB file pushed and pulled, best of three,
  run twice for each build.

## Result

`evidence.txt` is the scripts' output.

| | before | after |
|---|---|---|
| push 200,000 empty files | exit 1 after 35 ms, 768 files, "transfer transport peer closed" | exit 0, 2,871 ms, 200,000 files |
| mirror again (nothing to do) | exit 1 after 16 ms | exit 0, 1,504 ms |
| pull them back | (only 768 files were there) | exit 0, 2,547 ms, 200,000 files |
| `find`, reader paused 2 s | exit 1, 3 of 3, about 17,500 of 202,001 lines | exit 0, 3 of 3, 202,001 lines |
| one 2 GiB file, push | 287 ms, 282 ms | 286 ms, 285 ms |
| one 2 GiB file, pull | 240 ms, 245 ms | 244 ms, 244 ms |

## Limits

- Linux only, loopback only. No run on Windows or macOS, none across a real
  network.
- The first sign was the old `du` failing on its second run during the
  utility-verbs speed measurement; that request is being replaced and its
  numbers are not recorded here.
