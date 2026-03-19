//! Main order book implementation.

use crate::memory_pool::MemoryPool;
use crate::side::{OrderBookSide, OrderBookSideStats};
use crypto_exchange_common::{
    order::{Order, OrderSide, OrderType, TimeInForce},
    price::Price,
    ExchangeError, ExchangeResult,
};
use parking_lot::RwLock;
use std::sync::Arc;

/// Main order book implementation with lock-free operations
pub struct OrderBook {
    /// Bid orders (buy orders)
    bids: OrderBookSide,
    /// Ask orders (sell orders)
    asks: OrderBookSide,
    /// Memory pool for allocations
    memory_pool: Arc<MemoryPool>,
    /// Trading pair symbol
    pair: String,
    /// Next trade sequence number
    next_sequence: Arc<RwLock<u64>>,
}

impl OrderBook {
    /// Creates a new order book
    pub fn new(pair: String) -> Self {
        let memory_pool = Arc::new(MemoryPool::new(10000));
        Self {
            bids: OrderBookSide::new(true, memory_pool.clone()),
            asks: OrderBookSide::new(false, memory_pool),
            memory_pool,
            pair,
            next_sequence: Arc::new(RwLock::new(1)),
        }
    }

    /// Gets the trading pair
    pub fn pair(&self) -> &str {
        &self.pair
    }

    /// Adds a limit order to the order book
    pub fn add_limit_order(&mut self, order: Order) -> ExchangeResult<()> {
        if order.order_type != OrderType::Limit {
            return Err(ExchangeError::invalid_order("Expected limit order"));
        }

        match order.side {
            OrderSide::Buy => {
                self.bids.add_order(order)?;
            }
            OrderSide::Sell => {
                self.asks.add_order(order)?;
            }
        }

        Ok(())
    }

    /// Cancels an order from the order book
    pub fn cancel_order(&mut self, order_id: u64) -> ExchangeResult<Order> {
        // Find and remove the order
        // In a real implementation, we'd need to track order nodes by ID
        // For now, this is a simplified version
        Err(ExchangeError::order_not_found(order_id))
    }

    /// Gets the best bid price
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.best_price()
    }

    /// Gets the best ask price
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.best_price()
    }

    /// Gets the bid-ask spread
    pub fn spread(&self) -> Option<u64> {
        self.bids.spread(&self.asks)
    }

    /// Gets the mid price (average of best bid and ask)
    pub fn mid_price(&self) -> Option<Price> {
        let best_bid = self.best_bid()?;
        let best_ask = self.best_ask()?;
        best_bid.midpoint(best_ask)
    }

    /// Gets the order book depth (total number of price levels)
    pub fn depth(&self) -> usize {
        self.bids.depth() + self.asks.depth()
    }

    /// Gets the total bid quantity
    pub fn total_bid_quantity(&self) -> u64 {
        self.bids.total_quantity()
    }

    /// Gets the total ask quantity
    pub fn total_ask_quantity(&self) -> u64 {
        self.asks.total_quantity()
    }

    /// Gets the total number of orders
    pub fn total_order_count(&self) -> u64 {
        self.bids.total_order_count() + self.asks.total_order_count()
    }

    /// Gets the order book snapshot
    pub fn get_snapshot(&self, depth: usize) -> OrderBookSnapshot {
        let bids = self.bids.get_snapshot(depth);
        let asks = self.asks.get_snapshot(depth);

        OrderBookSnapshot {
            pair: self.pair.clone(),
            bids,
            asks,
            best_bid: self.best_bid(),
            best_ask: self.best_ask(),
            spread: self.spread(),
            timestamp: crypto_exchange_common::timestamp::now(),
        }
    }

    /// Gets the order book levels in a price range
    pub fn get_levels_in_range(&self, min_price: u64, max_price: u64) -> OrderBookLevels {
        let bids = self.bids.get_levels_in_range(min_price, max_price);
        let asks = self.asks.get_levels_in_range(min_price, max_price);

        OrderBookLevels {
            pair: self.pair.clone(),
            bids,
            asks,
            timestamp: crypto_exchange_common::timestamp::now(),
        }
    }

    /// Estimates the market price for a given quantity
    pub fn estimate_market_price(&self, side: OrderSide, quantity: u64) -> Option<Price> {
        match side {
            OrderSide::Buy => self.asks.estimate_market_price(quantity),
            OrderSide::Sell => self.bids.estimate_market_price(quantity),
        }
    }

    /// Checks if there's enough liquidity for a market order
    pub fn can_match_market(&self, side: OrderSide, quantity: u64) -> bool {
        match side {
            OrderSide::Buy => self.asks.can_match_market(quantity),
            OrderSide::Sell => self.bids.can_match_market(quantity),
        }
    }

    /// Gets the next trade sequence number
    pub fn next_sequence(&self) -> u64 {
        let mut seq = self.next_sequence.write();
        let current = *seq;
        *seq += 1;
        current
    }

    /// Validates the order book for consistency
    pub fn validate(&self) -> ExchangeResult<()> {
        self.bids.validate()?;
        self.asks.validate()?;

        // Additional cross-side validations
        if let (Some(best_bid), Some(best_ask)) = (self.best_bid(), self.best_ask()) {
            if best_bid.value() > best_ask.value() {
                return Err(ExchangeError::system_error(
                    "Best bid price exceeds best ask price"
                ));
            }
        }

        Ok(())
    }

    /// Returns comprehensive order book statistics
    pub fn stats(&self) -> OrderBookStats {
        OrderBookStats {
            pair: self.pair.clone(),
            bid_stats: self.bids.stats(),
            ask_stats: self.asks.stats(),
            best_bid: self.best_bid(),
            best_ask: self.best_ask(),
            spread: self.spread(),
            mid_price: self.mid_price(),
            depth: self.depth(),
            total_bid_quantity: self.total_bid_quantity(),
            total_ask_quantity: self.total_ask_quantity(),
            total_orders: self.total_order_count(),
            memory_pool_stats: self.memory_pool.stats(),
        }
    }

    /// Clears the order book
    pub fn clear(&mut self) {
        // Note: In a real implementation, we'd need to properly deallocate all nodes
        // For now, this is simplified
        self.bids = OrderBookSide::new(true, self.memory_pool.clone());
        self.asks = OrderBookSide::new(false, self.memory_pool.clone());
    }
}

