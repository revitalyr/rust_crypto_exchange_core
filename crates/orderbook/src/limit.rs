//! Limit order handling utilities.

use crypto_exchange_common::{
    order::{Order, OrderSide, OrderType, TimeInForce},
    price::Price,
    ExchangeError, ExchangeResult,
};

/// Limit order validator
pub struct LimitOrderValidator {
    /// Minimum order size
    pub min_order_size: u64,
    /// Maximum order size
    pub max_order_size: u64,
    /// Minimum price
    pub min_price: u64,
    /// Maximum price
    pub max_price: u64,
    /// Tick size
    pub tick_size: u64,
    /// Lot size
    pub lot_size: u64,
}

impl LimitOrderValidator {
    /// Creates a new limit order validator
    pub fn new(
        min_order_size: u64,
        max_order_size: u64,
        min_price: u64,
        max_price: u64,
        tick_size: u64,
        lot_size: u64,
    ) -> Self {
        Self {
            min_order_size,
            max_order_size,
            min_price,
            max_price,
            tick_size,
            lot_size,
        }
    }

    /// Validates a limit order
    pub fn validate(&self, order: &Order) -> ExchangeResult<()> {
        // Check order type
        if order.order_type != OrderType::Limit {
            return Err(ExchangeError::invalid_order(
                "Expected limit order",
            ));
        }

        // Check price
        let price = order.price.ok_or_else(|| {
            ExchangeError::invalid_order("Limit order must have a price")
        })?;

        if price.value() < self.min_price {
            return Err(ExchangeError::invalid_price(price.value()));
        }

        if price.value() > self.max_price {
            return Err(ExchangeError::invalid_price(price.value()));
        }

        // Check price alignment with tick size
        if price.value() % self.tick_size != 0 {
            return Err(ExchangeError::invalid_order(
                "Price must align with tick size",
            ));
        }

        // Check quantity
        if order.quantity < self.min_order_size {
            return Err(ExchangeError::invalid_quantity(order.quantity));
        }

        if order.quantity > self.max_order_size {
            return Err(ExchangeError::invalid_quantity(order.quantity));
        }

        // Check quantity alignment with lot size
        if order.quantity % self.lot_size != 0 {
            return Err(ExchangeError::invalid_order(
                "Quantity must align with lot size",
            ));
        }

        // Check time in force
        match order.time_in_force {
            TimeInForce::GTC | TimeInForce::IOC | TimeInForce::FOK => {}
        }

        Ok(())
    }

    /// Returns the tick-aligned price
    pub fn align_price(&self, price: u64) -> u64 {
        (price / self.tick_size) * self.tick_size
    }

    /// Returns the lot-aligned quantity
    pub fn align_quantity(&self, quantity: u64) -> u64 {
        (quantity / self.lot_size) * self.lot_size
    }

    /// Checks if a price is valid
    pub fn is_valid_price(&self, price: u64) -> bool {
        price >= self.min_price
            && price <= self.max_price
            && price % self.tick_size == 0
    }

    /// Checks if a quantity is valid
    pub fn is_valid_quantity(&self, quantity: u64) -> bool {
        quantity >= self.min_order_size
            && quantity <= self.max_order_size
            && quantity % self.lot_size == 0
    }
}

impl Default for LimitOrderValidator {
    fn default() -> Self {
        Self::new(
            1,          // min_order_size
            1_000_000,  // max_order_size
            1,          // min_price
            1_000_000_000, // max_price
            1,          // tick_size
            1,          // lot_size
        )
    }
}

/// Limit order execution context
#[derive(Debug, Clone)]
pub struct LimitOrderContext {
    /// Order ID
    pub order_id: u64,
    /// User ID
    pub user_id: u64,
    /// Order side
    pub side: OrderSide,
    /// Order price
    pub price: Price,
    /// Order quantity
    pub quantity: u64,
    /// Remaining quantity
    pub remaining_quantity: u64,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Creation timestamp
    pub created_at: u64,
}

impl LimitOrderContext {
    /// Creates a new limit order context
    pub fn new(order: &Order) -> ExchangeResult<Self> {
        if order.order_type != OrderType::Limit {
            return Err(ExchangeError::invalid_order(
                "Expected limit order",
            ));
        }

        let price = order.price.ok_or_else(|| {
            ExchangeError::invalid_order("Limit order must have a price")
        })?;

        Ok(Self {
            order_id: order.id,
            user_id: order.user_id,
            side: order.side,
            price,
            quantity: order.quantity,
            remaining_quantity: order.remaining_quantity(),
            time_in_force: order.time_in_force,
            created_at: order.created_at,
        })
    }

