//! Event types for the exchange system.

use crate::{order::Order, trade::Trade, Asset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents different types of exchange events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExchangeEvent {
    /// Order was placed
    OrderPlaced {
        order_id: u64,
        user_id: u64,
        pair: String,
        side: String,
        price: Option<u64>,
        quantity: u64,
        timestamp: u64,
    },

    /// Order was cancelled
    OrderCancelled {
        order_id: u64,
        user_id: u64,
        reason: String,
        timestamp: u64,
    },

    /// Trade was executed
    TradeExecuted {
        trade_id: Uuid,
        maker_order_id: u64,
        taker_order_id: u64,
        price: u64,
        quantity: u64,
        maker_user_id: u64,
        taker_user_id: u64,
        pair: String,
        timestamp: u64,
    },

    /// Deposit was confirmed
    DepositConfirmed {
        deposit_id: Uuid,
        user_id: u64,
        asset: Asset,
        amount: u64,
        tx_hash: String,
        confirmations: u32,
        timestamp: u64,
    },

    /// Withdrawal was processed
    WithdrawalProcessed {
        withdrawal_id: Uuid,
        user_id: u64,
        asset: Asset,
        amount: u64,
        address: String,
        tx_hash: String,
        timestamp: u64,
    },

    /// Balance was updated
    BalanceUpdated {
        user_id: u64,
        asset: Asset,
        old_balance: u64,
        new_balance: u64,
        old_reserved: u64,
        new_reserved: u64,
        timestamp: u64,
    },

    /// Account was created
    AccountCreated {
        user_id: u64,
        timestamp: u64,
    },

    /// Risk check was performed
    RiskCheckPerformed {
        user_id: u64,
        order_id: u64,
        passed: bool,
        reason: Option<String>,
        timestamp: u64,
    },

    /// System status update
    SystemStatus {
        component: String,
        status: String,
        message: Option<String>,
        timestamp: u64,
    },
}

impl ExchangeEvent {
    /// Returns the timestamp of the event
    pub fn timestamp(&self) -> u64 {
        match self {
            ExchangeEvent::OrderPlaced { timestamp, .. }
            | ExchangeEvent::OrderCancelled { timestamp, .. }
            | ExchangeEvent::TradeExecuted { timestamp, .. }
            | ExchangeEvent::DepositConfirmed { timestamp, .. }
            | ExchangeEvent::WithdrawalProcessed { timestamp, .. }
            | ExchangeEvent::BalanceUpdated { timestamp, .. }
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
            ExchangeEvent::BalanceUpdated { .. } => "balance_updated",
            ExchangeEvent::AccountCreated { .. } => "account_created",
            ExchangeEvent::RiskCheckPerformed { .. } => "risk_check_performed",
            ExchangeEvent::SystemStatus { .. } => "system_status",
        }
    }

    /// Returns the user ID associated with the event, if any
    pub fn user_id(&self) -> Option<u64> {
        match self {
            ExchangeEvent::OrderPlaced { user_id, .. }
            | ExchangeEvent::OrderCancelled { user_id, .. }
            | ExchangeEvent::DepositConfirmed { user_id, .. }
            | ExchangeEvent::WithdrawalProcessed { user_id, .. }
            | ExchangeEvent::BalanceUpdated { user_id, .. }
            | ExchangeEvent::AccountCreated { user_id, .. }
            | ExchangeEvent::RiskCheckPerformed { user_id, .. } => Some(*user_id),

            ExchangeEvent::TradeExecuted {
                maker_user_id,
                taker_user_id,
                ..
            } => Some(*maker_user_id), // Return maker user ID

            ExchangeEvent::SystemStatus { .. } => None,
        }
    }
}

/// Event listener trait
pub trait EventListener: Send + Sync {
    /// Handle an exchange event
    fn handle_event(&self, event: &ExchangeEvent);
}

/// In-memory event bus
#[derive(Debug, Default)]
pub struct EventBus {
    listeners: Vec<Box<dyn EventListener>>,
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
