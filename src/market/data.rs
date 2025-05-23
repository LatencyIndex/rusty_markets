//! Elemental market datatypes, not specific to any exchange.

pub mod binance;
pub mod bitstamp;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(transparent)]
    ParseDecimalError(#[from] rust_decimal::Error),
    #[error("incorrect array length")]
    LengthError,
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Ask {
    pub exchange_id: usize,
    pub book_id: u64,
    pub order_id: usize,
    pub price: Decimal,
    pub amount: Decimal,
}

impl Ask {
    fn from_raw(
        exchange_id: usize,
        book_id: u64,
        order_id: usize,
        value: &Vec<Decimal>,
    ) -> Result<Self, ParseError> {
        // Parse a [price, amount] array of length 2.
        match *value.as_slice() {
            // Ordering is very important, to not confuse price & amount.
            [price, amount] => Ok(Self {
                exchange_id,
                book_id,
                order_id,
                price,
                amount,
            }),
            _ => Err(ParseError::LengthError),
        }
    }
}

impl PartialEq for Ask {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Ask {}

impl PartialOrd for Ask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The better ask (lower price, then greater amount) compares smaller, i.e. it comes first.
impl Ord for Ask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price
            .cmp(&other.price)
            .then(self.amount.cmp(&other.amount).reverse())
            // Following are not needed for ordering, but are needed for equality,
            // because two orders should compare equal only when they represent the same order.
            .then(self.exchange_id.cmp(&other.exchange_id))
            .then(self.book_id.cmp(&other.book_id))
            .then(self.order_id.cmp(&other.order_id))
    }
}

// Bid is basically identical to Ask, but this similarity is only incidental, not intrinsic
// (i.e. it is possible to imagine a different implementation/data for bids),
// so we do not attempt to avoid this code duplication.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Bid {
    pub exchange_id: usize,
    pub book_id: u64,
    pub order_id: usize,
    pub price: Decimal,
    pub amount: Decimal,
}

impl Bid {
    fn from_raw(
        exchange_id: usize,
        book_id: u64,
        order_id: usize,
        value: &Vec<Decimal>,
    ) -> Result<Self, ParseError> {
        // Parse a [price, amount] array of length 2.
        match *value.as_slice() {
            // Ordering is very important, to not confuse price & amount.
            [price, amount] => Ok(Self {
                exchange_id,
                book_id,
                order_id,
                price,
                amount,
            }),
            _ => Err(ParseError::LengthError),
        }
    }
}

impl PartialEq for Bid {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Bid {}

impl PartialOrd for Bid {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The better bid (greater price, then greater amount) compares smaller, i.e. it comes first.
impl Ord for Bid {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price
            .cmp(&other.price)
            .reverse()
            .then(self.amount.cmp(&other.amount).reverse())
            // Following are not needed for ordering, but are needed for equality,
            // because two orders should compare equal only when they represent the same order.
            .then(self.exchange_id.cmp(&other.exchange_id))
            .then(self.book_id.cmp(&other.book_id))
            .then(self.order_id.cmp(&other.order_id))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OrderBookFormat {
    Binance,
    Bitstamp,
}

#[derive(Clone, Debug, Default, Serialize)]
#[allow(non_snake_case)]
pub struct OrderBook {
    pub asks: Vec<Ask>,
    pub bids: Vec<Bid>,
}

impl OrderBook {
    pub fn from_raw(
        msg: &str,
        exchange_id: usize,
        book_id: u64,
        format: OrderBookFormat,
    ) -> Result<Self, ParseError> {
        match format {
            OrderBookFormat::Binance => serde_json::from_str::<binance::RawOrderBook>(msg)
                .map_err(ParseError::from)
                .and_then(|raw_order_book| raw_order_book.to_order_book(exchange_id, book_id)),
            OrderBookFormat::Bitstamp => serde_json::from_str::<bitstamp::RawOrderBook>(msg)
                .map_err(ParseError::from)
                .and_then(|raw_order_book| raw_order_book.to_order_book(exchange_id, book_id)),
        }
    }
}
