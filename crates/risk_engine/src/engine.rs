//! Main risk engine implementation.

use crypto_exchange_common::{
    assets::{Asset, TradingPair},
    events::{EventBus, EventListener, ExchangeEvent},
    order::{Order, OrderSide, OrderType},
    price::Price,
    Balance, ExchangeError, ExchangeResult, Timestamp, UserId,
};
use crypto_exchange_accounts::Account;
use crate::{
    validator::{RiskValidator, RiskValidationResult, MarketDataProvider},
    limits::{LimitsManager, LimitStats, SystemLimits, UserLimits},
};

/// Risk engine command types
#[derive(Debug, Clone)]
pub enum RiskEngineCommand {
    /// Validate an order
    ValidateOrder {
        user_id: UserId,
        order: Order,
        trading_pair: TradingPair,
    },
    /// Check user balance
    CheckBalance {
        user_id: UserId,
        asset: Asset,
        amount: Balance,
    },
    /// Update user limits
    UpdateUserLimits {
        user_id: UserId,
        limits: UserLimits,
    },
    /// Get risk statistics
    GetRiskStats,
    /// Update system limits
    UpdateSystemLimits {
        limits: SystemLimits,
    },
}

/// Risk engine response types
#[derive(Debug, Clone)]
pub enum RiskEngineResponse {
    /// Order validation result
    OrderValidation {
        order_id: u64,
        user_id: UserId,
        passed: bool,
        failed_checks: Vec<String>,
        warnings: Vec<String>,
        timestamp: Timestamp,
    },
    /// Balance check result
    BalanceCheck {
        user_id: UserId,
        asset: Asset,
        sufficient: bool,
        available: Balance,
        required: Balance,
        timestamp: Timestamp,
    },
    /// User limits updated
    UserLimitsUpdated {
        user_id: UserId,
        success: bool,
        timestamp: Timestamp,
    },
    /// Risk statistics
    RiskStats {
        total_users: u64,
        total_validations: u64,
        failed_validations: u64,
        warnings_issued: u64,
        timestamp: Timestamp,
    },
    /// System limits updated
    SystemLimitsUpdated {
        success: bool,
        timestamp: Timestamp,
    },
    /// Error response
    Error {
        error: String,
        timestamp: Timestamp,
    },
}

/// Risk engine event types
#[derive(Debug, Clone)]
pub enum RiskEngineEvent {
    /// Order validation completed
    OrderValidated {
        order_id: u64,
        user_id: UserId,
        passed: bool,
        timestamp: Timestamp,
    },
    /// Risk limit breached
    RiskLimitBreached {
        user_id: UserId,
        limit_type: String,
        details: String,
        timestamp: Timestamp,
    },
    /// High volatility detected
    HighVolatilityDetected {
        trading_pair: TradingPair,
        volatility: f64,
        timestamp: Timestamp,
    },
    /// User account suspended
    UserAccountSuspended {
        user_id: UserId,
        reason: String,
        timestamp: Timestamp,
    },
    /// System risk status updated
    SystemRiskStatusUpdated {
        status: String,
        details: Option<String>,
        timestamp: Timestamp,
    },
}

impl RiskEngineEvent {
    /// Returns the event timestamp
    pub fn timestamp(&self) -> Timestamp {
        match self {
            RiskEngineEvent::OrderValidated { timestamp, .. }
            | RiskEngineEvent::RiskLimitBreached { timestamp, .. }
            | RiskEngineEvent::HighVolatilityDetected { timestamp, .. }
            | RiskEngineEvent::UserAccountSuspended { timestamp, .. }
            | RiskEngineEvent::SystemRiskStatusUpdated { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the user ID associated with the event, if any
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            RiskEngineEvent::OrderValidated { user_id, .. }
            | RiskEngineEvent::RiskLimitBreached { user_id, .. }
            | RiskEngineEvent::UserAccountSuspended { user_id, .. } => Some(*user_id),
            RiskEngineEvent::HighVolatilityDetected { .. }
            | RiskEngineEvent::SystemRiskStatusUpdated { .. } => None,
        }
    }
}

/// Main risk engine implementation
pub struct RiskEngine {
    /// Risk validator
    validator: RiskValidator,
    /// Event bus for publishing events
    event_bus: EventBus,
    /// Current limit statistics
    current_stats: LimitStats,
    /// Risk engine statistics
    stats: RiskEngineStats,
    /// Start timestamp
    start_timestamp: Timestamp,
}

impl RiskEngine {
    /// Creates a new risk engine
    pub fn new(validator: RiskValidator) -> Self {
        let start_timestamp = crypto_exchange_common::timestamp::now();
        
        Self {
            validator,
            event_bus: EventBus::new(),
            current_stats: LimitStats::default(),
            stats: RiskEngineStats::default(),
            start_timestamp,
        }
    }

