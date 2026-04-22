use lob::{OrderBook, Price, Qty, Side};
use signal::OfiExtractor;

/// Inject persistent bid-side pressure over a known interval and verify
/// that cumulative OFI[0] is positive (tracking the bid-side injection).
#[test]
fn ofi_tracks_bid_side_pressure() {
    let mut extractor = OfiExtractor::new();
    let mut book = OrderBook::new();

    // Seed symmetric initial book
    for i in 0..5i64 {
        let _ = book.add_limit(Side::Bid, Price(1000 - i), Qty(50), i as u64);
        let _ = book.add_limit(Side::Ask, Price(1001 + i), Qty(50), i as u64);
    }

    // Baseline update
    extractor.update(&book, 0, 0, 0, 0);

    // Inject 20 rounds of bid-side pressure: add large bid qty, minimal ask
    let mut cumulative_ofi = 0.0f64;
    for i in 0..20usize {
        let ts = (100 + i) as u64;
        // Add large bid at best bid
        let _ = book.add_limit(Side::Bid, Price(1000), Qty(100), ts);
        // Add small ask (noise)
        let _ = book.add_limit(Side::Ask, Price(1001), Qty(5), ts);

        let features = extractor.update(&book, 0, 0, 0, 0);
        cumulative_ofi += features.ofi[0];
    }

    assert!(
        cumulative_ofi > 0.0,
        "OFI[0] should be positive under bid-side pressure, got {}",
        cumulative_ofi
    );
}

/// Inject persistent ask-side pressure and verify OFI[0] is negative.
#[test]
fn ofi_tracks_ask_side_pressure() {
    let mut extractor = OfiExtractor::new();
    let mut book = OrderBook::new();

    for i in 0..5i64 {
        let _ = book.add_limit(Side::Bid, Price(1000 - i), Qty(50), i as u64);
        let _ = book.add_limit(Side::Ask, Price(1001 + i), Qty(50), i as u64);
    }

    extractor.update(&book, 0, 0, 0, 0);

    let mut cumulative_ofi = 0.0f64;
    for i in 0..20usize {
        let ts = (100 + i) as u64;
        let _ = book.add_limit(Side::Ask, Price(1001), Qty(100), ts);
        let _ = book.add_limit(Side::Bid, Price(1000), Qty(5), ts);
        let features = extractor.update(&book, 0, 0, 0, 0);
        cumulative_ofi += features.ofi[0];
    }

    assert!(
        cumulative_ofi < 0.0,
        "OFI[0] should be negative under ask-side pressure, got {}",
        cumulative_ofi
    );
}