/// Order book snapshot
#[derive(Debug, Clone)]
pub struct OrderBookSnapshot {
    /// Trading pair
    pub pair: String,
    /// Bid levels (best to worst)
    pub bids: Vec<crate::side::PriceLevel>,
    /// Ask levels (best to worst)
    pub asks: Vec<crate::side::PriceLevel>,
    /// Best bid price
    pub best_bid: Option<Price>,
    /// Best ask price
    pub best_ask: Option<Price>,
    /// Current spread
    pub spread: Option<u64>,
    /// Snapshot timestamp
    pub timestamp: u64,
}

/// Order book levels in a price range
#[derive(Debug, Clone)]
pub struct OrderBookLevels {
    /// Trading pair
    pub pair: String,
    /// Bid levels in range
    pub bids: Vec<crate::side::PriceLevel>,
    /// Ask levels in range
    pub asks: Vec<crate::side::PriceLevel>,
    /// Timestamp
    pub timestamp: u64,
}

/// Comprehensive order book statistics
#[derive(Debug, Clone)]
pub struct OrderBookStats {
    /// Trading pair
    pub pair: String,
    /// Bid side statistics
    pub bid_stats: OrderBookSideStats,
    /// Ask side statistics
    pub ask_stats: OrderBookSideStats,
    /// Best bid price
    pub best_bid: Option<Price>,
    /// Best ask price
    pub best_ask: Option<Price>,
    /// Current spread
    pub spread: Option<u64>,
    /// Mid price
    pub mid_price: Option<Price>,
    /// Total depth (number of price levels)
    pub depth: usize,
    /// Total bid quantity
    pub total_bid_quantity: u64,
    /// Total ask quantity
    pub total_ask_quantity: u64,
    /// Total number of orders
    pub total_orders: u64,
    /// Memory pool statistics
    pub memory_pool_stats: crate::memory_pool::MemoryPoolStats,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new("BTC/USDT".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::order::OrderSide;

    fn create_test_order(id: u64, side: OrderSide, price: u64, quantity: u64) -> Order {
        Order::new(
            id,
            100,
            "BTC/USDT".to_string(),
            side,
            OrderType::Limit,
            Some(Price::new(price)),
            quantity,
            TimeInForce::GTC,
            1234567890,
        )
    }

    #[test]
    fn test_order_book_basic() {
        let mut book = OrderBook::new("BTC/USDT".to_string());

        // Initially empty
        assert_eq!(book.best_bid(), None);
        assert_eq!(book.best_ask(), None);
        assert_eq!(book.spread(), None);
        assert_eq!(book.depth(), 0);
        assert_eq!(book.total_order_count(), 0);
    }

    #[test]
    fn test_order_book_add_orders() {
        let mut book = OrderBook::new("BTC/USDT".to_string());

        // Add bid order
        let bid_order = create_test_order(1, OrderSide::Buy, 50000, 1000);
        book.add_limit_order(bid_order).unwrap();

        assert_eq!(book.best_bid(), Some(Price::new(50000)));
        assert_eq!(book.total_bid_quantity(), 1000);
        assert_eq!(book.depth(), 1);

        // Add ask order
        let ask_order = create_test_order(2, OrderSide::Sell, 50100, 500);
        book.add_limit_order(ask_order).unwrap();

        assert_eq!(book.best_ask(), Some(Price::new(50100)));
        assert_eq!(book.total_ask_quantity(), 500);
        assert_eq!(book.depth(), 2);
        assert_eq!(book.spread(), Some(100));
    }

    #[test]
    fn test_order_book_spread() {
        let mut book = OrderBook::new("BTC/USDT".to_string());

        // Add orders with spread
        let bid_order = create_test_order(1, OrderSide::Buy, 50000, 1000);
        let ask_order = create_test_order(2, OrderSide::Sell, 50100, 500);

        book.add_limit_order(bid_order).unwrap();
        book.add_limit_order(ask_order).unwrap();

        assert_eq!(book.spread(), Some(100));
        assert_eq!(book.mid_price(), Some(Price::new(50050)));
    }

    #[test]
    fn test_order_book_snapshot() {
        let mut book = OrderBook::new("BTC/USDT".to_string());

        // Add some orders
        let bid_order = create_test_order(1, OrderSide::Buy, 50000, 1000);
        let ask_order = create_test_order(2, OrderSide::Sell, 50100, 500);

        book.add_limit_order(bid_order).unwrap();
        book.add_limit_order(ask_order).unwrap();

        // Get snapshot
        let snapshot = book.get_snapshot(10);
        assert_eq!(snapshot.pair, "BTC/USDT");
        assert_eq!(snapshot.bids.len(), 1);
        assert_eq!(snapshot.asks.len(), 1);
        assert_eq!(snapshot.best_bid, Some(Price::new(50000)));
        assert_eq!(snapshot.best_ask, Some(Price::new(50100)));
        assert_eq!(snapshot.spread, Some(100));
    }

    #[test]
    fn test_order_book_validation() {
        let mut book = OrderBook::new("BTC/USDT".to_string());

        // Add orders
        let bid_order = create_test_order(1, OrderSide::Buy, 50000, 1000);
        let ask_order = create_test_order(2, OrderSide::Sell, 50100, 500);

        book.add_limit_order(bid_order).unwrap();
        book.add_limit_order(ask_order).unwrap();

        // Validation should pass
        assert!(book.validate().is_ok());
    }

    #[test]
    fn test_market_price_estimation() {
        let mut book = OrderBook::new("BTC/USDT".to_string());

        // Add ask orders
        let ask1 = create_test_order(1, OrderSide::Sell, 50100, 500);
        let ask2 = create_test_order(2, OrderSide::Sell, 50200, 1000);

        book.add_limit_order(ask1).unwrap();
        book.add_limit_order(ask2).unwrap();

        // Estimate market price for buy order
        let estimated_price = book.estimate_market_price(OrderSide::Buy, 750);
        assert!(estimated_price.is_some());

        // Check liquidity
        assert!(book.can_match_market(OrderSide::Buy, 750));
        assert!(book.can_match_market(OrderSide::Buy, 1500));
        assert!(!book.can_match_market(OrderSide::Buy, 2000)); // Not enough liquidity
    }

    #[test]
    fn test_order_book_stats() {
        let mut book = OrderBook::new("BTC/USDT".to_string());

        // Add orders
        let bid_order = create_test_order(1, OrderSide::Buy, 50000, 1000);
        let ask_order = create_test_order(2, OrderSide::Sell, 50100, 500);

        book.add_limit_order(bid_order).unwrap();
        book.add_limit_order(ask_order).unwrap();

        // Get stats
        let stats = book.stats();
        assert_eq!(stats.pair, "BTC/USDT");
        assert_eq!(stats.best_bid, Some(Price::new(50000)));
        assert_eq!(stats.best_ask, Some(Price::new(50100)));
        assert_eq!(stats.spread, Some(100));
        assert_eq!(stats.total_bid_quantity, 1000);
        assert_eq!(stats.total_ask_quantity, 500);
        assert_eq!(stats.total_orders, 2);
    }
}
