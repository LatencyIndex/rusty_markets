use crate::market::data::{Ask, Bid, OrderBook, ParseError};
use rust_decimal::Decimal;
use serde::Deserialize;

// Form in which Binance sends order book data.
// Ask & bid data is in the form of [[price, amount], [price, amount],..],
// where the prices and amounts are numbers encoded as strings.
// Only elements we need are included - any extra data is ignored by the parser,
// so that the format is potentially compatible with other exchanges as well.
#[derive(Deserialize, Debug)]
pub struct RawOrderBook {
    asks: Vec<Vec<Decimal>>,
    bids: Vec<Vec<Decimal>>,
    // Ignore unused elements:
    // lastUpdateId: u128
}

impl RawOrderBook {
    pub fn to_order_book(self, exchange_id: usize, book_id: u64) -> Result<OrderBook, ParseError> {
        let asks = self
            .asks
            .iter()
            .enumerate()
            .map(|(order_id, order)| Ask::from_raw(exchange_id, book_id, order_id, order))
            .collect::<Result<Vec<Ask>, _>>()?;
        let bids = self
            .bids
            .iter()
            .enumerate()
            .map(|(order_id, order)| Bid::from_raw(exchange_id, book_id, order_id, order))
            .collect::<Result<Vec<Bid>, _>>()?;
        Ok(OrderBook { asks, bids })
    }
}
