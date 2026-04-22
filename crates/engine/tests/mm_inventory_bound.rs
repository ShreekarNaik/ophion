use engine::Engine;
use feed::{FeedParams, SyntheticFeed};
use proptest::prelude::*;
use strategy::{MarketMaker, Strategy};

const BOUND: i64 = 10;

proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 500,
        ..Default::default()
    })]
    #[test]
    fn mm_inventory_never_exceeds_bound(seed in any::<u64>(), n in 500u64..5000u64) {
        let feed = SyntheticFeed::new(seed, FeedParams::default());
        let strategy = MarketMaker::new(1, 0.05, 0.1, BOUND, 1.0);
        let mut engine = Engine::with_warmup(feed, strategy, 1.0, 500);
        engine.run(n);
        prop_assert!(
            engine.strategy.inventory().abs() <= BOUND,
            "inventory {} exceeded bound {} (seed={}, n={})",
            engine.strategy.inventory(),
            BOUND,
            seed,
            n
        );
    }
}
