//! Risk checks and validation functions.

use crypto_exchange_common::{
    assets::{Asset, TradingPair},
    order::{Order, OrderSide, OrderType},
    price::Price,
    Balance, ExchangeError, ExchangeResult, UserId,
};
use crypto_exchange_accounts::{Account, Wallet};

/// Risk check result
#[derive(Debug, Clone)]
pub struct RiskCheckResult {
    /// Whether the check passed
    pub passed: bool,
    /// Reason for failure (if any)
    pub reason: Option<String>,
    /// Additional details
    pub details: Option<String>,
}

impl RiskCheckResult {
    /// Creates a successful result
    pub fn success() -> Self {
        Self {
            passed: true,
            reason: None,
            details: None,
        }
    }

    /// Creates a failed result
    pub fn failure(reason: String) -> Self {
        Self {
            passed: false,
            reason: Some(reason.clone()),
            details: Some(reason),
        }
    }

    /// Creates a failed result with details
    pub fn failure_with_details(reason: String, details: String) -> Self {
        Self {
            passed: false,
            reason: Some(reason.clone()),
            details: Some(details),
        }
    }

    /// Returns an ExchangeError if the check failed
    pub fn to_error(self) -> ExchangeResult<()> {
        if self.passed {
            Ok(())
        } else {
            Err(ExchangeError::risk_check_failed(
                self.reason.unwrap_or_else(|| "Risk check failed".to_string())
            ))
        }
    }
}

/// Balance checker for validating sufficient funds
pub struct BalanceChecker;

impl BalanceChecker {
    /// Checks if user has sufficient balance for an order
    pub fn check_sufficient_balance(
        account: &Account,
        order: &Order,
        trading_pair: &TradingPair,
    ) -> RiskCheckResult {
        match order.side {
            OrderSide::Buy => {
                // For buy orders, check quote currency balance
                let required_balance = Self::calculate_required_buy_balance(order);
                let available_balance = account.get_available_balance(&trading_pair.quote);
                
                if available_balance >= required_balance {
                    RiskCheckResult::success()
                } else {
                    RiskCheckResult::failure_with_details(
                        "Insufficient quote currency balance".to_string(),
                        format!(
                            "Required: {}, Available: {}",
                            Self::format_balance(required_balance, trading_pair.quote.decimals()),
                            Self::format_balance(available_balance, trading_pair.quote.decimals())
                        )
                    )
                }
            }
            OrderSide::Sell => {
                // For sell orders, check base currency balance
                let required_balance = order.quantity;
                let available_balance = account.get_available_balance(&trading_pair.base);
                
                if available_balance >= required_balance {
                    RiskCheckResult::success()
                } else {
                    RiskCheckResult::failure_with_details(
                        "Insufficient base currency balance".to_string(),
                        format!(
                            "Required: {}, Available: {}",
                            Self::format_balance(required_balance, trading_pair.base.decimals()),
                            Self::format_balance(available_balance, trading_pair.base.decimals())
                        )
                    )
                }
            }
        }
    }

    /// Checks if user has sufficient balance for a market order
    pub fn check_sufficient_market_balance(
        account: &Account,
        order: &Order,
        trading_pair: &TradingPair,
        estimated_price: Option<Price>,
    ) -> RiskCheckResult {
        if order.order_type != OrderType::Market {
            return RiskCheckResult::failure("Not a market order".to_string());
        }

        match order.side {
            OrderSide::Buy => {
                // For market buy orders, we need an estimated price
                let estimated_price = estimated_price.ok_or_else(|| {
                    RiskCheckResult::failure("No price estimate available for market buy order".to_string())
                })?;

                let required_balance = estimated_price.value().checked_mul(order.quantity)
                    .ok_or_else(|| {
                        RiskCheckResult::failure("Balance calculation overflow".to_string())
                    })?;

                let available_balance = account.get_available_balance(&trading_pair.quote);
                
                if available_balance >= required_balance {
                    RiskCheckResult::success()
                } else {
                    RiskCheckResult::failure_with_details(
                        "Insufficient quote currency balance for market order".to_string(),
                        format!(
                            "Estimated required: {}, Available: {}",
                            Self::format_balance(required_balance, trading_pair.quote.decimals()),
                            Self::format_balance(available_balance, trading_pair.quote.decimals())
                        )
                    )
                }
            }
            OrderSide::Sell => {
                // For market sell orders, just check base currency balance
                let required_balance = order.quantity;
                let available_balance = account.get_available_balance(&trading_pair.base);
                
                if available_balance >= required_balance {
                    RiskCheckResult::success()
                } else {
                    RiskCheckResult::failure_with_details(
                        "Insufficient base currency balance for market order".to_string(),
                        format!(
                            "Required: {}, Available: {}",
                            Self::format_balance(required_balance, trading_pair.base.decimals()),
                            Self::format_balance(available_balance, trading_pair.base.decimals())
                        )
                    )
                }
            }
        }
    }

