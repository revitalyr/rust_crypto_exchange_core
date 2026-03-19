//! Risk limits and position management.

use crypto_exchange_common::{
    assets::{Asset, TradingPair},
    order::{Order, OrderSide, OrderType},
    price::Price,
    Balance, ExchangeError, ExchangeResult, UserId,
};
use crypto_exchange_accounts::Account;
use std::collections::HashMap;

/// Position limit configuration
#[derive(Debug, Clone)]
pub struct PositionLimit {
    /// Maximum position size (absolute value)
    pub max_position: Balance,
    /// Maximum long position
    pub max_long_position: Balance,
    /// Maximum short position
    pub max_short_position: Balance,
    /// Maximum leverage (as a multiplier, e.g., 10 for 10x)
    pub max_leverage: u32,
}

impl PositionLimit {
    /// Creates a new position limit
    pub fn new(max_position: Balance, max_leverage: u32) -> Self {
        Self {
            max_position,
            max_long_position: max_position,
            max_short_position: max_position,
            max_leverage,
        }
    }

    /// Creates a position limit with asymmetric long/short limits
    pub fn new_asymmetric(
        max_position: Balance,
        max_long_position: Balance,
        max_short_position: Balance,
        max_leverage: u32,
    ) -> Self {
        Self {
            max_position,
            max_long_position,
            max_short_position,
            max_leverage,
        }
    }

    /// Checks if a position is within limits
    pub fn is_position_within_limits(&self, position: i128) -> bool {
        let abs_position = position.abs();
        abs_position <= self.max_position as i128
    }

    /// Checks if a long position is within limits
    pub fn is_long_position_within_limits(&self, position: i128) -> bool {
        position >= 0 && position <= self.max_long_position as i128
    }

    /// Checks if a short position is within limits
    pub fn is_short_position_within_limits(&self, position: i128) -> bool {
        position <= 0 && position >= -(self.max_short_position as i128)
    }

    /// Calculates the maximum order size based on current position
    pub fn max_order_size(&self, current_position: i128, side: OrderSide) -> Balance {
        match side {
            OrderSide::Buy => {
                // For buy orders, check how much more we can buy
                let max_buyable = self.max_long_position as i128 - current_position;
                if max_buyable > 0 {
                    max_buyable as Balance
                } else {
                    0
                }
            }
            OrderSide::Sell => {
                // For sell orders, check how much more we can sell
                let max_sellable = current_position + self.max_short_position as i128;
                if max_sellable > 0 {
                    max_sellable as Balance
                } else {
                    0
                }
            }
        }
    }
}

impl Default for PositionLimit {
    fn default() -> Self {
        Self::new(1_000_000_000, 10) // 10 BTC position, 10x leverage
    }
}

/// Daily trading limit configuration
#[derive(Debug, Clone)]
pub struct DailyTradingLimit {
    /// Maximum daily trading volume
    pub max_daily_volume: Balance,
    /// Maximum number of trades per day
    pub max_trades_per_day: u32,
    /// Maximum loss per day (in quote currency)
    pub max_daily_loss: Balance,
    /// Maximum order size per trade
    pub max_order_size_per_trade: Balance,
}

impl DailyTradingLimit {
    /// Creates a new daily trading limit
    pub fn new(
        max_daily_volume: Balance,
        max_trades_per_day: u32,
        max_daily_loss: Balance,
        max_order_size_per_trade: Balance,
    ) -> Self {
        Self {
            max_daily_volume,
            max_trades_per_day,
            max_daily_loss,
            max_order_size_per_trade,
        }
    }

    /// Checks if trading volume is within daily limit
    pub fn is_volume_within_limit(&self, current_volume: Balance, additional_volume: Balance) -> bool {
        current_volume.saturating_add(additional_volume) <= self.max_daily_volume
    }

    /// Checks if trade count is within daily limit
    pub fn is_trade_count_within_limit(&self, current_trades: u32) -> bool {
        current_trades < self.max_trades_per_day
    }

