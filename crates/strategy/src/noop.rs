use crate::traits::{Action, Strategy};
use lob::{Fill, OrderBook};
use signal::Features;

pub struct NoopStrategy;

impl Strategy for NoopStrategy {
    fn on_book(&mut self, _book: &OrderBook, _features: &Features, _ts: u64) -> Vec<Action> {
        vec![]
    }
    fn on_fill(&mut self, _fill: &Fill) {}
}
