pub mod algorithm;
pub mod market;
pub mod network;

use crate::{
    market::{data::OrderBookFormat, ExchangeConfig},
    network::websocket::DurableWSConfig,
};
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub exchanges: Vec<ExchangeConfig>,
    pub connection: DurableWSConfig,
    pub order_lifetime: Duration,
    pub nb_orders: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
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
        }
    }
}
