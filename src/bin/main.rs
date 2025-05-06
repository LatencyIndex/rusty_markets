use futures_util::StreamExt;
use rust_l2::market::exchanges::binance;
use std::pin::pin;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse("wss://stream.binance.com:9443/ws/ethbtc@depth20@100ms")?;
    let mut stream = pin!(binance::order_stream(&url).take(3));
    while let Some(x) = stream.next().await {
        println!("{x:?}");
    }

    Ok(())
}
