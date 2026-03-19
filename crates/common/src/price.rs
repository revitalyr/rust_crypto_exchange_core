//! Price handling utilities.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a price with fixed-point arithmetic
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Price {
    /// Price value in smallest unit (e.g., cents for USD)
    value: u64,
}

impl Price {
    /// Creates a new price from a raw value
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    /// Creates a price from a floating point value
    /// 
    /// # Arguments
    /// * `value` - The price as a floating point number
    /// * `decimals` - Number of decimal places to use
    pub fn from_float(value: f64, decimals: u8) -> Option<Self> {
        if value < 0.0 || !value.is_finite() {
            return None;
        }

        let multiplier = 10_f64.powi(decimals as i32);
        let scaled_value = (value * multiplier).round() as u64;
        Some(Self::new(scaled_value))
    }

    /// Returns the raw price value
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Converts the price to a floating point value
    pub fn to_float(&self, decimals: u8) -> f64 {
        let divisor = 10_f64.powi(decimals as i32);
        self.value as f64 / divisor
    }

    /// Adds two prices
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.value.checked_add(other.value).map(Self::new)
    }

    /// Subtracts two prices
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.value.checked_sub(other.value).map(Self::new)
    }

    /// Multiplies price by a scalar
    pub fn checked_mul(self, multiplier: u64) -> Option<Self> {
        self.value.checked_mul(multiplier).map(Self::new)
    }

    /// Divides price by a scalar
    pub fn checked_div(self, divisor: u64) -> Option<Self> {
        if divisor == 0 {
            return None;
        }
        Some(Self::new(self.value / divisor))
    }

    /// Returns the midpoint between two prices
    pub fn midpoint(self, other: Self) -> Option<Self> {
        self.checked_add(other)?.checked_div(2)
    }

    /// Rounds the price to the nearest tick
    pub fn round_to_tick(self, tick_size: u64) -> Self {
        let remainder = self.value % tick_size;
        if remainder >= tick_size / 2 {
            Self::new(self.value + (tick_size - remainder))
        } else {
            Self::new(self.value - remainder)
        }
    }

    /// Rounds down to the nearest tick
    pub fn round_down_to_tick(self, tick_size: u64) -> Self {
        Self::new(self.value - (self.value % tick_size))
    }

    /// Rounds up to the nearest tick
    pub fn round_up_to_tick(self, tick_size: u64) -> Self {
        let remainder = self.value % tick_size;
        if remainder == 0 {
            *self
        } else {
            Self::new(self.value + (tick_size - remainder))
        }
    }

    /// Returns the tick position (how many ticks from zero)
    pub fn tick_position(self, tick_size: u64) -> u64 {
        self.value / tick_size
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display with 4 decimal places by default
        write!(f, "{:.4}", self.to_float(4))
    }
}

/// Price level for order book
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Price,
    pub quantity: u64,
}

impl PriceLevel {
    /// Creates a new price level
    pub fn new(price: Price, quantity: u64) -> Self {
        Self { price, quantity }
    }

    /// Returns the total value (price * quantity)
    pub fn total_value(&self) -> Option<u64> {
        self.price.value.checked_mul(self.quantity)
    }

    /// Adds quantity to the price level
    pub fn add_quantity(&mut self, quantity: u64) -> bool {
        match self.quantity.checked_add(quantity) {
            Some(new_quantity) => {
                self.quantity = new_quantity;
                true
            }
            None => false,
        }
    }

    /// Removes quantity from the price level
    pub fn remove_quantity(&mut self, quantity: u64) -> bool {
        if quantity > self.quantity {
            return false;
        }
        self.quantity -= quantity;
        true
    }

    /// Checks if the price level is empty
    pub fn is_empty(&self) -> bool {
        self.quantity == 0
    }
}

impl fmt::Display for PriceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.quantity, self.price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_creation() {
        let price = Price::new(5000000); // $50.000 with 4 decimals
        assert_eq!(price.value(), 5000000);
        assert_eq!(price.to_float(4), 50000.0);
    }

    #[test]
    fn test_price_from_float() {
        let price = Price::from_float(50.1234, 4).unwrap();
        assert_eq!(price.value(), 501234);
        assert_eq!(price.to_float(4), 50.1234);
    }

    #[test]
    fn test_price_arithmetic() {
        let price1 = Price::new(100000);
        let price2 = Price::new(50000);

        assert_eq!(price1.checked_add(price2).unwrap().value(), 150000);
        assert_eq!(price1.checked_sub(price2).unwrap().value(), 50000);
        assert_eq!(price1.checked_mul(2).unwrap().value(), 200000);
        assert_eq!(price1.checked_div(2).unwrap().value(), 50000);
    }

    #[test]
    fn test_price_rounding() {
        let price = Price::new(501234); // $50.1234
        let tick_size = 100; // $0.01

        assert_eq!(price.round_to_tick(tick_size).value(), 501200); // $50.12
        assert_eq!(price.round_down_to_tick(tick_size).value(), 501200); // $50.12
        assert_eq!(price.round_up_to_tick(tick_size).value(), 501300); // $50.13
    }

    #[test]
    fn test_price_level() {
        let price = Price::new(50000);
        let mut level = PriceLevel::new(price, 1000);

        assert_eq!(level.quantity, 1000);
        assert!(level.add_quantity(500));
        assert_eq!(level.quantity, 1500);
        assert!(level.remove_quantity(200));
        assert_eq!(level.quantity, 1300);
        assert!(!level.is_empty());
    }
}