    /// Processes a risk engine command
    pub fn process_command(&mut self, command: RiskEngineCommand) -> RiskEngineResponse {
        let timestamp = crypto_exchange_common::timestamp::now();
        
        match command {
            RiskEngineCommand::ValidateOrder { user_id, order, trading_pair } => {
                self.process_validate_order(user_id, order, trading_pair, timestamp)
            }
            RiskEngineCommand::CheckBalance { user_id, asset, amount } => {
                self.process_check_balance(user_id, asset, amount, timestamp)
            }
            RiskEngineCommand::UpdateUserLimits { user_id, limits } => {
                self.process_update_user_limits(user_id, limits, timestamp)
            }
            RiskEngineCommand::GetRiskStats => {
                self.process_get_risk_stats(timestamp)
            }
            RiskEngineCommand::UpdateSystemLimits { limits } => {
                self.process_update_system_limits(limits, timestamp)
            }
        }
    }

    /// Validates an order
    pub fn validate_order(
        &mut self,
        account: &Account,
        order: &Order,
        trading_pair: &TradingPair,
    ) -> RiskValidationResult {
        let result = self.validator.validate_order(account, order, trading_pair, &self.current_stats);
        
        // Update statistics
        self.stats.total_validations += 1;
        if !result.passed {
            self.stats.failed_validations += 1;
        }
        if !result.warnings.is_empty() {
            self.stats.warnings_issued += 1;
        }

        // Publish validation event
        let event = RiskEngineEvent::OrderValidated {
            order_id: order.id,
            user_id: account.user_id(),
            passed: result.passed,
            timestamp: crypto_exchange_common::timestamp::now(),
        };

        self.event_bus.publish(event.to_exchange_event());

        result
    }

    /// Checks if user has sufficient balance
    pub fn check_balance(&self, user_id: UserId, asset: &Asset, required_amount: Balance) -> bool {
        // In a real implementation, we would query the account service
        // For now, this is a placeholder
        true
    }

    /// Updates limit statistics
    pub fn update_limit_stats(&mut self, stats: LimitStats) {
        self.current_stats = stats;
    }

    /// Gets current risk statistics
    pub fn get_risk_stats(&self) -> RiskEngineStats {
        let mut stats = self.stats.clone();
        stats.uptime_ns = crypto_exchange_common::timestamp::now() - self.start_timestamp;
        stats
    }

    /// Resets risk statistics
    pub fn reset_stats(&mut self) {
        self.stats = RiskEngineStats::default();
    }

    /// Adds an event listener
    pub fn add_event_listener(&mut self, listener: Box<dyn EventListener>) {
        self.event_bus.add_listener(listener);
    }

    /// Gets the risk validator
    pub fn get_validator(&self) -> &RiskValidator {
        &self.validator
    }

    /// Gets mutable reference to the risk validator
    pub fn get_validator_mut(&mut self) -> &mut RiskValidator {
        &mut self.validator
    }

    /// Processes order validation command
    fn process_validate_order(
        &mut self,
        user_id: UserId,
        order: Order,
        trading_pair: TradingPair,
        timestamp: Timestamp,
    ) -> RiskEngineResponse {
        // In a real implementation, we would fetch the account
        // For now, we'll create a dummy account for validation
        let account = Account::new(user_id);
        
        let result = self.validate_order(&account, &order, &trading_pair);
        
        RiskEngineResponse::OrderValidation {
            order_id: order.id,
            user_id,
            passed: result.passed,
            failed_checks: result.failed_checks,
            warnings: result.warnings,
            timestamp,
        }
    }

    /// Processes balance check command
    fn process_check_balance(
        &self,
        user_id: UserId,
        asset: Asset,
        amount: Balance,
        timestamp: Timestamp,
    ) -> RiskEngineResponse {
        let sufficient = self.check_balance(user_id, &asset, amount);
        
        RiskEngineResponse::BalanceCheck {
            user_id,
            asset,
            sufficient,
            available: 0, // Would be fetched from account service
            required: amount,
            timestamp,
        }
    }

