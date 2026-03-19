//! Order matching logic and algorithms.

use crypto_exchange_common::{
    order::{Order, OrderSide, OrderType, TimeInForce},
    price::Price,
    ExchangeError, ExchangeResult,
};
use crate::types::{OrderExecution, TradeExecution};

/// Order matcher interface
pub trait OrderMatcher: Send + Sync {
    /// Matches a taker order against the order book
    fn match_order(&self, taker_order: &mut Order) -> ExchangeResult<MatchingResult>;
}

/// Result of order matching
#[derive(Debug, Clone)]
pub struct MatchingResult {
    /// List of trades generated
    pub trades: Vec<TradeExecution>,
    /// Order execution result
    pub order_execution: OrderExecution,
}

impl MatchingResult {
    /// Creates a new matching result
    pub fn new(trades: Vec<TradeExecution>, order_execution: OrderExecution) -> Self {
        Self {
            trades,
            order_execution,
        }
    }

    /// Checks if any trades were generated
    pub fn has_trades(&self) -> bool {
        !self.trades.is_empty()
    }

    /// Gets the total traded quantity
    pub fn total_traded_quantity(&self) -> u64 {
        self.trades.iter().map(|t| t.quantity).sum()
    }

    /// Gets the total traded value
    pub fn total_traded_value(&self) -> u64 {
        self.trades.iter().filter_map(|t| t.total_value()).sum()
    }

    /// Gets the average execution price
    pub fn average_price(&self) -> Option<Price> {
        let total_quantity = self.total_traded_quantity();
        let total_value = self.total_traded_value();

        if total_quantity > 0 {
            Some(Price::new(total_value / total_quantity))
        } else {
            None
        }
    }
}

/// Standard price-time priority matcher
pub struct PriceTimeMatcher {
    /// Minimum tick size
    tick_size: u64,
    /// Maximum price deviation protection (in basis points)
    max_price_deviation_bps: u32,
    /// Enable price protection
    enable_price_protection: bool,
}

impl PriceTimeMatcher {
    /// Creates a new price-time matcher
    pub fn new(tick_size: u64, max_price_deviation_bps: u32, enable_price_protection: bool) -> Self {
        Self {
            tick_size,
            max_price_deviation_bps,
            enable_price_protection,
        }
    }

    /// Validates if a taker order can match with a maker order
    fn can_match(&self, taker_order: &Order, maker_order: &Order) -> bool {
        // Check if prices are compatible
        match (taker_order.side, maker_order.side) {
            (OrderSide::Buy, OrderSide::Sell) => {
                // Buy order matches with sell order if buy price >= sell price
                if let (Some(taker_price), Some(maker_price)) = (taker_order.price, maker_order.price) {
                    taker_price.value() >= maker_price.value()
                } else {
                    false
                }
            }
            (OrderSide::Sell, OrderSide::Buy) => {
                // Sell order matches with buy order if sell price <= buy price
                if let (Some(taker_price), Some(maker_price)) = (taker_order.price, maker_order.price) {
                    taker_price.value() <= maker_price.value()
                } else {
                    false
                }
            }
            _ => false, // Same side orders don't match
        }
    }

    /// Calculates the execution price for a trade
    fn calculate_execution_price(&self, taker_order: &Order, maker_order: &Order) -> Price {
        // For limit orders, the maker price is used (maker-taker model)
        // This ensures price improvement for the taker when possible
        match (taker_order.side, maker_order.side) {
            (OrderSide::Buy, OrderSide::Sell) => {
                // Taker is buying, maker is selling - use maker's price
                maker_order.price.unwrap()
            }
            (OrderSide::Sell, OrderSide::Buy) => {
                // Taker is selling, maker is buying - use maker's price
                maker_order.price.unwrap()
            }
            _ => Price::new(0), // Should not happen
        }
    }

