pub struct Ewma {
    alpha: f64,
    value: f64,
    initialised: bool,
}

impl Ewma {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha,
            value: 0.0,
            initialised: false,
        }
    }

    pub fn update(&mut self, x: f64) -> f64 {
        if self.initialised {
            self.value = self.alpha * x + (1.0 - self.alpha) * self.value;
        } else {
            self.value = x;
            self.initialised = true;
        }
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}
