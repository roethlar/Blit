# release-win-ci-handshake-stall-test — make the Windows timeout guard deterministic

**Severity**: MEDIUM — the latest published Windows CI job is red and release
artifact jobs are skipped, while the failure demonstrates a false test premise
rather than the intended production timeout behavior.
**Status**: Open
**Branch**: `master`
**Commit**: Pending

## Evidence

GitHub CI run `29584631185` at published head
`dcf924538400006c2bd6acc87348085b935f4852` passed Linux, macOS, formatting,
and clippy. Windows failed only
`remote::transfer::socket::tests::dial_token_write_stall_times_out_bounded_and_retryable`.

The test sends a 64 MiB handshake to a loopback peer that never reads and
assumes `write_all` must block until its 200 ms timeout. On the hosted Windows
runner the socket stack accepted the entire write, so `dial_data_plane_with_timeouts`
returned `Ok(TcpStream)` and `expect_err("stalled handshake must time out")`
panicked. The current local head retains the same test shape in
`crates/blit-core/src/remote/transfer/socket.rs`.

## Predicted observable failure

Windows CI can remain red or vary with socket-buffer behavior even when the
production timeout is correct. Because the release-build matrix depends on the
test jobs, no release artifacts are produced.

## What

Replace the operating-system buffer-size assumption with a deterministic
blocked-write setup that exercises the same token-write timeout and preserves
the retryable `TimedOut` assertions on every supported platform.

## Known gaps

No fix is implemented. The exact local head has not run on hosted Windows.
