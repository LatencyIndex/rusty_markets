use futures_util::StreamExt;
use rusty_markets::{
    lsl::slice::sort_descending,
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
    fn new(mut book: NamedOrderBook, len: usize) -> Self {
        // Asks and bids should already be sorted correctly,
        // but this is not guaranteed, so we sort again,
        // instead of statically requiring that NamedOrderBook is always sorted.
        sort_descending(&mut book.asks);
        sort_descending(&mut book.bids);
        Self {
            spread: match (book.asks.first(), book.bids.first()) {
                (Some(ask), Some(bid)) => ask.price - bid.price,
                _ => 0.0,
            },
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
