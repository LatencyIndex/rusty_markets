use crate::network::backoff::{ExpBackoff, ExpBackoffConfig};
use async_stream::stream;
use futures_util::{SinkExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct DurableWSConfig {
    /// If longer than this is spent waiting to receive a message, the connection is considered dead,
    /// is dropped, and a re-connect and re-send are attempted.
    pub recv_timeout: Duration,
    /// Exponential backoff parameters for reconnection attempts.
    pub backoff: ExpBackoffConfig,
}

impl Default for DurableWSConfig {
    fn default() -> Self {
        Self {
            recv_timeout: Duration::from_secs(10),
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

#[derive(Debug)]
pub enum DurableMessage {
    /// Successfully received a message from the underlying websocket.
    Recv(Message),
    /// Timeout elapsed while trying to receive a new message.
    /// Will keep trying to receive, but no further timeouts
    /// will be sent until at least one valid message is received again.
    TimeoutElapsed,
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
    pub async fn recv(&mut self) -> DurableMessage {
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
                        .max(self.config.recv_timeout);
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
                    match timeout(self.config.recv_timeout, stream.next()).await {
                        // Non-error message received within timeout
                        Ok(Some(Ok(msg))) => return DurableMessage::Recv(msg),
                        // Anything else is interpreted as the stream needing a reconnect.
                        _ => {
                            self.stream = None;
                            return DurableMessage::TimeoutElapsed;
                        }
                    }
                }
            }
        }
    }
    /// Convert into a stream via try_recv.
    // Implementing the Stream trait is 'non-trivial', so we settle for converting into a new Stream object.
    pub fn into_stream(mut self) -> impl Stream<Item = DurableMessage> {
        stream! {
            loop {
                yield self.recv().await;
            }
        }
    }
}
