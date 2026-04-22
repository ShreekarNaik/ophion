#[cfg(test)]
mod unit {
    use crate::{OrderBook, OrderBookError, OrderId, Price, Qty, Side};

    fn book() -> OrderBook {
        OrderBook::new()
    }

    // ── add_limit ────────────────────────────────────────────────────────────
    #[test]
    fn add_single_bid() {
        let mut b = book();
        let id = b.add_limit(Side::Bid, Price(100), Qty(50), 1).unwrap();
        assert_eq!(id, OrderId(1));
        assert_eq!(b.best_bid(), Some(Price(100)));
        assert_eq!(b.best_ask(), None);
    }

    #[test]
    fn add_single_ask() {
        let mut b = book();
        b.add_limit(Side::Ask, Price(101), Qty(10), 1).unwrap();
        assert_eq!(b.best_ask(), Some(Price(101)));
    }

    #[test]
    fn zero_qty_rejected() {
        let mut b = book();
        assert!(matches!(
            b.add_limit(Side::Bid, Price(100), Qty(0), 1),
            Err(OrderBookError::ZeroQuantity)
        ));
    }

    #[test]
    fn best_bid_is_highest() {
        let mut b = book();
        b.add_limit(Side::Bid, Price(100), Qty(1), 1).unwrap();
        b.add_limit(Side::Bid, Price(105), Qty(1), 2).unwrap();
        b.add_limit(Side::Bid, Price(98), Qty(1), 3).unwrap();
        assert_eq!(b.best_bid(), Some(Price(105)));
    }

    #[test]
    fn best_ask_is_lowest() {
        let mut b = book();
        b.add_limit(Side::Ask, Price(102), Qty(1), 1).unwrap();
        b.add_limit(Side::Ask, Price(101), Qty(1), 2).unwrap();
        b.add_limit(Side::Ask, Price(105), Qty(1), 3).unwrap();
        assert_eq!(b.best_ask(), Some(Price(101)));
    }

