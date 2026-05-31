Reviewed sha: `e60ae29c9755b040a476fa05cfedc3987ac78ab9`

Reopened.

The round-2 function body now wraps the full `Endpoint::connect()` future in `tokio::time::timeout`, so the slow-DNS behavior is fixed. The remaining blocker is stale documentation in the same helper file: `crates/blit-app/src/client.rs:6-8` still says the helper centralizes the connection with a bounded `connect_timeout`, and `crates/blit-app/src/client.rs:15-17` still says the timeout bounds DNS "on tonic's connector".

That statement is exactly the false premise from the first reopen: tonic/hyper-util resolve DNS before applying `HttpConnector::set_connect_timeout`. The docs need to say that DNS is bounded by the outer `tokio::time::timeout`, while `Endpoint::connect_timeout` is only the inner TCP-phase bound.

Review gates: not run; reopened from source review before gates because the remaining issue is documentation correctness.
