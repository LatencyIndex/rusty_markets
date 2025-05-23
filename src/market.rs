pub mod data;

use crate::{
    market::data::{OrderBook, OrderBookFormat},
    network::websocket::{DurableMessage, DurableWSConfig, DurableWebSocket},
};
use futures_util::{stream, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use url::Url;

#[derive(Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    /// Label that will be given to data coming from this source.
    pub label: String,
    pub url: String,
    /// Initialization messages to send upon establishing a connection.
    /// Also sent when connection is automatically re-established after a disconnect.
    pub inits: Vec<String>,
    /// Format in which the exchange sends order books.
    pub order_book_format: OrderBookFormat,
}

/// Stream of order books from an exchange.
/// Parses text Messages into OrderBooks, and yields everything else,
/// including failed parses, as errors containing the original message.
pub fn order_stream(
    exchange_config: ExchangeConfig,
    connection_config: DurableWSConfig,
    exchange_id: usize,
) -> impl Stream<Item = Result<OrderBook, DurableMessage>> {
    let url = Url::parse(&exchange_config.url).unwrap();
    DurableWebSocket::new(
        url,
        connection_config,
        exchange_config
            .inits
            .into_iter()
            .map(Message::text)
            .collect(),
    )
    .into_stream()
    // Add an id to each message. A simple enumerate() risks overflowing
    .zip(stream::iter(u64::MIN..u64::MAX).cycle())
    .map(move |(msg, msg_id)| {
        match msg {
            // Don't use Message's own .to_text() method,
            // because it will try to convert binary Messages also.
            DurableMessage::Recv(Message::Text(txt)) => {
                OrderBook::from_raw(
                    txt.as_str(),
                    exchange_id,
                    msg_id,
                    exchange_config.order_book_format,
                )
                // Default to original message if parsing fails
                .map_err(|_| DurableMessage::Recv(Message::Text(txt)))
            }
            other => Err(other),
        }
    })
}