    /// Checks if the order can be partially filled
    pub fn can_partial_fill(&self) -> bool {
        matches!(self.time_in_force, TimeInForce::GTC | TimeInForce::IOC)
    }

    /// Checks if the order requires immediate execution
    pub fn requires_immediate_execution(&self) -> bool {
        matches!(self.time_in_force, TimeInForce::IOC | TimeInForce::FOK)
    }

    /// Checks if the order should be cancelled if not fully filled
    pub fn cancel_if_not_filled(&self) -> bool {
        matches!(self.time_in_force, TimeInForce::IOC | TimeInForce::FOK)
    }
}

/// Limit order matching result
#[derive(Debug, Clone)]
pub struct LimitOrderMatchResult {
    /// Order ID
    pub order_id: u64,
    /// Matched quantity
    pub matched_quantity: u64,
    /// Average execution price
    pub avg_price: Price,
    /// Total cost
    pub total_cost: u64,
    /// Remaining quantity
    pub remaining_quantity: u64,
    /// Order status
    pub status: crypto_exchange_common::order::OrderStatus,
    /// Whether the order should be cancelled
    pub should_cancel: bool,
}

impl LimitOrderMatchResult {
    /// Creates a new match result
    pub fn new(
        order_id: u64,
        matched_quantity: u64,
        avg_price: Price,
        total_cost: u64,
        remaining_quantity: u64,
        status: crypto_exchange_common::order::OrderStatus,
        should_cancel: bool,
    ) -> Self {
        Self {
            order_id,
            matched_quantity,
            avg_price,
            total_cost,
            remaining_quantity,
            status,
            should_cancel,
        }
    }

    /// Creates a result for a fully filled order
    pub fn fully_filled(order_id: u64, avg_price: Price, total_cost: u64) -> Self {
        Self {
            order_id,
            matched_quantity: total_cost / avg_price.value(),
            avg_price,
            total_cost,
            remaining_quantity: 0,
            status: crypto_exchange_common::order::OrderStatus::Filled,
            should_cancel: false,
        }
    }

    /// Creates a result for a partially filled order
    pub fn partially_filled(
        order_id: u64,
        matched_quantity: u64,
        avg_price: Price,
        total_cost: u64,
        remaining_quantity: u64,
    ) -> Self {
        Self {
            order_id,
            matched_quantity,
            avg_price,
            total_cost,
            remaining_quantity,
            status: crypto_exchange_common::order::OrderStatus::PartiallyFilled,
            should_cancel: false,
        }
    }

    /// Creates a result for an unfilled order
    pub fn unfilled(order_id: u64, should_cancel: bool) -> Self {
        Self {
            order_id,
            matched_quantity: 0,
            avg_price: Price::new(0),
            total_cost: 0,
            remaining_quantity: 0,
            status: crypto_exchange_common::order::OrderStatus::Active,
            should_cancel,
        }
    }

    /// Checks if the order was filled
    pub fn is_filled(&self) -> bool {
        matches!(self.status, crypto_exchange_common::order::OrderStatus::Filled)
    }

    /// Checks if the order was partially filled
    pub fn is_partially_filled(&self) -> bool {
        matches!(
            self.status,
            crypto_exchange_common::order::OrderStatus::PartiallyFilled
        )
    }

    /// Checks if the order has any matches
    pub fn has_matches(&self) -> bool {
        self.matched_quantity > 0
    }
}

/// Limit order utilities
pub struct LimitOrderUtils;

impl LimitOrderUtils {
    /// Calculates the execution price for a limit order
    pub fn calculate_execution_price(
        order_price: Price,
        market_price: Price,
        side: OrderSide,
    ) -> Price {
        match side {
            OrderSide::Buy => {
                // Buy orders execute at the lower of order price and market price
                if order_price.value() <= market_price.value() {
                    order_price
                } else {
                    market_price
                }
            }
            OrderSide::Sell => {
                // Sell orders execute at the higher of order price and market price
                if order_price.value() >= market_price.value() {
                    order_price
                } else {
                    market_price
                }
            }
        }
    }

    /// Checks if a limit order can match at a given price
    pub fn can_match_at_price(
        order_price: Price,
        match_price: Price,
        side: OrderSide,
    ) -> bool {
        match side {
            OrderSide::Buy => order_price.value() >= match_price.value(),
            OrderSide::Sell => order_price.value() <= match_price.value(),
        }
    }

    /// Gets the effective price for a limit order
    pub fn get_effective_price(order_price: Price, side: OrderSide) -> Price {
        // For limit orders, the effective price is the order price
        // This could be extended with slippage protection, etc.
        order_price
    }

