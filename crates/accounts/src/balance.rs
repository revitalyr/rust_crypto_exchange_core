//! Balance management utilities and types.

use crypto_exchange_common::{
    assets::Asset,
    Balance, ExchangeError, ExchangeResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Balance change type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BalanceChangeType {
    /// Deposit
    Deposit,
    /// Withdrawal
    Withdrawal,
    /// Trade execution
    Trade,
    /// Fee payment
    Fee,
    /// Adjustment (manual correction)
    Adjustment,
    /// Transfer in
    TransferIn,
    /// Transfer out
    TransferOut,
    /// Reserve for order
    Reserve,
    /// Release from order
    Release,
}

impl BalanceChangeType {
    /// Checks if the change increases total balance
    pub fn is_inflow(&self) -> bool {
        matches!(
            self,
            BalanceChangeType::Deposit
                | BalanceChangeType::Trade
                | BalanceChangeType::TransferIn
                | BalanceChangeType::Adjustment
                | BalanceChangeType::Release
        )
    }

    /// Checks if the change decreases total balance
    pub fn is_outflow(&self) -> bool {
        matches!(
            self,
            BalanceChangeType::Withdrawal
                | BalanceChangeType::Fee
                | BalanceChangeType::TransferOut
                | BalanceChangeType::Reserve
        )
    }

    /// Checks if the change affects available balance
    pub fn affects_available(&self) -> bool {
        !matches!(self, BalanceChangeType::Reserve | BalanceChangeType::Release)
    }

    /// Checks if the change affects reserved balance
    pub fn affects_reserved(&self) -> bool {
        matches!(self, BalanceChangeType::Reserve | BalanceChangeType::Release)
    }
}

/// Balance change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceChange {
    /// Change type
    pub change_type: BalanceChangeType,
    /// Asset
    pub asset: Asset,
    /// Amount (positive for increases, negative for decreases)
    pub amount: i128,
    /// Available balance before change
    pub available_before: Balance,
    /// Reserved balance before change
    pub reserved_before: Balance,
    /// Available balance after change
    pub available_after: Balance,
    /// Reserved balance after change
    pub reserved_after: Balance,
    /// Timestamp
    pub timestamp: u64,
    /// Reference ID (e.g., transaction ID, order ID)
    pub reference_id: Option<String>,
    /// Reason/memo
    pub reason: Option<String>,
}

impl BalanceChange {
    /// Creates a new balance change
    pub fn new(
        change_type: BalanceChangeType,
        asset: Asset,
        amount: i128,
        available_before: Balance,
        reserved_before: Balance,
        available_after: Balance,
        reserved_after: Balance,
        timestamp: u64,
        reference_id: Option<String>,
        reason: Option<String>,
    ) -> Self {
        Self {
            change_type,
            asset,
            amount,
            available_before,
            reserved_before,
            available_after,
            reserved_after,
            timestamp,
            reference_id,
            reason,
        }
    }

    /// Gets the total balance before change
    pub fn total_before(&self) -> Balance {
        self.available_before.saturating_add(self.reserved_before)
    }

    /// Gets the total balance after change
    pub fn total_after(&self) -> Balance {
        self.available_after.saturating_add(self.reserved_after)
    }

    /// Checks if the change is valid
    pub fn is_valid(&self) -> bool {
        // Check that amounts make sense
        let total_before = self.total_before();
        let total_after = self.total_after();
        
        // For inflows, total should increase
        if self.change_type.is_inflow() && self.amount > 0 {
            total_after >= total_before
        }
        // For outflows, total should decrease
        else if self.change_type.is_outflow() && self.amount < 0 {
            total_after <= total_before
        }
        // For reserve/release, total should stay the same
        else if self.change_type.affects_reserved() {
            total_after == total_before
        } else {
            true
        }
    }
}

/// Balance manager for handling balance changes
pub struct BalanceManager {
    /// Balance change history
    history: Vec<BalanceChange>,
    /// Maximum history size
    max_history_size: usize,
}

