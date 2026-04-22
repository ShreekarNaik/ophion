// Parameter sweep over (threshold, position_limit).
// Usage: cargo run --release --bin sweep [-- --seed 42 --events 100000 --out sweep.csv]
use analytics::{run_report, SweepRow};
use engine::Engine;
use feed::{FeedParams, SyntheticFeed};
use strategy::TakerStrategy;

struct Args {
    seed: u64,
    events: u64,
    out: String,
}

fn parse_args() -> Args {
    let mut seed = 42u64;
    let mut events = 100_000u64;
    let mut out = "sweep.csv".to_string();
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
                    events = v.replace('_', "").parse().unwrap_or(100_000);
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
    let thresholds: &[f64] = &[0.0, 0.1, 0.5, 1.0, 2.0, 5.0];
    let position_limits: &[i64] = &[1, 5, 10, 25, 50];
    let total = thresholds.len() * position_limits.len();

    println!(
        "sweep  seed={}  events={}  out={}  cells={}",
        args.seed, args.events, args.out, total
    );

    let mut wtr = csv::Writer::from_path(&args.out)?;
    let mut done = 0usize;

    for &threshold in thresholds {
        for &pos_limit in position_limits {
            let feed = SyntheticFeed::new(args.seed, FeedParams::default());
            let strategy = TakerStrategy::new(threshold, 1.0, pos_limit);
            let mut engine = Engine::with_warmup(feed, strategy, 1.0, 1_000);
            engine.run(args.events);

            let report = run_report(&engine.pnl_trace, engine.account.fill_count);
            let row = SweepRow {
                threshold,
                position_limit: pos_limit,
                sharpe: report.sharpe,
                total_pnl: report.total_pnl,
                max_drawdown: report.max_drawdown,
            };
            wtr.serialize(&row)?;

            done += 1;
            print!(
                "\r  [{}/{}]  threshold={:.1}  pos_limit={:3}  sharpe={:+.3}",
                done, total, threshold, pos_limit, report.sharpe
            );
        }
    }
    println!();

    wtr.flush()?;
    println!("wrote {}", args.out);
    Ok(())
}

fn main() {
    let args = parse_args();
    if let Err(e) = run(&args) {
        eprintln!("sweep error: {e}");
        std::process::exit(1);
    }
}