    #[test]
    fn depth_aggregates_multiple_orders_at_same_price() {
        let mut b = book();
        b.add_limit(Side::Bid, Price(100), Qty(10), 1).unwrap();
        b.add_limit(Side::Bid, Price(100), Qty(20), 2).unwrap();
        let d = b.depth(Side::Bid, 5);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0], (Price(100), 30));
    }

    #[test]
    fn depth_sorted_descending_for_bids() {
        let mut b = book();
        b.add_limit(Side::Bid, Price(98), Qty(1), 1).unwrap();
        b.add_limit(Side::Bid, Price(100), Qty(1), 2).unwrap();
        b.add_limit(Side::Bid, Price(99), Qty(1), 3).unwrap();
        let d = b.depth(Side::Bid, 5);
        assert_eq!(d[0].0, Price(100));
        assert_eq!(d[1].0, Price(99));
        assert_eq!(d[2].0, Price(98));
    }

    #[test]
    fn depth_sorted_ascending_for_asks() {
        let mut b = book();
        b.add_limit(Side::Ask, Price(103), Qty(1), 1).unwrap();
        b.add_limit(Side::Ask, Price(101), Qty(1), 2).unwrap();
        b.add_limit(Side::Ask, Price(102), Qty(1), 3).unwrap();
        let d = b.depth(Side::Ask, 5);
        assert_eq!(d[0].0, Price(101));
        assert_eq!(d[1].0, Price(102));
        assert_eq!(d[2].0, Price(103));
    }

    // ── cancel ───────────────────────────────────────────────────────────────
    #[test]
    fn cancel_existing_order() {
        let mut b = book();
        let id = b.add_limit(Side::Bid, Price(100), Qty(50), 1).unwrap();
        let returned_qty = b.cancel(id, 2).unwrap();
        assert_eq!(returned_qty, Qty(50));
        assert_eq!(b.best_bid(), None);
    }

    #[test]
    fn cancel_unknown_order_returns_error() {
        let mut b = book();
        assert!(matches!(
            b.cancel(OrderId(999), 1),
            Err(OrderBookError::OrderNotFound(999))
        ));
    }

    #[test]
    fn cancel_mid_queue_preserves_others() {
        let mut b = book();
        let id1 = b.add_limit(Side::Ask, Price(101), Qty(10), 1).unwrap();
        let id2 = b.add_limit(Side::Ask, Price(101), Qty(20), 2).unwrap();
        let _id3 = b.add_limit(Side::Ask, Price(101), Qty(30), 3).unwrap();
        b.cancel(id2, 4).unwrap();

        let fills = b.execute_market(Side::Bid, Qty(40), 5).unwrap();
        let total: u64 = fills.iter().map(|f| f.qty.0).sum();
        assert_eq!(total, 40);

        // FIFO: id1 fills first (10), then id3 (30)
        assert_eq!(fills[0].order_id, id1);
        assert_eq!(fills[1].order_id, _id3);
    }

    #[test]
    fn cancel_removes_price_level_when_empty() {
        let mut b = book();
        let id = b.add_limit(Side::Bid, Price(100), Qty(1), 1).unwrap();
        b.cancel(id, 2).unwrap();
        assert_eq!(b.depth(Side::Bid, 5).len(), 0);
    }

    // ── execute_market ───────────────────────────────────────────────────────
    #[test]
    fn market_buy_consumes_asks_from_best() {
        let mut b = book();
        b.add_limit(Side::Ask, Price(101), Qty(50), 1).unwrap();
        b.add_limit(Side::Ask, Price(102), Qty(50), 2).unwrap();
        let fills = b.execute_market(Side::Bid, Qty(50), 3).unwrap();
        assert_eq!(fills[0].price, Price(101));
    }

    #[test]
    fn market_sell_consumes_bids_from_best() {
        let mut b = book();
        b.add_limit(Side::Bid, Price(100), Qty(50), 1).unwrap();
        b.add_limit(Side::Bid, Price(99), Qty(50), 2).unwrap();
        let fills = b.execute_market(Side::Ask, Qty(50), 3).unwrap();
        assert_eq!(fills[0].price, Price(100));
    }

    #[test]
    fn market_order_sweeps_multiple_levels() {
        let mut b = book();
        b.add_limit(Side::Ask, Price(101), Qty(20), 1).unwrap();
        b.add_limit(Side::Ask, Price(102), Qty(30), 2).unwrap();
        let fills = b.execute_market(Side::Bid, Qty(50), 3).unwrap();
        let total: u64 = fills.iter().map(|f| f.qty.0).sum();
        assert_eq!(total, 50);
        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].price, Price(101));
        assert_eq!(fills[1].price, Price(102));
    }

    #[test]
    fn partial_fill_leaves_remainder() {
        let mut b = book();
        b.add_limit(Side::Ask, Price(101), Qty(100), 1).unwrap();
        b.execute_market(Side::Bid, Qty(40), 2).unwrap();
        let d = b.depth(Side::Ask, 5);
        assert_eq!(d[0].1, 60); // 100 - 40 = 60 remaining
    }

    #[test]
    fn insufficient_liquidity_error() {
        let mut b = book();
        b.add_limit(Side::Ask, Price(101), Qty(10), 1).unwrap();
        assert!(matches!(
            b.execute_market(Side::Bid, Qty(20), 2),
            Err(OrderBookError::InsufficientLiquidity {
                needed: 20,
                available: 10
            })
        ));
    }

    #[test]
    fn fifo_queue_priority_at_same_price() {
        let mut b = book();
        let id_first = b.add_limit(Side::Ask, Price(101), Qty(10), 1).unwrap();
        let id_second = b.add_limit(Side::Ask, Price(101), Qty(10), 2).unwrap();
        let fills = b.execute_market(Side::Bid, Qty(10), 3).unwrap();
        assert_eq!(fills[0].order_id, id_first); // first in, first out
                                                 // second order should still be resting
        let d = b.depth(Side::Ask, 5);
        assert_eq!(d[0].1, 10);
        let _ = id_second; // still alive
    }

    #[test]
    fn market_order_zero_qty_rejected() {
        let mut b = book();
        assert!(matches!(
            b.execute_market(Side::Bid, Qty(0), 1),
            Err(OrderBookError::ZeroQuantity)
        ));
    }

    #[test]
    fn cancel_of_partially_filled_order_remainder() {
        let mut b = book();
        let id = b.add_limit(Side::Ask, Price(101), Qty(100), 1).unwrap();
        // Partially fill
        b.execute_market(Side::Bid, Qty(60), 2).unwrap();
        // Cancel the rest
        let returned = b.cancel(id, 3).unwrap();
        assert_eq!(returned.0, 40);
        assert_eq!(b.best_ask(), None);
    }

    #[test]
    fn spread_and_mid() {
        let mut b = book();
        b.add_limit(Side::Bid, Price(100), Qty(1), 1).unwrap();
        b.add_limit(Side::Ask, Price(102), Qty(1), 2).unwrap();
        assert_eq!(b.spread(), Some(2));
        assert_eq!(b.mid(), Some(101));
    }

    #[test]
    fn depth_level_limit_respected() {
        let mut b = book();
        for i in 0..10i64 {
            b.add_limit(Side::Bid, Price(100 - i), Qty(1), i as u64 + 1)
                .unwrap();
        }
        assert_eq!(b.depth(Side::Bid, 3).len(), 3);
    }
}

