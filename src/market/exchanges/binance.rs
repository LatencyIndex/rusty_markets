use crate::{
    market::data::OrderBook,
    network::websocket::{DurableWSConfig, DurableWebSocket},
};
use futures_util::{Stream, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use url::Url;

// TODO: Pass DurableWSConfig by parameter
// TODO: A clean way to select channel (i.e. the currency pair, and any other characteristics)
// TODO: Guard against using the wrong domain. Possibly hardcode domain, and use an enum to select subdomain,
//       without exposing any raw URLs to the user, to eliminate possibility of error there.
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