    /// Checks if daily loss is within limit
    pub fn is_loss_within_limit(&self, current_loss: Balance, additional_loss: Balance) -> bool {
        current_loss.saturating_add(additional_loss) <= self.max_daily_loss
    }

    /// Checks if order size is within per-trade limit
    pub fn is_order_size_within_limit(&self, order_size: Balance) -> bool {
        order_size <= self.max_order_size_per_trade
    }
}

impl Default for DailyTradingLimit {
    fn default() -> Self {
        Self::new(
            100_000_000_000, // 1000 USDT daily volume
            1000,            // 1000 trades per day
            10_000_000,      // 100 USDT max daily loss
            10_000_000,      // 100 USDT max per trade
        )
    }
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimit {
    /// Maximum requests per second
    pub max_requests_per_second: u64,
    /// Maximum requests per minute
    pub max_requests_per_minute: u64,
    /// Maximum requests per hour
    pub max_requests_per_hour: u64,
    /// Maximum orders per second
    pub max_orders_per_second: u64,
    /// Maximum orders per minute
    pub max_orders_per_minute: u64,
}

impl RateLimit {
    /// Creates a new rate limit
    pub fn new(
        max_requests_per_second: u64,
        max_requests_per_minute: u64,
        max_requests_per_hour: u64,
        max_orders_per_second: u64,
        max_orders_per_minute: u64,
    ) -> Self {
        Self {
            max_requests_per_second,
            max_requests_per_minute,
            max_requests_per_hour,
            max_orders_per_second,
            max_orders_per_minute,
        }
    }

    /// Creates a restrictive rate limit for high-risk operations
    pub fn restrictive() -> Self {
        Self::new(
            10,   // 10 requests per second
            100,  // 100 requests per minute
            1000, // 1000 requests per hour
            5,    // 5 orders per second
            50,   // 50 orders per minute
        )
    }

    /// Creates a permissive rate limit for normal operations
    pub fn permissive() -> Self {
        Self::new(
            100,  // 100 requests per second
            1000, // 1000 requests per minute
            10000, // 10000 requests per hour
            50,   // 50 orders per second
            500,  // 500 orders per minute
        )
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self::permissive()
    }
}

/// User-specific limits
#[derive(Debug, Clone)]
pub struct UserLimits {
    /// Position limits per asset
    pub position_limits: HashMap<Asset, PositionLimit>,
    /// Daily trading limits per pair
    pub daily_limits: HashMap<TradingPair, DailyTradingLimit>,
    /// Rate limits
    pub rate_limits: RateLimit,
    /// Maximum number of open orders
    pub max_open_orders: u32,
    /// Maximum number of cancel orders per minute
    pub max_cancels_per_minute: u32,
}

impl UserLimits {
    /// Creates a new user limits configuration
    pub fn new(
        position_limits: HashMap<Asset, PositionLimit>,
        daily_limits: HashMap<TradingPair, DailyTradingLimit>,
        rate_limits: RateLimit,
        max_open_orders: u32,
        max_cancels_per_minute: u32,
    ) -> Self {
        Self {
            position_limits,
            daily_limits,
            rate_limits,
            max_open_orders,
            max_cancels_per_minute,
        }
    }

    /// Gets the position limit for an asset
    pub fn get_position_limit(&self, asset: &Asset) -> Option<&PositionLimit> {
        self.position_limits.get(asset)
    }

    /// Gets the daily trading limit for a pair
    pub fn get_daily_limit(&self, pair: &TradingPair) -> Option<&DailyTradingLimit> {
        self.daily_limits.get(pair)
    }

    /// Checks if user can open more orders
    pub fn can_open_more_orders(&self, current_open_orders: u32) -> bool {
        current_open_orders < self.max_open_orders
    }

