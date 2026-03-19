//! Trade execution and order processing.

use crypto_exchange_common::{
    order::{Order, OrderSide, OrderType, TimeInForce},
    price::Price,
    ExchangeError, ExchangeResult,
};
use crate::{
    matcher::{OrderMatcher, PriceTimeMatcher, MarketOrderMatcher},
    types::{MatchingEngineConfig, OrderExecution, TradeExecution, MatchingResult, MatchingEngineEvent},
};

/// Trade executor responsible for executing trades and updating orders
pub struct TradeExecutor {
    /// Price-time matcher for limit orders
    limit_matcher: PriceTimeMatcher,
    /// Market order matcher
    market_matcher: MarketOrderMatcher,
    /// Engine configuration
    config: MatchingEngineConfig,
    /// Next trade sequence number
    next_sequence: u64,
}

impl TradeExecutor {
    /// Creates a new trade executor
    pub fn new(config: MatchingEngineConfig) -> Self {
        Self {
            limit_matcher: PriceTimeMatcher::new(
                config.tick_size,
                config.max_price_deviation_bps,
                config.enable_price_protection,
            ),
            market_matcher: MarketOrderMatcher::new(
                config.max_price_deviation_bps,
                config.max_price_deviation_bps,
            ),
            config,
            next_sequence: 1,
        }
    }

    /// Executes a limit order
    pub fn execute_limit_order(&mut self, mut order: Order) -> ExchangeResult<MatchingResult> {
        // Validate the order
        self.validate_limit_order(&order)?;

        // Use the limit matcher
        let result = self.limit_matcher.match_order(&mut order)?;
        
        // Generate events for each trade
        for trade in &result.trades {
            self.generate_trade_event(trade)?;
        }

        Ok(result)
    }

    /// Executes a market order
    pub fn execute_market_order(&mut self, mut order: Order) -> ExchangeResult<MatchingResult> {
        // Validate the order
        self.validate_market_order(&order)?;

        // Use the market matcher
        let result = self.market_matcher.match_order(&mut order)?;
        
        // Generate events for each trade
        for trade in &result.trades {
            self.generate_trade_event(trade)?;
        }

        Ok(result)
    }

    /// Executes an order based on its type
    pub fn execute_order(&mut self, order: Order) -> ExchangeResult<MatchingResult> {
        match order.order_type {
            OrderType::Limit => self.execute_limit_order(order),
            OrderType::Market => self.execute_market_order(order),
        }
    }

    /// Cancels an order
    pub fn cancel_order(&self, order_id: u64, user_id: u64, reason: Option<String>) -> ExchangeResult<MatchingEngineEvent> {
        // In a real implementation, we would find and remove the order from the order book
        // For now, we'll generate a cancellation event
        
        let event = MatchingEngineEvent::OrderCancelled {
            order_id,
            user_id,
            reason,
            timestamp: crypto_exchange_common::timestamp::now(),
        };

        Ok(event)
    }

    /// Validates a limit order
    fn validate_limit_order(&self, order: &Order) -> ExchangeResult<()> {
        if order.order_type != OrderType::Limit {
            return Err(ExchangeError::invalid_order("Expected limit order"));
        }

        if order.price.is_none() {
            return Err(ExchangeError::invalid_order("Limit order must have a price"));
        }

        let price = order.price.unwrap();

        // Check price range
        if price.value() == 0 {
            return Err(ExchangeError::invalid_price(price.value()));
        }

        // Check quantity
        if order.quantity < self.config.min_order_size {
            return Err(ExchangeError::invalid_quantity(order.quantity));
        }

        if order.quantity > self.config.max_order_size {
            return Err(ExchangeError::invalid_quantity(order.quantity));
        }

        // Check tick size alignment
        if price.value() % self.config.tick_size != 0 {
            return Err(ExchangeError::invalid_order("Price must align with tick size"));
        }

        // Check lot size alignment
        if order.quantity % self.config.lot_size != 0 {
            return Err(ExchangeError::invalid_order("Quantity must align with lot size"));
        }

        Ok(())
    }

    /// Validates a market order
    fn validate_market_order(&self, order: &Order) -> ExchangeResult<()> {
        if order.order_type != OrderType::Market {
            return Err(ExchangeError::invalid_order("Expected market order"));
        }

        if order.price.is_some() {
            return Err(ExchangeError::invalid_order("Market order should not have a price"));
        }

        // Check quantity
        if order.quantity < self.config.min_order_size {
            return Err(ExchangeError::invalid_quantity(order.quantity));
        }

        if order.quantity > self.config.max_order_size {
            return Err(ExchangeError::invalid_quantity(order.quantity));
        }

        // Market orders should use IOC or FOK
        match order.time_in_force {
            TimeInForce::IOC | TimeInForce::FOK => {}
            TimeInForce::GTC => {
                return Err(ExchangeError::invalid_order("Market orders cannot use GTC time in force"));
            }
        }

        Ok(())
    }

