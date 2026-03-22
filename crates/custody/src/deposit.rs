//! Deposit Pipeline
//! 
//! Handles cryptocurrency deposits with confirmation tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crypto_exchange_common::types::{Balance, Asset};

/// Represents a cryptocurrency deposit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deposit {
    /// Unique deposit identifier
    pub id: String,
    /// Blockchain transaction hash
    pub tx_hash: String,
    /// User ID receiving the deposit
    pub user_id: u64,
    /// Asset being deposited
    pub asset: Asset,
    /// Amount being deposited (in smallest units)
    pub amount: u128,
    /// Number of blockchain confirmations
    pub confirmations: u32,
    /// Required confirmations for credit
    pub required_confirmations: u32,
    /// Deposit status
    pub status: DepositStatus,
    /// Timestamp when deposit was first detected
    pub detected_at: DateTime<Utc>,
    /// Timestamp when deposit was confirmed and credited
    pub credited_at: Option<DateTime<Utc>>,
}

/// Deposit status tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepositStatus {
    /// Deposit detected but not confirmed
    Pending,
    /// Partially confirmed
    PartiallyConfirmed,
    /// Fully confirmed and credited to user balance
    Confirmed,
    /// Invalid deposit (wrong asset, insufficient amount, etc.)
    Invalid,
    /// Deposit failed for some reason
    Failed,
}

impl Deposit {
    /// Create a new deposit
    pub fn new(
        id: String,
        tx_hash: String,
        user_id: u64,
        asset: Asset,
        amount: u128,
        required_confirmations: u32,
    ) -> Self {
        Self {
            id,
            tx_hash,
            user_id,
            asset,
            amount,
            confirmations: 0,
            required_confirmations,
            status: DepositStatus::Pending,
            detected_at: Utc::now(),
            credited_at: None,
        }
    }

    /// Add confirmation to deposit
    pub fn add_confirmation(&mut self) -> bool {
        self.confirmations += 1;
        
        if self.confirmations >= self.required_confirmations {
            self.status = DepositStatus::Confirmed;
            self.credited_at = Some(Utc::now());
            return true; // Ready to credit
        }
        
        if self.confirmations > 0 {
            self.status = DepositStatus::PartiallyConfirmed;
        }
        
        false
    }

    /// Check if deposit is ready to be credited
    pub fn is_ready_to_credit(&self) -> bool {
        self.status == DepositStatus::Confirmed
    }

    /// Get confirmation progress (0.0 to 1.0)
    pub fn confirmation_progress(&self) -> f64 {
        if self.required_confirmations == 0 {
            return 1.0;
        }
        (self.confirmations as f64) / (self.required_confirmations as f64)
    }
}

/// Deposit manager interface
pub trait DepositManager {
    /// Process new deposit from blockchain
    fn process_new_deposit(&mut self, deposit: Deposit) -> anyhow::Result<()>;
    
    /// Update deposit confirmations
    fn update_confirmations(&mut self, tx_hash: &str, confirmations: u32) -> anyhow::Result<Option<Deposit>>;
    
    /// Get pending deposits for user
    fn get_pending_deposits(&self, user_id: u64) -> Vec<&Deposit>;
    
    /// Get all deposits
    fn get_all_deposits(&self) -> Vec<&Deposit>;
}
