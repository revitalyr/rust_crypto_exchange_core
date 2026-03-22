//! Price level management for the order book.

use crate::memory_pool::{MemoryPool, PriceLevelNode};
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Manages price levels for one side of the order book
pub struct PriceLevels {
    /// Price levels sorted by price
    levels: BTreeMap<u64, *mut PriceLevelNode>,
    /// Memory pool for allocations
    memory_pool: Arc<MemoryPool>,
    /// Best price (cached for fast access)
    best_price: RwLock<Option<u64>>,
}

impl PriceLevels {
    /// Creates a new price levels manager
    pub fn new(memory_pool: Arc<MemoryPool>) -> Self {
        Self {
            levels: BTreeMap::new(),
            memory_pool,
            best_price: RwLock::new(None),
        }
    }

    /// Gets the best price (highest for bids, lowest for asks)
    pub fn best_price(&self) -> Option<u64> {
        *self.best_price.read()
    }

    /// Gets the total quantity at a specific price
    pub fn quantity_at_price(&self, price: u64) -> Option<u64> {
        self.levels.get(&price).map(|&ptr| unsafe {
            (*ptr).total_quantity
        })
    }

    /// Gets the number of orders at a specific price
    pub fn order_count_at_price(&self, price: u64) -> Option<u64> {
        self.levels.get(&price).map(|&ptr| unsafe {
            (*ptr).order_count
        })
    }

    /// Checks if there are any orders at or better than the given price
    /// For bids: price >= given_price
    /// For asks: price <= given_price
    pub fn has_orders_at_or_better(&self, given_price: u64, is_bid: bool) -> bool {
        if is_bid {
            // For bids, we look for prices >= given_price
            self.levels.range(given_price..).next().is_some()
        } else {
            // For asks, we look for prices <= given_price
            self.levels.range(..=given_price).next_back().is_some()
        }
    }

    /// Gets the best price level
    pub fn best_level(&self) -> Option<*mut PriceLevelNode> {
        let best_price = *self.best_price.read();
        if let Some(price) = best_price {
            self.levels.get(&price).copied()
        } else {
            None
        }
    }

    /// Gets a price level by price
    pub fn get_level(&self, price: u64) -> Option<*mut PriceLevelNode> {
        self.levels.get(&price).copied()
    }

    /// Adds or updates a price level
    pub fn upsert_level(&mut self, price: u64) -> *mut PriceLevelNode {
        if let Some(&ptr) = self.levels.get(&price) {
            ptr
        } else {
            let ptr = self.memory_pool.allocate_price_level(price);
            self.levels.insert(price, ptr);
            
            // Update best price cache
            self.update_best_price();
            ptr
        }
    }

    /// Removes a price level if it's empty
    pub fn remove_level_if_empty(&mut self, price: u64) {
        if let Some(&ptr) = self.levels.get(&price) {
            unsafe {
                if (*ptr).is_empty() {
                    self.memory_pool.deallocate_price_level(ptr);
                    self.levels.remove(&price);
                    self.update_best_price();
                }
            }
        }
    }

    /// Updates the best price cache
    fn update_best_price(&self) {
        let mut best_price = self.best_price.write();
        if let Some((&price, _)) = self.levels.iter().next_back() {
            *best_price = Some(price);
        } else {
            *best_price = None;
        }
    }

    /// Gets an iterator over price levels from best to worst
    pub fn iter_best_to_worst(&self) -> impl Iterator<Item = (u64, *mut PriceLevelNode)> + '_ {
        self.levels.iter().rev().map(|(&price, &ptr)| (price, ptr))
    }

    /// Gets an iterator over price levels from worst to best
    pub fn iter_worst_to_best(&self) -> impl Iterator<Item = (u64, *mut PriceLevelNode)> + '_ {
        self.levels.iter().map(|(&price, &ptr)| (price, ptr))
    }

    /// Finds the best price that matches the criteria
    /// For bids: highest price <= max_price
    /// For asks: lowest price >= min_price
    pub fn find_matching_price(&self, price_limit: u64, is_bid: bool) -> Option<u64> {
        if is_bid {
            // For bids, find highest price <= max_price
            self.levels.range(..=price_limit).next_back().map(|(&price, _)| price)
        } else {
            // For asks, find lowest price >= min_price
            self.levels.range(price_limit..).next().map(|(&price, _)| price)
        }
    }

    /// Gets all price levels within a price range
    pub fn levels_in_range(&self, min_price: u64, max_price: u64) -> Vec<(u64, u64)> {
        self.levels
            .range(min_price..=max_price)
            .map(|(&price, &ptr)| unsafe { (price, (*ptr).total_quantity) })
            .collect()
    }

    /// Returns the total quantity across all price levels
    pub fn total_quantity(&self) -> u64 {
        self.levels
            .values()
            .map(|&ptr| unsafe { (*ptr).total_quantity })
            .sum()
    }

    /// Returns the total number of orders across all price levels
    pub fn total_order_count(&self) -> u64 {
        self.levels
            .values()
            .map(|&ptr| unsafe { (*ptr).order_count })
            .sum()
    }

    /// Clears all price levels
    pub fn clear(&mut self) {
        for &ptr in self.levels.values() {
            self.memory_pool.deallocate_price_level(ptr);
        }
        self.levels.clear();
        *self.best_price.write() = None;
    }

    /// Returns statistics about the price levels
    pub fn stats(&self) -> PriceLevelsStats {
        let level_count = self.levels.len();
        let total_quantity = self.total_quantity();
        let total_orders = self.total_order_count();
        let best_price = *self.best_price.read();

        PriceLevelsStats {
            level_count,
            total_quantity,
            total_orders,
            best_price,
        }
    }
}