    /// Processes user limits update command
    fn process_update_user_limits(
        &mut self,
        user_id: UserId,
        limits: UserLimits,
        timestamp: Timestamp,
    ) -> RiskEngineResponse {
        self.validator.get_limits_manager_mut().set_user_limits(user_id, limits);
        
        RiskEngineResponse::UserLimitsUpdated {
            user_id,
            success: true,
            timestamp,
        }
    }

    /// Processes get risk stats command
    fn process_get_risk_stats(&self, timestamp: Timestamp) -> RiskEngineResponse {
        let stats = self.get_risk_stats();
        
        RiskEngineResponse::RiskStats {
            total_users: 0, // Would be tracked in real implementation
            total_validations: stats.total_validations,
            failed_validations: stats.failed_validations,
            warnings_issued: stats.warnings_issued,
            timestamp,
        }
    }

    /// Processes system limits update command
    fn process_update_system_limits(
        &mut self,
        limits: SystemLimits,
        timestamp: Timestamp,
    ) -> RiskEngineResponse {
        self.validator.get_limits_manager_mut().update_system_limits(limits);
        
        RiskEngineResponse::SystemLimitsUpdated {
            success: true,
            timestamp,
        }
    }
}

/// Risk engine statistics
#[derive(Debug, Clone, Default)]
pub struct RiskEngineStats {
    /// Total number of validations performed
    pub total_validations: u64,
    /// Number of failed validations
    pub failed_validations: u64,
    /// Number of warnings issued
    pub warnings_issued: u64,
    /// Number of users currently tracked
    pub total_users: u64,
    /// Engine uptime in nanoseconds
    pub uptime_ns: u64,
}

impl RiskEngineStats {
    /// Creates new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the validation success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_validations == 0 {
            1.0
        } else {
            (self.total_validations - self.failed_validations) as f64 / self.total_validations as f64
        }
    }

    /// Gets the warning rate
    pub fn warning_rate(&self) -> f64 {
        if self.total_validations == 0 {
            0.0
        } else {
            self.warnings_issued as f64 / self.total_validations as f64
        }
    }
}

/// Risk engine builder for convenient configuration
pub struct RiskEngineBuilder {
    validator: Option<RiskValidator>,
}

impl RiskEngineBuilder {
    /// Creates a new builder
    pub fn new() -> Self {
        Self { validator: None }
    }

    /// Sets the risk validator
    pub fn with_validator(mut self, validator: RiskValidator) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Builds the risk engine
    pub fn build(self) -> Result<RiskEngine, ExchangeError> {
        let validator = self.validator.ok_or_else(|| {
            ExchangeError::system_error("Risk validator is required".to_string())
        })?;

        Ok(RiskEngine::new(validator))
    }
}

impl Default for RiskEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RiskEngine {
    fn default() -> Self {
        let validator = crate::validator::RiskValidatorBuilder::new().build();
        Self::new(validator)
    }
}

/// Event listener for risk engine events
pub struct RiskEventListener {
    /// Callback function for handling events
    callback: Box<dyn Fn(&ExchangeEvent) + Send + Sync>,
}

impl RiskEventListener {
    /// Creates a new risk event listener
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(&ExchangeEvent) + Send + Sync + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}

