use rand::{
    distributions::{Distribution, WeightedIndex},
    rngs::StdRng,
    Rng, SeedableRng,
};

use lob::{OrderId, Price, Qty, Side};

use crate::{Feed, Tick};

/// Parameters for the synthetic feed generator.
#[derive(Debug, Clone)]
pub struct FeedParams {
    /// Poisson rate: limit bid arrivals per unit time
    pub lambda_limit_bid: f64,
    /// Poisson rate: limit ask arrivals per unit time
    pub lambda_limit_ask: f64,
    /// Poisson rate: cancel bid per unit time
    pub lambda_cancel_bid: f64,
    /// Poisson rate: cancel ask per unit time
    pub lambda_cancel_ask: f64,
    /// Poisson rate: market bid arrivals per unit time
    pub lambda_market_bid: f64,
    /// Poisson rate: market ask arrivals per unit time
    pub lambda_market_ask: f64,
    /// OU mean-reversion speed (θ)
    pub ou_theta: f64,
    /// OU long-run mean in ticks
    pub ou_mu: f64,
    /// OU volatility (σ)
    pub ou_sigma: f64,
    /// Width of the geometric distribution for limit-price offset from mid (in ticks)
    pub price_offset_decay: f64,
    /// Max ticks away from mid for limit orders
    pub max_offset_ticks: u64,
    /// Typical order quantity (Poisson mean for qty)
    pub qty_mean: f64,
    /// Price impact per unit of order flow (ticks per qty unit).
    /// Limit/market bid arrivals push the OU mid up; ask arrivals push it down.
    /// Implements the Cont–Kukanov–Stoikov OFI→price feedback mechanism.
    pub ofi_impact: f64,
}

impl Default for FeedParams {
    fn default() -> Self {
        Self {
            lambda_limit_bid: 5.0,
            lambda_limit_ask: 5.0,
            lambda_cancel_bid: 1.5,
            lambda_cancel_ask: 1.5,
            lambda_market_bid: 1.0,
            lambda_market_ask: 1.0,
            ou_theta: 0.01,
            ou_mu: 10000.0, // mid starts at 100.00
            ou_sigma: 0.5,
            price_offset_decay: 0.8,
            max_offset_ticks: 10,
            qty_mean: 5.0,
            // 0.02 ticks/unit impact; with decay=0.8 orders cluster near best bid/ask,
            // so OFI imbalance quickly moves the book mid (Cont-Kukanov-Stoikov mechanism).
            ofi_impact: 0.02,
        }
    }
}

/// Synthetic market feed using independent Poisson processes per event type
/// and an Ornstein–Uhlenbeck process for the mid-price drift.
pub struct SyntheticFeed {
    rng: StdRng,
    params: FeedParams,
    /// Logical clock in nanoseconds
    ts: u64,
    /// Current mid price in ticks (f64 for OU continuity, rounded for price generation)
    mid: f64,
    /// Pending resting order IDs per side (for cancels)
    bid_ids: std::collections::VecDeque<OrderId>,
    ask_ids: std::collections::VecDeque<OrderId>,
    /// Monotonically assigned IDs mirroring the book (approximate; for cancel generation)
    next_ext_id: u64,
    /// Event type weights for sampling which process fires next
    weights: [f64; 6],
    weight_dist: Option<WeightedIndex<u64>>,
}

impl SyntheticFeed {
    pub fn new(seed: u64, params: FeedParams) -> Self {
        let weights = [
            params.lambda_limit_bid,
            params.lambda_limit_ask,
            params.lambda_cancel_bid,
            params.lambda_cancel_ask,
            params.lambda_market_bid,
            params.lambda_market_ask,
        ];
        // Convert to integer weights (×1000 for precision)
        let int_weights: [u64; 6] = weights.map(|w| (w * 1000.0) as u64 + 1);
        let weight_dist = WeightedIndex::new(int_weights).ok();
        let mid = params.ou_mu;
        Self {
            rng: StdRng::seed_from_u64(seed),
            params,
            ts: 1_000_000, // start at 1ms
            mid,
            bid_ids: std::collections::VecDeque::new(),
            ask_ids: std::collections::VecDeque::new(),
            next_ext_id: 1,
            weights,
            weight_dist,
        }
    }

    pub fn with_default_params(seed: u64) -> Self {
        Self::new(seed, FeedParams::default())
    }

