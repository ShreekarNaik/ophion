#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Qty(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone)]
pub struct Fill {
    pub order_id: OrderId,
    pub side: Side,
    pub price: Price,
    pub qty: Qty,
    pub ts: u64,
}

impl Price {
    pub fn ticks(self) -> i64 {
        self.0
    }
}

impl Qty {
    pub fn get(self) -> u64 {
        self.0
    }
}
