//! Market order handling utilities.

use crypto_exchange_common::{
    order::{Order, OrderSide, OrderType, TimeInForce},
    price::Price,
    ExchangeError, ExchangeResult,
};

/// Market order validator
pub struct MarketOrderValidator {
    /// Maximum market order size
    pub max_order_size: u64,
    /// Minimum market order size
    pub min_order_size: u64,
    /// Maximum market impact percentage (in basis points, 10000 = 100%)
    pub max_impact_bps: u32,
}

impl MarketOrderValidator {
    /// Creates a new market order validator
    pub fn new(
        min_order_size: u64,
        max_order_size: u64,
        max_impact_bps: u32,
    ) -> Self {
        Self {
            min_order_size,
            max_order_size,
            max_impact_bps,
        }
    }

    /// Validates a market order
    pub fn validate(&self, order: &Order) -> ExchangeResult<()> {
        // Check order type
        let OrderType::Market = order.order_type else {
            return Err(ExchangeError::invalid_order("Expected market order"));
        };

        // Market orders should not have a price
        if order.price.is_some() {
            return Err(ExchangeError::invalid_order(
                "Market order should not have a price",
            ));
        }

        // Check quantity
        if order.quantity < self.min_order_size {
            return Err(ExchangeError::invalid_quantity(order.quantity));
        }

        if order.quantity > self.max_order_size {
            return Err(ExchangeError::invalid_quantity(order.quantity));
        }

        // Market orders should use IOC or FOK time in force
        match order.time_in_force {
            TimeInForce::IOC | TimeInForce::FOK => {}
            TimeInForce::GTC => {
                return Err(ExchangeError::invalid_order(
                    "Market orders cannot use GTC time in force",
                ));
            }
        }

        Ok(())
    }

    /// Checks if a quantity is valid for market orders
    pub fn is_valid_quantity(&self, quantity: u64) -> bool {
        quantity >= self.min_order_size && quantity <= self.max_order_size
    }
}

impl Default for MarketOrderValidator {
    fn default() -> Self {
        Self::new(
            1,           // min_order_size
            1_000_000,   // max_order_size
            500,         // max_impact_bps (5%)
        )
    }
}

/// Market order execution context
#[derive(Debug, Clone)]
pub struct MarketOrderContext {
    /// Order ID
    pub order_id: u64,
    /// User ID
    pub user_id: u64,
    /// Order side
    pub side: OrderSide,
    /// Order quantity
    pub quantity: u64,
    /// Remaining quantity to fill
    pub remaining_quantity: u64,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Creation timestamp
    pub created_at: u64,
    /// Maximum slippage percentage (in basis points)
    pub max_slippage_bps: Option<u32>,
}

impl MarketOrderContext {
    /// Creates a new market order context
    pub fn new(order: &Order) -> ExchangeResult<Self> {
        let OrderType::Market = order.order_type else {
            return Err(ExchangeError::invalid_order("Expected market order"));
        };

        if order.price.is_some() {
            return Err(ExchangeError::invalid_order(
                "Market order should not have a price",
            ));
        }

        Ok(Self {
            order_id: order.id,
            user_id: order.user_id,
            side: order.side,
            quantity: order.quantity,
            remaining_quantity: order.quantity,
            time_in_force: order.time_in_force,
            created_at: order.created_at,
            max_slippage_bps: None, // Can be set separately
        })
    }

    /// Sets maximum slippage
    pub fn with_max_slippage(mut self, max_slippage_bps: u32) -> Self {
        self.max_slippage_bps = Some(max_slippage_bps);
        self
    }

    /// Checks if the order requires full execution
    pub fn requires_full_execution(&self) -> bool {
        matches!(self.time_in_force, TimeInForce::FOK)
    }

    /// Checks if the order can be partially filled
    pub fn can_partial_fill(&self) -> bool {
        matches!(self.time_in_force, TimeInForce::IOC)
    }

    /// Checks if the order should be cancelled if not fully filled
    pub fn cancel_if_not_filled(&self) -> bool {
        matches!(self.time_in_force, TimeInForce::FOK)
    }
}

/// Market order execution result
#[derive(Debug, Clone)]
pub struct MarketOrderResult {
    /// Order ID
    pub order_id: u64,
    /// Total quantity filled
    pub filled_quantity: u64,
    /// Average execution price
    pub avg_price: Price,
    /// Total cost
    pub total_cost: u64,
    /// Number of individual fills
    pub fill_count: usize,
    /// Order status
    pub status: crypto_exchange_common::order::OrderStatus,
    /// Slippage in basis points
    pub slippage_bps: Option<u32>,
    /// Market impact in basis points
    pub impact_bps: Option<u32>,
}

