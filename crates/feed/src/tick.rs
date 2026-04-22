use lob::{OrderId, Price, Qty, Side};

#[derive(Debug, Clone)]
pub enum Tick {
    LimitOrder {
        side: Side,
        price: Price,
        qty: Qty,
        ts: u64,
    },
    Cancel {
        order_id: OrderId,
        ts: u64,
    },
    MarketOrder {
        side: Side,
        qty: Qty,
        ts: u64,
    },
}

pub trait Feed {
    fn next(&mut self) -> Option<Tick>;
}
