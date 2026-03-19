//! Type definitions for the matching engine.

use crypto_exchange_common::{
    events::ExchangeEvent,
    order::{Order, OrderSide, OrderType, TimeInForce},
    price::Price,
    trade::{Trade, TradeRole},
    ExchangeError, ExchangeResult,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Matching engine command types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchingEngineCommand {
    /// Submit a new order
    SubmitOrder { order: Order },
    /// Cancel an existing order
    CancelOrder { order_id: u64, user_id: u64 },
    /// Replace an existing order
    ReplaceOrder {
        old_order_id: u64,
        new_order: Order,
    },
    /// Get order book snapshot
    GetOrderBook { depth: usize },
    /// Get order status
    GetOrderStatus { order_id: u64 },
}

/// Matching engine response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchingEngineResponse {
    /// Order submission result
    OrderSubmitted {
        order_id: u64,
        status: crypto_exchange_common::order::OrderStatus,
        timestamp: u64,
    },
    /// Order cancellation result
    OrderCancelled {
        order_id: u64,
        success: bool,
        reason: Option<String>,
        timestamp: u64,
    },
    /// Order replacement result
    OrderReplaced {
        old_order_id: u64,
        new_order_id: u64,
        success: bool,
        reason: Option<String>,
        timestamp: u64,
    },
    /// Order book snapshot
    OrderBookSnapshot {
        pair: String,
        bids: Vec<crypto_exchange_common::price::PriceLevel>,
        asks: Vec<crypto_exchange_common::price::PriceLevel>,
        timestamp: u64,
    },
    /// Order status
    OrderStatus {
        order_id: u64,
        status: crypto_exchange_common::order::OrderStatus,
        filled_quantity: u64,
        remaining_quantity: u64,
        timestamp: u64,
    },
    /// Error response
    Error {
        error: String,
        timestamp: u64,
    },
}

/// Trade execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    /// Trade ID
    pub trade_id: Uuid,
    /// Maker order ID
    pub maker_order_id: u64,
    /// Taker order ID
    pub taker_order_id: u64,
    /// Maker user ID
    pub maker_user_id: u64,
    /// Taker user ID
    pub taker_user_id: u64,
    /// Trading pair
    pub pair: String,
    /// Trade price
    pub price: Price,
    /// Trade quantity
    pub quantity: u64,
    /// Trade side (from taker's perspective)
    pub side: OrderSide,
    /// Trade timestamp
    pub timestamp: u64,
    /// Trade sequence number
    pub sequence: u64,
}

impl TradeExecution {
    /// Creates a new trade execution
    pub fn new(
        maker_order_id: u64,
        taker_order_id: u64,
        maker_user_id: u64,
        taker_user_id: u64,
        pair: String,
        price: Price,
        quantity: u64,
        side: OrderSide,
        timestamp: u64,
        sequence: u64,
    ) -> Self {
        Self {
            trade_id: Uuid::new_v4(),
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

    /// Converts to a Trade object
    pub fn to_trade(self) -> Trade {
        Trade::new(
            self.maker_order_id,
            self.taker_order_id,
            self.maker_user_id,
            self.taker_user_id,
            self.pair,
            self.price,
            self.quantity,
            self.side,
            self.timestamp,
            self.sequence,
        )
    }

    /// Returns the total trade value
    pub fn total_value(&self) -> Option<u64> {
        self.price.value().checked_mul(self.quantity)
    }
}

/// Order execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderExecution {
    /// Order ID
    pub order_id: u64,
    /// User ID
    pub user_id: u64,
    /// Order side
    pub side: OrderSide,
    /// Order type
    pub order_type: OrderType,
    /// Original quantity
    pub original_quantity: u64,
    /// Filled quantity
    pub filled_quantity: u64,
    /// Remaining quantity
    pub remaining_quantity: u64,
    /// Average execution price
    pub avg_price: Option<Price>,
    /// Total cost
    pub total_cost: u64,
    /// Order status
    pub status: crypto_exchange_common::order::OrderStatus,
    /// Execution timestamp
    pub timestamp: u64,
    /// Whether the order should be removed from the book
    pub should_remove: bool,
}

impl OrderExecution {
    /// Creates a new order execution
    pub fn new(
        order_id: u64,
        user_id: u64,
        side: OrderSide,
        order_type: OrderType,
        original_quantity: u64,
        filled_quantity: u64,
        avg_price: Option<Price>,
        total_cost: u64,
        status: crypto_exchange_common::order::OrderStatus,
        timestamp: u64,
        should_remove: bool,
    ) -> Self {
        Self {
            order_id,
            user_id,
            side,
            order_type,
            original_quantity,
            filled_quantity,
            remaining_quantity: original_quantity - filled_quantity,
            avg_price,
            total_cost,
            status,
            timestamp,
            should_remove,
        }
    }

