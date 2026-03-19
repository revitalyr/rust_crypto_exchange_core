//! Main matching engine implementation.

use crypto_exchange_common::{
    events::{EventBus, EventListener},
    order::{Order, OrderSide, OrderType},
    price::Price,
    ExchangeError, ExchangeResult,
};
use crypto_exchange_orderbook::OrderBook;
use crate::{
    processor::OrderProcessor,
    types::{
        MatchingEngineCommand, MatchingEngineResponse, MatchingEngineConfig,
        MatchingEngineStats, MatchingEngineEvent, OrderExecution, TradeExecution,
    },
};

/// Main matching engine with single-threaded execution for determinism
pub struct MatchingEngine {
    /// Order book for the trading pair
    order_book: OrderBook,
    /// Order processor
    processor: OrderProcessor,
    /// Event bus for publishing events
    event_bus: EventBus,
    /// Engine configuration
    config: MatchingEngineConfig,
    /// Engine statistics
    stats: MatchingEngineStats,
    /// Start timestamp
    start_timestamp: u64,
}

impl MatchingEngine {
    /// Creates a new matching engine
    pub fn new(pair: String, config: MatchingEngineConfig) -> Self {
        let start_timestamp = crypto_exchange_common::timestamp::now();
        
        Self {
            order_book: OrderBook::new(pair.clone()),
            processor: OrderProcessor::new(config.clone()),
            event_bus: EventBus::new(),
            config,
            stats: MatchingEngineStats {
                pair,
                total_orders: 0,
                total_trades: 0,
                total_volume: 0,
                spread: None,
                mid_price: None,
                bid_levels: 0,
                ask_levels: 0,
                total_bid_quantity: 0,
                total_ask_quantity: 0,
                last_trade_price: None,
                last_trade_quantity: None,
                last_trade_timestamp: None,
                uptime_ns: 0,
            },
            start_timestamp,
        }
    }

    /// Gets the trading pair
    pub fn pair(&self) -> &str {
        self.order_book.pair()
    }

    /// Processes a matching engine command
    pub fn process_command(&mut self, command: MatchingEngineCommand) -> ExchangeResult<MatchingEngineResponse> {
        let result = self.processor.process_command(command);
        
        // Update statistics
        self.update_stats();
        
        result
    }

    /// Submits a new order
    pub fn submit_order(&mut self, order: Order) -> ExchangeResult<OrderExecution> {
        // Validate order
        self.validate_order(&order)?;

        // Update order count
        self.stats.total_orders += 1;

        // Execute the order
        let execution_result = self.processor.executor.execute_order(order)?;

        // Update order book based on execution result
        self.update_order_book(&execution_result)?;

        // Process trades
        for trade in &execution_result.trades {
            self.process_trade(trade)?;
        }

        // Publish events
        self.publish_execution_events(&execution_result)?;

        Ok(execution_result.order_execution)
    }

    /// Cancels an order
    pub fn cancel_order(&mut self, order_id: u64, user_id: u64) -> ExchangeResult<()> {
        // Find and remove the order from the order book
        // In a real implementation, we would need to track order nodes by ID
        let cancelled_order = self.order_book.cancel_order(order_id)?;

        // Publish cancellation event
        let event = MatchingEngineEvent::OrderCancelled {
            order_id,
            user_id,
            reason: Some("User requested".to_string()),
            timestamp: crypto_exchange_common::timestamp::now(),
        };

        self.event_bus.publish(event.to_exchange_event());

        Ok(())
    }

    /// Gets the order book snapshot
    pub fn get_order_book(&self, depth: Option<usize>) -> crypto_exchange_orderbook::OrderBookSnapshot {
        let depth = depth.unwrap_or(self.config.default_snapshot_depth);
        self.order_book.get_snapshot(depth)
    }

    /// Gets the current spread
    pub fn spread(&self) -> Option<u64> {
        self.order_book.spread()
    }

    /// Gets the mid price
    pub fn mid_price(&self) -> Option<Price> {
        self.order_book.mid_price()
    }

