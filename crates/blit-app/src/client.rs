//! Shared control-plane gRPC client construction.
//!
//! audit-2: every `BlitClient::connect(uri)` in the app/CLI layer had
//! no connect deadline, so an unreachable daemon (slow DNS, hung TCP
//! handshake, network partition) made admin verbs and transfer commands
//! hang for the OS TCP timeout (60-127s). [`connect_with_timeout`]
//! centralizes the connection with a bounded `connect_timeout`, matching
//! the `feedback-server-await-timeouts` principle.

use blit_core::generated::blit_client::BlitClient;
use eyre::{Context, Result};
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};

/// Connect-timeout for control-plane gRPC connections. Bounds the TCP
/// connect (and DNS, on tonic's connector) so an unreachable daemon
/// fails predictably rather than hanging for the OS timeout.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Connect a [`BlitClient`] to `uri` with a bounded connect deadline.
/// Drop-in replacement for `BlitClient::connect(uri)` (which connects
/// with no deadline). Errors carry the URI for context.
///
/// audit-2a round 2: `Endpoint::connect_timeout` alone does NOT bound
/// slow DNS — tonic/hyper-util resolve the name *before* applying the
/// connect timeout, so it only bounds the post-resolution TCP attempt.
/// To honor the "slow DNS / hung handshake / partition" contract we
/// wrap the whole `connect()` future (DNS + TCP) in an outer
/// `tokio::time::timeout`. `connect_timeout` is kept as the inner,
/// TCP-phase bound.
pub async fn connect_with_timeout(uri: String) -> Result<BlitClient<Channel>> {
    let endpoint = Endpoint::from_shared(uri.clone())
        .with_context(|| format!("invalid daemon endpoint {uri}"))?
        .connect_timeout(CONNECT_TIMEOUT);
    let channel = tokio::time::timeout(CONNECT_TIMEOUT, endpoint.connect())
        .await
        .map_err(|_| eyre::eyre!("connecting to {uri} timed out after {CONNECT_TIMEOUT:?}"))?
        .with_context(|| format!("connecting to {uri}"))?;
    Ok(BlitClient::new(channel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_with_timeout_rejects_a_malformed_uri() {
        // A non-URI must surface a clear error (the from_shared parse
        // path), not a panic — exercises the helper's error wiring.
        let err = connect_with_timeout("not a uri".to_string())
            .await
            .expect_err("malformed URI must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("invalid daemon endpoint"),
            "unexpected error: {msg}"
        );
    }
}