    /// Checks if user can cancel more orders
    pub fn can_cancel_more_orders(&self, current_cancels: u32) -> bool {
        current_cancels < self.max_cancels_per_minute
    }
}

/// System-wide limits
#[derive(Debug, Clone)]
pub struct SystemLimits {
    /// Maximum total open orders across all users
    pub max_total_open_orders: u64,
    /// Maximum order book depth per side
    pub max_order_book_depth: usize,
    /// Maximum price deviation from reference price
    pub max_price_deviation_bps: u32,
    /// Circuit breaker threshold (price movement percentage)
    pub circuit_breaker_threshold_bps: u32,
    /// Maximum order size for any user
    pub max_system_order_size: Balance,
    /// Minimum order size for any user
    pub min_system_order_size: Balance,
}

impl SystemLimits {
    /// Creates a new system limits configuration
    pub fn new(
        max_total_open_orders: u64,
        max_order_book_depth: usize,
        max_price_deviation_bps: u32,
        circuit_breaker_threshold_bps: u32,
        max_system_order_size: Balance,
        min_system_order_size: Balance,
    ) -> Self {
        Self {
            max_total_open_orders,
            max_order_book_depth,
            max_price_deviation_bps,
            circuit_breaker_threshold_bps,
            max_system_order_size,
            min_system_order_size,
        }
    }

    /// Checks if total open orders are within system limits
    pub fn is_total_open_orders_within_limit(&self, current_total: u64) -> bool {
        current_total < self.max_total_open_orders
    }

    /// Checks if order book depth is within limits
    pub fn is_order_book_depth_within_limit(&self, current_depth: usize) -> bool {
        current_depth <= self.max_order_book_depth
    }

    /// Checks if price deviation is within limits
    pub fn is_price_deviation_within_limit(&self, deviation_bps: u32) -> bool {
        deviation_bps <= self.max_price_deviation_bps
    }

    /// Checks if circuit breaker should be triggered
    pub fn should_trigger_circuit_breaker(&self, price_movement_bps: u32) -> bool {
        price_movement_bps >= self.circuit_breaker_threshold_bps
    }

    /// Checks if order size is within system limits
    pub fn is_order_size_within_system_limits(&self, order_size: Balance) -> bool {
        order_size >= self.min_system_order_size && order_size <= self.max_system_order_size
    }
}

impl Default for SystemLimits {
    fn default() -> Self {
        Self::new(
            1_000_000,       // 1M total open orders
            1000,            // 1000 levels per side
            5000,            // 50% max price deviation
            1000,            // 10% circuit breaker
            10_000_000_000,  // 100 BTC max order size
            1,               // 1 satoshi min order size
        )
    }
}

/// Limits manager for enforcing all types of limits
pub struct LimitsManager {
    /// User-specific limits
    user_limits: HashMap<UserId, UserLimits>,
    /// System-wide limits
    system_limits: SystemLimits,
    /// Default user limits for new users
    default_user_limits: UserLimits,
}

impl LimitsManager {
    /// Creates a new limits manager
    pub fn new(system_limits: SystemLimits, default_user_limits: UserLimits) -> Self {
        Self {
            user_limits: HashMap::new(),
            system_limits,
            default_user_limits,
        }
    }

    /// Gets limits for a user
    pub fn get_user_limits(&self, user_id: UserId) -> &UserLimits {
        self.user_limits.get(&user_id).unwrap_or(&self.default_user_limits)
    }

    /// Sets limits for a user
    pub fn set_user_limits(&mut self, user_id: UserId, limits: UserLimits) {
        self.user_limits.insert(user_id, limits);
    }

    /// Removes limits for a user (reverts to defaults)
    pub fn remove_user_limits(&mut self, user_id: UserId) {
        self.user_limits.remove(&user_id);
    }

