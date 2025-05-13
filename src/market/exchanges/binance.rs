use crate::{
    market::{
        currencies,
        data::{Order, OrderBook, ParseError},
    },
    network::websocket::{DurableWSConfig, DurableWebSocket},
};
use futures_util::{Stream, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

// Binance sends ask & bid data in the form of [[price, amount], [price, amount],..],
// where the prices and amounts are numbers encoded as strings.
// The struct is defined to match the data that Binance returns, even if we don't need all of it.
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
#[allow(unused)]
struct BinanceRawOrderBook {
    asks: Vec<Vec<Decimal>>,
    bids: Vec<Vec<Decimal>>,
    // TODO: Is this the right number type?
    lastUpdateId: u128,
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

// TODO: Don't omit errors.
pub fn order_stream(
    symbol: currencies::SymbolPair,
    config: DurableWSConfig,
) -> impl Stream<Item = OrderBook> {
    let url = Url::parse(&format!(
        "wss://stream.binance.com:9443/ws/{symbol}@depth20@100ms"
    ))
    .unwrap();
    DurableWebSocket::new(url, config, vec![])
        .into_stream()
        // Keep only Text messages
        .filter_map(|x| async {
            match x {
                // Don't use Message's own .to_text() method,
                // because it will try to convert binary Messages also.
                Message::Text(msg) => serde_json::from_str::<BinanceRawOrderBook>(msg.as_str())
                    .map_err(ParseError::from)
                    .and_then(OrderBook::try_from)
                    .ok(),
                _ => None,
            }
        })
}