impl BalanceManager {
    /// Creates a new balance manager
    pub fn new(max_history_size: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history_size,
        }
    }

    /// Records a balance change
    pub fn record_change(&mut self, change: BalanceChange) -> ExchangeResult<()> {
        if !change.is_valid() {
            return Err(ExchangeError::system_error("Invalid balance change".to_string()));
        }

        self.history.push(change);

        // Trim history if it exceeds maximum size
        if self.history.len() > self.max_history_size {
            self.history.remove(0);
        }

        Ok(())
    }

    /// Gets balance change history
    pub fn get_history(&self) -> &[BalanceChange] {
        &self.history
    }

    /// Gets balance changes for a specific asset
    pub fn get_history_for_asset(&self, asset: &Asset) -> Vec<&BalanceChange> {
        self.history
            .iter()
            .filter(|change| change.asset == *asset)
            .collect()
    }

    /// Gets balance changes by type
    pub fn get_history_by_type(&self, change_type: BalanceChangeType) -> Vec<&BalanceChange> {
        self.history
            .iter()
            .filter(|change| change.change_type == change_type)
            .collect()
    }

    /// Gets balance changes in a time range
    pub fn get_history_in_range(&self, start_time: u64, end_time: u64) -> Vec<&BalanceChange> {
        self.history
            .iter()
            .filter(|change| change.timestamp >= start_time && change.timestamp <= end_time)
            .collect()
    }

    /// Clears balance history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Gets balance statistics for an asset
    pub fn get_asset_stats(&self, asset: &Asset) -> BalanceStats {
        let asset_changes = self.get_history_for_asset(asset);
        
        let total_inflows = asset_changes
            .iter()
            .filter(|change| change.change_type.is_inflow())
            .map(|change| change.amount.max(0) as Balance)
            .sum();

        let total_outflows = asset_changes
            .iter()
            .filter(|change| change.change_type.is_outflow())
            .map(|change| (-change.amount).max(0) as Balance)
            .sum();

        let change_count = asset_changes.len();
        let last_change = asset_changes.last();

        BalanceStats {
            asset: *asset,
            total_inflows,
            total_outflows,
            net_change: total_inflows.saturating_sub(total_outflows),
            change_count,
            last_change_time: last_change.map(|change| change.timestamp),
        }
    }

    /// Gets overall balance statistics
    pub fn get_overall_stats(&self) -> HashMap<Asset, BalanceStats> {
        let mut stats = HashMap::new();
        let mut assets = std::collections::HashSet::new();

        // Collect all unique assets
        for change in &self.history {
            assets.insert(change.asset);
        }

        // Calculate stats for each asset
        for asset in assets {
            stats.insert(asset, self.get_asset_stats(&asset));
        }

        stats
    }
}

/// Balance statistics for an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceStats {
    /// Asset
    pub asset: Asset,
    /// Total inflows
    pub total_inflows: Balance,
    /// Total outflows
    pub total_outflows: Balance,
    /// Net change (inflows - outflows)
    pub net_change: Balance,
    /// Number of changes
    pub change_count: usize,
    /// Last change timestamp
    pub last_change_time: Option<u64>,
}

impl Default for BalanceManager {
    fn default() -> Self {
        Self::new(10000) // Default to 10k history entries
    }
}

/// Balance calculator for complex operations
pub struct BalanceCalculator;

impl BalanceCalculator {
    /// Calculates the required balance for a trade
    pub fn calculate_trade_requirement(
        side: crypto_exchange_common::order::OrderSide,
        quantity: Balance,
        price: u64,
        fee_rate: f64,
    ) -> Balance {
        match side {
            crypto_exchange_common::order::OrderSide::Buy => {
                // For buys: need (quantity * price) + fees
                let cost = price.checked_mul(quantity).unwrap_or(0);
                let fee = (cost as f64 * fee_rate) as Balance;
                cost.saturating_add(fee)
            }
            crypto_exchange_common::order::OrderSide::Sell => {
                // For sells: just need the quantity (fees are deducted from proceeds)
                quantity
            }
        }
    }

    /// Calculates the fee for a trade
    pub fn calculate_trade_fee(
        side: crypto_exchange_common::order::OrderSide,
        quantity: Balance,
        price: u64,
        fee_rate: f64,
    ) -> Balance {
        let trade_value = price.checked_mul(quantity).unwrap_or(0);
        (trade_value as f64 * fee_rate) as Balance
    }

    /// Calculates the net proceeds from a trade
    pub fn calculate_trade_proceeds(
        side: crypto_exchange_common::order::OrderSide,
        quantity: Balance,
        price: u64,
        fee_rate: f64,
    ) -> Balance {
        let gross_proceeds = price.checked_mul(quantity).unwrap_or(0);
        let fee = Self::calculate_trade_fee(side, quantity, price, fee_rate);
        
        match side {
            crypto_exchange_common::order::OrderSide::Buy => {
                // For buys: receives the quantity
                quantity
            }
            crypto_exchange_common::order::OrderSide::Sell => {
                // For sells: receives proceeds minus fees
                gross_proceeds.saturating_sub(fee)
            }
        }
    }

