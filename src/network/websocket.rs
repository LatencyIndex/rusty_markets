use crate::network::backoff::ExpBackoff;
use async_stream::stream;
use futures_util::{Stream, StreamExt};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    time::{Duration, Instant},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, client::IntoClientRequest, handshake::client::Response, Message},
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

pub async fn connect_with_timeout<R>(
    duration: Duration,
    request: R,
) -> Result<(WSStream, Response), WSStreamError>
where
    R: IntoClientRequest + Unpin,
{
    tokio::time::timeout(duration, connect_async(request))
        .await
        .map_err(WSStreamError::from)
        .and_then(|x| x.map_err(WSStreamError::from))
}

pub struct DurableWSConfig {
    /// Minimum time before a connection attempt is aborted and retried.
    /// Actual time may be larger, as the waiting periods from exponential backoff
    /// during repeated failures are used to give a connection more time to complete.
    pub connect_timeout: Duration,
    /// If longer than this is spent waiting for a message, the connection is considered dead,
    /// is dropped, and a new connection is attempted.
    pub read_timeout: Duration,
    /// If no messages are received for this amount of time, the connection is considered beyond recovery,
    /// and the stream is ended.
    pub silence_timeout: Duration,
    /// Exponential backoff parameters for reconnection attempts.
    pub backoff: ExpBackoff,
}

impl Default for DurableWSConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(10),
            silence_timeout: Duration::from_secs(60),
            backoff: ExpBackoff::new(Duration::from_secs(2), Duration::from_secs(300)),
        }
    }
}

/// Automatically reconnecting WebSocket
// The irony of building a reliable channel atop TCP is not lost on me.
pub struct DurableWebSocket<R> {
    request: R,
    config: DurableWSConfig,
    stream: Option<WSStream>,
    // Time of last successfull reception, i.e. not an error.
    silence_start: Instant,
}

impl<R: IntoClientRequest + Unpin + Clone> DurableWebSocket<R> {
    pub fn new(request: R, config: DurableWSConfig) -> DurableWebSocket<R> {
        DurableWebSocket {
            request,
            config,
            stream: None,
            silence_start: Instant::now(),
        }
    }
    /// Yields None only when the connection is deemed unrecoverable.
    /// However, calling it again will launch new connection attempts,
    /// which may eventually succeed, so a Some could be returned even after a None.
    pub async fn recv(&mut self) -> Option<Result<Message, WSStreamError>> {
        loop {
            match &mut self.stream {
                // Try to establish connection
                None => {
                    // Exponential backoff to not spam the server
                    self.config.backoff.wait().await;
                    self.config.backoff.start_attempt();
                    // Instead of wasting the time between when the connection attempt times out,
                    // and when the next attempt can start due to backoff,
                    // we use it to give the connection more time to complete.
                    let timeout = self
                        .config
                        .backoff
                        .remaining_wait()
                        .unwrap_or(self.config.connect_timeout)
                        .max(self.config.connect_timeout);
                    // Try to connect
                    self.stream = connect_with_timeout(timeout, self.request.clone())
                        .await
                        .ok()
                        .map(|(stream, _response)| stream);
                    // Update backoff
                    if self.stream.is_some() {
                        self.config.backoff.reset();
                    }
                }
                // Try to receive from connection
                Some(stream) => {
                    match tokio::time::timeout(self.config.read_timeout, stream.next()).await {
                        // Valid message received within timeout
                        Ok(Some(Ok(msg))) => {
                            self.silence_start = Instant::now();
                            break Some(Ok(msg));
                        }
                        // Anything else is interpreted to mean the connection died/should be re-established.
                        // As such, the connection will be dropped without a close handshake,
                        // This is not strictly true, and there are cases where a close handshake might succeed,
                        // but we ignore that for now.
                        _ => {
                            self.stream = None;
                            // Sender has been radio silent for too long - give up.
                            // Must be _after_ a read attempt, so that we don't give up just because
                            // we didn't try to read for a long time, which makes it look like the sender was silent.
                            if self.silence_start.elapsed() > self.config.silence_timeout {
                                break None;
                            }
                        }
                    };
                }
            }
        }
    }
    pub fn reset(&mut self) {
        self.stream = None;
        self.silence_start = Instant::now();
        self.config.backoff.reset();
    }
    // Implementing the Stream trait is 'non-trivial', so we settle for converting into a new Stream object.
    pub fn into_stream(mut self) -> impl Stream<Item = Result<Message, WSStreamError>> {
        stream! {
            while let Some(msg) = self.recv().await {
                yield msg;
            }
        }
    }
}
