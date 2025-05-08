use crate::network::backoff::{ExpBackoff, ExpBackoffConfig};
use async_stream::stream;
use futures_util::{SinkExt, Stream, StreamExt};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    time::{timeout, Duration},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, client::IntoClientRequest, Message},
    MaybeTlsStream, WebSocketStream,
};

type WSStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Error, Debug)]
pub enum WSStreamError {
    #[error("websocket connection attempt timed out")]
    Timeout(#[from] tokio::time::error::Elapsed),
    #[error(transparent)]
    WebSocket(#[from] tungstenite::error::Error),
}

/// Open a WebSocket stream and send the given initialization messages through it.
/// Returns Ok only if all messages were sent successfully.
async fn init_stream<R, I>(request: R, inits: I) -> Result<WSStream, WSStreamError>
where
    R: IntoClientRequest + Unpin + Clone,
    I: IntoIterator<Item = Message>,
{
    let (mut stream, _response) = connect_async(request).await?;
    let mut inits = futures_util::stream::iter(inits).map(Ok);
    stream.send_all(&mut inits).await?;
    Ok(stream)
}

#[derive(Clone, Copy)]
pub struct DurableWSConfig {
    /// If longer than this is spent waiting to receive a message, the connection is considered dead,
    /// is dropped, and a re-connect and re-send are attempted.
    pub retry_timeout: Duration,
    /// If a receive attempt does not complete within this duration,
    /// the connection is considered irreparable, and no further attempts are made.
    pub final_timeout: Duration,
    /// Exponential backoff parameters for reconnection attempts.
    pub backoff: ExpBackoffConfig,
}

impl Default for DurableWSConfig {
    fn default() -> Self {
        Self {
            retry_timeout: Duration::from_secs(10),
            final_timeout: Duration::from_secs(60),
            backoff: ExpBackoffConfig {
                wait_min: Duration::from_secs(2),
                wait_max: Duration::from_secs(60),
            },
        }
    }
}

/// Automatically reconnecting WebSocket.
// The irony of building a reliable channel atop TCP is not lost on me.
pub struct DurableWebSocket<R> {
    request: R,
    config: DurableWSConfig,
    stream: Option<WSStream>,
    inits: Vec<Message>,
    backoff: ExpBackoff,
}

impl<R: IntoClientRequest + Unpin + Clone> DurableWebSocket<R> {
    /// On each reconnect, inits are sent through the stream.
    /// Intended to initialize the connection by e.g. subscribing to channels.
    pub fn new(request: R, config: DurableWSConfig, inits: Vec<Message>) -> Self {
        DurableWebSocket {
            request,
            config,
            stream: None,
            inits,
            backoff: ExpBackoff::new(config.backoff),
        }
    }
    /// Continuously tries to receive a message, reconnecting as necessary.
    pub async fn recv(&mut self) -> Message {
        loop {
            match &mut self.stream {
                // Disconnected - try to connect
                None => {
                    // Exponential backoff to not spam the server
                    self.backoff.wait().await;
                    self.backoff.start_attempt();
                    // Instead of wasting the time between when the connection attempt times out,
                    // and when the next attempt can start due to backoff,
                    // we use it to give the connection more time to complete.
                    let extended_timeout = self
                        .backoff
                        .remaining_wait()
                        .unwrap_or(Duration::ZERO)
                        .max(self.config.retry_timeout);
                    // Try to reconnect
                    self.stream = timeout(
                        extended_timeout,
                        init_stream(self.request.clone(), self.inits.clone()),
                    )
                    .await
                    .ok()
                    .and_then(Result::ok);
                    // Update backoff
                    if self.stream.is_some() {
                        self.backoff.reset();
                    }
                }
                // Connected - try to receive a message
                Some(stream) => {
                    match timeout(self.config.retry_timeout, stream.next()).await {
                        // Non-error message received within timeout
                        Ok(Some(Ok(msg))) => return msg,
                        // Anything else is interpreted as the stream needing a reconnect.
                        _ => self.stream = None,
                    }
                }
            }
        }
    }
    /// Yields None only when the connection is deemed unrecoverable.
    /// However, calling it again will launch new connection attempts,
    /// which may eventually succeed, so a Some could be returned even after a None.
    pub async fn try_recv(&mut self) -> Option<Message> {
        timeout(self.config.final_timeout, self.recv()).await.ok()
    }
    /// Drop connection and reset exponential backoff.
    pub fn reset(&mut self) {
        self.stream = None;
        self.backoff.reset();
    }
    /// Convert into a stream via try_recv.
    // Implementing the Stream trait is 'non-trivial', so we settle for converting into a new Stream object.
    pub fn into_stream(mut self) -> impl Stream<Item = Message> {
        stream! {
            while let Some(msg) = self.try_recv().await {
                yield msg;
            }
        }
    }
}
