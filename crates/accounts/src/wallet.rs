//! Wallet implementation for managing asset balances.

use crypto_exchange_common::{
    assets::Asset,
    Balance, ExchangeError, ExchangeResult,
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Wallet for managing balances of a single asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// Asset type
    pub asset: Asset,
    /// Available balance (can be used for trading)
    available_balance: Balance,
    /// Reserved balance (locked in orders, pending withdrawals, etc.)
    reserved_balance: Balance,
    /// Last updated timestamp
    updated_at: u64,
}

impl Wallet {
    /// Creates a new wallet with zero balance
    pub fn new(asset: Asset) -> Self {
        Self {
            asset,
            available_balance: 0,
            reserved_balance: 0,
            updated_at: crypto_exchange_common::timestamp::now(),
        }
    }

    /// Creates a wallet with initial balance
    pub fn with_balance(asset: Asset, balance: Balance) -> Self {
        Self {
            asset,
            available_balance: balance,
            reserved_balance: 0,
            updated_at: crypto_exchange_common::timestamp::now(),
        }
    }

    /// Gets the asset
    pub fn asset(&self) -> Asset {
        self.asset
    }

    /// Gets the available balance
    pub fn available_balance(&self) -> Balance {
        self.available_balance
    }

    /// Gets the reserved balance
    pub fn reserved_balance(&self) -> Balance {
        self.reserved_balance
    }

    /// Gets the total balance (available + reserved)
    pub fn total_balance(&self) -> Balance {
        self.available_balance.saturating_add(self.reserved_balance)
    }

    /// Gets the last updated timestamp
    pub fn updated_at(&self) -> u64 {
        self.updated_at
    }

    /// Sets the available balance
    pub fn set_balance(&mut self, balance: Balance) {
        self.available_balance = balance;
        self.updated_at = crypto_exchange_common::timestamp::now();
    }