    /// Checks if the order was fully filled
    pub fn is_fully_filled(&self) -> bool {
        self.filled_quantity >= self.original_quantity
    }

    /// Checks if the order was partially filled
    pub fn is_partially_filled(&self) -> bool {
        self.filled_quantity > 0 && self.filled_quantity < self.original_quantity
    }

    /// Checks if the order has any fills
    pub fn has_fills(&self) -> bool {
        self.filled_quantity > 0
    }

    /// Calculates the fill percentage
    pub fn fill_percentage(&self) -> f64 {
        if self.original_quantity == 0 {
            0.0
        } else {
            self.filled_quantity as f64 / self.original_quantity as f64
        }
    }
}

/// Matching engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingEngineConfig {
    /// Maximum order size
    pub max_order_size: u64,
    /// Minimum order size
    pub min_order_size: u64,
    /// Tick size
    pub tick_size: u64,
    /// Lot size
    pub lot_size: u64,
    /// Maximum price deviation (in basis points)
    pub max_price_deviation_bps: u32,
    /// Enable price protection
    pub enable_price_protection: bool,
    /// Maximum number of price levels
    pub max_price_levels: usize,
    /// Order book depth for snapshots
    pub default_snapshot_depth: usize,
}

impl MatchingEngineConfig {
    /// Creates a new matching engine configuration
    pub fn new(
        max_order_size: u64,
        min_order_size: u64,
        tick_size: u64,
        lot_size: u64,
        max_price_deviation_bps: u32,
        enable_price_protection: bool,
        max_price_levels: usize,
        default_snapshot_depth: usize,
    ) -> Self {
        Self {
            max_order_size,
            min_order_size,
            tick_size,
            lot_size,
            max_price_deviation_bps,
            enable_price_protection,
            max_price_levels,
            default_snapshot_depth,
        }
    }

    /// Returns a default configuration for BTC/USDT
    pub fn btc_usdt() -> Self {
        Self::new(
            1_000_000,    // max_order_size (10 BTC)
            1,            // min_order_size (1 satoshi)
            100,          // tick_size ($0.01)
            1,            // lot_size (1 satoshi)
            1000,         // max_price_deviation_bps (10%)
            true,         // enable_price_protection
            1000,         // max_price_levels
            20,           // default_snapshot_depth
        )
    }

    /// Returns a default configuration for ETH/USDT
    pub fn eth_usdt() -> Self {
        Self::new(
            10_000_000,   // max_order_size (10 ETH)
            1,            // min_order_size (1 wei)
            1000,         // tick_size ($0.10)
            1,            // lot_size (1 wei)
            1000,         // max_price_deviation_bps (10%)
            true,         // enable_price_protection
            1000,         // max_price_levels
            20,           // default_snapshot_depth
        )
    }
}

impl Default for MatchingEngineConfig {
    fn default() -> Self {
        Self::btc_usdt()
    }
}

/// Matching engine statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingEngineStats {
    /// Trading pair
    pub pair: String,
    /// Total number of orders processed
    pub total_orders: u64,
    /// Total number of trades executed
    pub total_trades: u64,
    /// Total volume traded
    pub total_volume: u64,
    /// Current bid-ask spread
    pub spread: Option<u64>,
    /// Current mid price
    pub mid_price: Option<Price>,
    /// Number of bid levels
    pub bid_levels: usize,
    /// Number of ask levels
    pub ask_levels: usize,
    /// Total bid quantity
    pub total_bid_quantity: u64,
    /// Total ask quantity
    pub total_ask_quantity: u64,
    /// Last trade price
    pub last_trade_price: Option<Price>,
    /// Last trade quantity
    pub last_trade_quantity: Option<u64>,
    /// Last trade timestamp
    pub last_trade_timestamp: Option<u64>,
    /// Engine uptime in nanoseconds
    pub uptime_ns: u64,
}

/// Matching engine event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchingEngineEvent {
    /// Order was accepted
    OrderAccepted {
        order_id: u64,
        user_id: u64,
        timestamp: u64,
    },
    /// Order was rejected
    OrderRejected {
        order_id: u64,
        user_id: u64,
        reason: String,
        timestamp: u64,
    },
    /// Trade was executed
    TradeExecuted(TradeExecution),
    /// Order was cancelled
    OrderCancelled {
        order_id: u64,
        user_id: u64,
        reason: Option<String>,
        timestamp: u64,
    },
    /// Order book updated
    OrderBookUpdated {
        pair: String,
        timestamp: u64,
    },
}

