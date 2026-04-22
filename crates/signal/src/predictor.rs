use crate::Features;

const N_FEATURES: usize = 9; // 5 OFI + 2 queue_depletion + 2 arrival_rate

pub struct LinearPredictor {
    coefficients: Option<[f64; N_FEATURES]>,
    warmup_x: Vec<[f64; N_FEATURES]>,
    warmup_y: Vec<f64>,
    warmup_size: usize,
    pub r_squared: f64,
}

impl LinearPredictor {
    pub fn new(warmup_size: usize) -> Self {
        Self {
            coefficients: None,
            warmup_x: Vec::with_capacity(warmup_size),
            warmup_y: Vec::with_capacity(warmup_size),
            warmup_size,
            r_squared: 0.0,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.coefficients.is_some()
    }

    pub fn add_warmup(&mut self, features: &Features, next_mid_return: f64) {
        if self.coefficients.is_some() {
            return;
        }
        let x = features_to_vec(features);
        self.warmup_x.push(x);
        self.warmup_y.push(next_mid_return);
        if self.warmup_x.len() >= self.warmup_size {
            self.fit();
        }
    }

    pub fn predict(&self, features: &Features) -> f64 {
        match &self.coefficients {
            None => 0.0,
            Some(coef) => {
                let x = features_to_vec(features);
                x.iter().zip(coef.iter()).map(|(a, b)| a * b).sum()
            }
        }
    }

    fn fit(&mut self) {
        let n = self.warmup_x.len();
        if n < N_FEATURES + 1 {
            return;
        }
        // Simple OLS via normal equations: beta = (X'X)^{-1} X'y
        // Use gradient-descent approximation for simplicity (no external linalg dep)
        let coef = ols_gradient(&self.warmup_x, &self.warmup_y);
        // Compute R²
        let y_mean: f64 = self.warmup_y.iter().sum::<f64>() / n as f64;
        let ss_tot: f64 = self.warmup_y.iter().map(|y| (y - y_mean).powi(2)).sum();
        let ss_res: f64 = self
            .warmup_x
            .iter()
            .zip(self.warmup_y.iter())
            .map(|(x, y)| {
                let pred: f64 = x.iter().zip(coef.iter()).map(|(a, b)| a * b).sum();
                (y - pred).powi(2)
            })
            .sum();
        self.r_squared = if ss_tot > 1e-12 {
            1.0 - ss_res / ss_tot
        } else {
            0.0
        };
        self.coefficients = Some(coef);
    }
}

fn features_to_vec(f: &Features) -> [f64; N_FEATURES] {
    [
        f.ofi[0],
        f.ofi[1],
        f.ofi[2],
        f.ofi[3],
        f.ofi[4],
        f.queue_depletion[0],
        f.queue_depletion[1],
        f.arrival_rate[0],
        f.arrival_rate[1],
    ]
}

fn ols_gradient(xs: &[[f64; N_FEATURES]], ys: &[f64]) -> [f64; N_FEATURES] {
    let n = xs.len() as f64;
    let mut coef = [0.0f64; N_FEATURES];
    let lr = 1e-4;
    let iters = 2000;

    // Normalise features for stable convergence
    let mut scale = [1.0f64; N_FEATURES];
    for j in 0..N_FEATURES {
        let var: f64 = xs.iter().map(|x| x[j] * x[j]).sum::<f64>() / n;
        scale[j] = if var > 1e-12 { var.sqrt() } else { 1.0 };
    }

    for _ in 0..iters {
        let mut grad = [0.0f64; N_FEATURES];
        for (x, y) in xs.iter().zip(ys.iter()) {
            let pred: f64 = x.iter().zip(coef.iter()).map(|(a, b)| a * b).sum();
            let residual = pred - y;
            for j in 0..N_FEATURES {
                grad[j] += residual * x[j] / scale[j];
            }
        }
        for j in 0..N_FEATURES {
            coef[j] -= lr * grad[j] / n / scale[j];
        }
    }
    coef
}