    /// Checks if an order complies with all applicable limits
    pub fn check_order_limits(
        &self,
        user_id: UserId,
        account: &Account,
        order: &Order,
        trading_pair: &TradingPair,
        current_stats: &LimitStats,
    ) -> ExchangeResult<()> {
        let user_limits = self.get_user_limits(user_id);

        // Check system limits first
        if !self.system_limits.is_order_size_within_system_limits(order.quantity) {
            return Err(ExchangeError::risk_check_failed(
                "Order size exceeds system limits".to_string()
            ));
        }

        // Check user-specific position limits
        if let Some(position_limit) = user_limits.get_position_limit(&trading_pair.base) {
            let current_position = account.get_position(&trading_pair.base);
            let new_position = Self::calculate_new_position(current_position, order);

            if !position_limit.is_position_within_limits(new_position) {
                return Err(ExchangeError::risk_check_failed(
                    "Position limit exceeded".to_string()
                ));
            }
        }

        // Check daily trading limits
        if let Some(daily_limit) = user_limits.get_daily_limit(trading_pair) {
            let order_volume = order.price.unwrap_or(Price::new(0)).value().saturating_mul(order.quantity);
            
            if !daily_limit.is_volume_within_limit(current_stats.daily_volume, order_volume) {
                return Err(ExchangeError::risk_check_failed(
                    "Daily trading volume limit exceeded".to_string()
                ));
            }

            if !daily_limit.is_trade_count_within_limit(current_stats.daily_trades) {
                return Err(ExchangeError::risk_check_failed(
                    "Daily trade count limit exceeded".to_string()
                ));
            }

            if !daily_limit.is_order_size_within_limit(order_volume) {
                return Err(ExchangeError::risk_check_failed(
                    "Order size exceeds per-trade limit".to_string()
                ));
            }
        }

        // Check open order limits
        if !user_limits.can_open_more_orders(current_stats.open_orders) {
            return Err(ExchangeError::risk_check_failed(
                "Maximum open orders limit exceeded".to_string()
            ));
        }

        Ok(())
    }

    /// Calculates new position after order execution
    fn calculate_new_position(current_position: Balance, order: &Order) -> i128 {
        match order.side {
            OrderSide::Buy => current_position as i128 + order.quantity as i128,
            OrderSide::Sell => current_position as i128 - order.quantity as i128,
        }
    }

    /// Gets system limits
    pub fn get_system_limits(&self) -> &SystemLimits {
        &self.system_limits
    }

