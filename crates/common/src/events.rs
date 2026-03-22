//! Event types for the exchange system.

use crate::Asset;
use crate::types::{UserId, Timestamp};
use uuid::Uuid;

/// Represents different types of exchange events
#[derive(Debug, Clone)]
pub enum ExchangeEvent {
    /// Order was placed
    OrderPlaced {
        order_id: crate::types::OrderId,
        user_id: UserId,
        pair: String,
        side: String,
        price: Option<crate::types::PriceValue>,
        quantity: crate::types::Quantity,
        timestamp: Timestamp,
    },

    /// Order was cancelled
    OrderCancelled {
        order_id: crate::types::OrderId,
        user_id: UserId,
        reason: String,
        timestamp: Timestamp,
    },

    /// Trade occurred
    TradeExecuted {
        trade_id: String,
        maker_order_id: crate::types::OrderId,
        taker_order_id: crate::types::OrderId,
        maker_user_id: UserId,
        taker_user_id: UserId,
        pair: String,
        price: crate::types::PriceValue,
        quantity: crate::types::Quantity,
        timestamp: Timestamp,
    },

    /// Deposit was confirmed
    DepositConfirmed {
        deposit_id: Uuid,
        user_id: UserId,
        asset: Asset,
        amount: crate::types::Quantity,
        tx_hash: String,
        confirmations: u32,
        timestamp: Timestamp,
    },

    /// Withdrawal was processed
    WithdrawalProcessed {
        withdrawal_id: Uuid,
        user_id: UserId,
        asset: Asset,
        amount: crate::types::Quantity,
        address: String,
        tx_hash: String,
        timestamp: Timestamp,
    },

    /// Balance changed
    BalanceChanged {
        user_id: UserId,
        asset: Asset,
        old_balance: crate::types::Balance,
        new_balance: crate::types::Balance,
        timestamp: Timestamp,
    },

    /// Position changed
    PositionChanged {
        user_id: UserId,
        asset: Asset,
        old_position: i64,
        new_position: i64,
        timestamp: Timestamp,
    },

    /// Risk limit breached
    RiskLimitBreached {
        user_id: UserId,
        limit_type: String,
        limit_value: crate::types::Quantity,
        current_value: crate::types::Quantity,
        timestamp: Timestamp,
    },

    /// Account was created
    AccountCreated {
        user_id: UserId,
        timestamp: Timestamp,
    },

    /// Risk check was performed
    RiskCheckPerformed {
        user_id: UserId,
        order_id: crate::types::OrderId,
        passed: bool,
        reason: Option<String>,
        timestamp: Timestamp,
    },

    /// System status update
    SystemStatus {
        component: String,
        status: String,
        message: Option<String>,
        timestamp: Timestamp,
    },
}

impl ExchangeEvent {
    /// Returns the timestamp of the event
    pub fn timestamp(&self) -> Timestamp {
        match self {
            ExchangeEvent::OrderPlaced { timestamp, .. }
            | ExchangeEvent::OrderCancelled { timestamp, .. }
            | ExchangeEvent::TradeExecuted { timestamp, .. }
            | ExchangeEvent::DepositConfirmed { timestamp, .. }
            | ExchangeEvent::WithdrawalProcessed { timestamp, .. }
            | ExchangeEvent::BalanceChanged { timestamp, .. }
            | ExchangeEvent::PositionChanged { timestamp, .. }
            | ExchangeEvent::RiskLimitBreached { timestamp, .. }
            | ExchangeEvent::AccountCreated { timestamp, .. }
            | ExchangeEvent::RiskCheckPerformed { timestamp, .. }
            | ExchangeEvent::SystemStatus { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the event type as a string
    pub fn event_type(&self) -> &'static str {
        match self {
            ExchangeEvent::OrderPlaced { .. } => "order_placed",
            ExchangeEvent::OrderCancelled { .. } => "order_cancelled",
            ExchangeEvent::TradeExecuted { .. } => "trade_executed",
            ExchangeEvent::DepositConfirmed { .. } => "deposit_confirmed",
            ExchangeEvent::WithdrawalProcessed { .. } => "withdrawal_processed",
            ExchangeEvent::BalanceChanged { .. } => "balance_changed",
            ExchangeEvent::PositionChanged { .. } => "position_changed",
            ExchangeEvent::RiskLimitBreached { .. } => "risk_limit_breached",
            ExchangeEvent::AccountCreated { .. } => "account_created",
            ExchangeEvent::RiskCheckPerformed { .. } => "risk_check_performed",
            ExchangeEvent::SystemStatus { .. } => "system_status",
        }
    }

    /// Returns the user ID associated with the event, if any
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            ExchangeEvent::OrderPlaced { user_id, .. }
            | ExchangeEvent::OrderCancelled { user_id, .. }
            | ExchangeEvent::DepositConfirmed { user_id, .. }
            | ExchangeEvent::WithdrawalProcessed { user_id, .. }
            | ExchangeEvent::BalanceChanged { user_id, .. }
            | ExchangeEvent::PositionChanged { user_id, .. }
            | ExchangeEvent::RiskLimitBreached { user_id, .. }
            | ExchangeEvent::AccountCreated { user_id, .. }
            | ExchangeEvent::RiskCheckPerformed { user_id, .. } => Some(*user_id),

            ExchangeEvent::TradeExecuted {
                maker_user_id,
                taker_user_id: _,
                ..
            } => Some(*maker_user_id), // Return maker user ID

            ExchangeEvent::SystemStatus { .. } => None,
        }
    }

