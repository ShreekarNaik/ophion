use engine::Engine;
use feed::{FeedParams, SyntheticFeed};
use sha2::{Digest, Sha256};
use strategy::NoopStrategy;

/// Hash the mid-price trace (feed-dependent) and PnL trace.
/// Mid-trace reflects the generative process (OU + Poisson); PnL reflects strategy fills.
fn run_and_hash(seed: u64, events: u64) -> [u8; 32] {
    let feed = SyntheticFeed::new(seed, FeedParams::default());
    let strategy = NoopStrategy;
    let mut engine = Engine::new(feed, strategy, 0.0);
    engine.run(events);

    let mut hasher = Sha256::new();
    for &mid in &engine.mid_trace {
        hasher.update(mid.to_le_bytes());
    }
    for pnl in &engine.pnl_trace {
        hasher.update(pnl.to_bits().to_le_bytes());
    }
    hasher.finalize().into()
}

#[test]
fn same_seed_produces_identical_trace() {
    let h1 = run_and_hash(42, 10_000);
    let h2 = run_and_hash(42, 10_000);
    assert_eq!(
        h1, h2,
        "determinism violated: same seed produced different traces"
    );
}

#[test]
fn different_seeds_produce_different_traces() {
    let h1 = run_and_hash(42, 10_000);
    let h2 = run_and_hash(99, 10_000);
    assert_ne!(
        h1, h2,
        "different seeds should produce different mid-price traces"
    );
}
