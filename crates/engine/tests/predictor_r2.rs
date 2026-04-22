use engine::Engine;
use feed::{FeedParams, SyntheticFeed};
use strategy::NoopStrategy;

#[test]
fn predictor_r2_nontrivial_on_default_regime() {
    let feed = SyntheticFeed::new(42, FeedParams::default());
    let strategy = NoopStrategy;
    // Small warmup so this runs fast in debug mode
    let mut engine = Engine::with_warmup(feed, strategy, 0.0, 500);
    engine.run(5_000);

    assert!(
        engine.predictor.is_ready(),
        "predictor should be fitted after 5k events (warmup=500)"
    );
    let r2 = engine.predictor.r_squared;
    assert!(
        r2 > 0.01,
        "in-sample R² should be > 0.01 on default synthetic regime, got {:.4}",
        r2
    );
    println!("in-sample R² = {:.4}", r2);
}
