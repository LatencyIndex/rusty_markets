//! Elemental market datatypes, not specific to any exchange.

use rust_decimal::Decimal;
use serde::Serialize;
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

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub price: Decimal,
    pub amount: Decimal,
}

impl TryFrom<&Vec<Decimal>> for Order {
    type Error = ParseError;
    /// Parse a [price, amount] array of length 2.
    fn try_from(value: &Vec<Decimal>) -> Result<Self, Self::Error> {
        match *value.as_slice() {
            // Ordering is very important, to not confuse price & amount.
            [price, amount] => Ok(Order { price, amount }),
            _ => Err(ParseError::LengthError),
        }
    }
}

#[derive(Debug)]
#[allow(non_snake_case)]
pub struct OrderBook {
    pub asks: Vec<Order>,
    pub bids: Vec<Order>,
}

#[derive(Serialize, Clone)]
pub struct NamedBid {
    pub exchange: String,
    pub price: Decimal,
    pub amount: Decimal,
}

impl NamedBid {
    pub fn new(exchange: String, order: Order) -> Self {
        Self {
            exchange,
            price: order.price,
            amount: order.amount,
        }
    }
}

impl PartialEq for NamedBid {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for NamedBid {}

impl PartialOrd for NamedBid {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The better bid (greater price, then greater amount) compares smaller, i.e. it comes first.
impl Ord for NamedBid {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price
            .cmp(&other.price)
            .reverse()
            .then(self.amount.cmp(&other.amount))
    }
}

#[derive(Serialize, Clone)]
pub struct NamedAsk {
    pub exchange: String,
    pub price: Decimal,
    pub amount: Decimal,
}

impl NamedAsk {
    pub fn new(exchange: String, order: Order) -> Self {
        Self {
            exchange,
            price: order.price,
            amount: order.amount,
        }
    }
}

impl PartialEq for NamedAsk {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for NamedAsk {}

impl PartialOrd for NamedAsk {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The better ask (lower price, then greater amount) compares smaller, i.e. it comes first.
impl Ord for NamedAsk {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price
            .cmp(&other.price)
            .then(self.amount.cmp(&other.amount))
    }
}

#[derive(Clone, Serialize)]
pub struct NamedOrderBook {
    pub asks: Vec<NamedAsk>,
    pub bids: Vec<NamedBid>,
}