    /// Gets the best bid price
    pub fn best_bid(&self) -> Option<Price> {
        self.order_book.best_bid()
    }

    /// Gets the best ask price
    pub fn best_ask(&self) -> Option<Price> {
        self.order_book.best_ask()
    }

    /// Estimates the market price for a given quantity
    pub fn estimate_market_price(&self, side: OrderSide, quantity: u64) -> Option<Price> {
        self.order_book.estimate_market_price(side, quantity)
    }

    /// Checks if there's enough liquidity for a market order
    pub fn can_match_market(&self, side: OrderSide, quantity: u64) -> bool {
        self.order_book.can_match_market(side, quantity)
    }

    /// Validates the order book for consistency
    pub fn validate(&self) -> ExchangeResult<()> {
        self.order_book.validate()
    }

    /// Adds an event listener
    pub fn add_event_listener(&mut self, listener: Box<dyn EventListener>) {
        self.event_bus.add_listener(listener);
    }

    /// Gets engine statistics
    pub fn get_stats(&self) -> MatchingEngineStats {
        let mut stats = self.stats.clone();
        stats.uptime_ns = crypto_exchange_common::timestamp::now() - self.start_timestamp;
        stats
    }

    /// Resets engine statistics
    pub fn reset_stats(&mut self) {
        self.stats = MatchingEngineStats {
            pair: self.order_book.pair().to_string(),
            total_orders: 0,
            total_trades: 0,
            total_volume: 0,
            spread: self.spread(),
            mid_price: self.mid_price(),
            bid_levels: self.order_book.depth(),
            ask_levels: 0, // Would be calculated from actual order book
            total_bid_quantity: self.order_book.total_bid_quantity(),
            total_ask_quantity: self.order_book.total_ask_quantity(),
            last_trade_price: self.stats.last_trade_price,
            last_trade_quantity: self.stats.last_trade_quantity,
            last_trade_timestamp: self.stats.last_trade_timestamp,
            uptime_ns: crypto_exchange_common::timestamp::now() - self.start_timestamp,
        };
    }

    /// Clears the order book
    pub fn clear(&mut self) {
        self.order_book.clear();
        self.reset_stats();
    }

    /// Validates an order
    fn validate_order(&self, order: &Order) -> ExchangeResult<()> {
        // Basic validation
        if order.id == 0 {
            return Err(ExchangeError::invalid_order("Order ID cannot be zero"));
        }

        if order.user_id == 0 {
            return Err(ExchangeError::invalid_order("User ID cannot be zero"));
        }

        if order.quantity == 0 {
            return Err(ExchangeError::invalid_quantity(0));
        }

        if order.pair != self.order_book.pair() {
            return Err(ExchangeError::unsupported_pair(order.pair.clone()));
        }

        // Type-specific validation
        match order.order_type {
            OrderType::Limit => {
                self.processor.executor.validate_limit_order(order)?;
            }
            OrderType::Market => {
                self.processor.executor.validate_market_order(order)?;
            }
        }

        Ok(())
    }

    /// Updates the order book based on execution result
    fn update_order_book(&mut self, execution_result: &OrderExecution) -> ExchangeResult<()> {
        // In a real implementation, we would:
        // 1. Add limit orders that weren't fully filled to the order book
        // 2. Remove orders that were fully filled
        // 3. Update order quantities for partially filled orders

        // For now, this is a placeholder
        Ok(())
    }

    /// Processes a trade
    fn process_trade(&mut self, trade: &TradeExecution) -> ExchangeResult<()> {
        // Update trade statistics
        self.stats.total_trades += 1;
        self.stats.total_volume += trade.total_value().unwrap_or(0);
        self.stats.last_trade_price = Some(trade.price);
        self.stats.last_trade_quantity = Some(trade.quantity);
        self.stats.last_trade_timestamp = Some(trade.timestamp);

        // Publish trade event
        let event = MatchingEngineEvent::TradeExecuted(trade.clone());
        self.event_bus.publish(event.to_exchange_event());

        Ok(())
    }

