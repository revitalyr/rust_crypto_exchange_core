//! Trade structures and related types.

use crate::{price::Price, Asset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a executed trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Unique trade identifier
    pub id: Uuid,
    /// Maker order ID
    pub maker_order_id: u64,
    /// Taker order ID
    pub taker_order_id: u64,
    /// Maker user ID
    pub maker_user_id: u64,
    /// Taker user ID
    pub taker_user_id: u64,
    /// Trading pair symbol
    pub pair: String,
    /// Trade price
    pub price: Price,
    /// Trade quantity
    pub quantity: u64,
    /// Trade side (from taker's perspective)
    pub side: crate::order::OrderSide,
    /// Trade timestamp (nanoseconds since epoch)
    pub timestamp: u64,
    /// Trade sequence number for ordering
    pub sequence: u64,
}

impl Trade {
    /// Creates a new trade
    pub fn new(
        maker_order_id: u64,
        taker_order_id: u64,
        maker_user_id: u64,
        taker_user_id: u64,
        pair: String,
        price: Price,
        quantity: u64,
        side: crate::order::OrderSide,
        timestamp: u64,
        sequence: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            maker_order_id,
            taker_order_id,
            maker_user_id,
            taker_user_id,
            pair,
            price,
            quantity,
            side,
            timestamp,
            sequence,
        }
    }

    /// Returns the total trade value (price * quantity)
    pub fn total_value(&self) -> Option<u64> {
        self.price.value().checked_mul(self.quantity)
    }

    /// Returns the maker fee (assuming maker fee rate)
    pub fn maker_fee(&self, maker_fee_rate: f64) -> Option<u64> {
        let total = self.total_value()?;
        Some((total as f64 * maker_fee_rate) as u64)
    }

    /// Returns the taker fee (assuming taker fee rate)
    pub fn taker_fee(&self, taker_fee_rate: f64) -> Option<u64> {
        let total = self.total_value()?;
        Some((total as f64 * taker_fee_rate) as u64)
    }

    /// Returns the net amount for the maker (after fees)
    pub fn maker_net_amount(&self, maker_fee_rate: f64) -> Option<u64> {
        let total = self.total_value()?;
        let fee = self.maker_fee(maker_fee_rate)?;
        total.checked_sub(fee)
    }

    /// Returns the net amount for the taker (after fees)
    pub fn taker_net_amount(&self, taker_fee_rate: f64) -> Option<u64> {
        let total = self.total_value()?;
        let fee = self.taker_fee(taker_fee_rate)?;
        total.checked_sub(fee)
    }
}

/// Trade execution report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    /// Trade ID
    pub trade_id: Uuid,
    /// Order ID
    pub order_id: u64,
    /// Trade price
    pub price: Price,
    /// Trade quantity
    pub quantity: u64,
    /// Trade side
    pub side: crate::order::OrderSide,
    /// Role in the trade (maker/taker)
    pub role: TradeRole,
    /// Fee amount
    pub fee: u64,
    /// Net amount (after fees)
    pub net_amount: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl TradeExecution {
    /// Creates a new trade execution report
    pub fn new(
        trade_id: Uuid,
        order_id: u64,
        price: Price,
        quantity: u64,
        side: crate::order::OrderSide,
        role: TradeRole,
        fee: u64,
        net_amount: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            trade_id,
            order_id,
            price,
            quantity,
            side,
            role,
            fee,
            net_amount,
            timestamp,
        }
    }
}

/// Role in a trade
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TradeRole {
    /// Maker order (provides liquidity)
    Maker,
    /// Taker order (takes liquidity)
    Taker,
}

impl TradeRole {
    /// Returns the opposite role
    pub fn opposite(self) -> Self {
        match self {
            TradeRole::Maker => TradeRole::Taker,
            TradeRole::Taker => TradeRole::Maker,
        }
    }
}

/// Fee model for calculating trade fees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeModel {
    /// Maker fee rate (as decimal, e.g., 0.001 for 0.1%)
    pub maker_fee_rate: f64,
    /// Taker fee rate (as decimal, e.g., 0.002 for 0.2%)
    pub taker_fee_rate: f64,
}

impl FeeModel {
    /// Creates a new fee model
    pub fn new(maker_fee_rate: f64, taker_fee_rate: f64) -> Self {
        Self {
            maker_fee_rate,
            taker_fee_rate,
        }
    }

    /// Calculates the fee for a trade
    pub fn calculate_fee(&self, total_value: u64, role: TradeRole) -> u64 {
        let rate = match role {
            TradeRole::Maker => self.maker_fee_rate,
            TradeRole::Taker => self.taker_fee_rate,
        };
        (total_value as f64 * rate) as u64
    }

    /// Returns a zero-fee model
    pub fn zero_fee() -> Self {
        Self::new(0.0, 0.0)
    }

    /// Returns a standard fee model (0.1% maker, 0.2% taker)
    pub fn standard() -> Self {
        Self::new(0.001, 0.002)
    }
}

impl Default for FeeModel {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::OrderSide;

    #[test]
    fn test_trade_creation() {
        let trade = Trade::new(
            1,
            2,
            100,
            200,
            "BTC/USDT".to_string(),
            Price::new(50000),
            1000,
            OrderSide::Buy,
            1234567890,
            1,
        );

        assert_eq!(trade.maker_order_id, 1);
        assert_eq!(trade.taker_order_id, 2);
        assert_eq!(trade.quantity, 1000);
        assert_eq!(trade.price.value(), 50000);
    }

    #[test]
    fn test_trade_value() {
        let trade = Trade::new(
            1,
            2,
            100,
            200,
            "BTC/USDT".to_string(),
            Price::new(50000),
            1000,
            OrderSide::Buy,
            1234567890,
            1,
        );

        assert_eq!(trade.total_value(), Some(50_000_000)); // 50000 * 1000
    }

    #[test]
    fn test_fee_model() {
        let fee_model = FeeModel::standard();
        let total_value = 1_000_000; // $10.00 with 4 decimals

        let maker_fee = fee_model.calculate_fee(total_value, TradeRole::Maker);
        let taker_fee = fee_model.calculate_fee(total_value, TradeRole::Taker);

        assert_eq!(maker_fee, 1000); // 0.1% of $10.00
        assert_eq!(taker_fee, 2000); // 0.2% of $10.00
    }

    #[test]
    fn test_trade_execution() {
        let execution = TradeExecution::new(
            Uuid::new_v4(),
            1,
            Price::new(50000),
            1000,
            OrderSide::Buy,
            TradeRole::Taker,
            1000,
            49_000_000,
            1234567890,
        );

        assert_eq!(execution.order_id, 1);
        assert_eq!(execution.role, TradeRole::Taker);
        assert_eq!(execution.fee, 1000);
    }
}
