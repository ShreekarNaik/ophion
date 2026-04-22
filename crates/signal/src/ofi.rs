use crate::{ewma::Ewma, Features};
use lob::{OrderBook, Side};

const LEVELS: usize = 5;
const EWMA_ALPHA: f64 = 0.1;

pub struct OfiExtractor {
    prev_bid: Vec<(i64, u64)>, // (price_ticks, qty)
    prev_ask: Vec<(i64, u64)>,
    fill_ewma: [Ewma; 2],
    arrival_ewma: [Ewma; 2],
}

impl OfiExtractor {
    pub fn new() -> Self {
        Self {
            prev_bid: Vec::new(),
            prev_ask: Vec::new(),
            fill_ewma: [Ewma::new(EWMA_ALPHA), Ewma::new(EWMA_ALPHA)],
            arrival_ewma: [Ewma::new(EWMA_ALPHA), Ewma::new(EWMA_ALPHA)],
        }
    }

    pub fn update(
        &mut self,
        book: &OrderBook,
        filled_bid: u64,
        filled_ask: u64,
        market_bid: u64,
        market_ask: u64,
    ) -> Features {
        let bids: Vec<(i64, u64)> = book
            .depth(Side::Bid, LEVELS)
            .into_iter()
            .map(|(p, q)| (p.ticks(), q))
            .collect();
        let asks: Vec<(i64, u64)> = book
            .depth(Side::Ask, LEVELS)
            .into_iter()
            .map(|(p, q)| (p.ticks(), q))
            .collect();

        let best_bid_now = bids.first().map(|x| x.0);
        let best_bid_prev = self.prev_bid.first().map(|x| x.0);
        let best_ask_now = asks.first().map(|x| x.0);
        let best_ask_prev = self.prev_ask.first().map(|x| x.0);

        let mut ofi = [0.0f64; LEVELS];
        for (i, slot) in ofi.iter_mut().enumerate() {
            // Raw positive delta = how much that side's qty at level i increased.
            let bid_contribution =
                level_qty_delta(&bids, &self.prev_bid, i, best_bid_now, best_bid_prev);
            let ask_contribution =
                level_qty_delta(&asks, &self.prev_ask, i, best_ask_now, best_ask_prev);
            // OFI_k = Δbid_qty_k − Δask_qty_k  (Cont, Kukanov & Stoikov 2014)
            *slot = bid_contribution - ask_contribution;
        }

        let qd_bid = self.fill_ewma[0].update(filled_bid as f64);
        let qd_ask = self.fill_ewma[1].update(filled_ask as f64);
        let ar_bid = self.arrival_ewma[0].update(market_bid as f64);
        let ar_ask = self.arrival_ewma[1].update(market_ask as f64);

        self.prev_bid = bids;
        self.prev_ask = asks;

        Features {
            ofi,
            queue_depletion: [qd_bid, qd_ask],
            arrival_rate: [ar_bid, ar_ask],
            predicted_return: 0.0,
        }
    }
}

impl Default for OfiExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the signed change in qty at `level` for one side.
/// When the best price moves (Cont/Kukanov/Stoikov price-shift adjustment),
/// the new level qty is treated as a pure arrival to avoid sign artefacts
/// from level re-indexing.
fn level_qty_delta(
    curr: &[(i64, u64)],
    prev: &[(i64, u64)],
    level: usize,
    best_now: Option<i64>,
    best_prev: Option<i64>,
) -> f64 {
    let curr_qty = curr.get(level).map(|x| x.1).unwrap_or(0);
    let prev_qty = prev.get(level).map(|x| x.1).unwrap_or(0);

    let price_moved = match (best_now, best_prev) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    };

    if price_moved {
        // Treat the current qty as a fresh arrival (no prior baseline)
        curr_qty as f64
    } else {
        curr_qty as i64 as f64 - prev_qty as i64 as f64
    }
}