impl MarketOrderResult {
    /// Creates a new market order result
    pub fn new(
        order_id: u64,
        filled_quantity: u64,
        avg_price: Price,
        total_cost: u64,
        fill_count: usize,
        status: crypto_exchange_common::order::OrderStatus,
    ) -> Self {
        Self {
            order_id,
            filled_quantity,
            avg_price,
            total_cost,
            fill_count,
            status,
            slippage_bps: None,
            impact_bps: None,
        }
    }

    /// Creates a result for a fully filled market order
    pub fn fully_filled(
        order_id: u64,
        avg_price: Price,
        total_cost: u64,
        fill_count: usize,
    ) -> Self {
        Self {
            order_id,
            filled_quantity: total_cost / avg_price.value(),
            avg_price,
            total_cost,
            fill_count,
            status: crypto_exchange_common::order::OrderStatus::Filled,
            slippage_bps: None,
            impact_bps: None,
        }
    }

    /// Creates a result for a partially filled market order
    pub fn partially_filled(
        order_id: u64,
        filled_quantity: u64,
        avg_price: Price,
        total_cost: u64,
        fill_count: usize,
    ) -> Self {
        Self {
            order_id,
            filled_quantity,
            avg_price,
            total_cost,
            fill_count,
            status: crypto_exchange_common::order::OrderStatus::PartiallyFilled,
            slippage_bps: None,
            impact_bps: None,
        }
    }

    /// Creates a result for an unfilled market order
    pub fn unfilled(order_id: u64) -> Self {
        Self {
            order_id,
            filled_quantity: 0,
            avg_price: Price::new(0),
            total_cost: 0,
            fill_count: 0,
            status: crypto_exchange_common::order::OrderStatus::Cancelled,
            slippage_bps: None,
            impact_bps: None,
        }
    }

    /// Sets slippage in basis points
    pub fn with_slippage(mut self, slippage_bps: u32) -> Self {
        self.slippage_bps = Some(slippage_bps);
        self
    }

    /// Sets market impact in basis points
    pub fn with_impact(mut self, impact_bps: u32) -> Self {
        self.impact_bps = Some(impact_bps);
        self
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

    /// Checks if the order has any fills
    pub fn has_fills(&self) -> bool {
        self.filled_quantity > 0
    }

    /// Calculates the fill percentage
    pub fn fill_percentage(&self, original_quantity: u64) -> f64 {
        if original_quantity == 0 {
            0.0
        } else {
            self.filled_quantity as f64 / original_quantity as f64
        }
    }
}

/// Market order execution level
#[derive(Debug, Clone)]
pub struct MarketExecutionLevel {
    /// Price level
    pub price: Price,
    /// Available quantity at this level
    pub available_quantity: u64,
    /// Quantity to execute at this level
    pub execute_quantity: u64,
}

impl MarketExecutionLevel {
    /// Creates a new execution level
    pub fn new(price: Price, available_quantity: u64, execute_quantity: u64) -> Self {
        Self {
            price,
            available_quantity,
            execute_quantity,
        }
    }

    /// Checks if this level can provide the requested quantity
    pub fn can_fill(&self, quantity: u64) -> bool {
        self.available_quantity >= quantity
    }

    /// Gets the cost at this level
    pub fn cost(&self) -> Option<u64> {
        self.price.value().checked_mul(self.execute_quantity)
    }
}

/// Market order execution plan
#[derive(Debug, Clone)]
pub struct MarketExecutionPlan {
    /// Order ID
    pub order_id: u64,
    /// Order side
    pub side: OrderSide,
    /// Original quantity
    pub original_quantity: u64,
    /// Execution levels
    pub levels: Vec<MarketExecutionLevel>,
    /// Total quantity that can be filled
    pub fillable_quantity: u64,
    /// Estimated average price
    pub estimated_avg_price: Price,
    /// Estimated total cost
    pub estimated_cost: u64,
}

impl MarketExecutionPlan {
    /// Creates a new execution plan
    pub fn new(
        order_id: u64,
        side: OrderSide,
        original_quantity: u64,
        levels: Vec<MarketExecutionLevel>,
    ) -> ExchangeResult<Self> {
        let fillable_quantity = levels.iter().map(|l| l.execute_quantity).sum();
        let estimated_cost = levels.iter().filter_map(|l| l.cost()).sum();

        let estimated_avg_price = if fillable_quantity > 0 {
            Price::new(estimated_cost / fillable_quantity)
        } else {
            Price::new(0)
        };

        Ok(Self {
            order_id,
            side,
            original_quantity,
            levels,
            fillable_quantity,
            estimated_avg_price,
            estimated_cost,
        })
    }