    /// Calculates the maximum order size based on available balance
    pub fn calculate_max_order_size(
        available_balance: Balance,
        price: u64,
        fee_rate: f64,
        side: crypto_exchange_common::order::OrderSide,
    ) -> Balance {
        match side {
            crypto_exchange_common::order::OrderSide::Buy => {
                // For buys: max_size = available_balance / (price * (1 + fee_rate))
                let price_with_fee = price as f64 * (1.0 + fee_rate);
                if price_with_fee > 0.0 {
                    (available_balance as f64 / price_with_fee) as Balance
                } else {
                    0
                }
            }
            crypto_exchange_common::order::OrderSide::Sell => {
                // For sells: can sell all available balance
                available_balance
            }
        }
    }

    /// Calculates the impact of a series of trades
    pub fn calculate_trade_impact(
        trades: &[(Balance, u64, f64)], // (quantity, price, fee_rate)
        side: crypto_exchange_common::order::OrderSide,
    ) -> TradeImpact {
        let mut total_quantity = 0;
        let mut total_cost = 0;
        let mut total_fees = 0;

        for (quantity, price, fee_rate) in trades {
            total_quantity += quantity;
            let trade_cost = price.checked_mul(*quantity).unwrap_or(0);
            total_cost += trade_cost;
            total_fees += Self::calculate_trade_fee(side, *quantity, *price, *fee_rate);
        }

        let avg_price = if total_quantity > 0 {
            total_cost / total_quantity
        } else {
            0
        };

        TradeImpact {
            total_quantity,
            total_cost,
            total_fees,
            avg_price,
            net_proceeds: Self::calculate_trade_proceeds(side, total_quantity, avg_price, 
                if total_cost > 0 { total_fees as f64 / total_cost as f64 } else { 0.0 }),
        }
    }
}

/// Trade impact calculation result
#[derive(Debug, Clone)]
pub struct TradeImpact {
    /// Total quantity traded
    pub total_quantity: Balance,
    /// Total cost (for buys) or proceeds (for sells)
    pub total_cost: Balance,
    /// Total fees paid
    pub total_fees: Balance,
    /// Average price
    pub avg_price: u64,
    /// Net proceeds after fees
    pub net_proceeds: Balance,
}

/// Balance validator for checking balance constraints
pub struct BalanceValidator;

impl BalanceValidator {
    /// Validates balance change constraints
    pub fn validate_balance_change(
        available_before: Balance,
        reserved_before: Balance,
        change_type: BalanceChangeType,
        amount: i128,
    ) -> ExchangeResult<(Balance, Balance)> {
        let total_before = available_before.saturating_add(reserved_before);

        match change_type {
            BalanceChangeType::Deposit => {
                if amount < 0 {
                    return Err(ExchangeError::system_error("Deposit amount must be positive".to_string()));
                }
                let new_available = available_before.checked_add(amount as Balance)
                    .ok_or_else(|| ExchangeError::system_error("Balance overflow".to_string()))?;
                Ok((new_available, reserved_before))
            }
            BalanceChangeType::Withdrawal => {
                if amount < 0 {
                    return Err(ExchangeError::system_error("Withdrawal amount must be positive".to_string()));
                }
                let withdraw_amount = amount as Balance;
                if withdraw_amount > available_before {
                    return Err(ExchangeError::insufficient_balance(available_before, withdraw_amount));
                }
                Ok((available_before - withdraw_amount, reserved_before))
            }
            BalanceChangeType::Reserve => {
                if amount < 0 {
                    return Err(ExchangeError::system_error("Reserve amount must be positive".to_string()));
                }
                let reserve_amount = amount as Balance;
                if reserve_amount > available_before {
                    return Err(ExchangeError::insufficient_balance(available_before, reserve_amount));
                }
                Ok((available_before - reserve_amount, reserved_before + reserve_amount))
            }
            BalanceChangeType::Release => {
                if amount < 0 {
                    return Err(ExchangeError::system_error("Release amount must be positive".to_string()));
                }
                let release_amount = amount as Balance;
                if release_amount > reserved_before {
                    return Err(ExchangeError::system_error("Cannot release more than reserved".to_string()));
                }
                Ok((available_before + release_amount, reserved_before - release_amount))
            }
            BalanceChangeType::Trade | BalanceChangeType::Fee | BalanceChangeType::TransferIn | 
            BalanceChangeType::TransferOut | BalanceChangeType::Adjustment => {
                // These are more complex and would need additional context
                Err(ExchangeError::system_error("Complex balance changes need additional validation".to_string()))
            }
        }
    }

    /// Validates that balances don't overflow
    pub fn validate_no_overflow(available: Balance, reserved: Balance) -> ExchangeResult<()> {
        let _total = available.checked_add(reserved)
            .ok_or_else(|| ExchangeError::system_error("Total balance overflow".to_string()))?;
        Ok(())
    }

