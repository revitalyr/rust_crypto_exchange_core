//! Trade structures and related types.

use crate::price::Price;
use crate::types::{OrderId, UserId, Quantity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a executed trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Unique trade identifier
    pub id: Uuid,
    /// Maker order ID
    pub maker_order_id: OrderId,
    /// Taker order ID
    pub taker_order_id: OrderId,
    /// Maker user ID
    pub maker_user_id: UserId,
    /// Taker user ID
    pub taker_user_id: UserId,
    /// Trading pair symbol
    pub pair: String,
    /// Trade price
    pub price: Price,
    /// Trade quantity
    pub quantity: Quantity,
    /// Trade side (from taker's perspective)
    pub side: crate::order::OrderSide,
    /// Trade timestamp (nanoseconds since epoch)
    pub timestamp: crate::types::Timestamp,
    /// Trade sequence number for ordering
    pub sequence: crate::types::Sequence,
}

impl Trade {
    /// Creates a new trade
    pub fn new(
        maker_order_id: OrderId,
        taker_order_id: OrderId,
        maker_user_id: UserId,
        taker_user_id: UserId,
        pair: String,
        price: Price,
        quantity: Quantity,
        side: crate::order::OrderSide,
        timestamp: crate::types::Timestamp,
        sequence: crate::types::Sequence,
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

    /// Returns the total value of the trade
    pub fn total_value(&self) -> Option<Quantity> {
        self.price.value().checked_mul(self.quantity)
    }

    /// Returns the maker fee amount
    pub fn maker_fee(&self, fee_rate: crate::types::FeeRate) -> Option<Quantity> {
        self.total_value().map(|value| (value as f64 * fee_rate) as Quantity)
    }

    /// Returns the taker fee amount
    pub fn taker_fee(&self, fee_rate: crate::types::FeeRate) -> Option<Quantity> {
        self.total_value().map(|value| (value as f64 * fee_rate) as Quantity)
    }

    /// Returns the net amount for the maker
    pub fn maker_net_amount(&self, fee_rate: crate::types::FeeRate) -> Option<Quantity> {
        self.total_value()?.checked_sub(self.maker_fee(fee_rate)?)
    }

    /// Returns the net amount for the taker
    pub fn taker_net_amount(&self, fee_rate: crate::types::FeeRate) -> Option<Quantity> {
        self.total_value()?.checked_sub(self.taker_fee(fee_rate)?)
    }

    /// Returns the trade as a string representation
    pub fn as_string(&self) -> String {
        format!(
            "Trade {}: {} {} @ {} ({})",
            self.id,
            self.quantity,
            self.pair.split('/').next().unwrap_or("UNKNOWN"),
            self.price,
            match self.side {
                crate::order::OrderSide::Buy => "BUY",
                crate::order::OrderSide::Sell => "SELL",
            }
        )
    }
}

/// Represents a collection of trades
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeHistory {
    /// List of trades
    pub trades: Vec<Trade>,
    /// Total number of trades
    pub total_count: crate::types::Count,
    /// Whether there are more trades available
    pub has_more: bool,
}

impl TradeHistory {
    /// Creates a new trade history
    pub fn new(trades: Vec<Trade>, total_count: crate::types::Count, has_more: bool) -> Self {
        Self {
            trades,
            total_count,
            has_more,
        }
    }

    /// Returns the total traded quantity
    pub fn total_traded_quantity(&self) -> Quantity {
        self.trades.iter().map(|trade| trade.quantity).sum()
    }

    /// Returns the total traded value
    pub fn total_traded_value(&self) -> Option<Quantity> {
        self.trades.iter().try_fold(0u64, |acc, trade| {
            acc.checked_add(trade.total_value()?)
        })
    }

    /// Returns trades for a specific user
    pub fn for_user(&self, user_id: UserId) -> Vec<&Trade> {
        self.trades.iter()
            .filter(|trade| trade.maker_user_id == user_id || trade.taker_user_id == user_id)
            .collect()
    }

    /// Returns trades for a specific trading pair
    pub fn for_pair(&self, pair: &str) -> Vec<&Trade> {
        self.trades.iter()
            .filter(|trade| trade.pair == pair)
            .collect()
    }

    /// Returns the latest trade
    pub fn latest(&self) -> Option<&Trade> {
        self.trades.last()
    }

    /// Returns the earliest trade
    pub fn earliest(&self) -> Option<&Trade> {
        self.trades.first()
    }
}

#[cfg(test)]
mod trade_tests {
    use super::*;
    use crate::price::Price;
    use crate::order::OrderSide;

    #[test]
    fn test_trade_creation() {
        let trade = Trade::new(
            1, 2, 100, 200,
            crate::types::symbols::BTC_USDT.to_string(),
            Price::new(50000_00), // $500.00
            1000, // 0.001 BTC
            OrderSide::Buy,
            1234567890_000_000_000,
            1,
        );

        assert_eq!(trade.maker_order_id, 1);
        assert_eq!(trade.taker_order_id, 2);
        assert_eq!(trade.quantity, 1000);
        assert_eq!(trade.pair, crate::types::symbols::BTC_USDT);
    }

    #[test]
    fn test_trade_value() {
        let trade = Trade::new(
            1, 2, 100, 200,
            crate::types::symbols::BTC_USDT.to_string(),
            Price::new(50000_00), // $500.00
            1000, // 0.001 BTC
            OrderSide::Buy,
            1234567890_000_000_000,
            1,
        );

        assert_eq!(trade.total_value(), Some(5_000_000_000)); // $500.00 * 1000
    }

