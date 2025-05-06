use crate::{
    market::data::OrderBook,
    network::websocket::{DurableWSConfig, DurableWebSocket},
};
use futures_util::{Stream, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use url::Url;

pub fn order_stream(url: &Url) -> impl Stream<Item = OrderBook> {
    let config = DurableWSConfig::default();
    DurableWebSocket::new(url, config)
        .into_stream()
        // Keep only Text messages
        .filter_map(|x| async {
            match x {
                // Don't use Message's own .to_text() method,
                // because it will try to convert binary Messages also.
                Ok(Message::Text(msg)) => msg.as_str().parse().ok(),
                _ => None,
            }
        })
}
