//! User account management.

use crypto_exchange_common::{
    assets::Asset,
    Balance, ExchangeError, ExchangeResult, UserId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Account status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccountStatus {
    /// Account is active and can trade
    Active,
    /// Account is suspended
    Suspended,
    /// Account is closed
    Closed,
    /// Account is frozen (temporary hold)
    Frozen,
    /// Account is under verification
    PendingVerification,
}

impl AccountStatus {
    /// Checks if account can perform trading operations
    pub fn can_trade(&self) -> bool {
        matches!(self, AccountStatus::Active)
    }

    /// Checks if account can receive deposits
    pub fn can_receive_deposits(&self) -> bool {
        matches!(self, AccountStatus::Active | AccountStatus::Frozen)
    }

    /// Checks if account can make withdrawals
    pub fn can_withdraw(&self) -> bool {
        matches!(self, AccountStatus::Active)
    }
}

/// User account with balances and positions
#[derive(Debug, Clone)]
pub struct Account {
    /// Unique user identifier
    user_id: UserId,
    /// Account status
    status: AccountStatus,
    /// Account creation timestamp
    created_at: u64,
    /// Last updated timestamp
    updated_at: u64,
    /// Wallet balances by asset
    balances: Arc<RwLock<HashMap<Asset, Wallet>>>,
    /// Positions by asset (for margin/futures trading)
    positions: Arc<RwLock<HashMap<Asset, i128>>>,
    /// Account metadata
    metadata: HashMap<String, String>,
}

impl Account {
    /// Creates a new account
    pub fn new(user_id: UserId) -> Self {
        let timestamp = crypto_exchange_common::timestamp::now();
        
        Self {
            user_id,
            status: AccountStatus::Active,
            created_at: timestamp,
            updated_at: timestamp,
            balances: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
            metadata: HashMap::new(),
        }
    }

    /// Gets the user ID
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// Gets the account status
    pub fn status(&self) -> AccountStatus {
        self.status
    }

    /// Sets the account status
    pub fn set_status(&mut self, status: AccountStatus) {
        self.status = status;
        self.updated_at = crypto_exchange_common::timestamp::now();
    }

    /// Gets the creation timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Gets the last updated timestamp
    pub fn updated_at(&self) -> u64 {
        self.updated_at
    }

    /// Gets the available balance for an asset
    pub fn get_available_balance(&self, asset: &Asset) -> Balance {
        let balances = self.balances.read().unwrap();
        balances
            .get(asset)
            .map(|wallet| wallet.available_balance())
            .unwrap_or(0)
    }

    /// Gets the total balance for an asset
    pub fn get_total_balance(&self, asset: &Asset) -> Balance {
        let balances = self.balances.read().unwrap();
        balances
            .get(asset)
            .map(|wallet| wallet.total_balance())
            .unwrap_or(0)
    }

    /// Gets the reserved balance for an asset
    pub fn get_reserved_balance(&self, asset: &Asset) -> Balance {
        let balances = self.balances.read().unwrap();
        balances
            .get(asset)
            .map(|wallet| wallet.reserved_balance())
            .unwrap_or(0)
    }

    /// Sets the balance for an asset
    pub fn set_balance(&mut self, asset: Asset, balance: Balance) {
        let mut balances = self.balances.write().unwrap();
        let wallet = balances.entry(asset).or_insert_with(|| Wallet::new(asset));
        wallet.set_balance(balance);
        self.updated_at = crypto_exchange_common::timestamp::now();
    }

    /// Adds to the available balance
    pub fn add_balance(&mut self, asset: Asset, amount: Balance) -> ExchangeResult<()> {
        let mut balances = self.balances.write().unwrap();
        let wallet = balances.entry(asset).or_insert_with(|| Wallet::new(asset));
        wallet.add_available(amount)?;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Subtracts from the available balance
    pub fn subtract_balance(&mut self, asset: Asset, amount: Balance) -> ExchangeResult<()> {
        let mut balances = self.balances.write().unwrap();
        let wallet = balances.entry(asset).or_insert_with(|| Wallet::new(asset));
        wallet.subtract_available(amount)?;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Reserves balance for orders
    pub fn reserve_balance(&mut self, asset: Asset, amount: Balance) -> ExchangeResult<()> {
        let mut balances = self.balances.write().unwrap();
        let wallet = balances.entry(asset).or_insert_with(|| Wallet::new(asset));
        wallet.reserve(amount)?;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Releases reserved balance
    pub fn release_balance(&mut self, asset: Asset, amount: Balance) -> ExchangeResult<()> {
        let mut balances = self.balances.write().unwrap();
        let wallet = balances.entry(asset).or_insert_with(|| Wallet::new(asset));
        wallet.release(amount)?;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Uses reserved balance (executes trade)
    pub fn use_reserved_balance(&mut self, asset: Asset, amount: Balance) -> ExchangeResult<()> {
        let mut balances = self.balances.write().unwrap();
        let wallet = balances.entry(asset).or_insert_with(|| Wallet::new(asset));
        wallet.use_reserved(amount)?;
        self.updated_at = crypto_exchange_common::timestamp::now();
        Ok(())
    }

    /// Gets the position for an asset
    pub fn get_position(&self, asset: &Asset) -> Balance {
        let positions = self.positions.read().unwrap();
        positions.get(asset).copied().unwrap_or(0) as Balance
    }

    /// Sets the position for an asset
    pub fn set_position(&mut self, asset: Asset, position: i128) {
        let mut positions = self.positions.write().unwrap();
        if position == 0 {
            positions.remove(&asset);
        } else {
            positions.insert(asset, position);
        }
        self.updated_at = crypto_exchange_common::timestamp::now();
    }

    /// Updates position by adding/subtracting
    pub fn update_position(&mut self, asset: Asset, delta: i128) {
        let mut positions = self.positions.write().unwrap();
        let current = positions.get(&asset).copied().unwrap_or(0);
        let new_position = current + delta;
        
        if new_position == 0 {
            positions.remove(&asset);
        } else {
            positions.insert(asset, new_position);
        }
        self.updated_at = crypto_exchange_common::timestamp::now();
    }

    /// Gets all balances
    pub fn get_all_balances(&self) -> HashMap<Asset, Balance> {
        let balances = self.balances.read().unwrap();
        balances
            .iter()
            .map(|(asset, wallet)| (*asset, wallet.total_balance()))
            .collect()
    }

    /// Gets all positions
    pub fn get_all_positions(&self) -> HashMap<Asset, i128> {
        let positions = self.positions.read().unwrap();
        positions.clone()
    }

    /// Gets account metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Sets account metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = crypto_exchange_common::timestamp::now();
    }

    /// Removes account metadata
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        let result = self.metadata.remove(key);
        if result.is_some() {
            self.updated_at = crypto_exchange_common::timestamp::now();
        }
        result
    }

    /// Checks if account has sufficient available balance
    pub fn has_sufficient_balance(&self, asset: &Asset, required_amount: Balance) -> bool {
        self.get_available_balance(asset) >= required_amount
    }

    /// Gets account summary
    pub fn get_summary(&self) -> AccountSummary {
        let balances = self.balances.read().unwrap();
        let positions = self.positions.read().unwrap();
        
        let total_balance_usd = balances
            .iter()
            .filter_map(|(asset, wallet)| {
                // In a real implementation, we would convert to USD using current prices
                // For now, we'll just sum USDT balances
                if *asset == Asset::USDT {
                    Some(wallet.total_balance())
                } else {
                    None
                }
            })
            .sum();

        AccountSummary {
            user_id: self.user_id,
            status: self.status,
            total_balance_usd,
            asset_count: balances.len(),
            position_count: positions.len(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    /// Validates account state
    pub fn validate(&self) -> ExchangeResult<()> {
        let balances = self.balances.read().unwrap();
        
        for (asset, wallet) in balances.iter() {
            wallet.validate()?;
        }

        // Check that positions don't exceed available balances for spot trading
        let positions = self.positions.read().unwrap();
        for (asset, &position) in positions.iter() {
            if let Some(wallet) = balances.get(asset) {
                if position > 0 && position as Balance > wallet.total_balance() {
                    return Err(ExchangeError::system_error(
                        format!("Position exceeds available balance for asset: {:?}", asset)
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Account summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSummary {
    /// User ID
    pub user_id: UserId,
    /// Account status
    pub status: AccountStatus,
    /// Total balance in USD (approximate)
    pub total_balance_usd: Balance,
    /// Number of different assets
    pub asset_count: usize,
    /// Number of open positions
    pub position_count: usize,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
}

/// Account manager for handling multiple accounts
pub struct AccountManager {
    /// Accounts by user ID
    accounts: Arc<RwLock<HashMap<UserId, Account>>>,
    /// Next account ID to assign (for auto-generation)
    next_user_id: Arc<RwLock<UserId>>,
}

impl AccountManager {
    /// Creates a new account manager
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            next_user_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Creates a new account
    pub fn create_account(&self) -> ExchangeResult<UserId> {
        let user_id = {
            let mut next_id = self.next_user_id.write().unwrap();
            let id = *next_id;
            *next_id = id + 1;
            id
        };

        let account = Account::new(user_id);
        let mut accounts = self.accounts.write().unwrap();
        
        if accounts.contains_key(&user_id) {
            return Err(ExchangeError::system_error("Account ID already exists".to_string()));
        }
        
        accounts.insert(user_id, account);
        Ok(user_id)
    }

    /// Gets an account by user ID
    pub fn get_account(&self, user_id: UserId) -> Option<Account> {
        let accounts = self.accounts.read().unwrap();
        accounts.get(&user_id).cloned()
    }

    /// Updates an account
    pub fn update_account<F>(&self, user_id: UserId, updater: F) -> ExchangeResult<()>
    where
        F: FnOnce(&mut Account) -> ExchangeResult<()>,
    {
        let mut accounts = self.accounts.write().unwrap();
        if let Some(account) = accounts.get_mut(&user_id) {
            updater(account)?;
            Ok(())
        } else {
            Err(ExchangeError::account_not_found(user_id))
        }
    }

    /// Deletes an account
    pub fn delete_account(&self, user_id: UserId) -> ExchangeResult<()> {
        let mut accounts = self.accounts.write().unwrap();
        if accounts.remove(&user_id).is_some() {
            Ok(())
        } else {
            Err(ExchangeError::account_not_found(user_id))
        }
    }

    /// Gets all accounts
    pub fn get_all_accounts(&self) -> Vec<Account> {
        let accounts = self.accounts.read().unwrap();
        accounts.values().cloned().collect()
    }

    /// Gets accounts by status
    pub fn get_accounts_by_status(&self, status: AccountStatus) -> Vec<Account> {
        let accounts = self.accounts.read().unwrap();
        accounts
            .values()
            .filter(|account| account.status() == status)
            .cloned()
            .collect()
    }

    /// Suspends an account
    pub fn suspend_account(&self, user_id: UserId, reason: &str) -> ExchangeResult<()> {
        self.update_account(user_id, |account| {
            account.set_status(AccountStatus::Suspended);
            account.set_metadata("suspension_reason".to_string(), reason.to_string());
            Ok(())
        })
    }

    /// Activates an account
    pub fn activate_account(&self, user_id: UserId) -> ExchangeResult<()> {
        self.update_account(user_id, |account| {
            account.set_status(AccountStatus::Active);
            account.remove_metadata("suspension_reason");
            Ok(())
        })
    }

    /// Freezes an account
    pub fn freeze_account(&self, user_id: UserId, reason: &str) -> ExchangeResult<()> {
        self.update_account(user_id, |account| {
            account.set_status(AccountStatus::Frozen);
            account.set_metadata("freeze_reason".to_string(), reason.to_string());
            Ok(())
        })
    }

    /// Gets account statistics
    pub fn get_stats(&self) -> AccountManagerStats {
        let accounts = self.accounts.read().unwrap();
        
        let total_accounts = accounts.len() as u64;
        let active_accounts = accounts.values().filter(|a| a.status() == AccountStatus::Active).count() as u64;
        let suspended_accounts = accounts.values().filter(|a| a.status() == AccountStatus::Suspended).count() as u64;
        let frozen_accounts = accounts.values().filter(|a| a.status() == AccountStatus::Frozen).count() as u64;

        AccountManagerStats {
            total_accounts,
            active_accounts,
            suspended_accounts,
            frozen_accounts,
            next_user_id: *self.next_user_id.read().unwrap(),
        }
    }
}

/// Account manager statistics
#[derive(Debug, Clone)]
pub struct AccountManagerStats {
    /// Total number of accounts
    pub total_accounts: u64,
    /// Number of active accounts
    pub active_accounts: u64,
    /// Number of suspended accounts
    pub suspended_accounts: u64,
    /// Number of frozen accounts
    pub frozen_accounts: u64,
    /// Next user ID to be assigned
    pub next_user_id: UserId,
}

impl Default for AccountManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_creation() {
        let account = Account::new(100);
        
        assert_eq!(account.user_id(), 100);
        assert_eq!(account.status(), AccountStatus::Active);
        assert!(account.created_at() > 0);
        assert_eq!(account.get_available_balance(&Asset::BTC), 0);
    }

    #[test]
    fn test_balance_operations() {
        let mut account = Account::new(100);
        
        // Test setting balance
        account.set_balance(Asset::BTC, 1_000_000); // 0.01 BTC
        assert_eq!(account.get_total_balance(&Asset::BTC), 1_000_000);
        assert_eq!(account.get_available_balance(&Asset::BTC), 1_000_000);
        assert_eq!(account.get_reserved_balance(&Asset::BTC), 0);

        // Test adding balance
        account.add_balance(Asset::BTC, 500_000).unwrap(); // 0.005 BTC
        assert_eq!(account.get_total_balance(&Asset::BTC), 1_500_000);

        // Test reserving balance
        account.reserve_balance(Asset::BTC, 200_000).unwrap(); // 0.002 BTC
        assert_eq!(account.get_available_balance(&Asset::BTC), 1_300_000);
        assert_eq!(account.get_reserved_balance(&Asset::BTC), 200_000);

        // Test using reserved balance
        account.use_reserved_balance(Asset::BTC, 100_000).unwrap(); // 0.001 BTC
        assert_eq!(account.get_total_balance(&Asset::BTC), 1_400_000);
        assert_eq!(account.get_reserved_balance(&Asset::BTC), 100_000);

        // Test releasing reserved balance
        account.release_balance(Asset::BTC, 100_000).unwrap();
        assert_eq!(account.get_available_balance(&Asset::BTC), 1_400_000);
        assert_eq!(account.get_reserved_balance(&Asset::BTC), 0);
    }

    #[test]
    fn test_position_operations() {
        let mut account = Account::new(100);
        
        // Test setting position
        account.set_position(Asset::BTC, 1_000_000); // 0.01 BTC
        assert_eq!(account.get_position(&Asset::BTC), 1_000_000);

        // Test updating position
        account.update_position(Asset::BTC, 500_000); // Add 0.005 BTC
        assert_eq!(account.get_position(&Asset::BTC), 1_500_000);

        account.update_position(Asset::BTC, -2_000_000); // Subtract 0.02 BTC
        assert_eq!(account.get_position(&Asset::BTC), -500_000);

        // Test removing position
        account.set_position(Asset::BTC, 0);
        assert_eq!(account.get_position(&Asset::BTC), 0);
    }

    #[test]
    fn test_account_status() {
        let mut account = Account::new(100);
        
        assert!(account.status().can_trade());
        assert!(account.status().can_receive_deposits());
        assert!(account.status().can_withdraw());

        account.set_status(AccountStatus::Suspended);
        assert!(!account.status().can_trade());
        assert!(!account.status().can_receive_deposits());
        assert!(!account.status().can_withdraw());

        account.set_status(AccountStatus::Frozen);
        assert!(!account.status().can_trade());
        assert!(account.status().can_receive_deposits());
        assert!(!account.status().can_withdraw());
    }

    #[test]
    fn test_account_metadata() {
        let mut account = Account::new(100);
        
        account.set_metadata("email".to_string(), "test@example.com".to_string());
        assert_eq!(account.get_metadata("email"), Some(&"test@example.com".to_string()));
        
        account.remove_metadata("email");
        assert_eq!(account.get_metadata("email"), None);
    }

    #[test]
    fn test_account_manager() {
        let manager = AccountManager::new();
        
        // Test creating account
        let user_id = manager.create_account().unwrap();
        assert!(user_id > 0);

        // Test getting account
        let account = manager.get_account(user_id).unwrap();
        assert_eq!(account.user_id(), user_id);

        // Test updating account
        manager.update_account(user_id, |account| {
            account.set_balance(Asset::BTC, 1_000_000);
            Ok(())
        }).unwrap();

        let updated_account = manager.get_account(user_id).unwrap();
        assert_eq!(updated_account.get_total_balance(&Asset::BTC), 1_000_000);

        // Test suspending account
        manager.suspend_account(user_id, "Test suspension").unwrap();
        let suspended_account = manager.get_account(user_id).unwrap();
        assert_eq!(suspended_account.status(), AccountStatus::Suspended);
        assert_eq!(suspended_account.get_metadata("suspension_reason"), Some(&"Test suspension".to_string()));

        // Test activating account
        manager.activate_account(user_id).unwrap();
        let active_account = manager.get_account(user_id).unwrap();
        assert_eq!(active_account.status(), AccountStatus::Active);
        assert_eq!(active_account.get_metadata("suspension_reason"), None);

        // Test deleting account
        manager.delete_account(user_id).unwrap();
        assert!(manager.get_account(user_id).is_none());
    }

    #[test]
    fn test_account_manager_stats() {
        let manager = AccountManager::new();
        
        // Create some accounts
        let user1 = manager.create_account().unwrap();
        let user2 = manager.create_account().unwrap();
        let user3 = manager.create_account().unwrap();

        // Suspend one account
        manager.suspend_account(user1, "Test").unwrap();

        // Freeze one account
        manager.freeze_account(user2, "Test").unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_accounts, 3);
        assert_eq!(stats.active_accounts, 1);
        assert_eq!(stats.suspended_accounts, 1);
        assert_eq!(stats.frozen_accounts, 1);
        assert_eq!(stats.next_user_id, 4);
    }

    #[test]
    fn test_account_validation() {
        let mut account = Account::new(100);
        
        // Should pass with empty account
        assert!(account.validate().is_ok());

        // Add some balances
        account.set_balance(Asset::BTC, 1_000_000);
        assert!(account.validate().is_ok());

        // Add position that exceeds balance (should fail)
        account.set_position(Asset::BTC, 2_000_000);
        assert!(account.validate().is_err());
    }
}
