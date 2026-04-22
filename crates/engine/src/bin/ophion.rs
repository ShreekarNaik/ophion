use std::time::Instant;

use analytics::run_report;
use engine::Engine;
use feed::{FeedParams, SyntheticFeed};
use strategy::NoopStrategy;

struct Args {
    seed: u64,
    events: u64,
}

fn parse_args() -> Args {
    let mut seed = 42u64;
    let mut events = 1_000_000u64;
    let raw: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--seed" => {
                if let Some(v) = raw.get(i + 1) {
                    seed = v.replace('_', "").parse().unwrap_or(42);
                }
                i += 2;
            }
            "--events" => {
                if let Some(v) = raw.get(i + 1) {
                    events = v.replace('_', "").parse().unwrap_or(1_000_000);
                }
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
    Args { seed, events }
}

fn main() {
    let args = parse_args();
    println!("ophion  seed={}  events={}", args.seed, args.events);

    let feed = SyntheticFeed::new(args.seed, FeedParams::default());
    let strategy = NoopStrategy;
    let mut engine = Engine::new(feed, strategy, 1.0);

    let t0 = Instant::now();
    engine.run(args.events);
    let elapsed = t0.elapsed();

    let report = run_report(&engine.pnl_trace, engine.account.fill_count);
    let eps = engine.event_count as f64 / elapsed.as_secs_f64();

    println!("events processed : {}", engine.event_count);
    println!("elapsed          : {:.3}s", elapsed.as_secs_f64());
    println!("throughput       : {:.0} events/s", eps);
    println!("final PnL        : {:.4}", report.total_pnl);
    println!("sharpe           : {:.4}", report.sharpe);
    println!("max drawdown     : {:.4}", report.max_drawdown);
    println!("fill count       : {}", report.fill_count);
}
