//! Order processing and command handling.

use crypto_exchange_common::{
    order::{Order, OrderSide, OrderType},
    ExchangeError, ExchangeResult,
};
use crate::{
    executor::TradeExecutor,
    types::{MatchingEngineCommand, MatchingEngineResponse, MatchingEngineConfig},
    matcher::OrderMatcher,
};

/// Order processor for handling matching engine commands
pub struct OrderProcessor {
    /// Trade executor
    executor: TradeExecutor,
    /// Engine configuration
    config: MatchingEngineConfig,
}

impl OrderProcessor {
    /// Creates a new order processor
    pub fn new(config: MatchingEngineConfig) -> Self {
        Self {
            executor: TradeExecutor::new(config.clone()),
            config,
        }
    }

    /// Processes a matching engine command
    pub fn process_command(&mut self, command: MatchingEngineCommand) -> ExchangeResult<MatchingEngineResponse> {
        match command {
            MatchingEngineCommand::SubmitOrder { order } => {
                self.process_submit_order(order)
            }
            MatchingEngineCommand::CancelOrder { order_id, user_id } => {
                self.process_cancel_order(order_id, user_id)
            }
            MatchingEngineCommand::ReplaceOrder { old_order_id, new_order } => {
                self.process_replace_order(old_order_id, new_order)
            }
            MatchingEngineCommand::GetOrderBook { depth } => {
                self.process_get_orderbook(depth)
            }
            MatchingEngineCommand::GetOrderStatus { order_id } => {
                self.process_get_order_status(order_id)
            }
        }
    }

    /// Processes order submission
    fn process_submit_order(&mut self, order: Order) -> ExchangeResult<MatchingEngineResponse> {
        let timestamp = crypto_exchange_common::timestamp::now();

        // Validate order
        if let Err(e) = self.validate_order(&order) {
            return Ok(MatchingEngineResponse::Error {
                error: e.to_string(),
                timestamp,
            });
        }

        // Execute the order
        match self.executor.execute_order(order) {
            Ok(result) => {
                // In a real implementation, we would update the order book
                // and publish events
                
                Ok(MatchingEngineResponse::OrderSubmitted {
                    order_id: result.order_execution.order_id,
                    status: result.order_execution.status,
                    timestamp,
                })
            }
            Err(e) => Ok(MatchingEngineResponse::Error {
                error: e.to_string(),
                timestamp,
            }),
        }
    }

    /// Processes order cancellation
    fn process_cancel_order(&mut self, order_id: u64, user_id: u64) -> ExchangeResult<MatchingEngineResponse> {
        let timestamp = crypto_exchange_common::timestamp::now();

        // In a real implementation, we would:
        // 1. Find the order in the order book
        // 2. Check if the user is authorized to cancel it
        // 3. Remove it from the order book
        // 4. Update any reserved balances

        match self.executor.cancel_order(order_id, user_id, None) {
            Ok(_) => Ok(MatchingEngineResponse::OrderCancelled {
                order_id,
                success: true,
                reason: None,
                timestamp,
            }),
            Err(e) => Ok(MatchingEngineResponse::OrderCancelled {
                order_id,
                success: false,
                reason: Some(e.to_string()),
                timestamp,
            }),
        }
    }

    /// Processes order replacement
    fn process_replace_order(&mut self, old_order_id: u64, new_order: Order) -> ExchangeResult<MatchingEngineResponse> {
        let timestamp = crypto_exchange_common::timestamp::now();

        // In a real implementation, we would:
        // 1. Cancel the old order
        // 2. Submit the new order
        // 3. Ensure atomicity

        // First, cancel the old order
        if let Err(e) = self.executor.cancel_order(old_order_id, new_order.user_id, Some("Order replacement".to_string())) {
            return Ok(MatchingEngineResponse::OrderReplaced {
                old_order_id,
                new_order_id: new_order.id,
                success: false,
                reason: Some(format!("Failed to cancel old order: {}", e)),
                timestamp,
            });
        }

        // Then submit the new order
        match self.executor.execute_order(new_order) {
            Ok(_) => Ok(MatchingEngineResponse::OrderReplaced {
                old_order_id,
                new_order_id: new_order.id,
                success: true,
                reason: None,
                timestamp,
            }),
            Err(e) => Ok(MatchingEngineResponse::OrderReplaced {
                old_order_id,
                new_order_id: new_order.id,
                success: false,
                reason: Some(format!("Failed to submit new order: {}", e)),
                timestamp,
            }),
        }
    }

