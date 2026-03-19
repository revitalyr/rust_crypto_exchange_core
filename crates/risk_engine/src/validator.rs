//! Risk validator for comprehensive order validation.

use crypto_exchange_common::{
    assets::{Asset, TradingPair},
    order::{Order, OrderSide, OrderType},
    price::Price,
    Balance, ExchangeError, ExchangeResult, UserId,
};
use crypto_exchange_accounts::Account;
use crate::{
    checks::{BalanceChecker, OrderChecker, PositionChecker, RiskCheckResult},
    limits::{LimitsManager, LimitStats, PositionLimit, DailyTradingLimit, UserLimits},
};

/// Risk validator result
#[derive(Debug, Clone)]
pub struct RiskValidationResult {
    /// Whether validation passed
    pub passed: bool,
    /// List of failed checks
    pub failed_checks: Vec<String>,
    /// List of warnings
    pub warnings: Vec<String>,
    /// Additional details
    pub details: Option<String>,
}

impl RiskValidationResult {
    /// Creates a successful validation result
    pub fn success() -> Self {
        Self {
            passed: true,
            failed_checks: Vec::new(),
            warnings: Vec::new(),
            details: None,
        }
    }

    /// Creates a failed validation result
    pub fn failure(failed_checks: Vec<String>) -> Self {
        Self {
            passed: false,
            failed_checks,
            warnings: Vec::new(),
            details: None,
        }
    }

    /// Creates a validation result with warnings
    pub fn success_with_warnings(warnings: Vec<String>) -> Self {
        Self {
            passed: true,
            failed_checks: Vec::new(),
            warnings,
            details: None,
        }
    }

    /// Adds a warning to the result
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Adds details to the result
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Returns an ExchangeError if validation failed
    pub fn to_error(self) -> ExchangeResult<()> {
        if self.passed {
            Ok(())
        } else {
            Err(ExchangeError::risk_check_failed(
                self.failed_checks.join("; ")
            ))
        }
    }
}

/// Market data provider interface
pub trait MarketDataProvider: Send + Sync {
    /// Gets the current market price for a trading pair
    fn get_market_price(&self, pair: &TradingPair) -> Option<Price>;
    
    /// Gets the best bid price
    fn get_best_bid(&self, pair: &TradingPair) -> Option<Price>;
    
    /// Gets the best ask price
    fn get_best_ask(&self, pair: &TradingPair) -> Option<Price>;
    
    /// Gets the 24h volume for a trading pair
    fn get_24h_volume(&self, pair: &TradingPair) -> Option<Balance>;
    
    /// Gets the current price volatility
    fn get_volatility(&self, pair: &TradingPair) -> Option<f64>;
}

/// Mock market data provider for testing
pub struct MockMarketDataProvider {
    market_prices: std::collections::HashMap<String, Price>,
    best_bids: std::collections::HashMap<String, Price>,
    best_asks: std::collections::HashMap<String, Price>,
    volumes: std::collections::HashMap<String, Balance>,
    volatilities: std::collections::HashMap<String, f64>,
}

impl MockMarketDataProvider {
    /// Creates a new mock market data provider
    pub fn new() -> Self {
        Self {
            market_prices: std::collections::HashMap::new(),
            best_bids: std::collections::HashMap::new(),
            best_asks: std::collections::HashMap::new(),
            volumes: std::collections::HashMap::new(),
            volatilities: std::collections::HashMap::new(),
        }
    }

    /// Sets the market price for a pair
    pub fn set_market_price(&mut self, pair: &str, price: Price) {
        self.market_prices.insert(pair.to_string(), price);
    }

    /// Sets the best bid for a pair
    pub fn set_best_bid(&mut self, pair: &str, bid: Price) {
        self.best_bids.insert(pair.to_string(), bid);
    }

    /// Sets the best ask for a pair
    pub fn set_best_ask(&mut self, pair: &str, ask: Price) {
        self.best_asks.insert(pair.to_string(), ask);
    }

    /// Sets the 24h volume for a pair
    pub fn set_24h_volume(&mut self, pair: &str, volume: Balance) {
        self.volumes.insert(pair.to_string(), volume);
    }

