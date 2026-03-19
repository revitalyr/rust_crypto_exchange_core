//! Price level operations and utilities.

use crate::memory_pool::OrderNode;
use crypto_exchange_common::price::Price;
use std::collections::VecDeque;

/// Price level representation
#[derive(Debug, Clone)]
pub struct PriceLevel {
    /// Price
    pub price: Price,
    /// Total quantity at this price
    pub quantity: u64,
    /// Number of orders at this price
    pub order_count: u64,
}

impl PriceLevel {
    /// Creates a new price level
    pub fn new(price: Price, quantity: u64, order_count: u64) -> Self {
        Self {
            price,
            quantity,
            order_count,
        }
    }

    /// Returns the total value at this price level
    pub fn total_value(&self) -> Option<u64> {
        self.price.value().checked_mul(self.quantity)
    }

    /// Checks if this price level is empty
    pub fn is_empty(&self) -> bool {
        self.quantity == 0 || self.order_count == 0
    }
}

/// Order queue at a specific price level
#[derive(Debug)]
pub struct OrderQueue {
    /// Price of this level
    price: Price,
    /// Queue of orders (FIFO)
    orders: VecDeque<*mut OrderNode>,
    /// Total quantity at this level
    total_quantity: u64,
}

impl OrderQueue {
    /// Creates a new order queue
    pub fn new(price: Price) -> Self {
        Self {
            price,
            orders: VecDeque::new(),
            total_quantity: 0,
        }
    }

    /// Adds an order to the queue
    pub fn push_back(&mut self, order_node: *mut OrderNode) {
        unsafe {
            let quantity = (*order_node).order.remaining_quantity();
            self.orders.push_back(order_node);
            self.total_quantity += quantity;
        }
    }

    /// Removes and returns the front order
    pub fn pop_front(&mut self) -> Option<*mut OrderNode> {
        let order_node = self.orders.pop_front()?;
        
        unsafe {
            let quantity = (*order_node).order.remaining_quantity();
            self.total_quantity -= quantity;
        }
        
        Some(order_node)
    }

    /// Peeks at the front order without removing it
    pub fn peek_front(&self) -> Option<*mut OrderNode> {
        self.orders.front().copied()
    }

    /// Removes a specific order from the queue
    pub fn remove(&mut self, target_order_id: u64) -> Option<*mut OrderNode> {
        let pos = self.orders.iter().position(|&node| unsafe {
            (*node).order.id == target_order_id
        })?;

        let order_node = self.orders.remove(pos)?;
        
        unsafe {
            let quantity = (*order_node).order.remaining_quantity();
            self.total_quantity -= quantity;
        }
        
        Some(order_node)
    }

    /// Gets the number of orders in the queue
    pub fn len(&self) -> usize {
        self.orders.len()
    }

    /// Checks if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    /// Gets the total quantity at this level
    pub fn total_quantity(&self) -> u64 {
        self.total_quantity
    }

    /// Gets the price of this level
    pub fn price(&self) -> Price {
        self.price
    }

    /// Updates the total quantity (called when an order is partially filled)
    pub fn update_quantity(&mut self, delta: i64) {
        if delta >= 0 {
            self.total_quantity += delta as u64;
        } else {
            self.total_quantity = self.total_quantity.saturating_sub((-delta) as u64);
        }
    }

    /// Returns an iterator over the orders in this queue
    pub fn iter(&self) -> impl Iterator<Item = *mut OrderNode> + '_ {
        self.orders.iter().copied()
    }

    /// Clears all orders from the queue
    pub fn clear(&mut self) {
        self.orders.clear();
        self.total_quantity = 0;
    }
}

/// Price level manager with efficient operations
#[derive(Debug)]
pub struct PriceLevelManager {
    /// Price levels sorted by price
    levels: std::collections::BTreeMap<u64, OrderQueue>,
    /// Whether this manages bids (true) or asks (false)
    is_bid: bool,
}

impl PriceLevelManager {
    /// Creates a new price level manager
    pub fn new(is_bid: bool) -> Self {
        Self {
            levels: std::collections::BTreeMap::new(),
            is_bid,
        }
    }

    /// Gets or creates an order queue for the given price
    pub fn get_or_create_queue(&mut self, price: Price) -> &mut OrderQueue {
        self.levels
            .entry(price.value())
            .or_insert_with(|| OrderQueue::new(price))
    }

    /// Gets the order queue for the given price
    pub fn get_queue(&self, price: Price) -> Option<&OrderQueue> {
        self.levels.get(&price.value())
    }

    /// Gets the best price level
    pub fn best_level(&self) -> Option<&OrderQueue> {
        if self.is_bid {
            // For bids, best is the highest price
            self.levels.values().next_back()
        } else {
            // For asks, best is the lowest price
            self.levels.values().next()
        }
    }

    /// Gets the best price
    pub fn best_price(&self) -> Option<Price> {
        self.best_level().map(|queue| queue.price())
    }

    /// Removes an empty price level
    pub fn remove_empty_level(&mut self, price: Price) {
        if let Some(queue) = self.levels.get(&price.value()) {
            if queue.is_empty() {
                self.levels.remove(&price.value());
            }
        }
    }

