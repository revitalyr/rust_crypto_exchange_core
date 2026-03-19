//! Order types and related structures.

use crate::{error::ExchangeError, price::Price};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderSide {
    /// Buy order
    Buy,
    /// Sell order
    Sell,
}

impl OrderSide {
    /// Returns the opposite side
    pub fn opposite(self) -> Self {
        match self {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        }
    }
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "buy"),
            OrderSide::Sell => write!(f, "sell"),
        }
    }
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    /// Limit order with specific price
    Limit,
    /// Market order (executed immediately at best price)
    Market,
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderType::Limit => write!(f, "limit"),
            OrderType::Market => write!(f, "market"),
        }
    }
}

/// Order time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good until cancelled
    GTC,
    /// Immediate or cancel
    IOC,
    /// Fill or kill
    FOK,
}

impl Default for TimeInForce {
    fn default() -> Self {
        Self::GTC
    }
}

impl fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeInForce::GTC => write!(f, "GTC"),
            TimeInForce::IOC => write!(f, "IOC"),
            TimeInForce::FOK => write!(f, "FOK"),
        }
    }
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is pending submission
    Pending,
    /// Order is active in the order book
    Active,
    /// Order is partially filled
    PartiallyFilled,
    /// Order is completely filled
    Filled,
    /// Order is cancelled
    Cancelled,
    /// Order is rejected
    Rejected,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderStatus::Pending => write!(f, "pending"),
            OrderStatus::Active => write!(f, "active"),
            OrderStatus::PartiallyFilled => write!(f, "partially_filled"),
            OrderStatus::Filled => write!(f, "filled"),
            OrderStatus::Cancelled => write!(f, "cancelled"),
            OrderStatus::Rejected => write!(f, "rejected"),
        }
    }
}

/// Represents a trading order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Unique order identifier
    pub id: u64,
    /// User ID who placed the order
    pub user_id: u64,
    /// Trading pair symbol
    pub pair: String,
    /// Order side (buy/sell)
    pub side: OrderSide,
    /// Order type (limit/market)
    pub order_type: OrderType,
    /// Order price (None for market orders)
    pub price: Option<Price>,
    /// Order quantity
    pub quantity: u64,
    /// Filled quantity
    pub filled_quantity: u64,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Order status
    pub status: OrderStatus,
    /// Creation timestamp (nanoseconds since epoch)
    pub created_at: u64,
    /// Last update timestamp (nanoseconds since epoch)
    pub updated_at: u64,
}

impl Order {
    /// Creates a new order
    pub fn new(
        id: u64,
        user_id: u64,
        pair: String,
        side: OrderSide,
        order_type: OrderType,
        price: Option<Price>,
        quantity: u64,
        time_in_force: TimeInForce,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            user_id,
            pair,
            side,
            order_type,
            price,
            quantity,
            filled_quantity: 0,
            time_in_force,
            status: OrderStatus::Pending,
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    /// Returns the remaining quantity
    pub fn remaining_quantity(&self) -> u64 {
        self.quantity - self.filled_quantity
    }

    /// Returns the fill percentage (0.0 to 1.0)
    pub fn fill_percentage(&self) -> f64 {
        if self.quantity == 0 {
            0.0
        } else {
            self.filled_quantity as f64 / self.quantity as f64
        }
    }

    /// Checks if the order is completely filled
    pub fn is_filled(&self) -> bool {
        self.filled_quantity >= self.quantity
    }

    /// Checks if the order can be partially filled
    pub fn can_partial_fill(&self) -> bool {
        matches!(self.time_in_force, TimeInForce::GTC | TimeInForce::IOC)
    }

    /// Updates the filled quantity
    pub fn fill(&mut self, quantity: u64, timestamp: u64) -> Result<(), ExchangeError> {
        if quantity > self.remaining_quantity() {
            return Err(ExchangeError::invalid_order(
                "Fill quantity exceeds remaining quantity",
            ));
        }

        self.filled_quantity += quantity;
        self.updated_at = timestamp;

        if self.is_filled() {
            self.status = OrderStatus::Filled;
        } else {
            self.status = OrderStatus::PartiallyFilled;
        }

        Ok(())
    }

    /// Cancels the order
    pub fn cancel(&mut self, reason: &str, timestamp: u64) -> Result<(), ExchangeError> {
        if matches!(self.status, OrderStatus::Filled | OrderStatus::Cancelled) {
            return Err(ExchangeError::invalid_order(
                "Cannot cancel filled or already cancelled order",
            ));
        }

        self.status = OrderStatus::Cancelled;
        self.updated_at = timestamp;
        Ok(())
    }

    /// Validates the order
    pub fn validate(&self) -> Result<(), ExchangeError> {
        if self.quantity == 0 {
            return Err(ExchangeError::invalid_quantity(0));
        }

        if self.order_type == OrderType::Limit && self.price.is_none() {
            return Err(ExchangeError::invalid_order(
                "Limit order must have a price",
            ));
        }

        if self.order_type == OrderType::Market && self.price.is_some() {
            return Err(ExchangeError::invalid_order(
                "Market order cannot have a price",
            ));
        }

        Ok(())
    }
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Order[id={}, user={}, pair={}, side={}, type={}, price={:?}, qty={}, filled={}, status={}]",
            self.id,
            self.user_id,
            self.pair,
            self.side,
            self.order_type,
            self.price,
            self.quantity,
            self.filled_quantity,
            self.status
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_creation() {
        let order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            TimeInForce::GTC,
            1234567890,
        );

        assert_eq!(order.id, 1);
        assert_eq!(order.user_id, 100);
        assert_eq!(order.remaining_quantity(), 1000);
        assert_eq!(order.fill_percentage(), 0.0);
    }

    #[test]
    fn test_order_fill() {
        let mut order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            TimeInForce::GTC,
            1234567890,
        );

        order.fill(500, 1234567891).unwrap();
        assert_eq!(order.filled_quantity, 500);
        assert_eq!(order.remaining_quantity(), 500);
        assert_eq!(order.fill_percentage(), 0.5);
        assert_eq!(order.status, OrderStatus::PartiallyFilled);

        order.fill(500, 1234567892).unwrap();
        assert_eq!(order.status, OrderStatus::Filled);
    }

    #[test]
    fn test_order_validation() {
        let valid_order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            TimeInForce::GTC,
            1234567890,
        );

        assert!(valid_order.validate().is_ok());

        let invalid_order = Order::new(
            2,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            None, // No price for limit order
            1000,
            TimeInForce::GTC,
            1234567890,
        );

        assert!(invalid_order.validate().is_err());
    }
}
