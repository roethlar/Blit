Reviewed sha: `5ded6c92c21aee414950c3d7b04297cfd7cdd740`

Reopened.

`connect_with_timeout` only applies `Endpoint::connect_timeout(CONNECT_TIMEOUT)` before awaiting `Endpoint::connect()` (`crates/blit-app/src/client.rs:23`). In tonic 0.14.5's default HTTP connector, that timeout is passed to hyper-util's `HttpConnector::set_connect_timeout`, but hyper-util resolves DNS before it constructs the TCP connector that uses the timeout. The timeout therefore bounds socket connect attempts after name resolution, but it does not bound the slow-DNS case called out by this finding.

This leaves the admin verbs and bridge path still able to hang on slow DNS despite the contract text saying "slow DNS, hung TCP handshake, network partition" are bounded. The helper needs an outer deadline around the whole `Endpoint::connect()` future (or an equivalent DNS-aware wrapper), not only `Endpoint::connect_timeout`.

Review gates: not run; reopened from source review before gates because the timeout mechanism does not satisfy the finding contract.