    /// Processes order book snapshot request
    fn process_get_orderbook(&self, depth: usize) -> ExchangeResult<MatchingEngineResponse> {
        let timestamp = crypto_exchange_common::timestamp::now();

        // In a real implementation, we would get the actual order book snapshot
        // For now, we'll return an empty snapshot
        
        Ok(MatchingEngineResponse::OrderBookSnapshot {
            pair: "BTC/USDT".to_string(),
            bids: Vec::new(),
            asks: Vec::new(),
            timestamp,
        })
    }

    /// Processes order status request
    fn process_get_order_status(&self, order_id: u64) -> ExchangeResult<MatchingEngineResponse> {
        let timestamp = crypto_exchange_common::timestamp::now();

        // In a real implementation, we would look up the order status
        // For now, we'll return a not found error
        
        Ok(MatchingEngineResponse::Error {
            error: format!("Order {} not found", order_id),
            timestamp,
        })
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

        if order.pair.is_empty() {
            return Err(ExchangeError::invalid_order("Trading pair cannot be empty"));
        }

        // Type-specific validation
        match order.order_type {
            OrderType::Limit => {
                self.executor.validate_limit_order(order)?;
            }
            OrderType::Market => {
                self.executor.validate_market_order(order)?;
            }
        }

        Ok(())
    }

    /// Gets processor statistics
    pub fn get_stats(&self) -> ProcessorStats {
        ProcessorStats {
            // In a real implementation, we would track actual statistics
            orders_processed: 0,
            trades_executed: 0,
            total_volume: 0,
            active_orders: 0,
            uptime_ns: crypto_exchange_common::timestamp::now(),
        }
    }
}

/// Processor statistics
#[derive(Debug, Clone)]
pub struct ProcessorStats {
    /// Total orders processed
    pub orders_processed: u64,
    /// Total trades executed
    pub trades_executed: u64,
    /// Total trading volume
    pub total_volume: u64,
    /// Number of active orders
    pub active_orders: u64,
    /// Processor uptime in nanoseconds
    pub uptime_ns: u64,
}

/// Batch processor for handling multiple orders
pub struct BatchProcessor {
    /// Order processor
    processor: OrderProcessor,
    /// Maximum batch size
    max_batch_size: usize,
}

impl BatchProcessor {
    /// Creates a new batch processor
    pub fn new(config: MatchingEngineConfig, max_batch_size: usize) -> Self {
        Self {
            processor: OrderProcessor::new(config),
            max_batch_size,
        }
    }

    /// Processes a batch of commands
    pub fn process_batch(&mut self, commands: Vec<MatchingEngineCommand>) -> Vec<ExchangeResult<MatchingEngineResponse>> {
        if commands.len() > self.max_batch_size {
            return vec![Err(ExchangeError::system_error(
                format!("Batch size {} exceeds maximum {}", commands.len(), self.max_batch_size)
            ))];
        }

        commands
            .into_iter()
            .map(|cmd| self.processor.process_command(cmd))
            .collect()
    }

    /// Processes a batch of order submissions
    pub fn process_order_batch(&mut self, orders: Vec<Order>) -> Vec<ExchangeResult<MatchingEngineResponse>> {
        let commands: Vec<MatchingEngineCommand> = orders
            .into_iter()
            .map(|order| MatchingEngineCommand::SubmitOrder { order })
            .collect();

        self.process_batch(commands)
    }