    /// Generates a trade event
    fn generate_trade_event(&mut self, trade: &TradeExecution) -> ExchangeResult<()> {
        // In a real implementation, this would publish the event to an event bus
        // For now, we'll just increment the sequence number
        self.next_sequence += 1;
        Ok(())
    }

    /// Gets the next trade sequence number
    pub fn next_sequence(&mut self) -> u64 {
        let seq = self.next_sequence;
        self.next_sequence += 1;
        seq
    }

    /// Simulates executing a trade between two orders
    pub fn simulate_trade(
        &mut self,
        maker_order: &mut Order,
        taker_order: &mut Order,
    ) -> ExchangeResult<TradeExecution> {
        // Validate that orders can match
        if maker_order.side == taker_order.side {
            return Err(ExchangeError::invalid_order("Orders must be on opposite sides"));
        }

        if maker_order.remaining_quantity() == 0 || taker_order.remaining_quantity() == 0 {
            return Err(ExchangeError::invalid_order("One or both orders have no remaining quantity"));
        }

        // Determine trade quantity
        let trade_quantity = maker_order.remaining_quantity().min(taker_order.remaining_quantity());

        // Determine execution price (maker price)
        let execution_price = maker_order.price.ok_or_else(|| {
            ExchangeError::invalid_order("Maker order must have a price")
        })?;

        // Validate price protection for taker order
        if taker_order.order_type == OrderType::Limit && self.config.enable_price_protection {
            let taker_price = taker_order.price.unwrap();
            let price_deviation_bps = match taker_order.side {
                OrderSide::Buy => {
                    if execution_price.value() > taker_price.value() {
                        ((execution_price.value() - taker_price.value()) * 10000) / taker_price.value()
                    } else {
                        0
                    }
                }
                OrderSide::Sell => {
                    if execution_price.value() < taker_price.value() {
                        ((taker_price.value() - execution_price.value()) * 10000) / taker_price.value()
                    } else {
                        0
                    }
                }
            };

            if price_deviation_bps > self.config.max_price_deviation_bps {
                return Err(ExchangeError::market_order_error(
                    format!("Price deviation {} bps exceeds maximum {}", price_deviation_bps, self.config.max_price_deviation_bps)
                ));
            }
        }

        // Update orders
        let timestamp = crypto_exchange_common::timestamp::now();
        maker_order.fill(trade_quantity, timestamp)?;
        taker_order.fill(trade_quantity, timestamp)?;

        // Determine maker and taker roles (maker is the order that was in the book first)
        let (maker_order_id, taker_order_id, maker_user_id, taker_user_id) = 
            if maker_order.created_at <= taker_order.created_at {
                (maker_order.id, taker_order.id, maker_order.user_id, taker_order.user_id)
            } else {
                // This case would be unusual but could happen in some implementations
                (taker_order.id, maker_order.id, taker_order.user_id, maker_order.user_id)
            };

        // Create trade execution
        let trade_execution = TradeExecution::new(
            maker_order_id,
            taker_order_id,
            maker_user_id,
            taker_user_id,
            maker_order.pair.clone(),
            execution_price,
            trade_quantity,
            taker_order.side,
            timestamp,
            self.next_sequence(),
        );

        Ok(trade_execution)
    }

    /// Calculates the execution statistics for a set of trades
    pub fn calculate_execution_stats(&self, trades: &[TradeExecution]) -> ExecutionStats {
        let total_quantity: u64 = trades.iter().map(|t| t.quantity).sum();
        let total_value: u64 = trades.iter().filter_map(|t| t.total_value()).sum();
        let avg_price = if total_quantity > 0 {
            Some(Price::new(total_value / total_quantity))
        } else {
            None
        };

        let (buy_volume, sell_volume) = trades.iter().fold((0u64, 0u64), |(buy, sell), trade| {
            if let Some(value) = trade.total_value() {
                match trade.side {
                    OrderSide::Buy => (buy + value, sell),
                    OrderSide::Sell => (buy, sell + value),
                }
            } else {
                (buy, sell)
            }
        });

        ExecutionStats {
            trade_count: trades.len(),
            total_quantity,
            total_value,
            avg_price,
            buy_volume,
            sell_volume,
        }
    }
}

/// Execution statistics for a set of trades
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    /// Number of trades
    pub trade_count: usize,
    /// Total quantity traded
    pub total_quantity: u64,
    /// Total value traded
    pub total_value: u64,
    /// Average execution price
    pub avg_price: Option<Price>,
    /// Total buy volume
    pub buy_volume: u64,
    /// Total sell volume
    pub sell_volume: u64,
}

impl ExecutionStats {
    /// Creates empty execution stats
    pub fn empty() -> Self {
        Self {
            trade_count: 0,
            total_quantity: 0,
            total_value: 0,
            avg_price: None,
            buy_volume: 0,
            sell_volume: 0,
        }
    }

    /// Checks if there are any trades
    pub fn has_trades(&self) -> bool {
        self.trade_count > 0
    }

