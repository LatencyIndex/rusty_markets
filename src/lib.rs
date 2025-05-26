pub mod algorithm;
pub mod market;
pub mod network;

use crate::{market::ExchangeConfig, network::websocket::DurableWSConfig};
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub exchanges: Vec<ExchangeConfig>,
    pub connection: DurableWSConfig,
    pub order_lifetime: Duration,
    pub nb_orders: usize,
}