    /// Sets the volatility for a pair
    pub fn set_volatility(&mut self, pair: &str, volatility: f64) {
        self.volatilities.insert(pair.to_string(), volatility);
    }
}

impl MarketDataProvider for MockMarketDataProvider {
    fn get_market_price(&self, pair: &TradingPair) -> Option<Price> {
        self.market_prices.get(&pair.to_string()).copied()
    }

    fn get_best_bid(&self, pair: &TradingPair) -> Option<Price> {
        self.best_bids.get(&pair.to_string()).copied()
    }

    fn get_best_ask(&self, pair: &TradingPair) -> Option<Price> {
        self.best_asks.get(&pair.to_string()).copied()
    }

    fn get_24h_volume(&self, pair: &TradingPair) -> Option<Balance> {
        self.volumes.get(&pair.to_string()).copied()
    }

    fn get_volatility(&self, pair: &TradingPair) -> Option<f64> {
        self.volatilities.get(&pair.to_string()).copied()
    }
}

impl Default for MockMarketDataProvider {
    fn default() -> Self {
        let mut provider = Self::new();
        // Set some default values
        provider.set_market_price("BTC/USDT", Price::new(50000));
        provider.set_best_bid("BTC/USDT", Price::new(49900));
        provider.set_best_ask("BTC/USDT", Price::new(50100));
        provider.set_24h_volume("BTC/USDT", 1_000_000_000); // 10,000 USDT
        provider.set_volatility("BTC/USDT", 0.02); // 2%
        provider
    }
}

/// Comprehensive risk validator
pub struct RiskValidator {
    /// Limits manager
    limits_manager: LimitsManager,
    /// Market data provider
    market_data: Box<dyn MarketDataProvider>,
    /// Maximum price deviation for market orders (in basis points)
    max_market_price_deviation_bps: u32,
    /// Enable volatility-based risk checks
    enable_volatility_checks: bool,
    /// Maximum allowed volatility
    max_volatility: f64,
}

impl RiskValidator {
    /// Creates a new risk validator
    pub fn new(
        limits_manager: LimitsManager,
        market_data: Box<dyn MarketDataProvider>,
        max_market_price_deviation_bps: u32,
        enable_volatility_checks: bool,
        max_volatility: f64,
    ) -> Self {
        Self {
            limits_manager,
            market_data,
            max_market_price_deviation_bps,
            enable_volatility_checks,
            max_volatility,
        }
    }

    /// Validates an order comprehensively
    pub fn validate_order(
        &self,
        account: &Account,
        order: &Order,
        trading_pair: &TradingPair,
        current_stats: &LimitStats,
    ) -> RiskValidationResult {
        let mut failed_checks = Vec::new();
        let mut warnings = Vec::new();

        // 1. Basic order validation
        if let Err(e) = self.validate_basic_order(order) {
            failed_checks.push(e.to_string());
        }

        // 2. Balance check
        let balance_check = if order.order_type == OrderType::Market {
            let estimated_price = self.estimate_market_price(order, trading_pair);
            BalanceChecker::check_sufficient_market_balance(account, order, trading_pair, estimated_price)
        } else {
            BalanceChecker::check_sufficient_balance(account, order, trading_pair)
        };

        if !balance_check.passed {
            failed_checks.push(balance_check.reason.unwrap_or_else(|| "Balance check failed".to_string()));
        }

        // 3. Position limits check
        let user_limits = self.limits_manager.get_user_limits(account.user_id());
        if let Some(position_limit) = user_limits.get_position_limit(&trading_pair.base) {
            let current_position = account.get_position(&trading_pair.base);
            let new_position = PositionChecker::calculate_new_position(current_position, order);

            if !position_limit.is_position_within_limits(new_position) {
                failed_checks.push("Position limit exceeded".to_string());
            }
        }

        // 4. Daily trading limits check
        if let Some(daily_limit) = user_limits.get_daily_limit(trading_pair) {
            let order_volume = self.calculate_order_volume(order, trading_pair);

            if !daily_limit.is_volume_within_limit(current_stats.daily_volume, order_volume) {
                failed_checks.push("Daily trading volume limit exceeded".to_string());
            }

            if !daily_limit.is_trade_count_within_limit(current_stats.daily_trades) {
                failed_checks.push("Daily trade count limit exceeded".to_string());
            }
        }

        // 5. Order size limits check
        if let Err(e) = self.validate_order_size(order, user_limits) {
            failed_checks.push(e.to_string());
        }

        // 6. Price reasonableness check
        if order.order_type == OrderType::Limit {
            if let Err(e) = self.validate_price_reasonableness(order, trading_pair) {
                failed_checks.push(e.to_string());
            }
        }

        // 7. Market order specific checks
        if order.order_type == OrderType::Market {
            if let Err(e) = self.validate_market_order(order, trading_pair) {
                failed_checks.push(e.to_string());
            }
        }

        // 8. Volatility check (if enabled)
        if self.enable_volatility_checks {
            if let Some(warning) = self.check_volatility(trading_pair) {
                warnings.push(warning);
            }
        }

        // 9. System limits check
        if let Err(e) = self.validate_system_limits(order, current_stats) {
            failed_checks.push(e.to_string());
        }

        // 10. Rate limiting check
        if let Err(e) = self.validate_rate_limits(account.user_id(), current_stats) {
            failed_checks.push(e.to_string());
        }

        if failed_checks.is_empty() {
            if warnings.is_empty() {
                RiskValidationResult::success()
            } else {
                RiskValidationResult::success_with_warnings(warnings)
            }
        } else {
            RiskValidationResult::failure(failed_checks)
        }
    }

