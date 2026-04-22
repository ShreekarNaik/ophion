use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SweepRow {
    pub threshold: f64,
    pub position_limit: i64,
    pub sharpe: f64,
    pub total_pnl: f64,
    pub max_drawdown: f64,
}