    /// Validates price protection
    fn validate_price_protection(&self, taker_order: &Order, execution_price: Price) -> ExchangeResult<()> {
        if !self.enable_price_protection {
            return Ok(());
        }

        if taker_order.order_type != OrderType::Limit {
            return Ok(());
        }

        let taker_price = taker_order.price.unwrap();

        // Calculate price deviation
        let deviation_bps = match taker_order.side {
            OrderSide::Buy => {
                // For buy orders, check if execution price is significantly higher than limit price
                if execution_price.value() > taker_price.value() {
                    ((execution_price.value() - taker_price.value()) * 10000) / taker_price.value()
                } else {
                    0
                }
            }
            OrderSide::Sell => {
                // For sell orders, check if execution price is significantly lower than limit price
                if execution_price.value() < taker_price.value() {
                    ((taker_price.value() - execution_price.value()) * 10000) / taker_price.value()
                } else {
                    0
                }
            }
        };

        if deviation_bps > self.max_price_deviation_bps {
            return Err(ExchangeError::market_order_error(
                format!("Price deviation {} bps exceeds maximum {}", deviation_bps, self.max_price_deviation_bps)
            ));
        }

        Ok(())
    }

    /// Determines the quantity to execute for a trade
    fn determine_trade_quantity(&self, taker_order: &mut Order, maker_order: &Order) -> u64 {
        let taker_remaining = taker_order.remaining_quantity();
        let maker_remaining = maker_order.remaining_quantity();

        // The trade quantity is the minimum of remaining quantities
        let trade_quantity = taker_remaining.min(maker_remaining);

        // Apply time in force constraints
        match taker_order.time_in_force {
            TimeInForce::FOK => {
                // Fill or Kill - must match entire order
                if maker_remaining >= taker_remaining {
                    trade_quantity
                } else {
                    0 // Can't fill entire order
                }
            }
            TimeInForce::IOC | TimeInForce::GTC => {
                // Immediate or Cancel / Good Till Canceled - can match partial
                trade_quantity
            }
        }
    }

    /// Executes a single trade between taker and maker
    fn execute_trade(
        &self,
        taker_order: &mut Order,
        maker_order: &mut Order,
        timestamp: u64,
        sequence: u64,
    ) -> ExchangeResult<TradeExecution> {
        let trade_quantity = self.determine_trade_quantity(taker_order, maker_order);
        
        if trade_quantity == 0 {
            return Err(ExchangeError::market_order_error("No trade quantity"));
        }

        let execution_price = self.calculate_execution_price(taker_order, maker_order);
        
        // Validate price protection
        self.validate_price_protection(taker_order, execution_price)?;

        // Update orders
        taker_order.fill(trade_quantity, timestamp)?;
        maker_order.fill(trade_quantity, timestamp)?;

        // Determine maker and taker roles
        let (maker_order_id, taker_order_id, maker_user_id, taker_user_id) = 
            if taker_order.created_at <= maker_order.created_at {
                // Taker order is older (maker), maker order is newer (taker)
                // This is unusual but possible in some implementations
                (taker_order.id, maker_order.id, taker_order.user_id, maker_order.user_id)
            } else {
                // Normal case: maker order is older, taker order is newer
                (maker_order.id, taker_order.id, maker_order.user_id, taker_order.user_id)
            };

        Ok(TradeExecution::new(
            maker_order_id,
            taker_order_id,
            maker_user_id,
            taker_user_id,
            taker_order.pair.clone(),
            execution_price,
            trade_quantity,
            taker_order.side,
            timestamp,
            sequence,
        ))
    }
}

