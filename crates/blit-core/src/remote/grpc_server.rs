//! Production-shaped tonic server construction.
//!
//! Single owner of the gRPC control-plane HTTP/2 keepalive policy, so
//! the real daemon and every in-process test server start from the
//! same builder and cannot drift apart (w9-3,
//! tests-fake-server-config-skew — before this module, every fake
//! tonic server in the test tree was a bare `Server::builder()` with
//! no keepalive, while production set it, so wire tests exercised a
//! server shaped differently from every deployed daemon).

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

/// The `Server::builder()` every Blit gRPC server starts from — the
/// production daemon and in-process test fakes alike — so the
/// keepalive policy above is applied in exactly one place.
pub fn production_server_builder() -> Server {
    Server::builder()
        .http2_keepalive_interval(Some(HTTP2_KEEPALIVE_INTERVAL))
        .http2_keepalive_timeout(Some(HTTP2_KEEPALIVE_TIMEOUT))
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
}