    /// Adds to available balance
    pub fn add_available(&mut self, amount: Balance) -> ExchangeResult<()> {
        let new_balance = self.available_balance.checked_add(amount)
            .ok_or_else(|| ExchangeError::system_error("Balance overflow".to_string()))?;
        
        self.available_balance = new_balance;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Subtracts from available balance
    pub fn subtract_available(&mut self, amount: Balance) -> ExchangeResult<()> {
        if amount > self.available_balance {
            return Err(ExchangeError::insufficient_balance(
                self.available_balance,
                amount,
            ));
        }

        self.available_balance -= amount;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Reserves balance (moves from available to reserved)
    pub fn reserve(&mut self, amount: Balance) -> ExchangeResult<()> {
        if amount > self.available_balance {
            return Err(ExchangeError::insufficient_balance(
                self.available_balance,
                amount,
            ));
        }

        self.available_balance -= amount;
        self.reserved_balance += amount;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Releases reserved balance (moves from reserved to available)
    pub fn release(&mut self, amount: Balance) -> ExchangeResult<()> {
        if amount > self.reserved_balance {
            return Err(ExchangeError::system_error(
                "Cannot release more than reserved balance".to_string()
            ));
        }

        self.reserved_balance -= amount;
        self.available_balance += amount;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Uses reserved balance (decreases both total and reserved)
    pub fn use_reserved(&mut self, amount: Balance) -> ExchangeResult<()> {
        if amount > self.reserved_balance {
            return Err(ExchangeError::system_error(
                "Cannot use more than reserved balance".to_string()
            ));
        }

        self.reserved_balance -= amount;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Directly adds to reserved balance (for deposits to pending withdrawals, etc.)
    pub fn add_reserved(&mut self, amount: Balance) -> ExchangeResult<()> {
        let new_reserved = self.reserved_balance.checked_add(amount)
            .ok_or_else(|| ExchangeError::system_error("Reserved balance overflow".to_string()))?;
        
        self.reserved_balance = new_reserved;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Directly subtracts from reserved balance
    pub fn subtract_reserved(&mut self, amount: Balance) -> ExchangeResult<()> {
        if amount > self.reserved_balance {
            return Err(ExchangeError::system_error(
                "Cannot subtract more than reserved balance".to_string()
            ));
        }

        self.reserved_balance -= amount;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Transfers amount to another wallet
    pub fn transfer_to(&mut self, other: &mut Wallet, amount: Balance) -> ExchangeResult<()> {
        if self.asset != other.asset {
            return Err(ExchangeError::system_error(
                "Cannot transfer between different assets".to_string()
            ));
        }

        self.subtract_available(amount)?;
        other.add_available(amount)?;
        Ok(())
    }

    /// Checks if wallet has sufficient available balance
    pub fn has_available(&self, amount: Balance) -> bool {
        self.available_balance >= amount
    }

    /// Checks if wallet has sufficient total balance
    pub fn has_total(&self, amount: Balance) -> bool {
        self.total_balance() >= amount
    }

    /// Gets wallet summary
    pub fn summary(&self) -> WalletSummary {
        WalletSummary {
            asset: self.asset,
            available_balance: self.available_balance,
            reserved_balance: self.reserved_balance,
            total_balance: self.total_balance(),
            updated_at: self.updated_at,
        }
    }

    /// Validates wallet state
    pub fn validate(&self) -> ExchangeResult<()> {
        // Check for overflow in total balance
        let _total = self.available_balance.checked_add(self.reserved_balance)
            .ok_or_else(|| ExchangeError::system_error("Total balance overflow".to_string()))?;

        // Check that reserved balance doesn't exceed reasonable limits
        if self.reserved_balance > self.total_balance() {
            return Err(ExchangeError::system_error(
                "Reserved balance exceeds total balance".to_string()
            ));
        }

        Ok(())
    }

    /// Formats balance for display
    pub fn format_balance(&self, balance_type: BalanceType) -> String {
        let balance = match balance_type {
            BalanceType::Available => self.available_balance,
            BalanceType::Reserved => self.reserved_balance,
            BalanceType::Total => self.total_balance(),
        };

        crypto_exchange_common::balance::format(balance, self.asset.decimals())
    }

    /// Gets balance in different units
    pub fn get_balance_units(&self) -> BalanceUnits {
        BalanceUnits {
            asset: self.asset,
            available: self.available_balance,
            reserved: self.reserved_balance,
            total: self.total_balance(),
            available_display: self.format_balance(BalanceType::Available),
            reserved_display: self.format_balance(BalanceType::Reserved),
            total_display: self.format_balance(BalanceType::Total),
        }
    }
}

/// Balance type for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceType {
    Available,
    Reserved,
    Total,
}

/// Wallet summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSummary {
    /// Asset type
    pub asset: Asset,
    /// Available balance
    pub available_balance: Balance,
    /// Reserved balance
    pub reserved_balance: Balance,
    /// Total balance
    pub total_balance: Balance,
    /// Last updated timestamp
    pub updated_at: u64,
}

/// Balance units with formatted display strings
#[derive(Debug, Clone)]
pub struct BalanceUnits {
    /// Asset type
    pub asset: Asset,
    /// Available balance in smallest units
    pub available: Balance,
    /// Reserved balance in smallest units
    pub reserved: Balance,
    /// Total balance in smallest units
    pub total: Balance,
    /// Formatted available balance
    pub available_display: String,
    /// Formatted reserved balance
    pub reserved_display: String,
    /// Formatted total balance
    pub total_display: String,
}

impl fmt::Display for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Wallet[{}: available={}, reserved={}, total={}]",
            self.asset,
            self.format_balance(BalanceType::Available),
            self.format_balance(BalanceType::Reserved),
            self.format_balance(BalanceType::Total)
        )
    }
}

/// Multi-asset wallet manager
#[derive(Debug, Clone)]
pub struct MultiWallet {
    /// Individual wallets by asset
    wallets: std::collections::HashMap<Asset, Wallet>,
}

impl MultiWallet {
    /// Creates a new multi-wallet
    pub fn new() -> Self {
        Self {
            wallets: std::collections::HashMap::new(),
        }
    }

    /// Gets or creates a wallet for an asset
    pub fn get_or_create_wallet(&mut self, asset: Asset) -> &mut Wallet {
        self.wallets.entry(asset).or_insert_with(|| Wallet::new(asset))
    }

    /// Gets a wallet for an asset
    pub fn get_wallet(&self, asset: &Asset) -> Option<&Wallet> {
        self.wallets.get(asset)
    }

    /// Gets a mutable wallet for an asset
    pub fn get_wallet_mut(&mut self, asset: &Asset) -> Option<&mut Wallet> {
        self.wallets.get_mut(asset)
    }

    /// Sets balance for an asset
    pub fn set_balance(&mut self, asset: Asset, balance: Balance) {
        let wallet = self.get_or_create_wallet(asset);
        wallet.set_balance(balance);
    }

    /// Adds balance for an asset
    pub fn add_balance(&mut self, asset: Asset, amount: Balance) -> ExchangeResult<()> {
        let wallet = self.get_or_create_wallet(asset);
        wallet.add_available(amount)
    }

    /// Subtracts balance for an asset
    pub fn subtract_balance(&mut self, asset: Asset, amount: Balance) -> ExchangeResult<()> {
        if let Some(wallet) = self.get_wallet_mut(&asset) {
            wallet.subtract_available(amount)
        } else {
            Err(ExchangeError::insufficient_balance(0, amount))
        }
    }

    /// Gets available balance for an asset
    pub fn get_available_balance(&self, asset: &Asset) -> Balance {
        self.wallets
            .get(asset)
            .map(|wallet| wallet.available_balance())
            .unwrap_or(0)
    }

    /// Gets total balance for an asset
    pub fn get_total_balance(&self, asset: &Asset) -> Balance {
        self.wallets
            .get(asset)
            .map(|wallet| wallet.total_balance())
            .unwrap_or(0)
    }

    /// Gets all assets
    pub fn get_assets(&self) -> Vec<Asset> {
        self.wallets.keys().copied().collect()
    }

    /// Gets all wallet summaries
    pub fn get_summaries(&self) -> Vec<WalletSummary> {
        self.wallets
            .values()
            .map(|wallet| wallet.summary())
            .collect()
    }

    /// Calculates total USD value (approximate, using USDT as USD)
    pub fn get_total_usd_value(&self) -> Balance {
        self.wallets
            .iter()
            .filter_map(|(asset, wallet)| {
                if *asset == Asset::USDT {
                    Some(wallet.total_balance())
                } else {
                    // In a real implementation, we would convert using current prices
                    None
                }
            })
            .sum()
    }

    /// Removes a wallet if it has zero balance
    pub fn prune_empty_wallets(&mut self) {
        self.wallets.retain(|_, wallet| wallet.total_balance() > 0);
    }

    /// Validates all wallets
    pub fn validate(&self) -> ExchangeResult<()> {
        for wallet in self.wallets.values() {
            wallet.validate()?;
        }
        Ok(())
    }

    /// Gets multi-wallet summary
    pub fn summary(&self) -> MultiWalletSummary {
        MultiWalletSummary {
            asset_count: self.wallets.len(),
            total_usd_value: self.get_total_usd_value(),
            summaries: self.get_summaries(),
        }
    }
}

/// Multi-wallet summary
#[derive(Debug, Clone)]
pub struct MultiWalletSummary {
    /// Number of different assets
    pub asset_count: usize,
    /// Total USD value (approximate)
    pub total_usd_value: Balance,
    /// Individual wallet summaries
    pub summaries: Vec<WalletSummary>,
}

impl Default for MultiWallet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new(Asset::BTC);
        
        assert_eq!(wallet.asset(), Asset::BTC);
        assert_eq!(wallet.available_balance(), 0);
        assert_eq!(wallet.reserved_balance(), 0);
        assert_eq!(wallet.total_balance(), 0);
    }