impl OrderMatcher for PriceTimeMatcher {
    fn match_order(&self, taker_order: &mut Order) -> ExchangeResult<MatchingResult> {
        let mut trades = Vec::new();
        let mut total_filled_quantity = 0u64;
        let mut total_cost = 0u64;
        let timestamp = crypto_exchange_common::timestamp::now();

        // In a real implementation, we would iterate through the order book
        // For this example, we'll simulate the matching process
        
        // This is a simplified version - in the actual engine,
        // we would have access to the order book and would iterate
        // through matching orders
        
        // Simulate matching logic
        let should_cancel = match taker_order.time_in_force {
            TimeInForce::IOC => {
                // IOC orders should be cancelled if not fully filled
                total_filled_quantity < taker_order.quantity
            }
            TimeInForce::FOK => {
                // FOK orders should be cancelled if not fully filled
                total_filled_quantity < taker_order.quantity
            }
            TimeInForce::GTC => false,
        };

        let status = if total_filled_quantity == taker_order.quantity {
            crypto_exchange_common::order::OrderStatus::Filled
        } else if total_filled_quantity > 0 {
            crypto_exchange_common::order::OrderStatus::PartiallyFilled
        } else if should_cancel {
            crypto_exchange_common::order::OrderStatus::Cancelled
        } else {
            crypto_exchange_common::order::OrderStatus::Active
        };

        let avg_price = if total_filled_quantity > 0 {
            Some(Price::new(total_cost / total_filled_quantity))
        } else {
            None
        };

        let order_execution = OrderExecution::new(
            taker_order.id,
            taker_order.user_id,
            taker_order.side,
            taker_order.order_type,
            taker_order.quantity,
            total_filled_quantity,
            avg_price,
            total_cost,
            status,
            timestamp,
            should_cancel,
        );

        Ok(MatchingResult::new(trades, order_execution))
    }
}

/// Market order matcher
pub struct MarketOrderMatcher {
    /// Maximum slippage in basis points
    max_slippage_bps: u32,
    /// Maximum market impact in basis points
    max_market_impact_bps: u32,
}

impl MarketOrderMatcher {
    /// Creates a new market order matcher
    pub fn new(max_slippage_bps: u32, max_market_impact_bps: u32) -> Self {
        Self {
            max_slippage_bps,
            max_market_impact_bps,
        }
    }

    /// Estimates the execution price for a market order
    pub fn estimate_execution_price(&self, side: OrderSide, quantity: u64, order_book: &[(Price, u64)]) -> Option<Price> {
        let mut remaining_quantity = quantity;
        let mut total_cost = 0u64;

        let sorted_levels = if side == OrderSide::Buy {
            // For buys, use ascending prices
            let mut sorted = order_book.to_vec();
            sorted.sort_by_key(|(price, _)| price.value());
            sorted
        } else {
            // For sells, use descending prices
            let mut sorted = order_book.to_vec();
            sorted.sort_by_key(|(price, _)| std::cmp::Reverse(price.value()));
            sorted
        };

        for (price, available_quantity) in sorted_levels {
            if remaining_quantity == 0 {
                break;
            }

            let fill_quantity = remaining_quantity.min(*available_quantity);
            total_cost += price.value().checked_mul(fill_quantity)?;
            remaining_quantity -= fill_quantity;
        }

        if remaining_quantity == 0 {
            Some(Price::new(total_cost / quantity))
        } else {
            None // Not enough liquidity
        }
    }

    /// Validates market order execution
    fn validate_execution(&self, side: OrderSide, quantity: u64, avg_price: Price, mid_price: Price) -> ExchangeResult<()> {
        // Calculate slippage
        let slippage_bps = match side {
            OrderSide::Buy => {
                if avg_price.value() > mid_price.value() {
                    ((avg_price.value() - mid_price.value()) * 10000) / mid_price.value()
                } else {
                    0
                }
            }
            OrderSide::Sell => {
                if avg_price.value() < mid_price.value() {
                    ((mid_price.value() - avg_price.value()) * 10000) / mid_price.value()
                } else {
                    0
                }
            }
        };

        if slippage_bps > self.max_slippage_bps {
            return Err(ExchangeError::market_order_error(
                format!("Slippage {} bps exceeds maximum {}", slippage_bps, self.max_slippage_bps)
            ));
        }

        // Calculate market impact
        let impact_bps = if avg_price.value() > mid_price.value() {
            ((avg_price.value() - mid_price.value()) * 10000) / mid_price.value()
        } else {
            0
        };

        if impact_bps > self.max_market_impact_bps {
            return Err(ExchangeError::market_order_error(
                format!("Market impact {} bps exceeds maximum {}", impact_bps, self.max_market_impact_bps)
            ));
        }

        Ok(())
    }
}