    /// Updates system limits
    pub fn update_system_limits(&mut self, new_limits: SystemLimits) {
        self.system_limits = new_limits;
    }
}

/// Current statistics for limit checking
#[derive(Debug, Clone)]
pub struct LimitStats {
    /// Daily trading volume
    pub daily_volume: Balance,
    /// Daily trade count
    pub daily_trades: u32,
    /// Current open orders
    pub open_orders: u32,
    /// Current total open orders system-wide
    pub total_open_orders: u64,
    /// Current order book depth
    pub order_book_depth: usize,
    /// Current price deviation from reference
    pub price_deviation_bps: u32,
}

impl LimitStats {
    /// Creates new limit statistics
    pub fn new(
        daily_volume: Balance,
        daily_trades: u32,
        open_orders: u32,
        total_open_orders: u64,
        order_book_depth: usize,
        price_deviation_bps: u32,
    ) -> Self {
        Self {
            daily_volume,
            daily_trades,
            open_orders,
            total_open_orders,
            order_book_depth,
            price_deviation_bps,
        }
    }
}

impl Default for LimitStats {
    fn default() -> Self {
        Self::new(0, 0, 0, 0, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::order::TimeInForce;

    #[test]
    fn test_position_limit() {
        let limit = PositionLimit::new(1_000_000, 10); // 0.01 BTC max position, 10x leverage

        // Test position within limits
        assert!(limit.is_position_within_limits(500_000)); // 0.005 BTC
        assert!(limit.is_long_position_within_limits(500_000));
        assert!(limit.is_short_position_within_limits(-500_000));

        // Test position exceeding limits
        assert!(!limit.is_position_within_limits(2_000_000)); // 0.02 BTC
        assert!(!limit.is_long_position_within_limits(2_000_000));
        assert!(!limit.is_short_position_within_limits(-2_000_000));

        // Test max order size calculation
        assert_eq!(limit.max_order_size(500_000, OrderSide::Buy), 500_000); // Can buy 0.005 more
        assert_eq!(limit.max_order_size(-500_000, OrderSide::Sell), 500_000); // Can sell 0.005 more
        assert_eq!(limit.max_order_size(1_500_000, OrderSide::Buy), 0); // Can't buy more
    }

    #[test]
    fn test_daily_trading_limit() {
        let limit = DailyTradingLimit::new(
            100_000_000, // 1000 USDT daily volume
            100,         // 100 trades per day
            10_000_000,  // 100 USDT max daily loss
            10_000_000,  // 100 USDT max per trade
        );

        assert!(limit.is_volume_within_limit(50_000_000, 25_000_000)); // 500 + 250 = 750 < 1000
        assert!(!limit.is_volume_within_limit(80_000_000, 30_000_000)); // 800 + 300 = 1100 > 1000

        assert!(limit.is_trade_count_within_limit(50));
        assert!(!limit.is_trade_count_within_limit(100));

        assert!(limit.is_loss_within_limit(5_000_000, 3_000_000)); // 50 + 30 = 80 < 100
        assert!(!limit.is_loss_within_limit(8_000_000, 5_000_000)); // 80 + 50 = 130 > 100

        assert!(limit.is_order_size_within_limit(5_000_000)); // 50 USDT < 100 USDT
        assert!(!limit.is_order_size_within_limit(15_000_000)); // 150 USDT > 100 USDT
    }

    #[test]
    fn test_rate_limit() {
        let restrictive = RateLimit::restrictive();
        let permissive = RateLimit::permissive();

        assert_eq!(restrictive.max_requests_per_second, 10);
        assert_eq!(permissive.max_requests_per_second, 100);
        assert_eq!(restrictive.max_orders_per_second, 5);
        assert_eq!(permissive.max_orders_per_second, 50);
    }

    #[test]
    fn test_system_limits() {
        let limits = SystemLimits::default();

        assert!(limits.is_total_open_orders_within_limit(500_000));
        assert!(!limits.is_total_open_orders_within_limit(1_500_000));

        assert!(limits.is_order_book_depth_within_limit(500));
        assert!(!limits.is_order_book_depth_within_limit(1500));

        assert!(limits.is_price_deviation_within_limit(1000)); // 10%
        assert!(!limits.is_price_deviation_within_limit(6000)); // 60%

        assert!(!limits.should_trigger_circuit_breaker(500)); // 5%
        assert!(limits.should_trigger_circuit_breaker(1500)); // 15%

        assert!(limits.is_order_size_within_system_limits(1_000_000)); // 0.01 BTC
        assert!(!limits.is_order_size_within_system_limits(20_000_000_000)); // 200 BTC
    }

    #[test]
    fn test_limits_manager() {
        let system_limits = SystemLimits::default();
        let default_user_limits = UserLimits::new(
            HashMap::new(),
            HashMap::new(),
            RateLimit::default(),
            100, // max open orders
            50,  // max cancels per minute
        );

        let mut manager = LimitsManager::new(system_limits, default_user_limits);

        // Test with default limits
        let account = Account::new(100);
        let trading_pair = TradingPair::new(Asset::BTC, Asset::USDT);
        let order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            TimeInForce::GTC,
            1234567890,
        );

        let stats = LimitStats::default();
        assert!(manager.check_order_limits(100, &account, &order, &trading_pair, &stats).is_ok());

        // Test with order exceeding system limits
        let large_order = Order::new(
            2,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)),
            20_000_000_000, // 200 BTC - exceeds system limit
            TimeInForce::GTC,
            1234567890,
        );

        assert!(manager.check_order_limits(100, &account, &large_order, &trading_pair, &stats).is_err());
    }
}