    /// Calculates the maximum quantity that can be matched
    pub fn max_matchable_quantity(
        order_quantity: u64,
        market_quantity: u64,
        time_in_force: TimeInForce,
    ) -> u64 {
        match time_in_force {
            TimeInForce::FOK => {
                // Fill or Kill - must match entire order
                if market_quantity >= order_quantity {
                    order_quantity
                } else {
                    0
                }
            }
            TimeInForce::IOC | TimeInForce::GTC => {
                // Immediate or Cancel / Good Till Canceled - can match partial
                order_quantity.min(market_quantity)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::order::OrderStatus;

    #[test]
    fn test_limit_order_validator() {
        let validator = LimitOrderValidator::new(
            1,      // min_order_size
            10000,  // max_order_size
            1000,   // min_price
            100000, // max_price
            100,    // tick_size
            10,     // lot_size
        );

        // Valid order
        let valid_order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)), // 500.00, aligned with tick size
            1000,                    // 100 lots, aligned with lot size
            TimeInForce::GTC,
            1234567890,
        );

        assert!(validator.validate(&valid_order).is_ok());

        // Invalid price (not aligned with tick size)
        let invalid_price_order = Order::new(
            2,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(5050)), // Not aligned with tick size of 100
            1000,
            TimeInForce::GTC,
            1234567890,
        );

        assert!(validator.validate(&invalid_price_order).is_err());

        // Invalid quantity (not aligned with lot size)
        let invalid_quantity_order = Order::new(
            3,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)),
            105, // Not aligned with lot size of 10
            TimeInForce::GTC,
            1234567890,
        );

        assert!(validator.validate(&invalid_quantity_order).is_err());
    }

    #[test]
    fn test_limit_order_context() {
        let order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            TimeInForce::IOC,
            1234567890,
        );

        let context = LimitOrderContext::new(&order).unwrap();
        assert_eq!(context.order_id, 1);
        assert_eq!(context.side, OrderSide::Buy);
        assert_eq!(context.price, Price::new(50000));
        assert_eq!(context.quantity, 1000);
        assert_eq!(context.remaining_quantity, 1000);
        assert!(context.can_partial_fill());
        assert!(context.requires_immediate_execution());
        assert!(context.cancel_if_not_filled());
    }

    #[test]
    fn test_limit_order_match_result() {
        let result = LimitOrderMatchResult::fully_filled(
            1,
            Price::new(50000),
            50_000_000, // 50000 * 1000
        );

        assert_eq!(result.order_id, 1);
        assert!(result.is_filled());
        assert!(result.has_matches());
        assert_eq!(result.matched_quantity, 1000);
        assert_eq!(result.remaining_quantity, 0);
        assert!(!result.should_cancel);

        let partial_result = LimitOrderMatchResult::partially_filled(
            2,
            500,
            Price::new(50000),
            25_000_000, // 50000 * 500
            500,
        );

        assert!(partial_result.is_partially_filled());
        assert!(partial_result.has_matches());
        assert_eq!(partial_result.matched_quantity, 500);
        assert_eq!(partial_result.remaining_quantity, 500);

        let unfilled_result = LimitOrderMatchResult::unfilled(3, true);
        assert!(!unfilled_result.has_matches());
        assert!(unfilled_result.should_cancel);
    }

    #[test]
    fn test_limit_order_utils() {
        let order_price = Price::new(50000);
        let market_price = Price::new(49500);

        // Test execution price calculation
        let exec_price_buy = LimitOrderUtils::calculate_execution_price(
            order_price,
            market_price,
            OrderSide::Buy,
        );
        assert_eq!(exec_price_buy, market_price); // Buy at lower price

        let exec_price_sell = LimitOrderUtils::calculate_execution_price(
            order_price,
            market_price,
            OrderSide::Sell,
        );
        assert_eq!(exec_price_sell, order_price); // Sell at higher price

        // Test matching conditions
        assert!(LimitOrderUtils::can_match_at_price(
            Price::new(50000),
            Price::new(49500),
            OrderSide::Buy
        )); // Buy order price >= market price

        assert!(LimitOrderUtils::can_match_at_price(
            Price::new(49500),
            Price::new(50000),
            OrderSide::Sell
        )); // Sell order price <= market price

        // Test max matchable quantity
        let max_qty = LimitOrderUtils::max_matchable_quantity(
            1000,
            500,
            TimeInForce::IOC,
        );
        assert_eq!(max_qty, 500);

        let max_qty_fok = LimitOrderUtils::max_matchable_quantity(
            1000,
            500,
            TimeInForce::FOK,
        );
        assert_eq!(max_qty_fok, 0); // Can't fill entire order
    }
}
