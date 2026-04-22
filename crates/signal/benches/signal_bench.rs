use criterion::{criterion_group, criterion_main, Criterion};
use lob::{OrderBook, Price, Qty, Side};
use signal::OfiExtractor;

fn bench_ofi_update(c: &mut Criterion) {
    let mut extractor = OfiExtractor::new();
    let mut book = OrderBook::new();
    // Seed some liquidity
    for i in 0..10i64 {
        let _ = book.add_limit(Side::Bid, Price(1000 - i), Qty(100), i as u64);
        let _ = book.add_limit(Side::Ask, Price(1001 + i), Qty(100), i as u64);
    }
    c.bench_function("ofi_update", |b| {
        b.iter(|| extractor.update(&book, 0, 0, 0, 0))
    });
}

criterion_group!(benches, bench_ofi_update);
criterion_main!(benches);
