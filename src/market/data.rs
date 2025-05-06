use serde::Deserialize;
use std::str::FromStr;
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

// The exchange sends data in this form - asks and bids look like
// [[price, amount], [price, amount],..],
// and the prices and amounts are strings that represent numbers .
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct RawOrderBook {
    asks: Vec<Vec<String>>,
    bids: Vec<Vec<String>>,
    lastUpdateId: u128,
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
    pub lastUpdateId: u128,
}

impl TryFrom<RawOrderBook> for OrderBook {
    type Error = ParseError;
    fn try_from(value: RawOrderBook) -> Result<Self, Self::Error> {
        let asks = value
            .asks
            .iter()
            .map(Order::try_from)
            .collect::<Result<Vec<Order>, _>>()?;
        let bids = value
            .bids
            .iter()
            .map(Order::try_from)
            .collect::<Result<Vec<Order>, _>>()?;
        Ok(Self {
            asks,
            bids,
            lastUpdateId: value.lastUpdateId,
        })
    }
}

impl FromStr for OrderBook {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<RawOrderBook>(s)
            .map_err(Self::Err::from)
            .and_then(OrderBook::try_from)
    }
}
