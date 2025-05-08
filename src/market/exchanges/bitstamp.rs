//! Bitstamp WebSocket API documentation: https://www.bitstamp.net/websocket/v2/

use crate::{
    market::{
        currencies,
        data::{Order, OrderBook, ParseError},
    },
    network::websocket::{DurableWSConfig, DurableWebSocket},
};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

// Bitstamp sends ask & bid data in the form of [[price, amount], [price, amount],..],
// where the prices and amounts are numbers encoded as strings.
// The struct is defined to match the data that Bitstamp returns, even if we don't need all of it.
#[derive(Deserialize, Debug)]
#[allow(unused)]
struct BitstampRawBidsAsks {
    asks: Vec<Vec<String>>,
    bids: Vec<Vec<String>>,
    microtimestamp: String,
    timestamp: String,
}

// The struct is defined to match the data that Bitstamp returns, even if we don't need all of it.
#[derive(Deserialize, Debug)]
#[allow(unused)]
struct BitstampRawOrderBook {
    channel: String,
    data: BitstampRawBidsAsks,
    event: String,
}

impl FromStr for BitstampRawOrderBook {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<BitstampRawOrderBook>(s).map_err(Self::Err::from)
    }
}

impl TryFrom<BitstampRawOrderBook> for OrderBook {
    type Error = ParseError;
    fn try_from(value: BitstampRawOrderBook) -> Result<Self, Self::Error> {
        let asks = value
            .data
            .asks
            .iter()
            .map(Order::try_from)
            .collect::<Result<Vec<Order>, _>>()?;
        let bids = value
            .data
            .bids
            .iter()
            .map(Order::try_from)
            .collect::<Result<Vec<Order>, _>>()?;
        Ok(Self { asks, bids })
    }
}

#[derive(Serialize)]
struct ChannelDef {
    channel: String,
}

#[derive(Serialize)]
struct SubRequest {
    event: String,
    data: ChannelDef,
}

impl SubRequest {
    fn new(symbol: currencies::SymbolPair) -> Self {
        SubRequest {
            event: "bts:subscribe".to_string(),
            data: ChannelDef {
                channel: format!("order_book_{symbol}"),
            },
        }
    }
    fn to_message(&self) -> Result<Message, serde_json::Error> {
        serde_json::to_string(&self).map(Message::text)
    }
}

// TODO: Don't omit errors.
pub fn order_stream(
    symbol: &currencies::SymbolPair,
    config: DurableWSConfig,
) -> impl Stream<Item = OrderBook> {
    // Hardcode URL, because this is not intended for use with any other address.
    let url = Url::parse("wss://ws.bitstamp.net").unwrap();
    let sub_request = SubRequest::new(symbol.clone()).to_message().unwrap();
    DurableWebSocket::new(url, config, vec![sub_request])
        .into_stream()
        // Keep only Text messages
        .filter_map(|x| async {
            match x {
                // Don't use Message's own .to_text() method,
                // because it will try to convert binary Messages also.
                Message::Text(msg) => msg
                    .as_str()
                    .parse::<BitstampRawOrderBook>()
                    .and_then(OrderBook::try_from)
                    .ok(),
                _ => None,
            }
        })
}
