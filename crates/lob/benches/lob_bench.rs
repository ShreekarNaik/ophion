use criterion::{criterion_group, criterion_main, Criterion};
use lob::{OrderBook, OrderId, Price, Qty, Side};

fn setup_book(levels: i64) -> OrderBook {
    let mut book = OrderBook::new();
    for i in 0..levels {
        let _ = book.add_limit(Side::Bid, Price(1000 - i), Qty(10), i as u64);
        let _ = book.add_limit(Side::Ask, Price(1001 + i), Qty(10), i as u64);
    }
    book
}

fn bench_insert(c: &mut Criterion) {
    c.bench_function("lob_insert_limit", |b| {
        let mut book = setup_book(10);
        let mut ts = 1000u64;
        b.iter(|| {
            // Alternate bids and asks away from the touch so they rest and don't cross
            ts += 1;
            let _ = book.add_limit(Side::Bid, Price(990), Qty(1), ts);
        });
    });
}

fn bench_cancel(c: &mut Criterion) {
    c.bench_function("lob_cancel", |b| {
        b.iter_batched(
            || {
                let mut book = setup_book(10);
                // Add the order we'll cancel in the bench iteration
                let id = book.add_limit(Side::Bid, Price(990), Qty(1), 9999).unwrap();
                (book, id)
            },
            |(mut book, id)| {
                let _ = book.cancel(id, 10000);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_market_match(c: &mut Criterion) {
    c.bench_function("lob_market_match_1_level", |b| {
        b.iter_batched(
            || {
                // Fresh book so there's always liquidity to consume
                setup_book(10)
            },
            |mut book| {
                // Market buy of qty=1 consumes best ask
                let _ = book.execute_market(Side::Bid, Qty(1), 99999);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_cancel_mid_queue(c: &mut Criterion) {
    c.bench_function("lob_cancel_mid_queue", |b| {
        b.iter_batched(
            || {
                let mut book = OrderBook::new();
                let mut ids = Vec::new();
                // 20 orders at the same price level
                for ts in 0..20u64 {
                    let id = book.add_limit(Side::Bid, Price(1000), Qty(1), ts).unwrap();
                    ids.push(id);
                }
                // Return the middle-queue order
                let mid_id = ids[10];
                (book, mid_id)
            },
            |(mut book, id): (OrderBook, OrderId)| {
                let _ = book.cancel(id, 99999);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_insert,
    bench_cancel,
    bench_market_match,
    bench_cancel_mid_queue
);
criterion_main!(benches);
