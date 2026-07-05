//! ONE_TRANSFER_PATH unified `Transfer` session — daemon side.
//!
//! otp-1 (D-2026-07-05-4) lands the wire surface only: the RPC, the
//! frame set, and the contract (`docs/TRANSFER_SESSION.md`). The
//! handler in `core.rs` refuses with UNIMPLEMENTED — pinned below —
//! until otp-3/otp-4 land the role-tagged session state machine,
//! which will live in this module.

#[cfg(test)]
mod tests {
    //! otp-1 pin: the `Transfer` RPC exists on the wire (same-build
    //! peers can reach it) and refuses with UNIMPLEMENTED — not
    //! UNKNOWN/NOT_FOUND — until the session lands. If the RPC
    //! vanished from the proto this file would not compile; if the
    //! stub's contract changed this test fails.

    use std::collections::HashMap;

    use blit_core::generated::blit_client::BlitClient;
    use blit_core::generated::blit_server::BlitServer;
    use blit_core::generated::TransferFrame;

    use crate::service::BlitService;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn transfer_rpc_exists_and_refuses_unimplemented() {
        let service = BlitService::with_modules(HashMap::new(), false);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind loopback listener");
        let port = listener.local_addr().expect("listener addr").port();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            blit_core::remote::grpc_server::production_server_builder()
                .add_service(BlitServer::new(service))
                .serve_with_incoming_shutdown(
                    tokio_stream::wrappers::TcpListenerStream::new(listener),
                    async {
                        let _ = shutdown_rx.await;
                    },
                )
                .await
                .expect("in-process daemon serves");
        });

        let mut client = BlitClient::connect(format!("http://127.0.0.1:{port}"))
            .await
            .expect("client connects");
        let outbound = tokio_stream::iter(vec![TransferFrame { frame: None }]);
        let status = match client.transfer(outbound).await {
            Err(status) => status,
            Ok(mut streaming) => streaming
                .get_mut()
                .message()
                .await
                .expect_err("otp-1 stub must refuse"),
        };
        assert_eq!(
            status.code(),
            tonic::Code::Unimplemented,
            "Transfer must exist on the wire and refuse with \
             UNIMPLEMENTED until otp-3/otp-4; got: {status}"
        );

        let _ = shutdown_tx.send(());
        server.await.expect("server task joins");
    }
}
