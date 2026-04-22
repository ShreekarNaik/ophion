use lob::{Fill, OrderBook, OrderId, Price, Qty, Side};
use signal::Features;

#[derive(Debug, Clone)]
pub enum Action {
    Submit { side: Side, price: Price, qty: Qty },
    Cancel(OrderId),
    TakeMarket { side: Side, qty: Qty },
}

pub trait Strategy {
    fn on_book(&mut self, book: &OrderBook, features: &Features, ts: u64) -> Vec<Action>;
    fn on_fill(&mut self, fill: &Fill);
}