/// Statistics for price levels
#[derive(Debug, Clone)]
pub struct PriceLevelsStats {
    /// Number of price levels
    pub level_count: usize,
    /// Total quantity across all levels
    pub total_quantity: u64,
    /// Total number of orders across all levels
    pub total_orders: u64,
    /// Best price (if any)
    pub best_price: Option<u64>,
}

impl Drop for PriceLevels {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::{order::OrderSide, price::Price};
    use crate::memory_pool::OrderNode;

    #[test]
    fn test_price_levels_basic() {
        let memory_pool = Arc::new(MemoryPool::new(100));
        let mut levels = PriceLevels::new(memory_pool.clone());

        // Initially empty
        assert_eq!(levels.best_price(), None);
        assert_eq!(levels.total_quantity(), 0);
        assert_eq!(levels.total_order_count(), 0);

        // Add a price level
        let level_ptr = levels.upsert_level(50000);
        unsafe {
            (*level_ptr).total_quantity = 1000;
            (*level_ptr).order_count = 2;
        }

        assert_eq!(levels.best_price(), Some(50000));
        assert_eq!(levels.quantity_at_price(50000), Some(1000));
        assert_eq!(levels.order_count_at_price(50000), Some(2));
    }

    #[test]
    fn test_price_levels_multiple() {
        let memory_pool = Arc::new(MemoryPool::new(100));
        let mut levels = PriceLevels::new(memory_pool.clone());

        // Add multiple price levels
        levels.upsert_level(50000);
        levels.upsert_level(50100);
        levels.upsert_level(49900);

        // Best price should be the highest
        assert_eq!(levels.best_price(), Some(50100));

        // Test range queries
        let range_levels = levels.levels_in_range(50000, 50100);
        assert_eq!(range_levels.len(), 2);
    }

    #[test]
    fn test_price_levels_matching() {
        let memory_pool = Arc::new(MemoryPool::new(100));
        let mut bids = PriceLevels::new(memory_pool.clone());
        let mut asks = PriceLevels::new(memory_pool.clone());

        // Setup bids: 49900, 50000, 50100
        bids.upsert_level(49900);
        bids.upsert_level(50000);
        bids.upsert_level(50100);

        // Setup asks: 50200, 50300, 50400
        asks.upsert_level(50200);
        asks.upsert_level(50300);
        asks.upsert_level(50400);

        // Test bid matching (highest price <= max_price)
        assert_eq!(bids.find_matching_price(50000, true), Some(50000));
        assert_eq!(bids.find_matching_price(50050, true), Some(50000));
        assert_eq!(bids.find_matching_price(50100, true), Some(50100));

        // Test ask matching (lowest price >= min_price)
        assert_eq!(asks.find_matching_price(50300, false), Some(50300));
        assert_eq!(asks.find_matching_price(50250, false), Some(50300));
        assert_eq!(asks.find_matching_price(50200, false), Some(50200));
    }

    #[test]
    fn test_price_levels_removal() {
        let memory_pool = Arc::new(MemoryPool::new(100));
        let mut levels = PriceLevels::new(memory_pool.clone());

        // Add and remove levels
        let level_ptr = levels.upsert_level(50000);
        assert_eq!(levels.best_price(), Some(50000));

        // Remove if empty (it should be removed)
        levels.remove_level_if_empty(50000);
        assert_eq!(levels.best_price(), None);

        // Add again and don't remove if not empty
        let level_ptr = levels.upsert_level(50000);
        unsafe {
            (*level_ptr).total_quantity = 1000;
            (*level_ptr).order_count = 1;
        }
        levels.remove_level_if_empty(50000); // Should not remove
        assert_eq!(levels.best_price(), Some(50000));
    }
}