    /// Validates that reserved balance doesn't exceed total
    pub fn validate_reserved_not_exceeds_total(available: Balance, reserved: Balance) -> ExchangeResult<()> {
        if reserved > available.saturating_add(reserved) {
            return Err(ExchangeError::system_error("Reserved balance exceeds total".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::order::OrderSide;

    #[test]
    fn test_balance_change_type() {
        assert!(BalanceChangeType::Deposit.is_inflow());
        assert!(!BalanceChangeType::Deposit.is_outflow());
        assert!(BalanceChangeType::Deposit.affects_available());
        assert!(!BalanceChangeType::Deposit.affects_reserved());

        assert!(BalanceChangeType::Withdrawal.is_outflow());
        assert!(!BalanceChangeType::Withdrawal.is_inflow());

        assert!(BalanceChangeType::Reserve.affects_reserved());
        assert!(!BalanceChangeType::Reserve.affects_available());
    }

    #[test]
    fn test_balance_change() {
        let change = BalanceChange::new(
            BalanceChangeType::Deposit,
            Asset::BTC,
            1_000_000,
            0, 0,
            1_000_000, 0,
            1234567890,
            Some("tx_123".to_string()),
            Some("Test deposit".to_string()),
        );

        assert_eq!(change.total_before(), 0);
        assert_eq!(change.total_after(), 1_000_000);
        assert!(change.is_valid());
    }

    #[test]
    fn test_balance_manager() {
        let mut manager = BalanceManager::new(100);
        
        let change = BalanceChange::new(
            BalanceChangeType::Deposit,
            Asset::BTC,
            1_000_000,
            0, 0,
            1_000_000, 0,
            1234567890,
            None,
            None,
        );

        manager.record_change(change).unwrap();
        assert_eq!(manager.get_history().len(), 1);

        let stats = manager.get_asset_stats(&Asset::BTC);
        assert_eq!(stats.total_inflows, 1_000_000);
        assert_eq!(stats.total_outflows, 0);
        assert_eq!(stats.net_change, 1_000_000);
        assert_eq!(stats.change_count, 1);
    }

    #[test]
    fn test_balance_calculator() {
        // Test trade requirement calculation
        let requirement = BalanceCalculator::calculate_trade_requirement(
            OrderSide::Buy,
            1_000_000, // 0.01 BTC
            50000,    // $50,000
            0.002,    // 0.2% fee
        );
        let expected = 50_000_000_000 + 100_000_000; // 500M + 1M (fee)
        assert_eq!(requirement, expected);

        // Test fee calculation
        let fee = BalanceCalculator::calculate_trade_fee(
            OrderSide::Sell,
            1_000_000,
            50000,
            0.002,
        );
        assert_eq!(fee, 100_000_000); // 1M USDT fee

        // Test max order size
        let max_size = BalanceCalculator::calculate_max_order_size(
            100_000_000_000, // 1000 USDT
            50000,
            0.002,
            OrderSide::Buy,
        );
        // Should be slightly less than 2000 due to fees
        assert!(max_size < 2_000_000);
        assert!(max_size > 1_990_000);
    }

    #[test]
    fn test_balance_validator() {
        // Test valid deposit
        let result = BalanceValidator::validate_balance_change(
            1_000_000, 0,
            BalanceChangeType::Deposit,
            500_000,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (1_500_000, 0));

        // Test invalid deposit (negative amount)
        let result = BalanceValidator::validate_balance_change(
            1_000_000, 0,
            BalanceChangeType::Deposit,
            -500_000,
        );
        assert!(result.is_err());

        // Test valid withdrawal
        let result = BalanceValidator::validate_balance_change(
            1_000_000, 0,
            BalanceChangeType::Withdrawal,
            500_000,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (500_000, 0));

        // Test insufficient balance
        let result = BalanceValidator::validate_balance_change(
            500_000, 0,
            BalanceChangeType::Withdrawal,
            1_000_000,
        );
        assert!(result.is_err());

        // Test valid reserve
        let result = BalanceValidator::validate_balance_change(
            1_000_000, 0,
            BalanceChangeType::Reserve,
            300_000,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (700_000, 300_000));

        // Test valid release
        let result = BalanceValidator::validate_balance_change(
            700_000, 300_000,
            BalanceChangeType::Release,
            200_000,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (900_000, 100_000));
    }

    #[test]
    fn test_trade_impact() {
        let trades = vec![
            (1_000_000, 50000, 0.002), // 0.01 BTC at $50,000
            (500_000, 50100, 0.002),  // 0.005 BTC at $50,100
        ];

        let impact = BalanceCalculator::calculate_trade_impact(&trades, OrderSide::Buy);
        
        assert_eq!(impact.total_quantity, 1_500_000);
        assert_eq!(impact.total_cost, 75_050_000_000); // 500M + 250.5M
        assert_eq!(impact.avg_price, 50_033); // Weighted average
        assert!(impact.total_fees > 0);
    }
}