    /// Calculates required balance for a buy order
    pub fn calculate_required_buy_balance(order: &Order) -> Balance {
        if let Some(price) = order.price {
            price.value().checked_mul(order.quantity).unwrap_or(0)
        } else {
            0 // Market orders require price estimation
        }
    }

    /// Formats balance for display
    fn format_balance(balance: Balance, decimals: u8) -> String {
        let divisor = 10_f64.powi(decimals as i32);
        format!("{:.precision$}", balance as f64 / divisor, precision = decimals as usize)
    }
}

/// Position checker for validating position limits
pub struct PositionChecker;

impl PositionChecker {
    /// Checks if order would exceed position limits
    pub fn check_position_limits(
        account: &Account,
        order: &Order,
        trading_pair: &TradingPair,
        max_position_size: Balance,
    ) -> RiskCheckResult {
        let current_position = account.get_position(&trading_pair.base);
        let new_position = Self::calculate_new_position(current_position, order);

        if new_position.abs() <= max_position_size as i128 {
            RiskCheckResult::success()
        } else {
            RiskCheckResult::failure_with_details(
                "Position size limit exceeded".to_string(),
                format!(
                    "Current: {}, New: {}, Limit: {}",
                    Self::format_balance(current_position.abs() as Balance, trading_pair.base.decimals()),
                    Self::format_balance(new_position.abs() as Balance, trading_pair.base.decimals()),
                    Self::format_balance(max_position_size, trading_pair.base.decimals())
                )
            )
        }
    }

    /// Checks if order would exceed daily trading limits
    pub fn check_daily_trading_limits(
        account: &Account,
        order: &Order,
        trading_pair: &TradingPair,
        daily_volume_limit: Balance,
        current_daily_volume: Balance,
    ) -> RiskCheckResult {
        let order_volume = if let Some(price) = order.price {
            price.value().checked_mul(order.quantity).unwrap_or(0)
        } else {
            0 // Market orders would need price estimation
        };

        let new_daily_volume = current_daily_volume.checked_add(order_volume).unwrap_or(0);

        if new_daily_volume <= daily_volume_limit {
            RiskCheckResult::success()
        } else {
            RiskCheckResult::failure_with_details(
                "Daily trading volume limit exceeded".to_string(),
                format!(
                    "Current: {}, Order: {}, New: {}, Limit: {}",
                    Self::format_balance(current_daily_volume, trading_pair.quote.decimals()),
                    Self::format_balance(order_volume, trading_pair.quote.decimals()),
                    Self::format_balance(new_daily_volume, trading_pair.quote.decimals()),
                    Self::format_balance(daily_volume_limit, trading_pair.quote.decimals())
                )
            )
        }
    }

    /// Calculates new position after order execution
    pub fn calculate_new_position(current_position: Balance, order: &Order) -> i128 {
        match order.side {
            OrderSide::Buy => current_position as i128 + order.quantity as i128,
            OrderSide::Sell => current_position as i128 - order.quantity as i128,
        }
    }

