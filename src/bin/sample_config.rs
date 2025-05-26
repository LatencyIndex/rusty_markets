use rusty_markets::{
    market::{data::OrderBookFormat, ExchangeConfig},
    network::websocket::DurableWSConfig,
    Config,
};
use tokio::time::Duration;

fn main() {
    let config = Config {
        exchanges: vec![
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
                    r#"{"event":"bts:subscribe","data":{"channel":"order_book_ethbtc"}}"#
                        .to_string(),
                ],
                order_book_format: OrderBookFormat::Bitstamp,
            },
        ],
        connection: DurableWSConfig::default(),
        order_lifetime: Duration::from_secs(8),
        nb_orders: 10,
    };
    let json = serde_json::to_string_pretty(&config).unwrap();
    println!("{json}");
}