    #[test]
    fn test_trade_fees() {
        let trade = Trade::new(
            1, 2, 100, 200,
            crate::types::symbols::BTC_USDT.to_string(),
            Price::new(50000_00), // $500.00
            1000, // 0.001 BTC
            OrderSide::Buy,
            1234567890_000_000_000,
            1,
        );

        let maker_fee = trade.maker_fee(crate::types::constants::DEFAULT_MAKER_FEE_RATE);
        assert_eq!(maker_fee, Some(5_000_000)); // 0.001 * 5_000_000_000

        let taker_fee = trade.taker_fee(crate::types::constants::DEFAULT_TAKER_FEE_RATE);
        assert_eq!(taker_fee, Some(10_000_000)); // 0.002 * 5_000_000_000
    }

    #[test]
    fn test_trade_history() {
        let trades = vec![
            Trade::new(1, 2, 100, 200, crate::types::symbols::BTC_USDT.to_string(), Price::new(50000_00), 1000, OrderSide::Buy, 1, 1),
            Trade::new(3, 4, 101, 201, crate::types::symbols::BTC_USDT.to_string(), Price::new(50100_00), 2000, OrderSide::Sell, 2, 2),
        ];

        let history = TradeHistory::new(trades.clone(), 2, false);

        assert_eq!(history.total_traded_quantity(), 3000);
        assert_eq!(history.total_traded_value(), Some(15_020_000_000));
        assert_eq!(history.for_user(100).len(), 1);
        assert_eq!(history.for_pair(crate::types::symbols::BTC_USDT).len(), 2);
        assert_eq!(history.latest().unwrap().sequence, 2);
        assert_eq!(history.earliest().unwrap().sequence, 1);
    }
}

/// Trade execution report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    /// Trade ID
    pub trade_id: Uuid,
    /// Order ID
    pub order_id: OrderId,
    /// Trade price
    pub price: Price,
    /// Trade quantity
    pub quantity: Quantity,
    /// Trade side
    pub side: crate::order::OrderSide,
    /// Role in the trade (maker/taker)
    pub role: TradeRole,
    /// Fee amount
    pub fee: Quantity,
    /// Net amount (after fees)
    pub net_amount: Quantity,
    /// Timestamp
    pub timestamp: crate::types::Timestamp,
}

impl TradeExecution {
    /// Creates a new trade execution report
    pub fn new(
        trade_id: Uuid,
        order_id: OrderId,
        price: Price,
        quantity: Quantity,
        side: crate::order::OrderSide,
        role: TradeRole,
        fee: Quantity,
        net_amount: Quantity,
        timestamp: crate::types::Timestamp,
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
    pub maker_fee_rate: crate::types::FeeRate,
    /// Taker fee rate (as decimal, e.g., 0.002 for 0.2%)
    pub taker_fee_rate: crate::types::FeeRate,
}

impl FeeModel {
    /// Creates a new fee model
    pub fn new(maker_fee_rate: crate::types::FeeRate, taker_fee_rate: crate::types::FeeRate) -> Self {
        Self {
            maker_fee_rate,
            taker_fee_rate,
        }
    }

    /// Calculates the fee for a trade
    pub fn calculate_fee(&self, total_value: Quantity, role: TradeRole) -> Quantity {
        let rate = match role {
            TradeRole::Maker => self.maker_fee_rate,
            TradeRole::Taker => self.taker_fee_rate,
        };
        (total_value as f64 * rate) as Quantity
    }

    /// Returns a zero-fee model
    pub fn zero_fee() -> Self {
        Self::new(0.0, 0.0)
    }

    /// Returns a standard fee model (0.1% maker, 0.2% taker)
    pub fn standard() -> Self {
        Self::new(
            crate::types::constants::DEFAULT_MAKER_FEE_RATE,
            crate::types::constants::DEFAULT_TAKER_FEE_RATE,
        )
    }
}

impl Default for FeeModel {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod fee_model_tests {
    use super::*;
    use crate::order::OrderSide;

    #[test]
    fn test_trade_creation() {
        let trade = Trade::new(
            1, 2, 100, 200,
            crate::types::symbols::BTC_USDT.to_string(),
            Price::new(50000_00), // $500.00
            1000, // 0.001 BTC
            OrderSide::Buy,
            1234567890_000_000_000,
            1,
        );

        assert_eq!(trade.maker_order_id, 1);
        assert_eq!(trade.taker_order_id, 2);
        assert_eq!(trade.quantity, 1000);
        assert_eq!(trade.pair, crate::types::symbols::BTC_USDT);
    }

    #[test]
    fn test_trade_value() {
        let trade = Trade::new(
            1, 2, 100, 200,
            crate::types::symbols::BTC_USDT.to_string(),
            Price::new(50000_00), // $500.00
            1000, // 0.001 BTC
            OrderSide::Buy,
            1234567890_000_000_000,
            1,
        );

        assert_eq!(trade.total_value(), Some(5_000_000_000)); // $500.00 * 1000
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
            Price::new(50000_00),
            1000,
            OrderSide::Buy,
            TradeRole::Taker,
            1000,
            49_000_000,
            1234567890_000_000_000,
        );

        assert_eq!(execution.order_id, 1);
        assert_eq!(execution.role, TradeRole::Taker);
        assert_eq!(execution.fee, 1000);
    }
}