    /// Formats balance for display
    fn format_balance(balance: Balance, decimals: u8) -> String {
        let divisor = 10_f64.powi(decimals as i32);
        format!("{:.precision$}", balance as f64 / divisor, precision = decimals as usize)
    }
}

/// Order checker for validating order parameters
pub struct OrderChecker;

impl OrderChecker {
    /// Checks if order size is within allowed limits
    pub fn check_order_size_limits(
        order: &Order,
        min_order_size: Balance,
        max_order_size: Balance,
    ) -> RiskCheckResult {
        if order.quantity < min_order_size {
            return RiskCheckResult::failure_with_details(
                "Order size below minimum".to_string(),
                format!(
                    "Order: {}, Minimum: {}",
                    order.quantity,
                    min_order_size
                )
            );
        }

        if order.quantity > max_order_size {
            return RiskCheckResult::failure_with_details(
                "Order size above maximum".to_string(),
                format!(
                    "Order: {}, Maximum: {}",
                    order.quantity,
                    max_order_size
                )
            );
        }

        RiskCheckResult::success()
    }

    /// Checks if order price is reasonable
    pub fn check_price_reasonableness(
        order: &Order,
        current_market_price: Option<Price>,
        max_price_deviation_bps: u32,
    ) -> RiskCheckResult {
        if order.order_type != OrderType::Limit {
            return RiskCheckResult::success(); // Market orders don't have price limits
        }

        let order_price = order.price.ok_or_else(|| {
            RiskCheckResult::failure("Limit order must have a price".to_string())
        })?;

        let Some(market_price) = current_market_price else {
            return RiskCheckResult::failure("No market price available for validation".to_string());
        };

        let deviation_bps = Self::calculate_price_deviation_bps(order_price, market_price, order.side);

        if deviation_bps <= max_price_deviation_bps {
            RiskCheckResult::success()
        } else {
            RiskCheckResult::failure_with_details(
                "Order price deviates too much from market price".to_string(),
                format!(
                    "Deviation: {} bps, Maximum: {} bps",
                    deviation_bps,
                    max_price_deviation_bps
                )
            )
        }
    }

    /// Calculates price deviation in basis points
    pub fn calculate_price_deviation_bps(
        order_price: Price,
        market_price: Price,
        side: OrderSide,
    ) -> u32 {
        let order_value = order_price.value();
        let market_value = market_price.value();

        match side {
            OrderSide::Buy => {
                // For buy orders, check if order price is significantly above market
                if order_value > market_value {
                    ((order_value - market_value) * 10000) / market_value
                } else {
                    0
                }
            }
            OrderSide::Sell => {
                // For sell orders, check if order price is significantly below market
                if order_value < market_value {
                    ((market_value - order_value) * 10000) / market_value
                } else {
                    0
                }
            }
        }
    }

    /// Checks if order frequency is within limits
    pub fn check_order_frequency_limits(
        user_id: UserId,
        current_time: u64,
        order_count: u64,
        max_orders_per_second: u64,
        last_order_time: u64,
    ) -> RiskCheckResult {
        // Simple rate limiting check
        if current_time > last_order_time {
            let time_diff = current_time - last_order_time;
            let min_time_between_orders = 1_000_000_000 / max_orders_per_second; // nanoseconds

            if time_diff < min_time_between_orders {
                return RiskCheckResult::failure_with_details(
                    "Order frequency limit exceeded".to_string(),
                    format!(
                        "Minimum time between orders: {} ns, Actual: {} ns",
                        min_time_between_orders,
                        time_diff
                    )
                );
            }
        }

        RiskCheckResult::success()
    }
}

/// Comprehensive risk checker that combines all checks
pub struct ComprehensiveRiskChecker {
    /// Maximum position size per user
    pub max_position_size: Balance,
    /// Daily trading volume limit per user
    pub daily_volume_limit: Balance,
    /// Minimum order size
    pub min_order_size: Balance,
    /// Maximum order size
    pub max_order_size: Balance,
    /// Maximum price deviation in basis points
    pub max_price_deviation_bps: u32,
    /// Maximum orders per second
    pub max_orders_per_second: u64,
}

