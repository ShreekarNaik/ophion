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
    /// Called after an aggressive fill (strategy sent TakeMarket). `fill.side` = aggressor side.
    fn on_fill(&mut self, fill: &Fill);
    /// Called by the engine after a successful `Action::Submit`, with the assigned OrderId.
    fn on_submitted(&mut self, _id: OrderId) {}
    /// Called when a synthetic market order passively fills one of the strategy's resting limits.
    /// `fill.side` is the *aggressor*'s side (opposite of the strategy's maker side).
    fn on_passive_fill(&mut self, _fill: &Fill) {}
    /// Current signed inventory (positive = long). Used by TUI and tests.
    fn inventory(&self) -> i64 {
        0
    }
}