    /// Returns the trading pair associated with the event, if any
    pub fn trading_pair(&self) -> Option<&str> {
        match self {
            ExchangeEvent::OrderPlaced { pair, .. }
            | ExchangeEvent::TradeExecuted { pair, .. } => Some(pair),
            _ => None,
        }
    }

    /// Returns the order ID associated with the event, if any
    pub fn order_id(&self) -> Option<crate::types::OrderId> {
        match self {
            ExchangeEvent::OrderPlaced { order_id, .. }
            | ExchangeEvent::OrderCancelled { order_id, .. }
            | ExchangeEvent::RiskCheckPerformed { order_id, .. } => Some(*order_id),
            _ => None,
        }
    }

    /// Returns the trade ID associated with the event, if any
    pub fn trade_id(&self) -> Option<&str> {
        match self {
            ExchangeEvent::TradeExecuted { trade_id, .. } => Some(trade_id),
            _ => None,
        }
    }

    /// Returns the asset associated with the event, if any
    pub fn asset(&self) -> Option<&Asset> {
        match self {
            ExchangeEvent::DepositConfirmed { asset, .. }
            | ExchangeEvent::WithdrawalProcessed { asset, .. }
            | ExchangeEvent::BalanceChanged { asset, .. }
            | ExchangeEvent::PositionChanged { asset, .. } => Some(asset),
            _ => None,
        }
    }
}

/// Event listener trait
pub trait EventListener: Send + Sync {
    /// Handle an exchange event
    fn handle_event(&self, event: &ExchangeEvent);
}

/// In-memory event bus
pub struct EventBus {
    listeners: Vec<Box<dyn EventListener>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }
}

impl EventBus {
    /// Creates a new event bus
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    /// Adds an event listener
    pub fn add_listener(&mut self, listener: Box<dyn EventListener>) {
        self.listeners.push(listener);
    }

    /// Publishes an event to all listeners
    pub fn publish(&self, event: ExchangeEvent) {
        for listener in &self.listeners {
            listener.handle_event(&event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct TestListener {
        events: Arc<Mutex<Vec<ExchangeEvent>>>,
    }

    impl EventListener for TestListener {
        fn handle_event(&self, event: &ExchangeEvent) {
            self.events.lock().unwrap().push(event.clone());
        }
    }

    #[test]
    fn test_event_bus() {
        let mut bus = EventBus::new();
        let events = Arc::new(Mutex::new(Vec::new()));
        let listener = TestListener {
            events: events.clone(),
        };

        bus.add_listener(Box::new(listener));

        let event = ExchangeEvent::OrderPlaced {
            order_id: 1,
            user_id: 100,
            pair: "BTC/USDT".to_string(),
            side: "buy".to_string(),
            price: Some(50000),
            quantity: 1000,
            timestamp: 1234567890,
        };

        bus.publish(event);

        let stored_events = events.lock().unwrap();
        assert_eq!(stored_events.len(), 1);
        assert_eq!(stored_events[0].event_type(), "order_placed");
    }
}
