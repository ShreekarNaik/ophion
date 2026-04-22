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
        let spread_ticks = match book.spread() {
            Some(s) => s as f64,
            None => return vec![],
        };
        let half_spread_ticks = spread_ticks / 2.0;
        // Fee cost expressed in ticks: at mid price M ticks, fee = M * bps/10000
        let mid_ticks = book.mid().unwrap_or(0) as f64;
        let fee_ticks = mid_ticks * self.fee_bps / 10_000.0;
        let hurdle = half_spread_ticks + fee_ticks + self.threshold;

        let pred = features.predicted_return; // ticks; same units as hurdle
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
