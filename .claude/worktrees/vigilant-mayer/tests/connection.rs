use blit_core::generated::blit_client::BlitClient;
use std::time::Duration;

#[tokio::test]
async fn test_server_connection() {
    // This test requires the `blitd` server to be running separately.
    // In a real CI environment, we would start the server as a background process.

    println!("Attempting to connect to blitd on localhost:50051...");

    let connection_future = BlitClient::connect("http://[::1]:50051");

    match tokio::time::timeout(Duration::from_secs(5), connection_future).await {
        Ok(Ok(_client)) => {
            println!("Successfully connected to blitd.");
            // The connection was successful, the test passes.
        }
        Ok(Err(e)) => {
            panic!("Failed to connect to server: {}. Is blitd running?", e);
        }
        Err(_) => {
            panic!("Connection attempt timed out after 5 seconds. Is blitd running?");
        }
    }
}