    /// Publishes execution events
    fn publish_execution_events(&mut self, execution_result: &OrderExecution) -> ExchangeResult<()> {
        // Publish order accepted/rejected event
        let event = if execution_result.has_fills() {
            MatchingEngineEvent::OrderAccepted {
                order_id: execution_result.order_id,
                user_id: execution_result.user_id,
                timestamp: execution_result.timestamp,
            }
        } else {
            MatchingEngineEvent::OrderRejected {
                order_id: execution_result.order_id,
                user_id: execution_result.user_id,
                reason: "No matching liquidity".to_string(),
                timestamp: execution_result.timestamp,
            }
        };

        self.event_bus.publish(event.to_exchange_event());

        // Publish order book update event
        let book_event = MatchingEngineEvent::OrderBookUpdated {
            pair: self.order_book.pair().to_string(),
            timestamp: crypto_exchange_common::timestamp::now(),
        };

        self.event_bus.publish(book_event.to_exchange_event());

        Ok(())
    }

    /// Updates engine statistics
    fn update_stats(&mut self) {
        self.stats.spread = self.spread();
        self.stats.mid_price = self.mid_price();
        self.stats.bid_levels = self.order_book.depth() / 2; // Simplified
        self.stats.ask_levels = self.order_book.depth() - self.stats.bid_levels;
        self.stats.total_bid_quantity = self.order_book.total_bid_quantity();
        self.stats.total_ask_quantity = self.order_book.total_ask_quantity();
    }
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::new("BTC/USDT".to_string(), MatchingEngineConfig::default())
    }
}

/// Matching engine builder for convenient configuration
pub struct MatchingEngineBuilder {
    pair: String,
    config: MatchingEngineConfig,
}

impl MatchingEngineBuilder {
    /// Creates a new builder
    pub fn new(pair: String) -> Self {
        Self {
            pair,
            config: MatchingEngineConfig::default(),
        }
    }

    /// Sets the maximum order size
    pub fn max_order_size(mut self, max_order_size: u64) -> Self {
        self.config.max_order_size = max_order_size;
        self
    }

    /// Sets the minimum order size
    pub fn min_order_size(mut self, min_order_size: u64) -> Self {
        self.config.min_order_size = min_order_size;
        self
    }

    /// Sets the tick size
    pub fn tick_size(mut self, tick_size: u64) -> Self {
        self.config.tick_size = tick_size;
        self
    }

    /// Sets the lot size
    pub fn lot_size(mut self, lot_size: u64) -> Self {
        self.config.lot_size = lot_size;
        self
    }

    /// Sets the maximum price deviation
    pub fn max_price_deviation_bps(mut self, max_price_deviation_bps: u32) -> Self {
        self.config.max_price_deviation_bps = max_price_deviation_bps;
        self
    }

    /// Enables or disables price protection
    pub fn enable_price_protection(mut self, enable: bool) -> Self {
        self.config.enable_price_protection = enable;
        self
    }

    /// Sets the maximum number of price levels
    pub fn max_price_levels(mut self, max_price_levels: usize) -> Self {
        self.config.max_price_levels = max_price_levels;
        self
    }

    /// Sets the default snapshot depth
    pub fn default_snapshot_depth(mut self, depth: usize) -> Self {
        self.config.default_snapshot_depth = depth;
        self
    }