impl EventListener for RiskEventListener {
    fn handle_event(&self, event: &ExchangeEvent) {
        (self.callback)(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::order::TimeInForce;

    fn create_test_account(user_id: UserId) -> Account {
        Account::new(user_id)
    }

    fn create_test_order(id: u64, side: OrderSide, order_type: OrderType, price: Option<u64>, quantity: u64) -> Order {
        Order::new(
            id,
            100,
            "BTC/USDT".to_string(),
            side,
            order_type,
            price.map(Price::new),
            quantity,
            TimeInForce::GTC,
            1234567890,
        )
    }

    #[test]
    fn test_risk_engine_creation() {
        let engine = RiskEngine::default();
        
        assert_eq!(engine.stats.total_validations, 0);
        assert_eq!(engine.stats.failed_validations, 0);
        assert_eq!(engine.stats.warnings_issued, 0);
    }

    #[test]
    fn test_risk_engine_builder() {
        let validator = crate::validator::RiskValidatorBuilder::new().build();
        let engine = RiskEngineBuilder::new()
            .with_validator(validator)
            .build()
            .unwrap();

        assert!(engine.get_validator().get_limits_manager().get_system_limits().max_total_open_orders > 0);
    }

    #[test]
    fn test_process_validate_order_command() {
        let mut engine = RiskEngine::default();
        
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        let command = RiskEngineCommand::ValidateOrder {
            user_id: 100,
            order,
            trading_pair,
        };

        let response = engine.process_command(command);
        
        match response {
            RiskEngineResponse::OrderValidation { order_id, user_id, passed, .. } => {
                assert_eq!(order_id, 1);
                assert_eq!(user_id, 100);
                // Should fail due to insufficient balance in dummy account
                assert!(!passed);
            }
            _ => panic!("Expected order validation response"),
        }

        assert_eq!(engine.stats.total_validations, 1);
        assert_eq!(engine.stats.failed_validations, 1);
    }

    #[test]
    fn test_process_check_balance_command() {
        let engine = RiskEngine::default();
        
        let command = RiskEngineCommand::CheckBalance {
            user_id: 100,
            asset: Asset::USDT,
            amount: 1000,
        };

        let response = engine.process_command(command);
        
        match response {
            RiskEngineResponse::BalanceCheck { user_id, asset, sufficient, required, .. } => {
                assert_eq!(user_id, 100);
                assert_eq!(asset, Asset::USDT);
                assert_eq!(required, 1000);
                // Dummy implementation returns true
                assert!(sufficient);
            }
            _ => panic!("Expected balance check response"),
        }
    }

    #[test]
    fn test_process_update_user_limits_command() {
        let mut engine = RiskEngine::default();
        
        let limits = UserLimits::new(
            std::collections::HashMap::new(),
            std::collections::HashMap::new(),
            crate::limits::RateLimit::default(),
            100,
            50,
        );

        let command = RiskEngineCommand::UpdateUserLimits {
            user_id: 100,
            limits,
        };

        let response = engine.process_command(command);
        
        match response {
            RiskEngineResponse::UserLimitsUpdated { user_id, success, .. } => {
                assert_eq!(user_id, 100);
                assert!(success);
            }
            _ => panic!("Expected user limits updated response"),
        }
    }

    #[test]
    fn test_process_get_risk_stats_command() {
        let mut engine = RiskEngine::default();
        
        // Perform some validation to generate stats
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        let account = create_test_account(100);
        
        engine.validate_order(&account, &order, &trading_pair);

        let command = RiskEngineCommand::GetRiskStats;
        let response = engine.process_command(command);
        
        match response {
            RiskEngineResponse::RiskStats { total_validations, failed_validations, .. } => {
                assert_eq!(total_validations, 1);
                assert_eq!(failed_validations, 1);
            }
            _ => panic!("Expected risk stats response"),
        }
    }

    #[test]
    fn test_process_update_system_limits_command() {
        let mut engine = RiskEngine::default();
        
        let limits = crate::limits::SystemLimits::default();
        let command = RiskEngineCommand::UpdateSystemLimits { limits };

        let response = engine.process_command(command);
        
        match response {
            RiskEngineResponse::SystemLimitsUpdated { success, .. } => {
                assert!(success);
            }
            _ => panic!("Expected system limits updated response"),
        }
    }

    #[test]
    fn test_risk_engine_stats() {
        let mut engine = RiskEngine::default();
        
        // Perform some validations
        for i in 1..=5 {
            let order = create_test_order(i, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
            let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
            let account = create_test_account(100);
            
            engine.validate_order(&account, &order, &trading_pair);
        }

        let stats = engine.get_risk_stats();
        assert_eq!(stats.total_validations, 5);
        assert_eq!(stats.failed_validations, 5); // All fail due to insufficient balance
        assert_eq!(stats.success_rate(), 0.0);
        assert!(stats.uptime_ns > 0);
    }

    #[test]
    fn test_risk_event_listener() {
        use std::sync::{Arc, Mutex};
        
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let listener = RiskEventListener::new(move |event| {
            events_clone.lock().unwrap().push(event.clone());
        });

        let mut engine = RiskEngine::default();
        engine.add_event_listener(Box::new(listener));

        // Trigger an event
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        let account = create_test_account(100);
        
        engine.validate_order(&account, &order, &trading_pair);

        // Check that event was published
        let stored_events = events.lock().unwrap();
        assert_eq!(stored_events.len(), 1);
    }
}