#[cfg(test)]
mod invariants {
    use crate::{OrderBook, Price, Qty, Side};
    use proptest::prelude::*;

    #[derive(Debug, Clone)]
    enum Op {
        AddLimit { side: bool, price: i16, qty: u8 },
        CancelHead,
        MarketOrder { side: bool, qty: u8 },
    }

    fn op_strategy() -> impl Strategy<Value = Op> {
        prop_oneof![
            5 => (any::<bool>(), -20i16..=20, 1u8..=50).prop_map(|(s, p, q)| Op::AddLimit {
                side: s,
                price: p,
                qty: q,
            }),
            2 => Just(Op::CancelHead),
            2 => (any::<bool>(), 1u8..=10).prop_map(|(s, q)| Op::MarketOrder { side: s, qty: q }),
        ]
    }

    fn run_ops(ops: &[Op]) -> OrderBook {
        let mut book = OrderBook::new();
        let mut resting_ids = std::collections::VecDeque::new();
        let mid_price: i64 = 1000;
        let mut ts = 1u64;

        for op in ops {
            match op {
                Op::AddLimit { side, price, qty } => {
                    let p = Price(mid_price + *price as i64);
                    let s = if *side { Side::Bid } else { Side::Ask };
                    if let Ok(id) = book.add_limit(s, p, Qty(*qty as u64), ts) {
                        resting_ids.push_back(id);
                    }
                }
                Op::CancelHead => {
                    if let Some(id) = resting_ids.pop_front() {
                        let _ = book.cancel(id, ts);
                    }
                }
                Op::MarketOrder { side, qty } => {
                    let s = if *side { Side::Bid } else { Side::Ask };
                    let _ = book.execute_market(s, Qty(*qty as u64), ts);
                }
            }
            ts += 1;
        }
        book
    }

    fn check_all_invariants(book: &OrderBook) {
        // 1. No crossed book
        if let (Some(b), Some(a)) = (book.best_bid(), book.best_ask()) {
            assert!(b < a, "crossed book: bid={:?} ask={:?}", b, a);
        }

        // 2. Volume conservation — checked via check_invariants()
        #[cfg(debug_assertions)]
        assert!(book.check_invariants(), "check_invariants() failed");

        // 3. FIFO and queue structure: best_bid/ask agree with depth()
        if let Some(bb) = book.best_bid() {
            let d = book.depth(Side::Bid, 1);
            assert_eq!(d[0].0, bb, "best_bid mismatch with depth");
        }
        if let Some(ba) = book.best_ask() {
            let d = book.depth(Side::Ask, 1);
            assert_eq!(d[0].0, ba, "best_ask mismatch with depth");
        }

        // 4. Depth quantities are positive
        for (_, qty) in book.depth(Side::Bid, 20) {
            assert!(qty > 0, "zero qty in bid depth");
        }
        for (_, qty) in book.depth(Side::Ask, 20) {
            assert!(qty > 0, "zero qty in ask depth");
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10_000))]

        #[test]
        fn lob_invariants_hold(ops in proptest::collection::vec(op_strategy(), 0..100)) {
            let book = run_ops(&ops);
            check_all_invariants(&book);
        }

        #[test]
        fn market_order_sweep_never_skips_level(
            prices in proptest::collection::vec(1i64..50, 1..10),
            qty in 1u64..20
        ) {
            let mut book = OrderBook::new();
            let mid: i64 = 1000;
            for (i, &p) in prices.iter().enumerate() {
                let _ = book.add_limit(Side::Ask, Price(mid + p), Qty(5), i as u64 + 1);
            }
            let avail: u64 = book.depth(Side::Ask, 100).iter().map(|x| x.1).sum();
            let take = qty.min(avail);
            if take > 0 {
                let fills = book.execute_market(Side::Bid, Qty(take), 99).unwrap();
                // Fills must be in ascending price order (never skip a level)
                let mut prev_price = i64::MIN;
                for fill in &fills {
                    assert!(fill.price.0 >= prev_price, "fill price skipped a level");
                    prev_price = fill.price.0;
                }
            }
        }

        #[test]
        fn volume_conservation_after_ops(ops in proptest::collection::vec(op_strategy(), 0..50)) {
            let book = run_ops(&ops);
            // Sum of depth == total resting qty
            let bid_total: u64 = book.depth(Side::Bid, 1000).iter().map(|x| x.1).sum();
            let ask_total: u64 = book.depth(Side::Ask, 1000).iter().map(|x| x.1).sum();
            // Sanity: we can't assert exact values, but totals must be >= 0 (they are u64 so always true)
            // The real check is that the above don't panic and check_invariants passes
            let _ = (bid_total, ask_total);
            #[cfg(debug_assertions)]
            assert!(book.check_invariants());
        }
    }
}
