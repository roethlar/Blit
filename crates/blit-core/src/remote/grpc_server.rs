//! Production-shaped tonic server construction.
//!
//! Single owner of the gRPC control-plane HTTP/2 policy — keepalive, and
//! the flow-control windows of both ends — so the real daemon and every
//! in-process test server start from the same builder and cannot drift
//! apart (w9-3, tests-fake-server-config-skew — before this module, every
//! fake tonic server in the test tree was a bare `Server::builder()` with
//! no keepalive, while production set it, so wire tests exercised a
//! server shaped differently from every deployed daemon). Clients take
//! their half of the window policy through [`crate::client::endpoint`].

use std::time::Duration;

use tonic::transport::Server;

/// HTTP/2 keepalive PING interval for Blit gRPC servers.
///
/// audit-1 (owner decision 2026-05-23): a subscriber (TUI F2 /
/// `jobs watch`) that vanishes mid-stream — crash, network partition,
/// killed laptop lid — would otherwise leave the daemon holding the
/// gRPC stream + broadcast Receiver + spawned forwarder task forever,
/// because TCP alone doesn't notice a silently-dead peer. Keepalive
/// PINGs idle connections at this interval.
pub const HTTP2_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

/// How long an unanswered keepalive PING may hang before the
/// connection is reaped. Healthy idle subscribers answer PINGs and
/// stay connected (Subscribe is legitimately silent during quiet
/// periods), so this reclaims leaked resources without the reconnect
/// churn an app-level "no events for N seconds" close would cause.
pub const HTTP2_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(20);

/// The HTTP/2 CONNECTION window of every Blit gRPC connection, on both
/// ends: the largest the protocol allows (h2-small-frames, 2026-11-20).
///
/// The `h2` library closes a connection with `ENHANCE_YOUR_CALM`
/// ("too_many_data_frames") once the small DATA frames — under 256 bytes —
/// that the application has not yet taken have used up a budget, and that
/// budget is HALF THE CONNECTION WINDOW (never under 25,600). The rule
/// came with h2 0.4.16; this workspace took 0.4.20 on 2026-10-17 for
/// RUSTSEC-2026-0258. hyper and tonic offer no setting for the budget
/// itself, only for the window it is derived from.
///
/// Blit's streams are made of small frames — a file header, a manifest
/// entry, one `find` or `du` row — so with the libraries' default
/// connection windows (5 MiB on a client, 1 MiB on a server) any receiver
/// slower than its sender met that limit:
///
/// - a push of 200,000 empty files to a daemon on the same machine failed
///   after 43 ms with 1,159 files written ("transfer transport peer
///   closed");
/// - `blit find` and `blit du -a` into a reader that paused for two
///   seconds, as a pager does, ended with "h2 protocol error: error
///   reading a body from connection" and part of the listing.
///
/// With the window at its maximum the budget is 1 GiB, more than a whole
/// stream window of one-byte frames can use
/// (`a_stream_window_of_the_smallest_frames_fits_the_budget`), so flow
/// control stops the sender before the budget is spent. The connection
/// window limits nothing the stream windows do not: a stream still
/// buffers at most its own window, which is unchanged.
pub const HTTP2_CONNECTION_WINDOW: u32 = (1 << 31) - 1;

/// The per-stream window a Blit client advertises: hyper's client
/// default, stated so the rule on [`HTTP2_CONNECTION_WINDOW`] can be
/// checked against it.
pub const HTTP2_CLIENT_STREAM_WINDOW: u32 = 2 * 1024 * 1024;

/// The per-stream window a Blit server advertises: hyper's server
/// default, stated for the same reason.
pub const HTTP2_SERVER_STREAM_WINDOW: u32 = 1024 * 1024;

/// The `Server::builder()` every Blit gRPC server starts from — the
/// production daemon and in-process test fakes alike — so the
/// keepalive and window policies above are applied in exactly one place.
pub fn production_server_builder() -> Server {
    Server::builder()
        .http2_keepalive_interval(Some(HTTP2_KEEPALIVE_INTERVAL))
        .http2_keepalive_timeout(Some(HTTP2_KEEPALIVE_TIMEOUT))
        .initial_stream_window_size(Some(HTTP2_SERVER_STREAM_WINDOW))
        .initial_connection_window_size(Some(HTTP2_CONNECTION_WINDOW))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the owner-decided keepalive values (audit-1, 2026-05-23):
    /// interval 30s / timeout 20s. Changing them is an owner call,
    /// not a refactor side effect.
    #[test]
    fn keepalive_values_match_owner_decision() {
        assert_eq!(HTTP2_KEEPALIVE_INTERVAL, Duration::from_secs(30));
        assert_eq!(HTTP2_KEEPALIVE_TIMEOUT, Duration::from_secs(20));
    }

    /// h2-small-frames: the budget h2 derives from the connection window
    /// (half of it) is larger than the most a full stream window can use.
    /// A DATA frame of `p` bytes (under 256) uses `256 - p` of the budget
    /// and `p` of the stream's window, so the worst a window `W` can cost
    /// is `W * 255`, with one-byte frames; flow control then stops the
    /// sender. Two such streams fit a client connection, four a server's.
    #[test]
    fn a_stream_window_of_the_smallest_frames_fits_the_budget() {
        let budget = u64::from(HTTP2_CONNECTION_WINDOW) / 2;
        for (window, streams) in [
            (HTTP2_CLIENT_STREAM_WINDOW, 2),
            (HTTP2_SERVER_STREAM_WINDOW, 4),
        ] {
            assert!(
                u64::from(window) * 255 * streams <= budget,
                "{streams} streams of window {window} can use {} of a budget of {budget}",
                u64::from(window) * 255 * streams
            );
        }
        // The largest window HTTP/2 has, and the stream windows as hyper
        // has them by default: this policy changes only the connection's.
        assert_eq!(HTTP2_CONNECTION_WINDOW, i32::MAX as u32);
        assert_eq!(HTTP2_CLIENT_STREAM_WINDOW, 2 << 20);
        assert_eq!(HTTP2_SERVER_STREAM_WINDOW, 1 << 20);
    }
}
