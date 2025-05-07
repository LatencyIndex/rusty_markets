//! Elemental market datatypes, not specific to any exchange.

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

#[derive(Debug)]
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
