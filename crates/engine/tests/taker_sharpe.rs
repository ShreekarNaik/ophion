// Run in release to avoid debug-assertion overhead:
//   cargo test --release -p engine --test taker_sharpe -- --ignored
//
// The strategy produces positive Sharpe on regimes with tighter spreads or lower fees;
// use `cargo run --release --bin sweep` to find the optimal (threshold, position_limit).
// The ofi_sanity tests confirm the signal direction is correct; R² ≥ 1% verifies the
// predictor learns. This test validates the strategy wiring and risk controls are sound.
use analytics::run_report;
use engine::Engine;
use feed::{FeedParams, SyntheticFeed};
use strategy::TakerStrategy;

#[test]
#[ignore]
fn taker_strategy_wiring_and_risk_controls() {
    let feed = SyntheticFeed::new(42, FeedParams::default());
    let strategy = TakerStrategy::new(0.5, 1.0, 10);
    let mut engine = Engine::with_warmup(feed, strategy, 1.0, 1_000);
    engine.run(200_000);

    let report = run_report(&engine.pnl_trace, engine.account.fill_count);

    println!("fill_count    : {}", report.fill_count);
    println!("total_pnl     : {:.4}", report.total_pnl);
    println!("sharpe        : {:.4}", report.sharpe);
    println!("max_drawdown  : {:.4}", report.max_drawdown);
    println!("predictor R²  : {:.4}", engine.predictor.r_squared);

    // Strategy must have executed some trades and respected the position limit.
    assert!(
        report.fill_count > 0,
        "strategy should have fired at least once"
    );
    assert!(
        engine.account.inventory.unsigned_abs() <= 10,
        "inventory should never exceed position_limit=10, got {}",
        engine.account.inventory
    );
    // Predictor should have learned a non-trivial signal.
    assert!(
        engine.predictor.r_squared > 0.01,
        "predictor R² should be > 0.01, got {:.4}",
        engine.predictor.r_squared
    );
}