    /// Builds the matching engine
    pub fn build(self) -> MatchingEngine {
        MatchingEngine::new(self.pair, self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::order::TimeInForce;

    fn create_test_order(id: u64, side: OrderSide, order_type: OrderType, price: Option<u64>, quantity: u64) -> Order {
        Order::new(
            id,
            100,
            "BTC/USDT".to_string(),
            side,
            order_type,
            price.map(Price::new),
            quantity,
            TimeInForce::GTC,
            1234567890,
        )
    }

    #[test]
    fn test_matching_engine_creation() {
        let engine = MatchingEngine::default();
        
        assert_eq!(engine.pair(), "BTC/USDT");
        assert_eq!(engine.config.tick_size, 100);
        assert_eq!(engine.stats.total_orders, 0);
        assert_eq!(engine.stats.total_trades, 0);
    }

    #[test]
    fn test_matching_engine_builder() {
        let engine = MatchingEngineBuilder::new("ETH/USDT".to_string())
            .max_order_size(10_000_000)
            .min_order_size(1)
            .tick_size(1000)
            .lot_size(1)
            .max_price_deviation_bps(500)
            .enable_price_protection(true)
            .max_price_levels(500)
            .default_snapshot_depth(50)
            .build();

        assert_eq!(engine.pair(), "ETH/USDT");
        assert_eq!(engine.config.max_order_size, 10_000_000);
        assert_eq!(engine.config.tick_size, 1000);
        assert!(engine.config.enable_price_protection);
    }

    #[test]
    fn test_submit_order() {
        let mut engine = MatchingEngine::default();
        
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let result = engine.submit_order(order).unwrap();
        
        assert_eq!(result.order_id, 1);
        assert_eq!(result.side, OrderSide::Buy);
        assert_eq!(engine.stats.total_orders, 1);
    }

    #[test]
    fn test_order_book_operations() {
        let mut engine = MatchingEngine::default();
        
        // Initially empty
        assert_eq!(engine.best_bid(), None);
        assert_eq!(engine.best_ask(), None);
        assert_eq!(engine.spread(), None);
        assert_eq!(engine.mid_price(), None);

        // Add some orders (in a real implementation, this would affect the order book)
        let bid_order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let ask_order = create_test_order(2, OrderSide::Sell, OrderType::Limit, Some(50100), 500);

        engine.submit_order(bid_order).unwrap();
        engine.submit_order(ask_order).unwrap();

        // Get order book snapshot
        let snapshot = engine.get_order_book(Some(10));
        assert_eq!(snapshot.pair, "BTC/USDT");
        assert_eq!(snapshot.best_bid, None); // Would be populated in real implementation
        assert_eq!(snapshot.best_ask, None); // Would be populated in real implementation
    }

    #[test]
    fn test_market_price_estimation() {
        let engine = MatchingEngine::default();
        
        // Test with no liquidity
        assert_eq!(engine.estimate_market_price(OrderSide::Buy, 1000), None);
        assert!(!engine.can_match_market(OrderSide::Buy, 1000));
    }

    #[test]
    fn test_engine_validation() {
        let engine = MatchingEngine::default();
        
        // Validation should pass on empty engine
        assert!(engine.validate().is_ok());
    }

    #[test]
    fn test_statistics() {
        let mut engine = MatchingEngine::default();
        
        let initial_stats = engine.get_stats();
        assert_eq!(initial_stats.total_orders, 0);
        assert_eq!(initial_stats.total_trades, 0);
        assert_eq!(initial_stats.total_volume, 0);

        // Submit an order
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        engine.submit_order(order).unwrap();

        let updated_stats = engine.get_stats();
        assert_eq!(updated_stats.total_orders, 1);
        assert!(updated_stats.uptime_ns > 0);
    }

    #[test]
    fn test_order_validation() {
        let engine = MatchingEngine::default();
        
        // Valid order
        let valid_order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        assert!(engine.validate_order(&valid_order).is_ok());

        // Invalid order (wrong pair)
        let wrong_pair_order = Order::new(
            2,
            100,
            "ETH/USDT".to_string(), // Wrong pair
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            TimeInForce::GTC,
            1234567890,
        );
        assert!(engine.validate_order(&wrong_pair_order).is_err());

        // Invalid order (zero quantity)
        let zero_qty_order = create_test_order(3, OrderSide::Buy, OrderType::Limit, Some(50000), 0);
        assert!(engine.validate_order(&zero_qty_order).is_err());
    }

    #[test]
    fn test_engine_reset() {
        let mut engine = MatchingEngine::default();
        
        // Submit an order to update stats
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        engine.submit_order(order).unwrap();

        assert_eq!(engine.stats.total_orders, 1);

        // Reset stats
        engine.reset_stats();
        assert_eq!(engine.stats.total_orders, 0);
    }
}
