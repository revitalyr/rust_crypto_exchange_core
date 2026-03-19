//! Common type definitions and utilities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Timestamp type (nanoseconds since epoch)
pub type Timestamp = u64;

/// User ID type
pub type UserId = u64;

/// Order ID type
pub type OrderId = u64;

/// Balance type (using u128 for large amounts)
pub type Balance = u128;

/// Quantity type
pub type Quantity = u64;

/// Sequence number type
pub type Sequence = u64;

/// Utility functions for timestamps
pub mod timestamp {
    use super::*;

    /// Gets the current timestamp in nanoseconds
    pub fn now() -> Timestamp {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as Timestamp
    }

    /// Converts timestamp to DateTime<Utc>
    pub fn to_datetime(timestamp: Timestamp) -> DateTime<Utc> {
        DateTime::from_timestamp_nanos(timestamp as i64)
    }

    /// Converts DateTime<Utc> to timestamp
    pub fn from_datetime(datetime: DateTime<Utc>) -> Timestamp {
        datetime.timestamp_nanos_opt().unwrap_or(0) as Timestamp
    }

    /// Adds duration to timestamp
    pub fn add(timestamp: Timestamp, duration: Duration) -> Timestamp {
        timestamp + duration.as_nanos() as Timestamp
    }

    /// Subtracts duration from timestamp
    pub fn sub(timestamp: Timestamp, duration: Duration) -> Timestamp {
        timestamp.saturating_sub(duration.as_nanos() as Timestamp)
    }
}

/// Utility functions for balances
pub mod balance {
    use super::*;

    /// Formats balance for display
    pub fn format(balance: Balance, decimals: u8) -> String {
        let divisor = 10_f64.powi(decimals as i32);
        format!("{:.precision$}", balance as f64 / divisor, precision = decimals as usize)
    }

    /// Parses balance from string
    pub fn parse(s: &str, decimals: u8) -> Option<Balance> {
        let value: f64 = s.parse().ok()?;
        if value < 0.0 || !value.is_finite() {
            return None;
        }

        let multiplier = 10_f64.powi(decimals as i32);
        Some((value * multiplier) as Balance)
    }

    /// Adds two balances
    pub fn checked_add(a: Balance, b: Balance) -> Option<Balance> {
        a.checked_add(b)
    }

    /// Subtracts two balances
    pub fn checked_sub(a: Balance, b: Balance) -> Option<Balance> {
        a.checked_sub(b)
    }

    /// Multiplies balance by a scalar
    pub fn checked_mul(balance: Balance, multiplier: u64) -> Option<Balance> {
        balance.checked_mul(multiplier as Balance)
    }

    /// Divides balance by a scalar
    pub fn checked_div(balance: Balance, divisor: u64) -> Option<Balance> {
        if divisor == 0 {
            return None;
        }
        Some(balance / divisor as Balance)
    }
}

/// Utility functions for quantities
pub mod quantity {
    use super::*;

    /// Validates quantity against lot size
    pub fn validate(quantity: Quantity, lot_size: Quantity) -> bool {
        quantity > 0 && quantity % lot_size == 0
    }

    /// Rounds quantity to lot size
    pub fn round_to_lot(quantity: Quantity, lot_size: Quantity) -> Quantity {
        if lot_size == 0 {
            return quantity;
        }
        (quantity / lot_size) * lot_size
    }

    /// Rounds down quantity to lot size
    pub fn round_down_to_lot(quantity: Quantity, lot_size: Quantity) -> Quantity {
        if lot_size == 0 {
            return quantity;
        }
        (quantity / lot_size) * lot_size
    }

    /// Rounds up quantity to lot size
    pub fn round_up_to_lot(quantity: Quantity, lot_size: Quantity) -> Quantity {
        if lot_size == 0 {
            return quantity;
        }
        let remainder = quantity % lot_size;
        if remainder == 0 {
            quantity
        } else {
            quantity + (lot_size - remainder)
        }
    }
}

/// Configuration for precision handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecisionConfig {
    /// Number of decimal places for price
    pub price_decimals: u8,
    /// Number of decimal places for quantity
    pub quantity_decimals: u8,
    /// Minimum tick size
    pub tick_size: Quantity,
    /// Minimum lot size
    pub lot_size: Quantity,
}

impl PrecisionConfig {
    /// Creates a new precision config
    pub fn new(
        price_decimals: u8,
        quantity_decimals: u8,
        tick_size: Quantity,
        lot_size: Quantity,
    ) -> Self {
        Self {
            price_decimals,
            quantity_decimals,
            tick_size,
            lot_size,
        }
    }

    /// Returns a standard BTC/USDT config
    pub fn btc_usdt() -> Self {
        Self::new(2, 8, 100, 1) // $0.01, 1 satoshi
    }

    /// Returns a standard ETH/USDT config
    pub fn eth_usdt() -> Self {
        Self::new(2, 18, 1000, 1) // $0.01, 1 wei
    }
}

impl Default for PrecisionConfig {
    fn default() -> Self {
        Self::btc_usdt()
    }
}

/// Rate limiter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_seconds: u32,
    /// Burst capacity
    pub burst_capacity: Option<u32>,
}

impl RateLimitConfig {
    /// Creates a new rate limit config
    pub fn new(max_requests: u32, window_seconds: u32) -> Self {
        Self {
            max_requests,
            window_seconds,
            burst_capacity: Some(max_requests),
        }
    }

    /// Creates a restrictive config for sensitive operations
    pub fn restrictive() -> Self {
        Self::new(10, 60) // 10 requests per minute
    }

    /// Creates a permissive config for general operations
    pub fn permissive() -> Self {
        Self::new(1000, 60) // 1000 requests per minute
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::permissive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_utils() {
        let now = timestamp::now();
        let datetime = timestamp::to_datetime(now);
        let back = timestamp::from_datetime(datetime);
        
        // Should be close (allowing for minor differences)
        assert!(back >= now.saturating_sub(1_000_000)); // 1ms tolerance
    }

    #[test]
    fn test_balance_utils() {
        let balance = 1_234_567_890; // $12.34567890 with 8 decimals
        let formatted = balance::format(balance, 8);
        assert_eq!(formatted, "12.34567890");

        let parsed = balance::parse("12.34567890", 8).unwrap();
        assert_eq!(parsed, balance);
    }

    #[test]
    fn test_quantity_utils() {
        assert!(quantity::validate(1000, 100));
        assert!(!quantity::validate(1050, 100));

        assert_eq!(quantity::round_to_lot(1050, 100), 1000);
        assert_eq!(quantity::round_up_to_lot(1050, 100), 1100);
    }

    #[test]
    fn test_precision_config() {
        let config = PrecisionConfig::btc_usdt();
        assert_eq!(config.price_decimals, 2);
        assert_eq!(config.quantity_decimals, 8);
        assert_eq!(config.tick_size, 100);
        assert_eq!(config.lot_size, 1);
    }

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::restrictive();
        assert_eq!(config.max_requests, 10);
        assert_eq!(config.window_seconds, 60);
    }
}
