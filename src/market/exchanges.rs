//! Code specific to individual exchanges.

pub mod binance;
pub mod bitstamp;

use crate::{
    lsl::slice::sort_descending,
    market::{
        currencies::{self},
        data::{NamedAsk, NamedBid, NamedOrderBook, OrderBook},
    },
    network::websocket::DurableWSConfig,
};
use async_stream::stream;
use futures_util::{Stream, StreamExt};
use std::collections::HashMap;

fn merge_order_books(order_books: &HashMap<String, OrderBook>) -> NamedOrderBook {
    let mut asks: Vec<NamedAsk> = order_books
        .iter()
        .flat_map(|(exchange, book)| {
            book.asks
                .iter()
                .map(|&ask| NamedAsk::new(exchange.to_string(), ask))
        })
        .collect();
    let mut bids: Vec<NamedBid> = order_books
        .iter()
        .flat_map(|(exchange, book)| {
            book.asks
                .iter()
                .map(|&bid| NamedBid::new(exchange.to_string(), bid))
        })
        .collect();
    // Asks & bids compare greater if they are better (higher bid, lower ask),
    // so sorting to descending order places the best asks & bids first.
    sort_descending(&mut asks);
    sort_descending(&mut bids);
    NamedOrderBook { asks, bids }
}

/// Stream of the combined order books from binance and bitstamp.
pub fn union_stream(
    symbol: currencies::SymbolPair,
    config: DurableWSConfig,
) -> impl Stream<Item = NamedOrderBook> {
    // TODO: Implement with async tasks and mpsc channels instead.
    let mut binance_stream = Box::pin(binance::order_stream(symbol.clone(), config));
    let mut bitstamp_stream = Box::pin(bitstamp::order_stream(symbol.clone(), config));
    let mut order_books = HashMap::<String, OrderBook>::new();
    stream! {
        loop {
            tokio::select! {
                book = binance_stream.next() => {
                    match book {
                        Some(book) => {
                            order_books.insert("binance".to_string(), book);
                            yield merge_order_books(&order_books);
                        }
                        // Stop if either stream fails.
                        None => {break;}
                    }
                }
                book = bitstamp_stream.next() => {
                    match book {
                        Some(book) => {
                            order_books.insert("bitstamp".to_string(), book);
                            yield merge_order_books(&order_books);
                        }
                        // Stop if either stream fails.
                        None => {break;}
                    }
                }
            }
        }
    }
}
