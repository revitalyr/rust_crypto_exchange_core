//! Order book side (bids or asks) implementation.

use crate::memory_pool::{MemoryPool, OrderNode, PriceLevelNode};
use crate::price_levels::PriceLevels;
use crypto_exchange_common::{
    order::{Order, OrderSide},
    price::Price,
    ExchangeError, ExchangeResult,
};
use parking_lot::RwLock;
use std::sync::Arc;

/// Represents one side of the order book (bids or asks)
pub struct OrderBookSide {
    /// Price levels for this side
    price_levels: PriceLevels,
    /// Whether this is the bid side (true) or ask side (false)
    is_bid: bool,
    /// Memory pool for allocations
    memory_pool: Arc<MemoryPool>,
}

impl OrderBookSide {
    /// Creates a new order book side
    pub fn new(is_bid: bool, memory_pool: Arc<MemoryPool>) -> Self {
        Self {
            price_levels: PriceLevels::new(memory_pool.clone()),
            is_bid,
            memory_pool,
        }
    }

    /// Adds an order to this side of the order book
    pub fn add_order(&mut self, order: Order) -> ExchangeResult<*mut OrderNode> {
        // Validate order
        order.validate()?;

        let price = order.price.ok_or_else(|| {
            ExchangeError::invalid_order("Order must have a price for limit orders")
        })?;

        let price_value = price.value();
        
        // Get or create price level
        let level_ptr = self.price_levels.upsert_level(price_value);
        
        // Allocate order node
        let order_node = self.memory_pool.allocate_order(order);
        
        // Add order to price level
        unsafe {
            (*level_ptr).add_order(order_node);
        }

        Ok(order_node)
    }

    /// Removes an order from this side of the order book
    pub fn remove_order(&mut self, order_node: *mut OrderNode) -> ExchangeResult<()> {
        if order_node.is_null() {
            return Err(ExchangeError::invalid_order("Null order node"));
        }

        unsafe {
            let order = &(*order_node).order;
            let price = order.price.ok_or_else(|| {
                ExchangeError::invalid_order("Order must have a price")
            })?;

            let price_value = price.value();
            
            // Remove from price level
            if let Some(level_ptr) = self.price_levels.get_level(price_value) {
                (*level_ptr).remove_order(order_node);
                
                // Remove price level if empty
                self.price_levels.remove_level_if_empty(price_value);
            }

            // Deallocate order node
            self.memory_pool.deallocate_order(order_node);
        }

        Ok(())
    }

    /// Finds the best order for matching against a market order
    pub fn find_best_match(&self, market_price: Option<u64>) -> Option<*mut OrderNode> {
        let target_price = if self.is_bid {
            // For bids, we want the highest price
            market_price
        } else {
            // For asks, we want the lowest price
            market_price
        };

        if let Some(price) = target_price {
            self.price_levels.find_matching_price(price, self.is_bid)
                .and_then(|price| self.price_levels.get_level(price))
                .and_then(|level_ptr| unsafe { (*level_ptr).head })
        } else {
            // No price limit, get the best order
            self.price_levels.best_level()
                .and_then(|level_ptr| unsafe { (*level_ptr).head })
        }
    }

    /// Gets the best price for this side
    pub fn best_price(&self) -> Option<Price> {
        self.price_levels.best_price().map(Price::new)
    }

    /// Gets the total quantity at the best price
    pub fn best_quantity(&self) -> Option<u64> {
        self.price_levels
            .best_level()
            .map(|ptr| unsafe { (*ptr).total_quantity })
    }

    /// Gets the best bid-ask spread
    pub fn spread(&self, other_side: &OrderBookSide) -> Option<u64> {
        let bid_price = if self.is_bid {
            self.best_price()?.value()
        } else {
            other_side.best_price()?.value()
        };

        let ask_price = if self.is_bid {
            other_side.best_price()?.value()
        } else {
            self.best_price()?.value()
        };

        if bid_price > 0 && ask_price > 0 {
            Some(ask_price - bid_price)
        } else {
            None
        }
    }

    /// Gets the order book depth (number of price levels)
    pub fn depth(&self) -> usize {
        self.price_levels.stats().level_count
    }

    /// Gets the total quantity across all price levels
    pub fn total_quantity(&self) -> u64 {
        self.price_levels.total_quantity()
    }

    /// Gets the total number of orders across all price levels
    pub fn total_order_count(&self) -> u64 {
        self.price_levels.total_order_count()
    }

    /// Gets the order book snapshot for a price range
    pub fn get_snapshot(&self, depth: usize) -> Vec<PriceLevel> {
        self.price_levels
            .iter_best_to_worst()
            .take(depth)
            .map(|(price, level_ptr)| unsafe {
                PriceLevel {
                    price: Price::new(price),
                    quantity: (*level_ptr).total_quantity,
                    order_count: (*level_ptr).order_count,
                }
            })
            .collect()
    }

    /// Gets the order book levels in a price range
    pub fn get_levels_in_range(&self, min_price: u64, max_price: u64) -> Vec<PriceLevel> {
        self.price_levels
            .levels_in_range(min_price, max_price)
            .into_iter()
            .map(|(price, quantity)| PriceLevel {
                price: Price::new(price),
                quantity,
                order_count: 1, // We don't track order count in this method
            })
            .collect()
    }

    /// Checks if this side can match a market order
    pub fn can_match_market(&self, quantity: u64) -> bool {
        self.price_levels.total_quantity() >= quantity
    }