    #[test]
    fn test_wallet_with_balance() {
        let wallet = Wallet::with_balance(Asset::BTC, 1_000_000);
        
        assert_eq!(wallet.asset(), Asset::BTC);
        assert_eq!(wallet.available_balance(), 1_000_000);
        assert_eq!(wallet.reserved_balance(), 0);
        assert_eq!(wallet.total_balance(), 1_000_000);
    }

    #[test]
    fn test_wallet_balance_operations() {
        let mut wallet = Wallet::new(Asset::BTC);
        
        // Test adding balance
        wallet.add_available(1_000_000).unwrap();
        assert_eq!(wallet.available_balance(), 1_000_000);
        assert_eq!(wallet.total_balance(), 1_000_000);

        // Test subtracting balance
        wallet.subtract_available(200_000).unwrap();
        assert_eq!(wallet.available_balance(), 800_000);
        assert_eq!(wallet.total_balance(), 800_000);

        // Test reserving balance
        wallet.reserve(300_000).unwrap();
        assert_eq!(wallet.available_balance(), 500_000);
        assert_eq!(wallet.reserved_balance(), 300_000);
        assert_eq!(wallet.total_balance(), 800_000);

        // Test releasing reserved balance
        wallet.release(100_000).unwrap();
        assert_eq!(wallet.available_balance(), 600_000);
        assert_eq!(wallet.reserved_balance(), 200_000);
        assert_eq!(wallet.total_balance(), 800_000);

        // Test using reserved balance
        wallet.use_reserved(150_000).unwrap();
        assert_eq!(wallet.available_balance(), 600_000);
        assert_eq!(wallet.reserved_balance(), 50_000);
        assert_eq!(wallet.total_balance(), 650_000);
    }

