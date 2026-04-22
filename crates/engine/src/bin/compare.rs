// Comparative backtest: TakerStrategy vs MarketMaker on the same seed.
// Usage: cargo run --release --bin compare [-- --seed 42 --events 200000 --out compare.csv]
use analytics::{run_report, SweepRow};
use engine::Engine;
use feed::{FeedParams, SyntheticFeed};
use strategy::{MarketMaker, Strategy, TakerStrategy};

use std::io::Write as _;

struct Args {
    seed: u64,
    events: u64,
    out: String,
}

fn parse_args() -> Args {
    let mut seed = 42u64;
    let mut events = 200_000u64;
    let mut out = "compare.csv".to_string();
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
                    events = v.replace('_', "").parse().unwrap_or(200_000);
                }
                i += 2;
            }
            "--out" => {
                if let Some(v) = raw.get(i + 1) {
                    out = v.clone();
                }
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
    Args { seed, events, out }
}

fn run(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "compare  seed={}  events={}  out={}",
        args.seed, args.events, args.out
    );

    // --- TakerStrategy run ---
    let taker_feed = SyntheticFeed::new(args.seed, FeedParams::default());
    let taker_strat = TakerStrategy::new(0.5, 1.0, 10);
    let mut taker_engine = Engine::with_warmup(taker_feed, taker_strat, 1.0, 1_000);
    taker_engine.run(args.events);
    let taker_inv: Vec<i64> = taker_engine
        .pnl_trace
        .iter()
        .map(|_| taker_engine.strategy.inventory())
        .collect();
    let taker_report = run_report(&taker_engine.pnl_trace, taker_engine.account.fill_count);

    // --- MarketMaker run ---
    let mm_feed = SyntheticFeed::new(args.seed, FeedParams::default());
    let mm_strat = MarketMaker::new(1, 0.05, 0.1, 25, 1.0);
    let mut mm_engine = Engine::with_warmup(mm_feed, mm_strat, 1.0, 1_000);
    mm_engine.run(args.events);
    let mm_max_inv = mm_engine.strategy.inventory().abs();
    let mm_report = run_report(&mm_engine.pnl_trace, mm_engine.account.fill_count);

    // --- Print side-by-side ---
    println!();
    println!(
        "{:<20} {:>14} {:>14}",
        "Metric", "TakerStrategy", "MarketMaker"
    );
    println!("{}", "-".repeat(50));
    println!(
        "{:<20} {:>14.4} {:>14.4}",
        "total_pnl", taker_report.total_pnl, mm_report.total_pnl
    );
    println!(
        "{:<20} {:>14.4} {:>14.4}",
        "sharpe", taker_report.sharpe, mm_report.sharpe
    );
    println!(
        "{:<20} {:>14.4} {:>14.4}",
        "max_drawdown", taker_report.max_drawdown, mm_report.max_drawdown
    );
    println!(
        "{:<20} {:>14.4} {:>14.4}",
        "hit_rate", taker_report.hit_rate, mm_report.hit_rate
    );
    println!(
        "{:<20} {:>14} {:>14}",
        "fill_count", taker_report.fill_count, mm_report.fill_count
    );
    let taker_max_inv = taker_inv.iter().map(|v| v.abs()).max().unwrap_or(0);
    println!(
        "{:<20} {:>14} {:>14}",
        "max_|inventory|", taker_max_inv, mm_max_inv
    );
    println!(
        "{:<20} {:>14} {:>14}",
        "final_inventory",
        taker_engine.strategy.inventory(),
        mm_engine.strategy.inventory()
    );

    // --- Write CSV ---
    let mut wtr = csv::Writer::from_path(&args.out)?;

    // Taker row
    wtr.serialize(SweepRow {
        threshold: 0.5,
        position_limit: 10,
        sharpe: taker_report.sharpe,
        total_pnl: taker_report.total_pnl,
        max_drawdown: taker_report.max_drawdown,
    })?;

    // MM row — reuse SweepRow with threshold=gamma, position_limit=bound
    wtr.serialize(SweepRow {
        threshold: 0.05,
        position_limit: 25,
        sharpe: mm_report.sharpe,
        total_pnl: mm_report.total_pnl,
        max_drawdown: mm_report.max_drawdown,
    })?;

    wtr.flush()?;

    // Also write a richer CSV with strategy label
    let rich_path = args.out.replace(".csv", "_full.csv");
    let mut f = std::fs::File::create(&rich_path)?;
    writeln!(
        f,
        "strategy,total_pnl,sharpe,max_drawdown,hit_rate,fill_count,max_inventory"
    )?;
    writeln!(
        f,
        "taker,{},{},{},{},{},{}",
        taker_report.total_pnl,
        taker_report.sharpe,
        taker_report.max_drawdown,
        taker_report.hit_rate,
        taker_report.fill_count,
        taker_max_inv
    )?;
    writeln!(
        f,
        "market_maker,{},{},{},{},{},{}",
        mm_report.total_pnl,
        mm_report.sharpe,
        mm_report.max_drawdown,
        mm_report.hit_rate,
        mm_report.fill_count,
        mm_max_inv
    )?;

    println!();
    println!("wrote {} and {}", args.out, rich_path);
    Ok(())
}

fn main() {
    let args = parse_args();
    if let Err(e) = run(&args) {
        eprintln!("compare error: {e}");
        std::process::exit(1);
    }
}