    /// Gets the volume imbalance (buy - sell)
    pub fn volume_imbalance(&self) -> i64 {
        self.buy_volume as i64 - self.sell_volume as i64
    }

    /// Gets the buy/sell ratio
    pub fn buy_sell_ratio(&self) -> Option<f64> {
        if self.sell_volume == 0 {
            None
        } else {
            Some(self.buy_volume as f64 / self.sell_volume as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::order::OrderStatus;

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
    fn test_trade_executor_creation() {
        let config = MatchingEngineConfig::default();
        let executor = TradeExecutor::new(config);
        
        assert_eq!(executor.next_sequence, 1);
    }

    #[test]
    fn test_limit_order_validation() {
        let config = MatchingEngineConfig::default();
        let executor = TradeExecutor::new(config);
        
        // Valid limit order
        let valid_order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        assert!(executor.validate_limit_order(&valid_order).is_ok());

        // Invalid limit order (no price)
        let invalid_order = create_test_order(2, OrderSide::Buy, OrderType::Limit, None, 1000);
        assert!(executor.validate_limit_order(&invalid_order).is_err());

        // Invalid limit order (wrong tick size)
        let bad_tick_order = create_test_order(3, OrderSide::Buy, OrderType::Limit, Some(50001), 1000);
        assert!(executor.validate_limit_order(&bad_tick_order).is_err());
    }

    #[test]
    fn test_market_order_validation() {
        let config = MatchingEngineConfig::default();
        let executor = TradeExecutor::new(config);
        
        // Valid market order
        let mut valid_order = create_test_order(1, OrderSide::Buy, OrderType::Market, None, 1000);
        valid_order.time_in_force = TimeInForce::IOC;
        assert!(executor.validate_market_order(&valid_order).is_ok());

        // Invalid market order (has price)
        let invalid_order = create_test_order(2, OrderSide::Buy, OrderType::Market, Some(50000), 1000);
        assert!(executor.validate_market_order(&invalid_order).is_err());

        // Invalid market order (GTC time in force)
        let mut bad_tif_order = create_test_order(3, OrderSide::Buy, OrderType::Market, None, 1000);
        bad_tif_order.time_in_force = TimeInForce::GTC;
        assert!(executor.validate_market_order(&bad_tif_order).is_err());
    }

    #[test]
    fn test_simulate_trade() {
        let config = MatchingEngineConfig::default();
        let mut executor = TradeExecutor::new(config);
        
        let mut maker_order = create_test_order(1, OrderSide::Sell, OrderType::Limit, Some(50000), 1000);
        let mut taker_order = create_test_order(2, OrderSide::Buy, OrderType::Limit, Some(50100), 500);
        
        let trade = executor.simulate_trade(&mut maker_order, &mut taker_order).unwrap();
        
        assert_eq!(trade.quantity, 500);
        assert_eq!(trade.price, Price::new(50000)); // Maker price
        assert_eq!(trade.maker_order_id, 1);
        assert_eq!(trade.taker_order_id, 2);
        assert_eq!(trade.side, OrderSide::Buy);
        
        // Check order updates
        assert_eq!(maker_order.filled_quantity, 500);
        assert_eq!(taker_order.filled_quantity, 500);
        assert_eq!(taker_order.status, OrderStatus::Filled);
    }

    #[test]
    fn test_execution_stats() {
        let trades = vec![
            TradeExecution::new(
                1, 2, 100, 200,
                "BTC/USDT".to_string(),
                Price::new(50000), 1000,
                OrderSide::Buy, 1234567890, 1,
            ),
            TradeExecution::new(
                3, 4, 100, 200,
                "BTC/USDT".to_string(),
                Price::new(50100), 500,
                OrderSide::Sell, 1234567891, 2,
            ),
        ];

        let stats = ExecutionStats {
            trade_count: trades.len(),
            total_quantity: trades.iter().map(|t| t.quantity).sum(),
            total_value: trades.iter().filter_map(|t| t.total_value()).sum(),
            avg_price: Some(Price::new(50033)), // Calculated manually
            buy_volume: 50_000_000,
            sell_volume: 25_050_000,
        };

        assert_eq!(stats.trade_count, 2);
        assert_eq!(stats.total_quantity, 1500);
        assert!(stats.has_trades());
        assert_eq!(stats.volume_imbalance(), 24_950_000);
        assert_eq!(stats.buy_sell_ratio(), Some(50_000_000.0 / 25_050_000.0));
    }

    #[test]
    fn test_order_cancellation() {
        let config = MatchingEngineConfig::default();
        let executor = TradeExecutor::new(config);
        
        let event = executor.cancel_order(1, 100, Some("User requested".to_string())).unwrap();
        
        match event {
            MatchingEngineEvent::OrderCancelled { order_id, user_id, reason, .. } => {
                assert_eq!(order_id, 1);
                assert_eq!(user_id, 100);
                assert_eq!(reason, Some("User requested".to_string()));
            }
            _ => panic!("Expected order cancelled event"),
        }
    }
}