    #[test]
    fn test_wallet_insufficient_balance() {
        let mut wallet = Wallet::with_balance(Asset::BTC, 1_000_000);
        
        // Test insufficient available balance
        assert!(wallet.subtract_available(2_000_000).is_err());
        assert!(wallet.reserve(2_000_000).is_err());

        // Reserve some balance first
        wallet.reserve(500_000).unwrap();
        
        // Test insufficient reserved balance
        assert!(wallet.release(600_000).is_err());
        assert!(wallet.use_reserved(600_000).is_err());
    }

    #[test]
    fn test_wallet_transfer() {
        let mut wallet1 = Wallet::with_balance(Asset::BTC, 1_000_000);
        let mut wallet2 = Wallet::new(Asset::BTC);
        
        // Test successful transfer
        wallet1.transfer_to(&mut wallet2, 300_000).unwrap();
        assert_eq!(wallet1.available_balance(), 700_000);
        assert_eq!(wallet2.available_balance(), 300_000);

        // Test transfer between different assets (should fail)
        let mut wallet3 = Wallet::new(Asset::USDT);
        assert!(wallet1.transfer_to(&mut wallet3, 100_000).is_err());
    }

    #[test]
    fn test_wallet_validation() {
        let wallet = Wallet::with_balance(Asset::BTC, 1_000_000);
        assert!(wallet.validate().is_ok());

        // Test with very large values (potential overflow)
        let mut large_wallet = Wallet::new(Asset::BTC);
        large_wallet.available_balance = u128::MAX / 2;
        large_wallet.reserved_balance = u128::MAX / 2;
        assert!(large_wallet.validate().is_err());
    }

    #[test]
    fn test_wallet_display() {
        let wallet = Wallet::with_balance(Asset::BTC, 1_000_000);
        let display = format!("{}", wallet);
        assert!(display.contains("BTC"));
        assert!(display.contains("available"));
        assert!(display.contains("reserved"));
        assert!(display.contains("total"));
    }

    #[test]
    fn test_multi_wallet() {
        let mut multi_wallet = MultiWallet::new();
        
        // Test adding balances
        multi_wallet.add_balance(Asset::BTC, 1_000_000).unwrap();
        multi_wallet.add_balance(Asset::USDT, 50_000_000).unwrap();
        
        assert_eq!(multi_wallet.get_available_balance(&Asset::BTC), 1_000_000);
        assert_eq!(multi_wallet.get_available_balance(&Asset::USDT), 50_000_000);
        assert_eq!(multi_wallet.get_available_balance(&Asset::ETH), 0);

        // Test getting assets
        let assets = multi_wallet.get_assets();
        assert_eq!(assets.len(), 2);
        assert!(assets.contains(&Asset::BTC));
        assert!(assets.contains(&Asset::USDT));

        // Test summaries
        let summaries = multi_wallet.get_summaries();
        assert_eq!(summaries.len(), 2);

        // Test total USD value
        let total_usd = multi_wallet.get_total_usd_value();
        assert_eq!(total_usd, 50_000_000); // Only USDT counts as USD

        // Test validation
        assert!(multi_wallet.validate().is_ok());
    }

    #[test]
    fn test_multi_wallet_pruning() {
        let mut multi_wallet = MultiWallet::new();
        
        // Add some balances
        multi_wallet.add_balance(Asset::BTC, 1_000_000).unwrap();
        multi_wallet.add_balance(Asset::USDT, 50_000_000).unwrap();
        multi_wallet.add_balance(Asset::ETH, 0).unwrap(); // Zero balance
        
        assert_eq!(multi_wallet.get_assets().len(), 3);
        
        // Prune empty wallets
        multi_wallet.prune_empty_wallets();
        assert_eq!(multi_wallet.get_assets().len(), 2);
        assert!(!multi_wallet.get_assets().contains(&Asset::ETH));
    }

    #[test]
    fn test_balance_units() {
        let mut wallet = Wallet::new(Asset::BTC);
        wallet.add_available(1_000_000).unwrap();
        wallet.reserve(200_000).unwrap();
        
        let units = wallet.get_balance_units();
        assert_eq!(units.available, 1_000_000);
        assert_eq!(units.reserved, 200_000);
        assert_eq!(units.total, 800_000);
        assert!(units.available_display.contains("0.01")); // BTC has 8 decimals
    }
}
