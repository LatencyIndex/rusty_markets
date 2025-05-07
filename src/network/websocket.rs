use crate::network::backoff::ExpBackoff;
use async_stream::stream;
use futures_util::{SinkExt, Stream, StreamExt};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    time::{timeout, Duration, Instant},
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
    #[error("connection lost and deemed unrecoverable")]
    DeadConnection,
}

pub async fn connect_with_timeout<R>(
    duration: Duration,
    request: R,
) -> Result<(WSStream, Response), WSStreamError>
where
    R: IntoClientRequest + Unpin,
{
    timeout(duration, connect_async(request))
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
    pub recv_timeout: Duration,
    /// If longer than this is spent waiting to send a message, the connection is considered dead,
    /// is dropped, and a new connection is attempted.
    pub send_timeout: Duration,
    /// Successfull reconnections don't interrupt this, because a connection is no good
    /// if it can't actually send or receive anything.
    pub fail_timeout: Duration,
    /// Exponential backoff parameters for reconnection attempts.
    pub backoff: ExpBackoff,
}

impl Default for DurableWSConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            recv_timeout: Duration::from_secs(10),
            send_timeout: Duration::from_secs(10),
            fail_timeout: Duration::from_secs(60),
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
    // If Some, the time interval [fail_start, present] starts with a failure, and contains no
    // successfull sends or receives (but may include successfull connects).
    fail_start: Option<Instant>,
}

impl<R: IntoClientRequest + Unpin + Clone> DurableWebSocket<R> {
    pub fn new(request: R, config: DurableWSConfig) -> DurableWebSocket<R> {
        DurableWebSocket {
            request,
            config,
            stream: None,
            fail_start: None,
        }
    }
    // Warning: Drops connection if already connected.
    async fn try_reconnect(&mut self) {
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
    /// Yields None only when the connection is deemed unrecoverable.
    /// However, calling it again will launch new connection attempts,
    /// which may eventually succeed, so a Some could be returned even after a None.
    pub async fn recv(&mut self) -> Option<Result<Message, WSStreamError>> {
        loop {
            match &mut self.stream {
                None => self.try_reconnect().await,
                // Try to receive from connection
                Some(stream) => {
                    match timeout(self.config.recv_timeout, stream.next()).await {
                        // Non-error message received within timeout
                        Ok(Some(Ok(msg))) => {
                            self.fail_start = None;
                            break Some(Ok(msg));
                        }
                        // Anything else is interpreted to mean the connection died/should be re-established.
                        // As such, the connection will be dropped without a close handshake,
                        // This is not strictly true, and there are cases where a close handshake might succeed,
                        // but we ignore that for now.
                        _ => {
                            self.stream = None;
                            if self.fail_start.is_none() {
                                self.fail_start = Some(Instant::now());
                            }
                        }
                    };
                }
            }
            // String of failures is too long - give up.
            if self
                .fail_start
                .is_some_and(|t| t.elapsed() > self.config.fail_timeout)
            {
                break None;
            }
        }
    }
    pub async fn send(&mut self, msg: Message) -> Result<(), WSStreamError> {
        loop {
            match &mut self.stream {
                None => self.try_reconnect().await,
                // Try to receive from connection
                Some(stream) => {
                    match timeout(self.config.send_timeout, stream.send(msg.clone())).await {
                        // Message sent successfully within timeout
                        Ok(Ok(())) => {
                            self.fail_start = None;
                            break Ok(());
                        }
                        // Anything else is interpreted to mean the connection died/should be re-established.
                        // As such, the connection will be dropped without a close handshake,
                        // This is not strictly true, and there are cases where a close handshake might succeed,
                        // but we ignore that for now.
                        _ => {
                            self.stream = None;
                            if self.fail_start.is_none() {
                                self.fail_start = Some(Instant::now());
                            }
                        }
                    };
                }
            }
            // String of failures is too long - give up.
            if self
                .fail_start
                .is_some_and(|t| t.elapsed() > self.config.fail_timeout)
            {
                break Err(WSStreamError::DeadConnection);
            }
        }
    }
    pub fn reset(&mut self) {
        self.stream = None;
        self.fail_start = None;
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
