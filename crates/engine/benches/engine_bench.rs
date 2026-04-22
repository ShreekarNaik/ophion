use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use engine::Engine;
use feed::{FeedParams, SyntheticFeed};
use strategy::NoopStrategy;

// 100k events per iteration: covers warmup and gives stable cache-warm throughput.
const EVENTS: u64 = 100_000;

fn make_engine() -> Engine<SyntheticFeed, NoopStrategy> {
    let feed = SyntheticFeed::new(42, FeedParams::default());
    Engine::new(feed, NoopStrategy, 1.0)
}

fn bench_engine_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine");
    group.throughput(Throughput::Elements(EVENTS));
    // Fewer samples since each iteration takes ~50 ms
    group.sample_size(20);
    group.bench_function("events_per_sec", |b| {
        b.iter_batched(
            make_engine,
            |mut engine| engine.run(EVENTS),
            criterion::BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group!(benches, bench_engine_throughput);
criterion_main!(benches);