    /// Checks if the plan can fully fill the order
    pub fn can_fully_fill(&self) -> bool {
        self.fillable_quantity >= self.original_quantity
    }

    /// Gets the fill percentage
    pub fn fill_percentage(&self) -> f64 {
        if self.original_quantity == 0 {
            0.0
        } else {
            self.fillable_quantity as f64 / self.original_quantity as f64
        }
    }

    /// Calculates slippage against a reference price
    pub fn calculate_slippage(&self, reference_price: Price) -> Option<u32> {
        if self.fillable_quantity == 0 {
            return None;
        }

        let reference_value = reference_price.value();
        let actual_value = self.estimated_avg_price.value();

        let slippage_bps = match self.side {
            OrderSide::Buy => {
                // For buys, slippage is when actual price > reference price
                if actual_value > reference_value {
                    ((actual_value - reference_value) * 10000) / reference_value
                } else {
                    0
                }
            }
            OrderSide::Sell => {
                // For sells, slippage is when actual price < reference price
                if actual_value < reference_value {
                    ((reference_value - actual_value) * 10000) / reference_value
                } else {
                    0
                }
            }
        };

        Some(slippage_bps as u32)
    }
}

/// Calculates the maximum quantity that can be filled at given levels
pub fn calculate_max_fillable_quantity(
    levels: &[(Price, u64)],
    order_quantity: u64,
    side: OrderSide,
) -> u64 {
    let mut remaining = order_quantity;
    let mut filled = 0u64;

    let sorted_levels = if side == OrderSide::Buy {
        // For buys, use ascending prices (best to worst)
        let mut sorted = levels.to_vec();
        sorted.sort_by_key(|(price, _)| price.value());
        sorted
    } else {
        // For sells, use descending prices (best to worst)
        let mut sorted = levels.to_vec();
        sorted.sort_by_key(|(price, _)| std::cmp::Reverse(price.value()));
        sorted
    };

    for (_price, quantity) in sorted_levels {
        if remaining == 0 {
            break;
        }

        let fill_quantity = remaining.min(quantity);
        filled += fill_quantity;
        remaining -= fill_quantity;
    }

    filled
}

/// Creates an execution plan for a market order
pub fn create_execution_plan(
    order_id: u64,
    side: OrderSide,
    quantity: u64,
    levels: &[(Price, u64)],
) -> ExchangeResult<MarketExecutionPlan> {
    let sorted_levels = if side == OrderSide::Buy {
        // For buys, use ascending prices
        let mut sorted = levels.to_vec();
        sorted.sort_by_key(|(price, _)| price.value());
        sorted
    } else {
        // For sells, use descending prices
        let mut sorted = levels.to_vec();
        sorted.sort_by_key(|(price, _)| std::cmp::Reverse(price.value()));
        sorted
    };

    let mut remaining = quantity;
    let mut execution_levels = Vec::new();

    for (price, available_quantity) in sorted_levels {
        if remaining == 0 {
            break;
        }

        let execute_quantity = remaining.min(available_quantity);
        execution_levels.push(MarketExecutionLevel::new(
            price,
            available_quantity,
            execute_quantity,
        ));

        remaining -= execute_quantity;
    }

    MarketExecutionPlan::new(order_id, side, quantity, execution_levels)
}

/// Calculates market impact
pub fn calculate_market_impact(
    mid_price: Price,
    execution_price: Price,
    side: OrderSide,
) -> u32 {
    let mid_value = mid_price.value();
    let exec_value = execution_price.value();

    let impact_bps = match side {
        OrderSide::Buy => {
            // For buys, impact is when execution price > mid price
            if exec_value > mid_value {
                ((exec_value - mid_value) * 10000) / mid_value
            } else {
                0
            }
        }
        OrderSide::Sell => {
            // For sells, impact is when execution price < mid price
            if exec_value < mid_value {
                ((mid_value - exec_value) * 10000) / mid_value
            } else {
                0
            }
        }
    };

    impact_bps as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::order::OrderStatus;

    #[test]
    fn test_market_order_validator() {
        let validator = MarketOrderValidator::new(1, 10000, 500);

        // Valid market order
        let valid_order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            None, // No price for market orders
            1000,
            TimeInForce::IOC,
            1234567890,
        );

        assert!(validator.validate(&valid_order).is_ok());

        // Invalid market order with price
        let invalid_price_order = Order::new(
            2,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            Some(Price::new(50000)), // Market orders shouldn't have price
            1000,
            TimeInForce::IOC,
            1234567890,
        );

        assert!(validator.validate(&invalid_price_order).is_err());

        // Invalid market order with GTC
        let invalid_tif_order = Order::new(
            3,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            None,
            1000,
            TimeInForce::GTC, // Market orders can't use GTC
            1234567890,
        );

        assert!(validator.validate(&invalid_tif_order).is_err());
    }

