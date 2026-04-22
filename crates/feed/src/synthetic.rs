use crate::{Feed, Tick};

pub struct SyntheticFeed;

impl SyntheticFeed {
    pub fn new(_seed: u64) -> Self {
        Self
    }
}

impl Feed for SyntheticFeed {
    fn next(&mut self) -> Option<Tick> {
        None // placeholder — implemented in Phase 2
    }
}
