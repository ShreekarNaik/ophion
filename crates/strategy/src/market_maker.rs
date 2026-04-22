use std::collections::VecDeque;

use crate::traits::{Action, Strategy};
use lob::{Fill, OrderBook, OrderId, Price, Qty, Side};
use signal::Features;

/// Inventory-aware market maker (Avellaneda–Stoikov-lite).
///
/// Each `on_book` call:
/// 1. Cancels both resting quotes.
/// 2. Computes skew = `−γ·inventory + β·predicted_return`.
/// 3. Posts bid at `best_bid − offset + skew` and ask at `best_ask + offset + skew`,
///    subject to the inventory bound guard.
pub struct MarketMaker {
    /// Ticks added inside the spread on each side before skew.
    pub offset: i64,
    /// Inventory skew coefficient (γ). Higher → more aggressive de-risking.
    pub gamma: f64,
    /// Predicted-return coefficient (β).
    pub beta: f64,
    /// Hard inventory bound. Strategy will not submit a quote that would push
    /// `|inventory|` beyond this value.
    pub inventory_bound: i64,
    pub inventory: i64,
    pub fee_bps: f64,
    bid_id: Option<OrderId>,
    ask_id: Option<OrderId>,
    /// Tracks which side each pending `on_submitted` call corresponds to.
    pending_submits: VecDeque<Side>,
}

impl MarketMaker {
    pub fn new(offset: i64, gamma: f64, beta: f64, inventory_bound: i64, fee_bps: f64) -> Self {
        Self {
            offset,
            gamma,
            beta,
            inventory_bound,
            inventory: 0,
            fee_bps,
            bid_id: None,
            ask_id: None,
            pending_submits: VecDeque::new(),
        }
    }
}

impl Strategy for MarketMaker {
    fn on_book(&mut self, book: &OrderBook, features: &Features, _ts: u64) -> Vec<Action> {
        let (best_bid, best_ask) = match (book.best_bid(), book.best_ask()) {
            (Some(b), Some(a)) => (b, a),
            _ => return vec![],
        };

        let mut actions: Vec<Action> = Vec::with_capacity(4);

        // Cancel existing quotes
        if let Some(id) = self.bid_id.take() {
            actions.push(Action::Cancel(id));
        }
        if let Some(id) = self.ask_id.take() {
            actions.push(Action::Cancel(id));
        }

        // Skew: positive inventory → skew down (encourage selling)
        let skew_ticks = (-self.gamma * self.inventory as f64
            + self.beta * features.predicted_return)
            .round() as i64;

        let bid_price_ticks = best_bid.ticks() - self.offset + skew_ticks;
        let ask_price_ticks = best_ask.ticks() + self.offset + skew_ticks;

        // Only quote if the resulting book-side price is positive
        // and doesn't violate the inventory bound.
        if bid_price_ticks > 0 && self.inventory < self.inventory_bound {
            actions.push(Action::Submit {
                side: Side::Bid,
                price: Price::from_ticks(bid_price_ticks),
                qty: Qty(1),
            });
            self.pending_submits.push_back(Side::Bid);
        }

        if ask_price_ticks > 0 && self.inventory > -self.inventory_bound {
            actions.push(Action::Submit {
                side: Side::Ask,
                price: Price::from_ticks(ask_price_ticks),
                qty: Qty(1),
            });
            self.pending_submits.push_back(Side::Ask);
        }

        actions
    }

    fn on_fill(&mut self, _fill: &Fill) {
        // MarketMaker doesn't issue aggressive orders; no-op.
    }

    fn on_submitted(&mut self, id: OrderId) {
        match self.pending_submits.pop_front() {
            Some(Side::Bid) => self.bid_id = Some(id),
            Some(Side::Ask) => self.ask_id = Some(id),
            None => {}
        }
    }

    fn on_passive_fill(&mut self, fill: &Fill) {
        // fill.side is the aggressor; we are the maker on the opposite side.
        match fill.side {
            Side::Bid => {
                // Someone bought from our ask → we sold
                self.inventory -= fill.qty.get() as i64;
                self.ask_id = None; // order consumed
            }
            Side::Ask => {
                // Someone sold to our bid → we bought
                self.inventory += fill.qty.get() as i64;
                self.bid_id = None; // order consumed
            }
        }
    }

    fn inventory(&self) -> i64 {
        self.inventory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mm() -> MarketMaker {
        MarketMaker::new(1, 0.1, 0.0, 10, 1.0)
    }

    #[test]
    fn passive_fill_buy_increments_inventory() {
        let mut mm = make_mm();
        // Simulate: strategy's bid resting, synthetic ask market order hits it.
        // fill.side = Ask (aggressor), maker is bid → we bought.
        let fill = Fill {
            order_id: OrderId(1),
            side: Side::Ask,
            price: Price::from_ticks(10000),
            qty: Qty(1),
            ts: 1,
        };
        mm.on_passive_fill(&fill);
        assert_eq!(mm.inventory, 1);
    }

    #[test]
    fn passive_fill_sell_decrements_inventory() {
        let mut mm = make_mm();
        mm.inventory = 3;
        // Simulate: strategy's ask resting, synthetic bid market order hits it.
        // fill.side = Bid (aggressor), maker is ask → we sold.
        let fill = Fill {
            order_id: OrderId(2),
            side: Side::Bid,
            price: Price::from_ticks(10000),
            qty: Qty(1),
            ts: 1,
        };
        mm.on_passive_fill(&fill);
        assert_eq!(mm.inventory, 2);
    }

    #[test]
    fn inventory_bound_prevents_submit() {
        let mut mm = MarketMaker::new(1, 0.0, 0.0, 2, 1.0);
        mm.inventory = 2; // at bound

        // Build a minimal book using a OrderBook stub — we just check action count
        // by verifying on_book with inventory==bound emits no bid action.
        let mut book = OrderBook::new();
        // Add some liquidity so best_bid / best_ask exist.
        let _ = book.add_limit(Side::Bid, Price::from_ticks(9990), Qty(10), 1);
        let _ = book.add_limit(Side::Ask, Price::from_ticks(10010), Qty(10), 1);

        let features = Features::default();
        let actions = mm.on_book(&book, &features, 2);

        // No bid submit (at bound), only ask submit allowed (inventory - 1 = 1 ≥ -2)
        let submits: Vec<_> = actions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    Action::Submit {
                        side: Side::Bid,
                        ..
                    }
                )
            })
            .collect();
        assert!(submits.is_empty(), "bid submit should be blocked at bound");
    }
}