    /// Validates a batch of orders before processing
    pub fn validate_batch(&self, orders: &[Order]) -> ExchangeResult<()> {
        if orders.len() > self.max_batch_size {
            return Err(ExchangeError::system_error(
                format!("Batch size {} exceeds maximum {}", orders.len(), self.max_batch_size)
            ));
        }

        // Validate each order
        for order in orders {
            self.processor.validate_order(order)?;
        }

        // Check for duplicate order IDs
        let mut order_ids = std::collections::HashSet::new();
        for order in orders {
            if !order_ids.insert(order.id) {
                return Err(ExchangeError::invalid_order(
                    format!("Duplicate order ID: {}", order.id)
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::{order::TimeInForce, price::Price};

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
    fn test_order_processor_creation() {
        let config = MatchingEngineConfig::default();
        let processor = OrderProcessor::new(config);
        
        assert_eq!(processor.config.tick_size, 100);
    }

    #[test]
    fn test_process_submit_order() {
        let config = MatchingEngineConfig::default();
        let mut processor = OrderProcessor::new(config);
        
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let command = MatchingEngineCommand::SubmitOrder { order };
        
        let response = processor.process_command(command).unwrap();
        
        match response {
            MatchingEngineResponse::OrderSubmitted { order_id, status, .. } => {
                assert_eq!(order_id, 1);
                assert_eq!(status, crypto_exchange_common::order::OrderStatus::Active);
            }
            _ => panic!("Expected order submitted response"),
        }
    }

    #[test]
    fn test_process_cancel_order() {
        let config = MatchingEngineConfig::default();
        let mut processor = OrderProcessor::new(config);
        
        let command = MatchingEngineCommand::CancelOrder { order_id: 1, user_id: 100 };
        let response = processor.process_command(command).unwrap();
        
        match response {
            MatchingEngineResponse::OrderCancelled { order_id, success, .. } => {
                assert_eq!(order_id, 1);
                assert!(success);
            }
            _ => panic!("Expected order cancelled response"),
        }
    }

    #[test]
    fn test_process_replace_order() {
        let config = MatchingEngineConfig::default();
        let mut processor = OrderProcessor::new(config);
        
        let new_order = create_test_order(2, OrderSide::Buy, OrderType::Limit, Some(50100), 1000);
        let command = MatchingEngineCommand::ReplaceOrder { old_order_id: 1, new_order };
        
        let response = processor.process_command(command).unwrap();
        
        match response {
            MatchingEngineResponse::OrderReplaced { old_order_id, new_order_id, success, .. } => {
                assert_eq!(old_order_id, 1);
                assert_eq!(new_order_id, 2);
                assert!(success);
            }
            _ => panic!("Expected order replaced response"),
        }
    }

    #[test]
    fn test_process_get_orderbook() {
        let config = MatchingEngineConfig::default();
        let mut processor = OrderProcessor::new(config);
        
        let command = MatchingEngineCommand::GetOrderBook { depth: 10 };
        let response = processor.process_command(command).unwrap();
        
        match response {
            MatchingEngineResponse::OrderBookSnapshot { pair, bids, asks, .. } => {
                assert_eq!(pair, "BTC/USDT");
                assert!(bids.is_empty());
                assert!(asks.is_empty());
            }
            _ => panic!("Expected order book snapshot response"),
        }
    }

    #[test]
    fn test_order_validation() {
        let config = MatchingEngineConfig::default();
        let processor = OrderProcessor::new(config);
        
        // Valid order
        let valid_order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        assert!(processor.validate_order(&valid_order).is_ok());

        // Invalid order (zero ID)
        let invalid_order = Order::new(
            0,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            TimeInForce::GTC,
            1234567890,
        );
        assert!(processor.validate_order(&invalid_order).is_err());

        // Invalid order (zero quantity)
        let zero_qty_order = create_test_order(2, OrderSide::Buy, OrderType::Limit, Some(50000), 0);
        assert!(processor.validate_order(&zero_qty_order).is_err());
    }

    #[test]
    fn test_batch_processor() {
        let config = MatchingEngineConfig::default();
        let mut batch_processor = BatchProcessor::new(config, 10);
        
        let orders = vec![
            create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000),
            create_test_order(2, OrderSide::Sell, OrderType::Limit, Some(50100), 500),
        ];
        
        // Validate batch
        assert!(batch_processor.validate_batch(&orders).is_ok());
        
        // Process batch
        let responses = batch_processor.process_order_batch(orders);
        assert_eq!(responses.len(), 2);
        
        // All responses should be successful
        for response in responses {
            assert!(response.is_ok());
        }
    }

    #[test]
    fn test_batch_processor_validation() {
        let config = MatchingEngineConfig::default();
        let batch_processor = BatchProcessor::new(config, 2);
        
        // Test duplicate order IDs
        let orders = vec![
            create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000),
            create_test_order(1, OrderSide::Sell, OrderType::Limit, Some(50100), 500), // Duplicate ID
        ];
        
        assert!(batch_processor.validate_batch(&orders).is_err());

        // Test batch size limit
        let large_batch: Vec<Order> = (0..5).map(|i| {
            create_test_order(i, OrderSide::Buy, OrderType::Limit, Some(50000), 1000)
        }).collect();
        
        assert!(batch_processor.validate_batch(&large_batch).is_err());
    }
}
