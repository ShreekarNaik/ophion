pub use lob::Fill;
use lob::Side;

#[derive(Debug, Default, Clone)]
pub struct AccountState {
    pub realized_pnl: f64,
    pub inventory: i64,
    pub fill_count: u64,
    pub fees_paid: f64,
}

impl AccountState {
    pub fn apply_fill(&mut self, fill: &Fill, fee_bps: f64) {
        let value = fill.price.ticks() as f64 * 0.01 * fill.qty.get() as f64;
        let fee = value * fee_bps / 10_000.0;
        match fill.side {
            Side::Bid => {
                self.realized_pnl -= value + fee;
                self.inventory += fill.qty.get() as i64;
            }
            Side::Ask => {
                self.realized_pnl += value - fee;
                self.inventory -= fill.qty.get() as i64;
            }
        }
        self.fees_paid += fee;
        self.fill_count += 1;
    }

    pub fn unrealized_pnl(&self, mid_price_ticks: i64) -> f64 {
        self.inventory as f64 * mid_price_ticks as f64 * 0.01
    }

    pub fn total_pnl(&self, mid_price_ticks: i64) -> f64 {
        self.realized_pnl + self.unrealized_pnl(mid_price_ticks)
    }
}
