use crate::market::data::{Ask, Bid, OrderBook, ParseError};
use rust_decimal::Decimal;
use serde::Deserialize;

// Form in which Bitstamp sends order book data.
// Ask & bid data is in the form of [[price, amount], [price, amount],..],
// where the prices and amounts are numbers encoded as strings.
// Only elements we need are included - any extra data is ignored by the parser,
// so that the format is potentially compatible with other exchanges as well.
#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct RawOrderBook {
    data: RawBidsAsks,
    // Ignore unused elements:
    // channel: String,
    // event: String,
}

#[derive(Deserialize, Debug)]
struct RawBidsAsks {
    asks: Vec<Vec<Decimal>>,
    bids: Vec<Vec<Decimal>>,
    // Ignore unused elements:
    // microtimestamp: String,
    // timestamp: String,
}

impl RawOrderBook {
    pub fn to_order_book(self, exchange_id: usize, book_id: u64) -> Result<OrderBook, ParseError> {
        let asks = self
            .data
            .asks
            .iter()
            .enumerate()
            .map(|(order_id, order)| Ask::from_raw(exchange_id, book_id, order_id, order))
            .collect::<Result<Vec<Ask>, _>>()?;
        let bids = self
            .data
            .bids
            .iter()
            .enumerate()
            .map(|(order_id, order)| Bid::from_raw(exchange_id, book_id, order_id, order))
            .collect::<Result<Vec<Bid>, _>>()?;
        Ok(OrderBook { asks, bids })
    }
}