    #[test]
    fn test_market_order_context() {
        let order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            None,
            1000,
            TimeInForce::IOC,
            1234567890,
        );

        let context = MarketOrderContext::new(&order).unwrap();
        assert_eq!(context.order_id, 1);
        assert_eq!(context.side, OrderSide::Buy);
        assert_eq!(context.quantity, 1000);
        assert_eq!(context.remaining_quantity, 1000);
        assert!(context.can_partial_fill());
        assert!(!context.requires_full_execution());

        let fok_context = MarketOrderContext::new(&order)
            .unwrap()
            .with_max_slippage(100); // 1% max slippage
        assert!(fok_context.requires_full_execution());
        assert_eq!(fok_context.max_slippage_bps, Some(100));
    }

    #[test]
    fn test_market_order_result() {
        let result = MarketOrderResult::fully_filled(
            1,
            Price::new(50000),
            50_000_000, // 50000 * 1000
            3,
        );

        assert_eq!(result.order_id, 1);
        assert!(result.is_filled());
        assert!(result.has_fills());
        assert_eq!(result.filled_quantity, 1000);
        assert_eq!(result.fill_count, 3);

        let partial_result = MarketOrderResult::partially_filled(
            2,
            500,
            Price::new(50500),
            25_250_000, // 50500 * 500
            2,
        )
        .with_slippage(100)
        .with_impact(50);

        assert!(partial_result.is_partially_filled());
        assert_eq!(partial_result.slippage_bps, Some(100));
        assert_eq!(partial_result.impact_bps, Some(50));
        assert_eq!(partial_result.fill_percentage(1000), 0.5);

        let unfilled_result = MarketOrderResult::unfilled(3);
        assert!(!unfilled_result.has_fills());
        assert_eq!(unfilled_result.status, OrderStatus::Cancelled);
    }

    #[test]
    fn test_market_execution_plan() {
        let levels = vec![
            (Price::new(50000), 500),
            (Price::new(50100), 300),
            (Price::new(50200), 1000),
        ];

        let plan = create_execution_plan(
            1,
            OrderSide::Buy,
            1000,
            &levels,
        )
        .unwrap();

        assert_eq!(plan.order_id, 1);
        assert_eq!(plan.original_quantity, 1000);
        assert_eq!(plan.fillable_quantity, 800); // 500 + 300
        assert!(!plan.can_fully_fill());
        assert_eq!(plan.fill_percentage(), 0.8);

        // Test slippage calculation
        let slippage = plan.calculate_slippage(Price::new(50000));
        assert!(slippage.is_some());
        assert!(slippage.unwrap() > 0); // Should have positive slippage for buy
    }

    #[test]
    fn test_market_order_utils() {
        let levels = vec![
            (Price::new(50000), 500),
            (Price::new(50100), 300),
            (Price::new(50200), 1000),
        ];

        // Test max fillable quantity
        let max_fillable = calculate_max_fillable_quantity(
            &levels,
            1000,
            OrderSide::Buy,
        );
        assert_eq!(max_fillable, 800); // 500 + 300

        // Test market impact calculation
        let impact = calculate_market_impact(
            Price::new(50100), // Mid price
            Price::new(50200), // Execution price
            OrderSide::Buy,
        );
        assert_eq!(impact, 199); // ~2% impact

        let impact_sell = calculate_market_impact(
            Price::new(50100),
            Price::new(50000),
            OrderSide::Sell,
        );
        assert_eq!(impact_sell, 199); // ~2% impact
    }
}
