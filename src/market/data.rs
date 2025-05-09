//! Elemental market datatypes, not specific to any exchange.

use serde::Serialize;
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error("incorrect array length")]
    LengthError,
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub price: f64,
    pub amount: f64,
}

impl TryFrom<&Vec<String>> for Order {
    type Error = ParseError;
    fn try_from(value: &Vec<String>) -> Result<Self, Self::Error> {
        let parsed: Result<Vec<f64>, ParseError> = value
            .iter()
            .map(|x| x.parse().map_err(ParseError::from))
            .collect();
        match parsed {
            Ok(parsed) => match *parsed.as_slice() {
                // Ordering is very important, to not confuse price & amount.
                [price, amount] => Ok(Order { price, amount }),
                _ => Err(ParseError::LengthError),
            },
            Err(e) => Err(e),
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
    pub price: f64,
    pub amount: f64,
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

/// The better bid (greater price, then greater amount) compares greater.
impl Ord for NamedBid {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price
            .total_cmp(&other.price)
            .then(self.amount.total_cmp(&other.amount))
    }
}

#[derive(Serialize, Clone)]
pub struct NamedAsk {
    pub exchange: String,
    pub price: f64,
    pub amount: f64,
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

/// The better ask (lower price, then greater amount) compares greater.
impl Ord for NamedAsk {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price
            .total_cmp(&other.price)
            .reverse()
            .then(self.amount.total_cmp(&other.amount))
    }
}

#[derive(Clone, Serialize)]
pub struct NamedOrderBook {
    pub asks: Vec<NamedAsk>,
    pub bids: Vec<NamedBid>,
}
