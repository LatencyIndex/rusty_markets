use futures_util::StreamExt;
use rust_decimal::Decimal;
use rusty_markets::{
    algorithm::k_min_tree::KMinTree,
    market::{
        data::{Ask, Bid},
        order_stream,
    },
    network::stream::{expire, splice},
    Config,
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
    // Read program input
    let args: Vec<String> = std::env::args().collect();
    let config = match &args[..] {
        [_program_name, config] => config,
        _ => panic!("Error: exactly one argument required - the config file path"),
    };
    let config =
        std::fs::read_to_string(config).expect("Error: could not read config file {config}");
    let config: Config = serde_json::from_str(&config).expect("Error parsing config file");

    let exchange_labels: Vec<&str> = config
        .exchanges
        .iter()
        .map(|conf| conf.label.as_str())
        .collect();
    let mut asks: KMinTree<Ask> = KMinTree::new(config.nb_orders, config.exchanges.len());
    let mut bids: KMinTree<Bid> = KMinTree::new(config.nb_orders, config.exchanges.len());

    let streams = config
        .exchanges
        .iter()
        .enumerate()
        .map(|(exchange_id, exchange_config)| {
            expire(
                config.order_lifetime,
                order_stream(exchange_config.clone(), config.connection, exchange_id)
                    // Omit errors, so that they don't interfere with expiry messages.
                    .filter_map(|msg| async { msg.ok() }),
            )
            // Tag messages with their source
            .map(move |msg| (exchange_id, msg))
        });
    // Join the tagged streams
    let mut stream = splice(streams);
    while let Some((exchange_id, msg)) = stream.recv().await {
        // msg is Err only if the order has expired.
        // In which case we use the default, empty order book, to clear this exchange's order.
        let order_book = msg.unwrap_or_default();
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