    /// Estimates the execution price for a market order
    pub fn estimate_market_price(&self, quantity: u64) -> Option<Price> {
        let mut remaining_quantity = quantity;
        let mut total_cost = 0u64;

        for (price, level_ptr) in self.price_levels.iter_best_to_worst() {
            unsafe {
                let level_quantity = (*level_ptr).total_quantity;
                let fill_quantity = remaining_quantity.min(level_quantity);
                
                total_cost += price.checked_mul(fill_quantity)?;
                remaining_quantity -= fill_quantity;

                if remaining_quantity == 0 {
                    break;
                }
            }
        }

        if remaining_quantity == 0 {
            Some(Price::new(total_cost / quantity))
        } else {
            None // Not enough liquidity
        }
    }

    /// Validates the order book side for consistency
    pub fn validate(&self) -> ExchangeResult<()> {
        let stats = self.price_levels.stats();
        
        // Check that total quantities are consistent
        let mut calculated_total = 0u64;
        let mut calculated_orders = 0u64;

        for (_, level_ptr) in self.price_levels.iter_best_to_worst() {
            unsafe {
                calculated_total += (*level_ptr).total_quantity;
                calculated_orders += (*level_ptr).order_count;
            }
        }

        if calculated_total != stats.total_quantity {
            return Err(ExchangeError::system_error(
                "Total quantity mismatch in order book validation"
            ));
        }

        if calculated_orders != stats.total_orders {
            return Err(ExchangeError::system_error(
                "Total order count mismatch in order book validation"
            ));
        }

        Ok(())
    }

    /// Returns statistics for this side
    pub fn stats(&self) -> OrderBookSideStats {
        let price_levels_stats = self.price_levels.stats();
        OrderBookSideStats {
            is_bid: self.is_bid,
            best_price: price_levels_stats.best_price.map(Price::new),
            depth: price_levels_stats.level_count,
            total_quantity: price_levels_stats.total_quantity,
            total_orders: price_levels_stats.total_orders,
        }
    }
}

/// Price level snapshot
#[derive(Debug, Clone)]
pub struct PriceLevel {
    /// Price
    pub price: Price,
    /// Total quantity at this price
    pub quantity: u64,
    /// Number of orders at this price
    pub order_count: u64,
}

/// Statistics for order book side
#[derive(Debug, Clone)]
pub struct OrderBookSideStats {
    /// Whether this is the bid side
    pub is_bid: bool,
    /// Best price (if any)
    pub best_price: Option<Price>,
    /// Number of price levels
    pub depth: usize,
    /// Total quantity
    pub total_quantity: u64,
    /// Total number of orders
    pub total_orders: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::{order::OrderType, order::TimeInForce};

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
    fn test_order_book_side_basic() {
        let memory_pool = Arc::new(MemoryPool::new(100));
        let mut bids = OrderBookSide::new(true, memory_pool.clone());

        // Initially empty
        assert_eq!(bids.best_price(), None);
        assert_eq!(bids.total_quantity(), 0);
        assert_eq!(bids.depth(), 0);

        // Add an order
        let order = create_test_order(1, 50000, 1000);
        let order_node = bids.add_order(order).unwrap();

        assert_eq!(bids.best_price(), Some(Price::new(50000)));
        assert_eq!(bids.total_quantity(), 1000);
        assert_eq!(bids.depth(), 1);

        // Remove the order
        bids.remove_order(order_node).unwrap();
        assert_eq!(bids.best_price(), None);
        assert_eq!(bids.total_quantity(), 0);
        assert_eq!(bids.depth(), 0);
    }

    #[test]
    fn test_order_book_side_multiple_prices() {
        let memory_pool = Arc::new(MemoryPool::new(100));
        let mut bids = OrderBookSide::new(true, memory_pool.clone());

        // Add orders at different prices
        let order1 = create_test_order(1, 50000, 1000);
        let order2 = create_test_order(2, 50100, 500);
        let order3 = create_test_order(3, 49900, 750);

        bids.add_order(order1).unwrap();
        bids.add_order(order2).unwrap();
        bids.add_order(order3).unwrap();

        // Best price should be the highest (for bids)
        assert_eq!(bids.best_price(), Some(Price::new(50100)));
        assert_eq!(bids.total_quantity(), 2250); // 1000 + 500 + 750
        assert_eq!(bids.depth(), 3);
    }

    #[test]
    fn test_market_order_matching() {
        let memory_pool = Arc::new(MemoryPool::new(100));
        let mut asks = OrderBookSide::new(false, memory_pool.clone());

        // Add ask orders
        let order1 = create_test_order(1, 50200, 1000);
        let order2 = create_test_order(2, 50300, 500);
        let order3 = create_test_order(3, 50100, 750);

        // Note: We're using buy orders for asks in this test, just for simplicity
        asks.add_order(order1).unwrap();
        asks.add_order(order2).unwrap();
        asks.add_order(order3).unwrap();

        // Best price should be the lowest (for asks)
        assert_eq!(asks.best_price(), Some(Price::new(50100)));

        // Test market order matching
        let best_match = asks.find_best_match(Some(50200));
        assert!(!best_match.is_null());

        // Test market price estimation
        let estimated_price = asks.estimate_market_price(1000);
        assert!(estimated_price.is_some());
    }

    #[test]
    fn test_order_book_validation() {
        let memory_pool = Arc::new(MemoryPool::new(100));
        let mut bids = OrderBookSide::new(true, memory_pool.clone());

        // Add some orders
        let order1 = create_test_order(1, 50000, 1000);
        let order2 = create_test_order(2, 50100, 500);

        bids.add_order(order1).unwrap();
        bids.add_order(order2).unwrap();

        // Validation should pass
        assert!(bids.validate().is_ok());
    }
}
