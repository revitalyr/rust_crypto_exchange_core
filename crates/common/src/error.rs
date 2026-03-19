//! Error types used throughout the exchange system.

use thiserror::Error;

/// Common error types for the exchange
#[derive(Error, Debug, Clone)]
pub enum ExchangeError {
    #[error("Invalid order: {reason}")]
    InvalidOrder { reason: String },

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },

    #[error("Order not found: {order_id}")]
    OrderNotFound { order_id: u64 },

    #[error("Account not found: {account_id}")]
    AccountNotFound { account_id: u64 },

    #[error("Invalid price: {price}")]
    InvalidPrice { price: u64 },

    #[error("Invalid quantity: {quantity}")]
    InvalidQuantity { quantity: u64 },

    #[error("Trading pair not supported: {pair}")]
    UnsupportedPair { pair: String },

    #[error("Market order cannot be executed: {reason}")]
    MarketOrderError { reason: String },

    #[error("Risk check failed: {reason}")]
    RiskCheckFailed { reason: String },

    #[error("Blockchain operation failed: {operation}")]
    BlockchainError { operation: String },

    #[error("Persistence error: {reason}")]
    PersistenceError { reason: String },

    #[error("Network error: {reason}")]
    NetworkError { reason: String },

    #[error("Serialization error: {reason}")]
    SerializationError { reason: String },

    #[error("System error: {reason}")]
    SystemError { reason: String },
}

impl ExchangeError {
    /// Creates an invalid order error
    pub fn invalid_order(reason: impl Into<String>) -> Self {
        Self::InvalidOrder {
            reason: reason.into(),
        }
    }

    /// Creates an insufficient balance error
    pub fn insufficient_balance(required: u64, available: u64) -> Self {
        Self::InsufficientBalance { required, available }
    }

    /// Creates an order not found error
    pub fn order_not_found(order_id: u64) -> Self {
        Self::OrderNotFound { order_id }
    }

    /// Creates an account not found error
    pub fn account_not_found(account_id: u64) -> Self {
        Self::AccountNotFound { account_id }
    }

    /// Creates an invalid price error
    pub fn invalid_price(price: u64) -> Self {
        Self::InvalidPrice { price }
    }

    /// Creates an invalid quantity error
    pub fn invalid_quantity(quantity: u64) -> Self {
        Self::InvalidQuantity { quantity }
    }

    /// Creates an unsupported pair error
    pub fn unsupported_pair(pair: impl Into<String>) -> Self {
        Self::UnsupportedPair {
            pair: pair.into(),
        }
    }

    /// Creates a market order error
    pub fn market_order_error(reason: impl Into<String>) -> Self {
        Self::MarketOrderError {
            reason: reason.into(),
        }
    }

    /// Creates a risk check failed error
    pub fn risk_check_failed(reason: impl Into<String>) -> Self {
        Self::RiskCheckFailed {
            reason: reason.into(),
        }
    }

    /// Creates a blockchain error
    pub fn blockchain_error(operation: impl Into<String>) -> Self {
        Self::BlockchainError {
            operation: operation.into(),
        }
    }

    /// Creates a persistence error
    pub fn persistence_error(reason: impl Into<String>) -> Self {
        Self::PersistenceError {
            reason: reason.into(),
        }
    }

    /// Creates a network error
    pub fn network_error(reason: impl Into<String>) -> Self {
        Self::NetworkError {
            reason: reason.into(),
        }
    }

    /// Creates a serialization error
    pub fn serialization_error(reason: impl Into<String>) -> Self {
        Self::SerializationError {
            reason: reason.into(),
        }
    }

    /// Creates a system error
    pub fn system_error(reason: impl Into<String>) -> Self {
        Self::SystemError {
            reason: reason.into(),
        }
    }
}

/// Result type for exchange operations
pub type ExchangeResult<T> = Result<T, ExchangeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = ExchangeError::invalid_order("Price too low");
        assert!(matches!(error, ExchangeError::InvalidOrder { .. }));
    }

    #[test]
    fn test_error_display() {
        let error = ExchangeError::insufficient_balance(1000, 500);
        assert_eq!(
            error.to_string(),
            "Insufficient balance: required 1000, available 500"
        );
    }
}
