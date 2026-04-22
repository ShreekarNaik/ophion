use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrderBookError {
    #[error("order {0} not found")]
    OrderNotFound(u64),
    #[error("insufficient liquidity: needed {needed}, available {available}")]
    InsufficientLiquidity { needed: u64, available: u64 },
    #[error("invalid price: {0}")]
    InvalidPrice(i64),
    #[error("zero quantity")]
    ZeroQuantity,
}