    /// Validates basic order parameters
    fn validate_basic_order(&self, order: &Order) -> ExchangeResult<()> {
        if order.id == 0 {
            return Err(ExchangeError::invalid_order("Order ID cannot be zero"));
        }

        if order.user_id == 0 {
            return Err(ExchangeError::invalid_order("User ID cannot be zero"));
        }

        if order.quantity == 0 {
            return Err(ExchangeError::invalid_quantity(0));
        }

        if order.pair.is_empty() {
            return Err(ExchangeError::invalid_order("Trading pair cannot be empty"));
        }

        Ok(())
    }

    /// Validates order size against limits
    fn validate_order_size(&self, order: &Order, user_limits: &UserLimits) -> ExchangeResult<()> {
        let system_limits = self.limits_manager.get_system_limits();

        if !system_limits.is_order_size_within_system_limits(order.quantity) {
            return Err(ExchangeError::invalid_quantity(order.quantity));
        }

        // Check per-trade limits
        if let Some(daily_limit) = user_limits.daily_limits.values().next() {
            let order_volume = order.price.unwrap_or(Price::new(0)).value().saturating_mul(order.quantity);
            if !daily_limit.is_order_size_within_limit(order_volume) {
                return Err(ExchangeError::invalid_quantity(order.quantity));
            }
        }

        Ok(())
    }

    /// Validates price reasonableness for limit orders
    fn validate_price_reasonableness(&self, order: &Order, trading_pair: &TradingPair) -> ExchangeResult<()> {
        let order_price = order.price.ok_or_else(|| {
            ExchangeError::invalid_order("Limit order must have a price")
        })?;

        let market_price = self.market_data.get_market_price(trading_pair)
            .ok_or_else(|| {
                ExchangeError::system_error("No market price available for validation".to_string())
            })?;

        let deviation_bps = OrderChecker::calculate_price_deviation_bps(order_price, market_price, order.side);
        let max_deviation = self.limits_manager.get_system_limits().max_price_deviation_bps;

        if deviation_bps > max_deviation {
            return Err(ExchangeError::risk_check_failed(
                format!("Price deviation {} bps exceeds maximum {}", deviation_bps, max_deviation)
            ));
        }

        Ok(())
    }

    /// Validates market order specific requirements
    fn validate_market_order(&self, order: &Order, trading_pair: &TradingPair) -> ExchangeResult<()> {
        if order.price.is_some() {
            return Err(ExchangeError::invalid_order("Market order should not have a price"));
        }

        // Check if there's sufficient liquidity
        let best_bid = self.market_data.get_best_bid(trading_pair);
        let best_ask = self.market_data.get_best_ask(trading_pair);

        match order.side {
            OrderSide::Buy => {
                if best_ask.is_none() {
                    return Err(ExchangeError::market_order_error("No ask liquidity available".to_string()));
                }
            }
            OrderSide::Sell => {
                if best_bid.is_none() {
                    return Err(ExchangeError::market_order_error("No bid liquidity available".to_string()));
                }
            }
        }

        Ok(())
    }

