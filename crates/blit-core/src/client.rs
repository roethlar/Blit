//! Shared control-plane gRPC client construction.
//!
//! audit-2: every `BlitClient::connect(uri)` in the app/CLI layer had
//! no connect deadline, so an unreachable daemon (slow DNS, hung TCP
//! handshake, network partition) made admin verbs and transfer commands
//! hang for the OS TCP timeout (60-127s). [`connect_with_timeout`]
//! centralizes the connection and bounds the whole `connect()` future
//! (DNS resolution + TCP handshake) in an outer `tokio::time::timeout`,
//! matching the `feedback-server-await-timeouts` principle. (An inner
//! `Endpoint::connect_timeout` alone does NOT bound slow DNS — see the
//! function docs.)

use crate::generated::blit_client::BlitClient;
use eyre::{Context, Result};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tonic::body::Body;
use tonic::codegen::{http, Bytes, Service};
use tonic::transport::{Channel, Endpoint};

/// Connect deadline for control-plane gRPC connections. Used two ways in
/// [`connect_with_timeout`]: as the outer `tokio::time::timeout` bounding
/// the whole `connect()` future (DNS + TCP), and as the inner
/// `Endpoint::connect_timeout` bounding just the post-resolution TCP
/// phase. The outer bound is what makes an unreachable daemon (incl. slow
/// DNS) fail predictably rather than hanging for the OS timeout.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// The [`Endpoint`] every Blit gRPC client connects through: `uri` with
/// the HTTP/2 window policy of [`crate::remote::grpc_server`]
/// (h2-small-frames). A client built from a bare `Endpoint` keeps the
/// library's 5 MiB connection window, and with it a limit on unread small
/// frames that a listing read slowly, or a pull of many small files,
/// runs into — the connection is closed under it.
pub fn endpoint(uri: String) -> std::result::Result<Endpoint, tonic::transport::Error> {
    use crate::remote::grpc_server::{HTTP2_CLIENT_STREAM_WINDOW, HTTP2_CONNECTION_WINDOW};
    Ok(Endpoint::from_shared(uri)?
        .initial_stream_window_size(HTTP2_CLIENT_STREAM_WINDOW)
        .initial_connection_window_size(HTTP2_CONNECTION_WINDOW))
}

/// What a Blit client talks through: the channel, behind the bound on
/// what one connection may hold unread ([`BoundedResponses`]).
pub type BlitChannel = BoundedResponses<Channel>;

/// `channel` behind that bound. Every client blit builds goes through
/// here ([`connect_with_timeout`], the transfer session's own connect).
pub fn bounded(channel: Channel) -> BlitChannel {
    BoundedResponses::new(channel)
}

/// cr-ecm8-4: at most [`HTTP2_MAX_CONCURRENT_STREAMS`] responses at a time
/// whose bodies have neither been read to their end nor dropped; a request
/// beyond that waits for one of them, as a request beyond the stream limit
/// waits for a stream.
///
/// This is what bounds the payload a client holds unread on one
/// connection, and what the small-frame budget is proved against
/// ([`crate::remote::grpc_server::HTTP2_CONNECTION_WINDOW`]): each
/// response holds at most its stream window unread, so the connection
/// holds at most this many windows. The stream limit alone did not give
/// that. It counts streams that are OPEN, and a stream that has ended
/// gives up its place while its body can still sit in the connection,
/// unread — nine one-row listings were held unread on one channel of
/// eight streams. With the connection window at its maximum nothing else
/// stood in the way of a tenth, or a thousandth.
///
/// The place is held by the response's BODY: taken before the request is
/// sent, given back when the body has been read to its end, or is
/// dropped. A front end that keeps more than eight responses unread on
/// one channel must read or drop one, or open another channel.
///
/// [`HTTP2_MAX_CONCURRENT_STREAMS`]: crate::remote::grpc_server::HTTP2_MAX_CONCURRENT_STREAMS
#[derive(Debug, Clone)]
pub struct BoundedResponses<S> {
    inner: S,
    places: Arc<Semaphore>,
}

impl<S> BoundedResponses<S> {
    pub fn new(inner: S) -> Self {
        let most = crate::remote::grpc_server::HTTP2_MAX_CONCURRENT_STREAMS as usize;
        Self {
            inner,
            places: Arc::new(Semaphore::new(most)),
        }
    }
}

impl<S> Service<http::Request<Body>> for BoundedResponses<S>
where
    S: Service<http::Request<Body>, Response = http::Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = http::Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: http::Request<Body>) -> Self::Future {
        // The service that was polled ready is the one that is called.
        let ready = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, ready);
        let places = Arc::clone(&self.places);
        Box::pin(async move {
            let place = places
                .acquire_owned()
                .await
                .expect("the places are never closed");
            let response = inner.call(request).await?;
            Ok(response.map(|body| {
                Body::new(Held {
                    body,
                    place: Some(place),
                })
            }))
        })
    }
}

/// A response body with the place it holds on its connection.
struct Held {
    body: Body,
    place: Option<OwnedSemaphorePermit>,
}