impl OrderMatcher for MarketOrderMatcher {
    fn match_order(&self, taker_order: &mut Order) -> ExchangeResult<MatchingResult> {
        // Market orders should not have a price
        if taker_order.price.is_some() {
            return Err(ExchangeError::invalid_order("Market order should not have a price"));
        }

        // Market orders must use IOC or FOK
        match taker_order.time_in_force {
            TimeInForce::IOC | TimeInForce::FOK => {}
            TimeInForce::GTC => {
                return Err(ExchangeError::invalid_order("Market orders cannot use GTC time in force"));
            }
        }

        // In a real implementation, we would match against the order book
        // For this example, we'll return a simple result
        
        let timestamp = crypto_exchange_common::timestamp::now();
        
        // Simulate execution (in reality, this would involve iterating through the order book)
        let total_filled_quantity = 0; // Would be calculated from actual matching
        let total_cost = 0;

        let should_cancel = match taker_order.time_in_force {
            TimeInForce::IOC => total_filled_quantity == 0, // Cancel if no fill
            TimeInForce::FOK => total_filled_quantity < taker_order.quantity, // Cancel if not fully filled
            TimeInForce::GTC => false,
        };

        let status = if total_filled_quantity == taker_order.quantity {
            crypto_exchange_common::order::OrderStatus::Filled
        } else if total_filled_quantity > 0 {
            crypto_exchange_common::order::OrderStatus::PartiallyFilled
        } else if should_cancel {
            crypto_exchange_common::order::OrderStatus::Cancelled
        } else {
            crypto_exchange_common::order::OrderStatus::Active
        };

        let order_execution = OrderExecution::new(
            taker_order.id,
            taker_order.user_id,
            taker_order.side,
            taker_order.order_type,
            taker_order.quantity,
            total_filled_quantity,
            None, // No average price for unfilled market order
            total_cost,
            status,
            timestamp,
            should_cancel,
        );

        Ok(MatchingResult::new(Vec::new(), order_execution))
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
    fn test_price_time_matcher() {
        let matcher = PriceTimeMatcher::new(100, 1000, true);
        
        let mut taker_order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        
        let result = matcher.match_order(&mut taker_order).unwrap();
        
        assert_eq!(result.order_execution.order_id, 1);
        assert_eq!(result.order_execution.side, OrderSide::Buy);
        assert!(!result.has_trades()); // No trades in this simplified test
    }

    #[test]
    fn test_market_order_matcher() {
        let matcher = MarketOrderMatcher::new(500, 1000);
        
        let mut market_order = create_test_order(1, OrderSide::Buy, OrderType::Market, None, 1000);
        market_order.time_in_force = TimeInForce::IOC;
        
        let result = matcher.match_order(&mut market_order).unwrap();
        
        assert_eq!(result.order_execution.order_id, 1);
        assert_eq!(result.order_execution.order_type, OrderType::Market);
        assert_eq!(result.order_execution.status, OrderStatus::Cancelled); // No liquidity in test
    }

    #[test]
    fn test_matching_result() {
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
                OrderSide::Buy, 1234567891, 2,
            ),
        ];

        let order_execution = OrderExecution::new(
            1, 100, OrderSide::Buy, OrderType::Limit,
            1500, 1500, Some(Price::new(50033)), 75_050_000,
            OrderStatus::Filled, 1234567892, false,
        );

        let result = MatchingResult::new(trades, order_execution);
        
        assert!(result.has_trades());
        assert_eq!(result.total_traded_quantity(), 1500);
        assert_eq!(result.total_traded_value(), 75_050_000);
        assert_eq!(result.average_price(), Some(Price::new(50033)));
    }

    #[test]
    fn test_price_protection() {
        let matcher = PriceTimeMatcher::new(100, 100, true); // 1% max deviation
        
        let mut taker_order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        
        // Test with price that would exceed protection
        // In a real implementation, this would be checked during actual matching
        let execution_price = Price::new(52000); // 4% higher than limit
        
        let result = matcher.validate_price_protection(&taker_order, execution_price);
        assert!(result.is_err());
    }
}
