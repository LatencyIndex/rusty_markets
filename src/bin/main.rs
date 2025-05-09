use futures_util::StreamExt;
use rusty_markets::{
    market::{
        currencies::{self},
        data::{NamedAsk, NamedBid, NamedOrderBook},
        exchanges,
    },
    network::websocket::DurableWSConfig,
};
use serde::Serialize;
use std::pin::pin;

#[derive(Clone, Serialize)]
pub struct DisplayBook {
    pub spread: f64,
    pub asks: Vec<NamedAsk>,
    pub bids: Vec<NamedBid>,
}

impl DisplayBook {
    fn new(book: NamedOrderBook, len: usize) -> Self {
        let ask = book.asks.first();
        let bid = book.bids.first();
        Self {
            spread: ask
                .and_then(|a| bid.map(|b| a.price - b.price))
                .unwrap_or(0.0),
            asks: book.asks.into_iter().take(len).collect(),
            bids: book.bids.into_iter().take(len).collect(),
        }
    }
}

#[tokio::main]
async fn main() {
    let symbol = currencies::SymbolPair::new("ethbtc".to_string()).unwrap();
    let config = DurableWSConfig::default();
    let n = 9;

    let mut stream = pin!(exchanges::union_stream(symbol, config)
        .map(|book| DisplayBook::new(book, 10))
        .take(n));

    while let Some(book) = stream.next().await {
        println!("{}", serde_json::to_string_pretty(&book).unwrap());
    }
}