    /// Validates against system-wide limits
    fn validate_system_limits(&self, order: &Order, current_stats: &LimitStats) -> ExchangeResult<()> {
        let system_limits = self.limits_manager.get_system_limits();

        if !system_limits.is_total_open_orders_within_limit(current_stats.total_open_orders) {
            return Err(ExchangeError::system_error("System open order limit exceeded".to_string()));
        }

        if !system_limits.is_order_book_depth_within_limit(current_stats.order_book_depth) {
            return Err(ExchangeError::system_error("Order book depth limit exceeded".to_string()));
        }

        Ok(())
    }

    /// Validates rate limits
    fn validate_rate_limits(&self, user_id: UserId, current_stats: &LimitStats) -> ExchangeResult<()> {
        let user_limits = self.limits_manager.get_user_limits(user_id);

        if !user_limits.can_open_more_orders(current_stats.open_orders) {
            return Err(ExchangeError::risk_check_failed("User open order limit exceeded".to_string()));
        }

        Ok(())
    }

    /// Checks volatility and returns warning if high
    fn check_volatility(&self, trading_pair: &TradingPair) -> Option<String> {
        if let Some(volatility) = self.market_data.get_volatility(trading_pair) {
            if volatility > self.max_volatility {
                Some(format!("High volatility detected: {:.2}%", volatility * 100.0))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Estimates market price for market orders
    fn estimate_market_price(&self, order: &Order, trading_pair: &TradingPair) -> Option<Price> {
        match order.side {
            OrderSide::Buy => self.market_data.get_best_ask(trading_pair),
            OrderSide::Sell => self.market_data.get_best_bid(trading_pair),
        }
    }

    /// Calculates order volume in quote currency
    fn calculate_order_volume(&self, order: &Order, trading_pair: &TradingPair) -> Balance {
        if let Some(price) = order.price {
            price.value().saturating_mul(order.quantity)
        } else {
            // For market orders, estimate using current market price
            self.market_data.get_market_price(trading_pair)
                .map(|price| price.value().saturating_mul(order.quantity))
                .unwrap_or(0)
        }
    }

    /// Gets the limits manager
    pub fn get_limits_manager(&self) -> &LimitsManager {
        &self.limits_manager
    }

    /// Gets mutable reference to limits manager
    pub fn get_limits_manager_mut(&mut self) -> &mut LimitsManager {
        &mut self.limits_manager
    }
}

/// Risk validator builder for convenient configuration
pub struct RiskValidatorBuilder {
    limits_manager: LimitsManager,
    market_data: Box<dyn MarketDataProvider>,
    max_market_price_deviation_bps: u32,
    enable_volatility_checks: bool,
    max_volatility: f64,
}

impl RiskValidatorBuilder {
    /// Creates a new builder
    pub fn new() -> Self {
        Self {
            limits_manager: LimitsManager::new(
                crate::limits::SystemLimits::default(),
                crate::limits::UserLimits::new(
                    std::collections::HashMap::new(),
                    std::collections::HashMap::new(),
                    crate::limits::RateLimit::default(),
                    100, // max open orders
                    50,  // max cancels per minute
                ),
            ),
            market_data: Box::new(MockMarketDataProvider::default()),
            max_market_price_deviation_bps: 1000, // 10%
            enable_volatility_checks: true,
            max_volatility: 0.05, // 5%
        }
    }

    /// Sets the limits manager
    pub fn with_limits_manager(mut self, limits_manager: LimitsManager) -> Self {
        self.limits_manager = limits_manager;
        self
    }

    /// Sets the market data provider
    pub fn with_market_data(mut self, market_data: Box<dyn MarketDataProvider>) -> Self {
        self.market_data = market_data;
        self
    }

    /// Sets the maximum market price deviation
    pub fn with_max_market_price_deviation(mut self, max_deviation_bps: u32) -> Self {
        self.max_market_price_deviation_bps = max_deviation_bps;
        self
    }

    /// Enables or disables volatility checks
    pub fn with_volatility_checks(mut self, enable: bool) -> Self {
        self.enable_volatility_checks = enable;
        self
    }

    /// Sets the maximum allowed volatility
    pub fn with_max_volatility(mut self, max_volatility: f64) -> Self {
        self.max_volatility = max_volatility;
        self
    }

    /// Builds the risk validator
    pub fn build(self) -> RiskValidator {
        RiskValidator::new(
            self.limits_manager,
            self.market_data,
            self.max_market_price_deviation_bps,
            self.enable_volatility_checks,
            self.max_volatility,
        )
    }
}

impl Default for RiskValidatorBuilder {
    fn default() -> Self {
        Self::new()
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
    fn test_risk_validator_basic() {
        let validator = RiskValidatorBuilder::new().build();
        let account = create_test_account(100);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let stats = LimitStats::default();

        let result = validator.validate_order(&account, &order, &trading_pair, &stats);
        
        // Should fail due to insufficient balance (account has no balance)
        assert!(!result.passed);
        assert!(result.failed_checks.iter().any(|check| check.contains("Insufficient")));
    }

    #[test]
    fn test_risk_validator_with_balance() {
        let validator = RiskValidatorBuilder::new().build();
        let mut account = create_test_account(100);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        
        // Set up sufficient balance
        account.set_balance(&Asset::USDT, 100_000_000); // 1000 USDT
        
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000); // 50 USDT
        let stats = LimitStats::default();

        let result = validator.validate_order(&account, &order, &trading_pair, &stats);
        
        // Should pass
        assert!(result.passed);
        assert!(result.failed_checks.is_empty());
    }

    #[test]
    fn test_risk_validator_invalid_order() {
        let validator = RiskValidatorBuilder::new().build();
        let account = create_test_account(100);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        
        // Invalid order (zero quantity)
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 0);
        let stats = LimitStats::default();

        let result = validator.validate_order(&account, &order, &trading_pair, &stats);
        
        assert!(!result.passed);
        assert!(result.failed_checks.iter().any(|check| check.contains("quantity")));
    }

    #[test]
    fn test_risk_validator_market_order() {
        let validator = RiskValidatorBuilder::new().build();
        let mut account = create_test_account(100);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        
        // Set up sufficient balance
        account.set_balance(&Asset::USDT, 100_000_000); // 1000 USDT
        
        let order = create_test_order(1, OrderSide::Buy, OrderType::Market, None, 1000);
        let stats = LimitStats::default();

        let result = validator.validate_order(&account, &order, &trading_pair, &stats);
        
        // Should pass (market data provider has default liquidity)
        assert!(result.passed);
    }

    #[test]
    fn test_risk_validator_volatility_warning() {
        let mut market_data = MockMarketDataProvider::new();
        market_data.set_volatility("BTC/USDT", 0.10); // 10% volatility
        
        let validator = RiskValidatorBuilder::new()
            .with_market_data(Box::new(market_data))
            .with_max_volatility(0.05) // 5% max
            .build();
        
        let mut account = create_test_account(100);
        account.set_balance(&Asset::USDT, 100_000_000);
        
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let stats = LimitStats::default();

        let result = validator.validate_order(&account, &order, &trading_pair, &stats);
        
        // Should pass but with volatility warning
        assert!(result.passed);
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|warning| warning.contains("volatility")));
    }

    #[test]
    fn test_mock_market_data_provider() {
        let mut provider = MockMarketDataProvider::new();
        let pair = TradingPair::new(Asset::BTC, Asset::USDT);
        
        provider.set_market_price("BTC/USDT", Price::new(50000));
        provider.set_best_bid("BTC/USDT", Price::new(49900));
        provider.set_best_ask("BTC/USDT", Price::new(50100));
        
        assert_eq!(provider.get_market_price(&pair), Some(Price::new(50000)));
        assert_eq!(provider.get_best_bid(&pair), Some(Price::new(49900)));
        assert_eq!(provider.get_best_ask(&pair), Some(Price::new(50100)));
    }
}
