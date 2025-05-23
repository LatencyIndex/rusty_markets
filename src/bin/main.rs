use futures_util::StreamExt;
use rust_decimal::Decimal;
use rusty_markets::{
    algorithm::k_min_tree::KMinTree,
    market::{
        data::{Ask, Bid, OrderBookFormat},
        order_stream, ExchangeConfig,
    },
    network::{stream::splice, websocket::DurableWSConfig},
};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct DisplayOrder<'a> {
    pub exchange: &'a str,
    pub price: Decimal,
    pub amount: Decimal,
}

// Market data in the requested output form.
#[derive(Clone, Serialize)]
pub struct DisplayBook<'a> {
    pub spread: Decimal,
    pub asks: Vec<DisplayOrder<'a>>,
    pub bids: Vec<DisplayOrder<'a>>,
}

impl<'a> DisplayBook<'a> {
    // Asks & bids must be sorted, or spread will be calculated incorrectly
    fn new(asks: &[Ask], bids: &[Bid], labels: &'a [&str]) -> Self {
        Self {
            spread: match (asks.first(), bids.first()) {
                (Some(ask), Some(bid)) => ask.price - bid.price,
                _ => Decimal::ZERO,
            },
            asks: asks
                .iter()
                .map(|ask| DisplayOrder {
                    exchange: labels[ask.exchange_id],
                    price: ask.price,
                    amount: ask.amount,
                })
                .collect(),
            bids: bids
                .iter()
                .map(|bid| DisplayOrder {
                    exchange: labels[bid.exchange_id],
                    price: bid.price,
                    amount: bid.amount,
                })
                .collect(),
        }
    }
}

#[tokio::main]
async fn main() {
    let exchange_configs = [
        ExchangeConfig {
            label: "binance".to_string(),
            url: "wss://stream.binance.com:9443/ws/ethbtc@depth20@100ms".to_string(),
            inits: Vec::new(),
            order_book_format: OrderBookFormat::Binance,
        },
        ExchangeConfig {
            label: "bitstamp".to_string(),
            url: "wss://ws.bitstamp.net".to_string(),
            inits: vec![
                r#"{"event":"bts:subscribe","data":{"channel":"order_book_ethbtc"}}"#.to_string(),
            ],
            order_book_format: OrderBookFormat::Bitstamp,
        },
    ];
    let connection_config = DurableWSConfig::default();
    let nb_orders: usize = 10;

    let exchange_labels: Vec<&str> = exchange_configs
        .iter()
        .map(|conf| conf.label.as_str())
        .collect();
    let mut asks: KMinTree<Ask> = KMinTree::new(nb_orders, exchange_configs.len());
    let mut bids: KMinTree<Bid> = KMinTree::new(nb_orders, exchange_configs.len());

    let streams = exchange_configs
        .iter()
        .enumerate()
        .map(|(exchange_id, exchange_config)| {
            order_stream(exchange_config.clone(), connection_config, exchange_id)
                // Tag messages with their source
                .map(move |msg| (exchange_id, msg))
        });
    // Join the tagged streams
    let mut stream = splice(streams);
    while let Some((exchange_id, msg)) = stream.recv().await {
        if let Ok(order_book) = msg {
            let asks_changed = asks.update_leaf(exchange_id, order_book.asks);
            let bids_changed = bids.update_leaf(exchange_id, order_book.bids);
            let book_changed = asks_changed || bids_changed;
            if book_changed {
                let book = DisplayBook::new(asks.get(), bids.get(), &exchange_labels);
                let json = serde_json::to_string_pretty(&book).unwrap();
                println!("{json}");
            }
        }
    }
}