impl http_body::Body for Held {
    type Data = Bytes;
    type Error = tonic::Status;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
        let polled = Pin::new(&mut self.body).poll_frame(cx);
        // Read to its end: nothing of it is left in the connection.
        if matches!(polled, Poll::Ready(None))
            || (polled.is_ready() && http_body::Body::is_end_stream(&self.body))
        {
            self.place = None;
        }
        polled
    }

    fn is_end_stream(&self) -> bool {
        http_body::Body::is_end_stream(&self.body)
    }

    fn size_hint(&self) -> http_body::SizeHint {
        http_body::Body::size_hint(&self.body)
    }
}

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
pub async fn connect_with_timeout(uri: String) -> Result<BlitClient<BlitChannel>> {
    let endpoint = endpoint(uri.clone())
        .with_context(|| format!("invalid daemon endpoint {uri}"))?
        .connect_timeout(CONNECT_TIMEOUT);
    let channel = tokio::time::timeout(CONNECT_TIMEOUT, endpoint.connect())
        .await
        .map_err(|_| eyre::eyre!("connecting to {uri} timed out after {CONNECT_TIMEOUT:?}"))?
        .with_context(|| format!("connecting to {uri}"))?;
    Ok(BlitClient::new(bounded(channel)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A service that answers every request at once, with a body of
    /// `frames` data frames.
    #[derive(Clone)]
    struct Answers {
        frames: usize,
    }

    impl Service<http::Request<Body>> for Answers {
        type Response = http::Response<Body>;
        type Error = std::convert::Infallible;
        type Future = std::future::Ready<std::result::Result<Self::Response, Self::Error>>;

        fn poll_ready(
            &mut self,
            _: &mut TaskContext<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: http::Request<Body>) -> Self::Future {
            let frames: Vec<std::result::Result<http_body::Frame<Bytes>, tonic::Status>> = (0
                ..self.frames)
                .map(|_| Ok(http_body::Frame::data(Bytes::from_static(b"row"))))
                .collect();
            std::future::ready(Ok(http::Response::new(Body::new(Frames(frames.into())))))
        }
    }

    /// A body of ready frames.
    struct Frames(
        std::collections::VecDeque<std::result::Result<http_body::Frame<Bytes>, tonic::Status>>,
    );

    impl http_body::Body for Frames {
        type Data = Bytes;
        type Error = tonic::Status;
        fn poll_frame(
            mut self: Pin<&mut Self>,
            _: &mut TaskContext<'_>,
        ) -> Poll<Option<std::result::Result<http_body::Frame<Bytes>, Self::Error>>> {
            Poll::Ready(self.0.pop_front())
        }
    }

    async fn next_frame(
        body: &mut Body,
    ) -> Option<std::result::Result<http_body::Frame<Bytes>, tonic::Status>> {
        std::future::poll_fn(|cx| http_body::Body::poll_frame(Pin::new(&mut *body), cx)).await
    }

    /// cr-ecm8-4: a channel holds a place for each response until its body
    /// has been read to its end or dropped — whether or not its stream has
    /// ended. Every response here is COMPLETE the moment it is answered,
    /// which is the case the stream limit does not cover: the ninth
    /// request waits while eight complete responses sit unread, and goes
    /// through when one of them is dropped, or read to its end.
    #[tokio::test]
    async fn a_ninth_response_waits_while_eight_are_unread() {
        let most = crate::remote::grpc_server::HTTP2_MAX_CONCURRENT_STREAMS as usize;
        let mut channel = BoundedResponses::new(Answers { frames: 2 });
        let ask = |channel: &mut BoundedResponses<Answers>| {
            channel.call(http::Request::new(Body::empty()))
        };
        /// The answer, when it comes within a moment; `None` while the
        /// request is still waiting.
        async fn soon<F: Future + Unpin>(asked: &mut F) -> Option<F::Output> {
            tokio::time::timeout(Duration::from_millis(100), asked)
                .await
                .ok()
        }

        let mut unread = Vec::new();
        for _ in 0..most {
            unread.push(ask(&mut channel).await.unwrap());
        }
        // A clone of the client is the same connection, and the same places.
        let mut other = channel.clone();
        let mut ninth = Box::pin(ask(&mut other));
        assert!(
            soon(&mut ninth).await.is_none(),
            "eight complete, unread responses: the ninth request must wait"
        );

        // Dropped: its place is free.
        unread.pop();
        let ninth = soon(&mut ninth)
            .await
            .expect("a place was given back")
            .unwrap();
        unread.push(ninth);
        let mut tenth = Box::pin(ask(&mut channel));
        assert!(soon(&mut tenth).await.is_none(), "eight again");

        // Read part of the way: still unread. Read to its end, and kept:
        // its place is free.
        let mut read = unread.remove(0);
        assert!(next_frame(read.body_mut()).await.is_some());
        assert!(
            soon(&mut tenth).await.is_none(),
            "one frame of two is not the end"
        );
        assert!(next_frame(read.body_mut()).await.is_some());
        assert!(next_frame(read.body_mut()).await.is_none());
        soon(&mut tenth).await.expect("read to its end").unwrap();
        drop(read);
    }

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
