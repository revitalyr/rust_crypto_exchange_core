//! Core Event Types
//! 
//! All exchange events in one place
//! This enables event sourcing and replay capabilities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crypto_exchange_common::{
    types::{OrderId, UserId, Quantity, Timestamp, Balance},
    order::{OrderSide, OrderType},
    assets::{Asset, TradingPair}
};

/// Core exchange event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExchangeEvent {
    /// Unique event identifier
    pub id: String,
    /// Event type discriminator
    pub event_type: EventType,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event sequence number
    pub sequence: u64,
    /// Event payload
    pub payload: EventPayload,
    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
    /// Event version
    pub version: u32,
}

/// Event type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Order lifecycle events
    OrderAccepted,
    OrderRejected,
    OrderCancelled,
    OrderPartiallyFilled,
    OrderFullyFilled,
    
    /// Trading events
    TradeExecuted,
    
    /// Balance events
    BalanceUpdated,
    BalanceReserved,
    BalanceReleased,
    
    /// Deposit events
    DepositDetected,
    DepositConfirmed,
    DepositFailed,
    
    /// Withdrawal events
    WithdrawalRequested,
    WithdrawalProcessed,
    WithdrawalFailed,
    WithdrawalConfirmed,
    
    /// System events
    MarketDataUpdated,
    SystemStatusChanged,
    RiskAlert,
}

/// Event payload data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventPayload {
    /// Order accepted event
    OrderAccepted {
        order_id: OrderId,
        user_id: UserId,
        pair: TradingPair,
        side: OrderSide,
        order_type: OrderType,
        quantity: Quantity,
        price: Option<u64>,
    },
    
    /// Order rejected event
    OrderRejected {
        order_id: OrderId,
        user_id: UserId,
        reason: String,
    },
    
    /// Order cancelled event
    OrderCancelled {
        order_id: OrderId,
        user_id: UserId,
        reason: Option<String>,
    },
    
    /// Order partially filled event
    OrderPartiallyFilled {
        order_id: OrderId,
        user_id: UserId,
        filled_quantity: Quantity,
        remaining_quantity: Quantity,
        price: u64,
        trade_id: String,
    },
    
    /// Order fully filled event
    OrderFullyFilled {
        order_id: OrderId,
        user_id: UserId,
        total_filled_quantity: Quantity,
        average_price: u64,
        trade_ids: Vec<String>,
    },
    
    /// Trade executed event
    TradeExecuted {
        trade_id: String,
        maker_order_id: OrderId,
        taker_order_id: OrderId,
        pair: TradingPair,
        price: u64,
        quantity: Quantity,
        maker_side: OrderSide,
        taker_side: OrderSide,
        maker_user_id: UserId,
        taker_user_id: UserId,
        timestamp: Timestamp,
    },
    
    /// Balance updated event
    BalanceUpdated {
        user_id: UserId,
        asset: Asset,
        old_balance: Balance,
        new_balance: Balance,
        reason: String,
    },
    
    /// Balance reserved event
    BalanceReserved {
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        reason: String,
    },
    
    /// Balance released event
    BalanceReleased {
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        reason: String,
    },
    
    /// Deposit detected event
    DepositDetected {
        deposit_id: String,
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        tx_hash: String,
        confirmations: u32,
    },
    
    /// Deposit confirmed event
    DepositConfirmed {
        deposit_id: String,
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        tx_hash: String,
        credited_at: DateTime<Utc>,
    },
    
    /// Deposit failed event
    DepositFailed {
        deposit_id: String,
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        reason: String,
    },
    
    /// Withdrawal requested event
    WithdrawalRequested {
        withdrawal_id: String,
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        address: String,
        network_fee: Balance,
    },
    
    /// Withdrawal processed event
    WithdrawalProcessed {
        withdrawal_id: String,
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        tx_hash: String,
        processed_at: DateTime<Utc>,
    },
    
    /// Withdrawal failed event
    WithdrawalFailed {
        withdrawal_id: String,
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        reason: String,
    },
    
    /// Withdrawal confirmed event
    WithdrawalConfirmed {
        withdrawal_id: String,
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        tx_hash: String,
        confirmed_at: DateTime<Utc>,
    },
    
    /// Market data updated event
    MarketDataUpdated {
        pair: TradingPair,
        best_bid: u64,
        best_ask: u64,
        last_price: u64,
        volume_24h: Quantity,
    },
    
    /// System status changed event
    SystemStatusChanged {
        component: String,
        old_status: String,
        new_status: String,
        timestamp: DateTime<Utc>,
    },
    
    /// Risk alert event
    RiskAlert {
        alert_type: String,
        severity: String,
        user_id: Option<UserId>,
        description: String,
        metadata: serde_json::Value,
    },
}

impl ExchangeEvent {
    /// Create new event
    pub fn new(
        event_type: EventType,
        payload: EventPayload,
        sequence: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            sequence,
            payload,
            correlation_id: None,
            version: 1,
        }
    }
    
    /// Create event with correlation ID
    pub fn with_correlation(
        event_type: EventType,
        payload: EventPayload,
        sequence: u64,
        correlation_id: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            sequence,
            payload,
            correlation_id: Some(correlation_id),
            version: 1,
        }
    }
    
    /// Get event type as string
    pub fn type_name(&self) -> &'static str {
        match self.event_type {
            EventType::OrderAccepted => "OrderAccepted",
            EventType::OrderRejected => "OrderRejected",
            EventType::OrderCancelled => "OrderCancelled",
            EventType::OrderPartiallyFilled => "OrderPartiallyFilled",
            EventType::OrderFullyFilled => "OrderFullyFilled",
            EventType::TradeExecuted => "TradeExecuted",
            EventType::BalanceUpdated => "BalanceUpdated",
            EventType::BalanceReserved => "BalanceReserved",
            EventType::BalanceReleased => "BalanceReleased",
            EventType::DepositDetected => "DepositDetected",
            EventType::DepositConfirmed => "DepositConfirmed",
            EventType::DepositFailed => "DepositFailed",
            EventType::WithdrawalRequested => "WithdrawalRequested",
            EventType::WithdrawalProcessed => "WithdrawalProcessed",
            EventType::WithdrawalFailed => "WithdrawalFailed",
            EventType::WithdrawalConfirmed => "WithdrawalConfirmed",
            EventType::MarketDataUpdated => "MarketDataUpdated",
            EventType::SystemStatusChanged => "SystemStatusChanged",
            EventType::RiskAlert => "RiskAlert",
        }
    }
    
    /// Check if event is order-related
    pub fn is_order_event(&self) -> bool {
        matches!(self.event_type, 
            EventType::OrderAccepted | 
            EventType::OrderRejected | 
            EventType::OrderCancelled |
            EventType::OrderPartiallyFilled |
            EventType::OrderFullyFilled
        )
    }
    
    /// Check if event is balance-related
    pub fn is_balance_event(&self) -> bool {
        matches!(self.event_type,
            EventType::BalanceUpdated |
            EventType::BalanceReserved |
            EventType::BalanceReleased
        )
    }
    
    /// Check if event is crypto-related
    pub fn is_crypto_event(&self) -> bool {
        matches!(self.event_type,
            EventType::DepositDetected |
            EventType::DepositConfirmed |
            EventType::DepositFailed |
            EventType::WithdrawalRequested |
            EventType::WithdrawalProcessed |
            EventType::WithdrawalFailed |
            EventType::WithdrawalConfirmed
        )
    }
}
