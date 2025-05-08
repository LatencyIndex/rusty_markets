use futures_util::StreamExt;
use rust_l2::{
    market::{
        currencies,
        exchanges::{binance, bitstamp},
    },
    network::websocket::DurableWSConfig,
};
use std::pin::pin;
use url::Url;

async fn _binance_example() -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse("wss://stream.binance.com:9443/ws/ethbtc@depth20@100ms")?;
    let mut stream = pin!(binance::order_stream(&url, DurableWSConfig::default()).take(3));
    while let Some(x) = stream.next().await {
        println!("{x:#?}");
    }
    Ok(())
}

async fn _bitstamp_example() -> Result<(), Box<dyn std::error::Error>> {
    let symbol = currencies::SymbolPair::new("ethbtc".to_string())?;
    let mut stream = pin!(bitstamp::order_stream(symbol, DurableWSConfig::default())
        .await
        .take(3));
    while let Some(x) = stream.next().await {
        println!("{x:#?}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    _bitstamp_example().await
}
