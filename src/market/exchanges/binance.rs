use crate::{
    market::data::{Order, OrderBook, ParseError},
    network::websocket::{DurableWSConfig, DurableWebSocket},
};
use futures_util::{Stream, StreamExt};
use serde::Deserialize;
use std::str::FromStr;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

// Binance sends ask & bid data in the form of [[price, amount], [price, amount],..],
// where the prices and amounts are numbers encoded as strings.
// The struct is defined to match the data that Binance returns, even if we don't need all of it.
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
#[allow(unused)]
struct BinanceRawOrderBook {
    asks: Vec<Vec<String>>,
    bids: Vec<Vec<String>>,
    lastUpdateId: u128,
}

impl FromStr for BinanceRawOrderBook {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<BinanceRawOrderBook>(s).map_err(Self::Err::from)
    }
}

impl TryFrom<BinanceRawOrderBook> for OrderBook {
    type Error = ParseError;
    fn try_from(value: BinanceRawOrderBook) -> Result<Self, Self::Error> {
        let asks = value
            .asks
            .iter()
            .map(Order::try_from)
            .collect::<Result<Vec<Order>, _>>()?;
        let bids = value
            .bids
            .iter()
            .map(Order::try_from)
            .collect::<Result<Vec<Order>, _>>()?;
        Ok(Self { asks, bids })
    }
}

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
                Ok(Message::Text(msg)) => msg
                    .as_str()
                    .parse::<BinanceRawOrderBook>()
                    .and_then(OrderBook::try_from)
                    .ok(),
                _ => None,
            }
        })
}
