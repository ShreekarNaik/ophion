use rustc_hash::FxHashMap;
use std::collections::{BTreeMap, VecDeque};

use crate::{Fill, OrderBookError, OrderId, Price, Qty, Side};

struct PriceLevel {
    total_qty: u64,
    queue: VecDeque<(OrderId, Qty)>,
}

struct OrderLocator {
    side: Side,
    price: Price,
}

pub struct OrderBook {
    bids: BTreeMap<Price, PriceLevel>,
    asks: BTreeMap<Price, PriceLevel>,
    orders: FxHashMap<u64, OrderLocator>,
    next_id: u64,
    pub last_ts: u64,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: FxHashMap::default(),
            next_id: 1,
            last_ts: 0,
        }
    }

    pub fn add_limit(
        &mut self,
        side: Side,
        price: Price,
        qty: Qty,
        ts: u64,
    ) -> Result<OrderId, OrderBookError> {
        if qty.0 == 0 {
            return Err(OrderBookError::ZeroQuantity);
        }
        self.last_ts = ts;

        // Continuous matching: sweep opposing side at prices that cross this order.
        let mut remaining = qty.0;
        loop {
            let opposing_price = match side {
                Side::Bid => self.asks.keys().next().copied().filter(|&ba| ba <= price),
                Side::Ask => self
                    .bids
                    .keys()
                    .next_back()
                    .copied()
                    .filter(|&bb| bb >= price),
            };
            let opposing_price = match opposing_price {
                Some(p) => p,
                None => break,
            };
            if remaining == 0 {
                break;
            }
            let level = match side {
                Side::Bid => self.asks.get_mut(&opposing_price),
                Side::Ask => self.bids.get_mut(&opposing_price),
            };
            if let Some(level) = level {
                while remaining > 0 {
                    let Some((_front_id, front_qty)) = level.queue.front_mut() else {
                        break;
                    };
                    let take = front_qty.0.min(remaining);
                    front_qty.0 -= take;
                    level.total_qty -= take;
                    remaining -= take;
                    if front_qty.0 == 0 {
                        if let Some((removed_id, _)) = level.queue.pop_front() {
                            self.orders.remove(&removed_id.0);
                        }
                    }
                }
            }
            // Remove exhausted level
            match side {
                Side::Bid => {
                    self.asks.retain(|_, l| l.total_qty > 0);
                }
                Side::Ask => {
                    self.bids.retain(|_, l| l.total_qty > 0);
                }
            }
        }

        if remaining == 0 {
            // Fully filled during matching; return a valid id but nothing rests
            let id = OrderId(self.next_id);
            self.next_id += 1;
            return Ok(id);
        }

        // Rest the unfilled remainder
        let id = OrderId(self.next_id);
        self.next_id += 1;
        let book_side = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };
        let level = book_side.entry(price).or_insert_with(|| PriceLevel {
            total_qty: 0,
            queue: VecDeque::new(),
        });
        level.total_qty += remaining;
        level.queue.push_back((id, Qty(remaining)));
        self.orders.insert(id.0, OrderLocator { side, price });
        Ok(id)
    }

    pub fn cancel(&mut self, id: OrderId, ts: u64) -> Result<Qty, OrderBookError> {
        let loc = self
            .orders
            .remove(&id.0)
            .ok_or(OrderBookError::OrderNotFound(id.0))?;
        self.last_ts = ts;

        let book_side = match loc.side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };
        let level = book_side
            .get_mut(&loc.price)
            .ok_or(OrderBookError::OrderNotFound(id.0))?;

        let pos = level
            .queue
            .iter()
            .position(|(oid, _)| *oid == id)
            .ok_or(OrderBookError::OrderNotFound(id.0))?;
        let (_, qty) = level
            .queue
            .remove(pos)
            .ok_or(OrderBookError::OrderNotFound(id.0))?;
        level.total_qty -= qty.0;
        if level.queue.is_empty() {
            book_side.remove(&loc.price);
        }
        Ok(qty)
    }

    pub fn execute_market(
        &mut self,
        side: Side,
        qty: Qty,
        ts: u64,
    ) -> Result<Vec<Fill>, OrderBookError> {
        if qty.0 == 0 {
            return Err(OrderBookError::ZeroQuantity);
        }
        self.last_ts = ts;

        let opposing = match side {
            Side::Bid => &mut self.asks,
            Side::Ask => &mut self.bids,
        };

        let avail: u64 = opposing.values().map(|l| l.total_qty).sum();
        if avail < qty.0 {
            return Err(OrderBookError::InsufficientLiquidity {
                needed: qty.0,
                available: avail,
            });
        }

        let mut remaining = qty.0;
        let mut fills = Vec::new();

        // Collect prices to process (avoid borrow issues)
        let prices: Vec<Price> = match side {
            Side::Bid => opposing.keys().copied().collect(), // ascending for asks
            Side::Ask => opposing.keys().rev().copied().collect(), // descending for bids
        };

        for price in prices {
            if remaining == 0 {
                break;
            }
            let level = match opposing.get_mut(&price) {
                Some(l) => l,
                None => continue,
            };
            while remaining > 0 {
                let Some((front_id, front_qty)) = level.queue.front_mut() else {
                    break;
                };
                let take = front_qty.0.min(remaining);
                fills.push(Fill {
                    order_id: *front_id,
                    side,
                    price,
                    qty: Qty(take),
                    ts,
                });
                front_qty.0 -= take;
                level.total_qty -= take;
                remaining -= take;
                if front_qty.0 == 0 {
                    if let Some((removed_id, _)) = level.queue.pop_front() {
                        self.orders.remove(&removed_id.0);
                    }
                }
            }
        }

        // Remove empty levels
        opposing.retain(|_, l| l.total_qty > 0);

        Ok(fills)
    }

    pub fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    pub fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    pub fn spread(&self) -> Option<i64> {
        Some(self.best_ask()?.0 - self.best_bid()?.0)
    }

    pub fn mid(&self) -> Option<i64> {
        Some((self.best_bid()?.0 + self.best_ask()?.0) / 2)
    }

    pub fn orders_contains(&self, id: OrderId) -> bool {
        self.orders.contains_key(&id.0)
    }

    pub fn depth(&self, side: Side, levels: usize) -> Vec<(Price, u64)> {
        match side {
            Side::Bid => self
                .bids
                .iter()
                .rev()
                .take(levels)
                .map(|(p, l)| (*p, l.total_qty))
                .collect(),
            Side::Ask => self
                .asks
                .iter()
                .take(levels)
                .map(|(p, l)| (*p, l.total_qty))
                .collect(),
        }
    }

    #[cfg(debug_assertions)]
    pub fn check_invariants(&self) -> bool {
        // Volume conservation
        for level in self.bids.values().chain(self.asks.values()) {
            let sum: u64 = level.queue.iter().map(|(_, q)| q.0).sum();
            if sum != level.total_qty {
                return false;
            }
        }
        // No crossed book
        if let (Some(b), Some(a)) = (self.best_bid(), self.best_ask()) {
            if b >= a {
                return false;
            }
        }
        // Locator consistency
        for (id, loc) in &self.orders {
            let book_side = match loc.side {
                Side::Bid => &self.bids,
                Side::Ask => &self.asks,
            };
            if let Some(level) = book_side.get(&loc.price) {
                if !level.queue.iter().any(|(oid, _)| oid.0 == *id) {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}
