use crate::accounting::AccountState;
use feed::{Feed, Tick};
use lob::OrderBook;
use signal::{Features, OfiExtractor};
use strategy::Strategy;

pub struct Engine<F: Feed, S: Strategy> {
    pub book: OrderBook,
    pub feed: F,
    pub strategy: S,
    pub ofi: OfiExtractor,
    pub account: AccountState,
    pub event_count: u64,
    pub pnl_trace: Vec<f64>,
    pub fee_bps: f64,
}

impl<F: Feed, S: Strategy> Engine<F, S> {
    pub fn new(feed: F, strategy: S, fee_bps: f64) -> Self {
        Self {
            book: OrderBook::new(),
            feed,
            strategy,
            ofi: OfiExtractor::new(),
            account: AccountState::default(),
            event_count: 0,
            pnl_trace: Vec::new(),
            fee_bps,
        }
    }

    pub fn step(&mut self) -> bool {
        let tick = match self.feed.next() {
            Some(t) => t,
            None => return false,
        };
        self.event_count += 1;

        let mut filled_bid = 0u64;
        let mut filled_ask = 0u64;
        let mut market_bid = 0u64;
        let mut market_ask = 0u64;

        match tick {
            Tick::LimitOrder {
                side,
                price,
                qty,
                ts,
            } => {
                let _ = self.book.add_limit(side, price, qty, ts);
            }
            Tick::Cancel { order_id, ts } => {
                let _ = self.book.cancel(order_id, ts);
            }
            Tick::MarketOrder { side, qty, ts } => {
                use lob::Side;
                match side {
                    Side::Bid => market_bid += qty.0,
                    Side::Ask => market_ask += qty.0,
                }
                if let Ok(fills) = self.book.execute_market(side, qty, ts) {
                    for fill in &fills {
                        match fill.side {
                            Side::Bid => filled_bid += fill.qty.0,
                            Side::Ask => filled_ask += fill.qty.0,
                        }
                    }
                }
            }
        }

        let features = self
            .ofi
            .update(&self.book, filled_bid, filled_ask, market_bid, market_ask);
        let ts = self.book.last_ts;
        let actions = self.strategy.on_book(&self.book, &features, ts);

        for action in actions {
            use strategy::Action;
            match action {
                Action::Submit { side, price, qty } => {
                    let _ = self.book.add_limit(side, price, qty, ts);
                }
                Action::Cancel(id) => {
                    let _ = self.book.cancel(id, ts);
                }
                Action::TakeMarket { side, qty } => {
                    if let Ok(fills) = self.book.execute_market(side, qty, ts) {
                        for fill in &fills {
                            self.account.apply_fill(fill, self.fee_bps);
                            self.strategy.on_fill(fill);
                        }
                    }
                }
            }
        }

        let mid = self.book.mid().unwrap_or(0);
        self.pnl_trace.push(self.account.total_pnl(mid));

        #[cfg(debug_assertions)]
        debug_assert!(
            self.book.check_invariants(),
            "LOB invariant violated at event {}",
            self.event_count
        );

        true
    }

    pub fn run(&mut self, max_events: u64) {
        while self.event_count < max_events && self.step() {}
    }

    pub fn current_features(&self) -> Features {
        Features::default()
    }
}