    /// Gets all price levels as a vector
    pub fn get_levels(&self, depth: usize) -> Vec<PriceLevel> {
        let levels: Vec<PriceLevel> = if self.is_bid {
            self.levels
                .iter()
                .rev()
                .take(depth)
                .map(|(&price, queue)| PriceLevel {
                    price: Price::new(price),
                    quantity: queue.total_quantity(),
                    order_count: queue.len() as u64,
                })
                .collect()
        } else {
            self.levels
                .iter()
                .take(depth)
                .map(|(&price, queue)| PriceLevel {
                    price: Price::new(price),
                    quantity: queue.total_quantity(),
                    order_count: queue.len() as u64,
                })
                .collect()
        };

        levels
    }

    /// Gets the total quantity across all levels
    pub fn total_quantity(&self) -> u64 {
        self.levels.values().map(|queue| queue.total_quantity()).sum()
    }

    /// Gets the total number of orders across all levels
    pub fn total_order_count(&self) -> u64 {
        self.levels.values().map(|queue| queue.len() as u64).sum()
    }

    /// Gets the number of price levels
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Finds the best price that can match a market order
    pub fn find_matching_price(&self, market_price: Option<u64>) -> Option<Price> {
        if let Some(market_price) = market_price {
            if self.is_bid {
                // For bids, find highest price <= market_price
                self.levels
                    .range(..=market_price)
                    .next_back()
                    .map(|(&price, _)| Price::new(price))
            } else {
                // For asks, find lowest price >= market_price
                self.levels
                    .range(market_price..)
                    .next()
                    .map(|(&price, _)| Price::new(price))
            }
        } else {
            // No price limit, return best price
            self.best_price()
        }
    }

    /// Clears all price levels
    pub fn clear(&mut self) {
        self.levels.clear();
    }

    /// Returns statistics
    pub fn stats(&self) -> PriceLevelManagerStats {
        PriceLevelManagerStats {
            level_count: self.level_count(),
            total_quantity: self.total_quantity(),
            total_orders: self.total_order_count(),
            best_price: self.best_price(),
        }
    }
}

/// Statistics for price level manager
#[derive(Debug, Clone)]
pub struct PriceLevelManagerStats {
    /// Number of price levels
    pub level_count: usize,
    /// Total quantity
    pub total_quantity: u64,
    /// Total number of orders
    pub total_orders: u64,
    /// Best price (if any)
    pub best_price: Option<Price>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_pool::MemoryPool;
    use crypto_exchange_common::{
        order::{Order, OrderSide, OrderType, TimeInForce},
        price::Price,
    };

    fn create_test_order(id: u64, price: u64, quantity: u64) -> Order {
        Order::new(
            id,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(price)),
            quantity,
            TimeInForce::GTC,
            1234567890,
        )
    }

    #[test]
    fn test_order_queue() {
        let memory_pool = MemoryPool::new(100);
        let mut queue = OrderQueue::new(Price::new(50000));

        assert!(queue.is_empty());
        assert_eq!(queue.total_quantity(), 0);

        // Add orders
        let order1 = create_test_order(1, 50000, 1000);
        let order2 = create_test_order(2, 50000, 500);

        let node1 = memory_pool.allocate_order(order1);
        let node2 = memory_pool.allocate_order(order2);

        queue.push_back(node1);
        queue.push_back(node2);

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.total_quantity(), 1500);

        // Peek and pop
        assert!(!queue.peek_front().is_null());
        let popped = queue.pop_front().unwrap();
        assert!(!popped.is_null());

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.total_quantity(), 500);

        // Clean up
        memory_pool.deallocate_order(node1);
        memory_pool.deallocate_order(node2);
    }

    #[test]
    fn test_price_level_manager() {
        let mut manager = PriceLevelManager::new(true); // Bids

        // Add some price levels
        let queue1 = manager.get_or_create_queue(Price::new(50000));
        let queue2 = manager.get_or_create_queue(Price::new(50100));
        let queue3 = manager.get_or_create_queue(Price::new(49900));

        // Best price should be the highest for bids
        assert_eq!(manager.best_price(), Some(Price::new(50100)));

        // Test getting levels
        let levels = manager.get_levels(10);
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0].price, Price::new(50100)); // Best first
        assert_eq!(levels[1].price, Price::new(50000));
        assert_eq!(levels[2].price, Price::new(49900));

        // Test statistics
        let stats = manager.stats();
        assert_eq!(stats.level_count, 3);
        assert_eq!(stats.best_price, Some(Price::new(50100)));
    }

    #[test]
    fn test_price_level_matching() {
        let mut manager = PriceLevelManager::new(false); // Asks

        // Add ask levels
        manager.get_or_create_queue(Price::new(50200));
        manager.get_or_create_queue(Price::new(50300));
        manager.get_or_create_queue(Price::new(50100));

        // Best price should be the lowest for asks
        assert_eq!(manager.best_price(), Some(Price::new(50100)));

        // Test matching
        assert_eq!(
            manager.find_matching_price(Some(50250)),
            Some(Price::new(50300)) // First ask >= 50250
        );
        assert_eq!(
            manager.find_matching_price(Some(50100)),
            Some(Price::new(50100)) // Exact match
        );
    }
}
