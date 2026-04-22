#[derive(Debug, Clone, Default)]
pub struct Features {
    /// Multi-level OFI for levels 0..K
    pub ofi: [f64; 5],
    /// EWMA fill-rate per side: [bid, ask]
    pub queue_depletion: [f64; 2],
    /// EWMA market-order arrival rate per side: [bid, ask]
    pub arrival_rate: [f64; 2],
    /// Linear predictor output (ticks). Zero until predictor is warmed up.
    pub predicted_return: f64,
}