impl ComprehensiveRiskChecker {
    /// Creates a new comprehensive risk checker
    pub fn new(
        max_position_size: Balance,
        daily_volume_limit: Balance,
        min_order_size: Balance,
        max_order_size: Balance,
        max_price_deviation_bps: u32,
        max_orders_per_second: u64,
    ) -> Self {
        Self {
            max_position_size,
            daily_volume_limit,
            min_order_size,
            max_order_size,
            max_price_deviation_bps,
            max_orders_per_second,
        }
    }

    /// Performs comprehensive risk check for an order
    pub fn check_order(
        &self,
        account: &Account,
        order: &Order,
        trading_pair: &TradingPair,
        current_market_price: Option<Price>,
        current_daily_volume: Balance,
        current_time: u64,
        last_order_time: u64,
    ) -> Vec<RiskCheckResult> {
        let mut results = Vec::new();

        // 1. Balance check
        if order.order_type == OrderType::Market {
            results.push(BalanceChecker::check_sufficient_market_balance(
                account,
                order,
                trading_pair,
                current_market_price,
            ));
        } else {
            results.push(BalanceChecker::check_sufficient_balance(
                account,
                order,
                trading_pair,
            ));
        }

        // 2. Position limits check
        results.push(PositionChecker::check_position_limits(
            account,
            order,
            trading_pair,
            self.max_position_size,
        ));

        // 3. Daily trading limits check
        results.push(PositionChecker::check_daily_trading_limits(
            account,
            order,
            trading_pair,
            self.daily_volume_limit,
            current_daily_volume,
        ));

        // 4. Order size limits check
        results.push(OrderChecker::check_order_size_limits(
            order,
            self.min_order_size,
            self.max_order_size,
        ));

        // 5. Price reasonableness check
        results.push(OrderChecker::check_price_reasonableness(
            order,
            current_market_price,
            self.max_price_deviation_bps,
        ));

        // 6. Order frequency check
        results.push(OrderChecker::check_order_frequency_limits(
            account.user_id(),
            current_time,
            0, // Would need to track actual order count
            self.max_orders_per_second,
            last_order_time,
        ));

        results
    }

    /// Checks if all risk checks passed
    pub fn all_checks_passed(&self, results: &[RiskCheckResult]) -> bool {
        results.iter().all(|result| result.passed)
    }

    /// Gets the reasons for any failed checks
    pub fn get_failure_reasons(&self, results: &[RiskCheckResult]) -> Vec<String> {
        results
            .iter()
            .filter(|result| !result.passed)
            .filter_map(|result| result.reason.clone())
            .collect()
    }
}

impl Default for ComprehensiveRiskChecker {
    fn default() -> Self {
        Self::new(
            1_000_000_000,    // max_position_size (10 BTC)
            10_000_000_000,   // daily_volume_limit (100M USDT)
            1,                // min_order_size
            1_000_000,        // max_order_size (10 BTC)
            1000,             // max_price_deviation_bps (10%)
            10,               // max_orders_per_second
        )
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
    fn test_balance_checker() {
        let mut account = create_test_account(100);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        
        // Set up some balances
        account.set_balance(&Asset::BTC, 1_000_000); // 0.01 BTC
        account.set_balance(&Asset::USDT, 50_000_000); // 500 USDT

        // Test buy order
        let buy_order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000); // 0.00001 BTC
        let result = BalanceChecker::check_sufficient_balance(&account, &buy_order, &trading_pair);
        assert!(result.passed);

        // Test sell order
        let sell_order = create_test_order(2, OrderSide::Sell, OrderType::Limit, Some(50000), 500); // 0.000005 BTC
        let result = BalanceChecker::check_sufficient_balance(&account, &sell_order, &trading_pair);
        assert!(result.passed);

        // Test insufficient balance
        let large_buy_order = create_test_order(3, OrderSide::Buy, OrderType::Limit, Some(50000), 2000); // 0.00002 BTC = 1000 USDT
        let result = BalanceChecker::check_sufficient_balance(&account, &large_buy_order, &trading_pair);
        assert!(!result.passed);
        assert!(result.reason.unwrap().contains("Insufficient"));
    }