    fn advance_ou(&mut self) {
        // Euler-Maruyama step for dX = θ(μ - X)dt + σ dW
        let dt = 1.0;
        let dw: f64 = self.rng.sample(rand::distributions::Standard);
        let dw = dw * 2.0 - 1.0; // uniform [-1,1] approximation
        self.mid +=
            self.params.ou_theta * (self.params.ou_mu - self.mid) * dt + self.params.ou_sigma * dw;
    }

    fn sample_qty(&mut self) -> Qty {
        // Poisson-distributed quantity with min 1
        let mean = self.params.qty_mean;
        let u: f64 = self.rng.gen();
        let qty = (-mean * u.ln()).ceil() as u64;
        Qty(qty.clamp(1, 100))
    }

    fn sample_price_offset(&mut self) -> u64 {
        // Geometric distribution: P(k) ∝ (1-p)^k * p, shifted to 1..max_offset
        let decay = self.params.price_offset_decay;
        let mut offset = 1u64;
        let max = self.params.max_offset_ticks;
        while offset < max && self.rng.gen::<f64>() > decay {
            offset += 1;
        }
        offset
    }

    fn next_ts(&mut self) -> u64 {
        // Interarrival time ~ Exp(total_rate); approximate with fixed step + small jitter
        let total_rate: f64 = self.weights.iter().sum();
        let step = (1_000_000_000.0 / total_rate) as u64; // ~nanoseconds per event
        let jitter = self.rng.gen_range(0..step / 2 + 1);
        self.ts += step + jitter;
        self.ts
    }
}

impl Feed for SyntheticFeed {
    fn next(&mut self) -> Option<Tick> {
        let ts = self.next_ts();
        self.advance_ou();

        let mid_ticks = self.mid.round() as i64;

        let dist = self.weight_dist.as_ref()?;
        let event_type = dist.sample(&mut self.rng);

        let impact = self.params.ofi_impact;
        let tick = match event_type {
            // Limit bid: price below mid
            0 => {
                let offset = self.sample_price_offset();
                let price = Price(mid_ticks - offset as i64);
                let qty = self.sample_qty();
                // Track id for potential cancel
                let id = OrderId(self.next_ext_id);
                self.next_ext_id += 1;
                self.bid_ids.push_back(id);
                if self.bid_ids.len() > 200 {
                    self.bid_ids.pop_front();
                }
                self.mid += impact * qty.get() as f64;
                Tick::LimitOrder {
                    side: Side::Bid,
                    price,
                    qty,
                    ts,
                }
            }
            // Limit ask: price above mid
            1 => {
                let offset = self.sample_price_offset();
                let price = Price(mid_ticks + offset as i64);
                let qty = self.sample_qty();
                let id = OrderId(self.next_ext_id);
                self.next_ext_id += 1;
                self.ask_ids.push_back(id);
                if self.ask_ids.len() > 200 {
                    self.ask_ids.pop_front();
                }
                self.mid -= impact * qty.get() as f64;
                Tick::LimitOrder {
                    side: Side::Ask,
                    price,
                    qty,
                    ts,
                }
            }
            // Cancel bid
            2 => {
                if let Some(id) = self.bid_ids.pop_front() {
                    Tick::Cancel { order_id: id, ts }
                } else {
                    // No bid to cancel; emit a harmless limit instead
                    let offset = self.sample_price_offset();
                    let price = Price(mid_ticks - offset as i64);
                    let qty = self.sample_qty();
                    Tick::LimitOrder {
                        side: Side::Bid,
                        price,
                        qty,
                        ts,
                    }
                }
            }
            // Cancel ask
            3 => {
                if let Some(id) = self.ask_ids.pop_front() {
                    Tick::Cancel { order_id: id, ts }
                } else {
                    let offset = self.sample_price_offset();
                    let price = Price(mid_ticks + offset as i64);
                    let qty = self.sample_qty();
                    Tick::LimitOrder {
                        side: Side::Ask,
                        price,
                        qty,
                        ts,
                    }
                }
            }
            // Market bid — stronger impact (more urgent, moves price more)
            4 => {
                let qty = self.sample_qty();
                self.mid += impact * 2.0 * qty.get() as f64;
                Tick::MarketOrder {
                    side: Side::Bid,
                    qty,
                    ts,
                }
            }
            // Market ask — stronger impact
            5 => {
                let qty = self.sample_qty();
                self.mid -= impact * 2.0 * qty.get() as f64;
                Tick::MarketOrder {
                    side: Side::Ask,
                    qty,
                    ts,
                }
            }
            _ => return None,
        };

        Some(tick)
    }
}
