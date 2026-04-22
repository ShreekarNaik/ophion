use crate::traits::{Action, Strategy};
use lob::{OrderBook, Qty, Side};
use signal::Features;

pub struct TakerStrategy {
    pub threshold: f64,
    pub fee_bps: f64,
    pub position_limit: i64,
    pub inventory: i64,
}

impl TakerStrategy {
    pub fn new(threshold: f64, fee_bps: f64, position_limit: i64) -> Self {
        Self {
            threshold,
            fee_bps,
            position_limit,
            inventory: 0,
        }
    }
}

impl Strategy for TakerStrategy {
    fn on_book(&mut self, book: &OrderBook, features: &Features, _ts: u64) -> Vec<Action> {
        let spread = match book.spread() {
            Some(s) => s as f64 * 0.01, // ticks → dollars
            None => return vec![],
        };
        let half_spread = spread / 2.0;
        let fee = self.fee_bps / 10_000.0;
        let hurdle = half_spread + fee + self.threshold;

        let pred = features.ofi[0]; // raw OFI as proxy until predictor wired in Phase 3
        let mut actions = vec![];

        if pred > hurdle && self.inventory < self.position_limit {
            actions.push(Action::TakeMarket {
                side: Side::Bid,
                qty: Qty(1),
            });
        } else if pred < -hurdle && self.inventory > -self.position_limit {
            actions.push(Action::TakeMarket {
                side: Side::Ask,
                qty: Qty(1),
            });
        }
        actions
    }

    fn on_fill(&mut self, fill: &lob::Fill) {
        match fill.side {
            Side::Bid => self.inventory += fill.qty.get() as i64,
            Side::Ask => self.inventory -= fill.qty.get() as i64,
        }
    }
}