    #[test]
    fn test_position_checker() {
        let mut account = create_test_account(100);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        
        // Set up initial position
        account.set_position(&Asset::BTC, 500_000); // 0.005 BTC

        // Test buy order within limits
        let buy_order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let result = PositionChecker::check_position_limits(&account, &buy_order, &trading_pair, 2_000_000);
        assert!(result.passed); // New position: 0.005 + 0.00001 = 0.00501 BTC < 0.02 BTC

        // Test sell order within limits
        let sell_order = create_test_order(2, OrderSide::Sell, OrderType::Limit, Some(50000), 1000);
        let result = PositionChecker::check_position_limits(&account, &sell_order, &trading_pair, 2_000_000);
        assert!(result.passed); // New position: 0.005 - 0.00001 = 0.00499 BTC < 0.02 BTC

        // Test order that would exceed limits
        let large_sell_order = create_test_order(3, OrderSide::Sell, OrderType::Limit, Some(50000), 600_000);
        let result = PositionChecker::check_position_limits(&account, &large_sell_order, &trading_pair, 2_000_000);
        assert!(!result.passed); // New position: 0.005 - 0.006 = -0.001 BTC, absolute > 0.02 BTC
    }

    #[test]
    fn test_order_checker() {
        // Test order size limits
        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let result = OrderChecker::check_order_size_limits(&order, 100, 1_000_000);
        assert!(result.passed);

        let small_order = create_test_order(2, OrderSide::Buy, OrderType::Limit, Some(50000), 50);
        let result = OrderChecker::check_order_size_limits(&small_order, 100, 1_000_000);
        assert!(!result.passed);

        let large_order = create_test_order(3, OrderSide::Buy, OrderType::Limit, Some(50000), 2_000_000);
        let result = OrderChecker::check_order_size_limits(&large_order, 100, 1_000_000);
        assert!(!result.passed);

        // Test price reasonableness
        let reasonable_order = create_test_order(4, OrderSide::Buy, OrderType::Limit, Some(50500), 1000);
        let result = OrderChecker::check_price_reasonableness(&reasonable_order, Some(Price::new(50000)), 1000);
        assert!(result.passed); // 1% deviation, within 10% limit

        let unreasonable_order = create_test_order(5, OrderSide::Buy, OrderType::Limit, Some(60000), 1000);
        let result = OrderChecker::check_price_reasonableness(&unreasonable_order, Some(Price::new(50000)), 1000);
        assert!(!result.passed); // 20% deviation, exceeds 10% limit
    }

    #[test]
    fn test_comprehensive_risk_checker() {
        let checker = ComprehensiveRiskChecker::default();
        let mut account = create_test_account(100);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        
        // Set up balances
        account.set_balance(&Asset::BTC, 1_000_000); // 0.01 BTC
        account.set_balance(&Asset::USDT, 50_000_000); // 500 USDT

        let order = create_test_order(1, OrderSide::Buy, OrderType::Limit, Some(50000), 1000);
        let results = checker.check_order(
            &account,
            &order,
            &trading_pair,
            Some(Price::new(50000)),
            0,
            1234567890,
            1234567880,
        );

        assert!(checker.all_checks_passed(&results));
        assert!(checker.get_failure_reasons(&results).is_empty());

        // Test with insufficient balance
        let large_order = create_test_order(2, OrderSide::Buy, OrderType::Limit, Some(50000), 2000);
        let results = checker.check_order(
            &account,
            &large_order,
            &trading_pair,
            Some(Price::new(50000)),
            0,
            1234567890,
            1234567880,
        );

        assert!(!checker.all_checks_passed(&results));
        let failure_reasons = checker.get_failure_reasons(&results);
        assert!(!failure_reasons.is_empty());
        assert!(failure_reasons.iter().any(|reason| reason.contains("Insufficient")));
    }
}