impl MatchingEngineEvent {
    /// Returns the event timestamp
    pub fn timestamp(&self) -> u64 {
        match self {
            MatchingEngineEvent::OrderAccepted { timestamp, .. }
            | MatchingEngineEvent::OrderRejected { timestamp, .. }
            | MatchingEngineEvent::TradeExecuted(exec) => exec.timestamp,
            | MatchingEngineEvent::OrderCancelled { timestamp, .. }
            | MatchingEngineEvent::OrderBookUpdated { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the user ID associated with the event, if any
    pub fn user_id(&self) -> Option<u64> {
        match self {
            MatchingEngineEvent::OrderAccepted { user_id, .. }
            | MatchingEngineEvent::OrderRejected { user_id, .. }
            | MatchingEngineEvent::OrderCancelled { user_id, .. } => Some(*user_id),
            MatchingEngineEvent::TradeExecuted(exec) => Some(exec.taker_user_id),
            MatchingEngineEvent::OrderBookUpdated { .. } => None,
        }
    }

    /// Converts to an ExchangeEvent
    pub fn to_exchange_event(&self) -> ExchangeEvent {
        match self {
            MatchingEngineEvent::OrderAccepted { order_id, user_id, timestamp } => {
                ExchangeEvent::OrderPlaced {
                    order_id: *order_id,
                    user_id: *user_id,
                    pair: "UNKNOWN".to_string(), // Would be filled in actual implementation
                    side: "UNKNOWN".to_string(),
                    price: None,
                    quantity: 0,
                    timestamp: *timestamp,
                }
            }
            MatchingEngineEvent::TradeExecuted(exec) => {
                ExchangeEvent::TradeExecuted {
                    trade_id: exec.trade_id,
                    maker_order_id: exec.maker_order_id,
                    taker_order_id: exec.taker_order_id,
                    price: exec.price,
                    quantity: exec.quantity,
                    maker_user_id: exec.maker_user_id,
                    taker_user_id: exec.taker_user_id,
                    pair: exec.pair.clone(),
                    timestamp: exec.timestamp,
                }
            }
            MatchingEngineEvent::OrderCancelled { order_id, user_id, reason, timestamp } => {
                ExchangeEvent::OrderCancelled {
                    order_id: *order_id,
                    user_id: *user_id,
                    reason: reason.clone().unwrap_or_else(|| "User requested".to_string()),
                    timestamp: *timestamp,
                }
            }
            _ => {
                // Other events would be converted appropriately
                ExchangeEvent::SystemStatus {
                    component: "matching_engine".to_string(),
                    status: "active".to_string(),
                    message: None,
                    timestamp: self.timestamp(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::order::OrderStatus;

    #[test]
    fn test_trade_execution() {
        let exec = TradeExecution::new(
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

        assert_eq!(exec.maker_order_id, 1);
        assert_eq!(exec.taker_order_id, 2);
        assert_eq!(exec.quantity, 1000);
        assert_eq!(exec.total_value(), Some(50_000_000));
    }

    #[test]
    fn test_order_execution() {
        let exec = OrderExecution::new(
            1,
            100,
            OrderSide::Buy,
            OrderType::Limit,
            1000,
            500,
            Some(Price::new(50000)),
            25_000_000,
            OrderStatus::PartiallyFilled,
            1234567890,
            false,
        );

        assert_eq!(exec.order_id, 1);
        assert_eq!(exec.filled_quantity, 500);
        assert_eq!(exec.remaining_quantity, 500);
        assert!(exec.is_partially_filled());
        assert_eq!(exec.fill_percentage(), 0.5);
    }

    #[test]
    fn test_matching_engine_config() {
        let config = MatchingEngineConfig::btc_usdt();
        assert_eq!(config.tick_size, 100);
        assert_eq!(config.lot_size, 1);
        assert!(config.enable_price_protection);

        let default_config = MatchingEngineConfig::default();
        assert_eq!(default_config.tick_size, 100);
        assert_eq!(default_config.lot_size, 1);
    }

    #[test]
    fn test_matching_engine_event() {
        let exec = TradeExecution::new(
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

        let event = MatchingEngineEvent::TradeExecuted(exec);
        assert_eq!(event.timestamp(), 1234567890);
        assert_eq!(event.user_id(), Some(200));

        let exchange_event = event.to_exchange_event();
        match exchange_event {
            ExchangeEvent::TradeExecuted { trade_id, .. } => {
                assert_eq!(trade_id, event.timestamp().into()); // Simplified check
            }
            _ => panic!("Expected trade executed event"),
        }
    }
}
