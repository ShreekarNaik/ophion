use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Report {
    pub total_pnl: f64,
    pub sharpe: f64,
    pub max_drawdown: f64,
    pub hit_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub fill_count: u64,
}

pub fn run_report(pnl_series: &[f64], fill_count: u64) -> Report {
    if pnl_series.len() < 2 {
        return Report {
            total_pnl: 0.0,
            sharpe: 0.0,
            max_drawdown: 0.0,
            hit_rate: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            fill_count,
        };
    }

    let returns: Vec<f64> = pnl_series.windows(2).map(|w| w[1] - w[0]).collect();
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();
    // Annualise assuming 252 * 6.5 * 3600 steps of 1-second granularity
    let annual_factor = (252.0_f64 * 6.5 * 3600.0).sqrt();
    let sharpe = if std_dev > 1e-12 {
        mean / std_dev * annual_factor
    } else {
        0.0
    };

    let mut peak = pnl_series[0];
    let mut max_dd = 0.0f64;
    for &v in pnl_series {
        if v > peak {
            peak = v;
        }
        let dd = peak - v;
        if dd > max_dd {
            max_dd = dd;
        }
    }

    let wins: Vec<f64> = returns.iter().copied().filter(|&r| r > 0.0).collect();
    let losses: Vec<f64> = returns.iter().copied().filter(|&r| r < 0.0).collect();
    let hit_rate = wins.len() as f64 / n;
    let avg_win = if wins.is_empty() {
        0.0
    } else {
        wins.iter().sum::<f64>() / wins.len() as f64
    };
    let avg_loss = if losses.is_empty() {
        0.0
    } else {
        losses.iter().sum::<f64>() / losses.len() as f64
    };

    Report {
        total_pnl: *pnl_series.last().unwrap_or(&0.0),
        sharpe,
        max_drawdown: max_dd,
        hit_rate,
        avg_win,
        avg_loss,
        fill_count,
    }
}
