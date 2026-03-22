//! Withdrawal Pipeline
//! 
//! Handles cryptocurrency withdrawals with risk management and signing

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crypto_exchange_common::types::{Balance, Asset};

/// Represents a cryptocurrency withdrawal request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawalRequest {
    /// Unique withdrawal identifier
    pub id: String,
    /// User ID requesting withdrawal
    pub user_id: u64,
    /// Asset to withdraw
    pub asset: Asset,
    /// Amount to withdraw (in smallest units)
    pub amount: u128,
    /// Destination blockchain address
    pub address: String,
    /// Withdrawal status
    pub status: WithdrawalStatus,
    /// Network fee for the withdrawal
    pub network_fee: u128,
    /// Timestamp when request was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when withdrawal was processed
    pub processed_at: Option<DateTime<Utc>>,
    /// Blockchain transaction hash (after broadcast)
    pub tx_hash: Option<String>,
    /// Risk check results
    pub risk_check: RiskCheckResult,
}

/// Withdrawal status tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WithdrawalStatus {
    /// Request received, pending review
    Pending,
    /// Passed risk checks, being processed
    Processing,
    /// Funds reserved, awaiting signature
    Reserved,
    /// Transaction signed and broadcast to network
    Broadcast,
    /// Transaction confirmed on blockchain
    Confirmed,
    /// Withdrawal failed or rejected
    Failed,
    /// Withdrawal cancelled
    Cancelled,
}

/// Risk check results for withdrawals
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskCheckResult {
    /// Whether withdrawal passed all risk checks
    pub passed: bool,
    /// Risk score (0.0 to 1.0, higher is riskier)
    pub risk_score: f64,
    /// Specific risk factors identified
    pub risk_factors: Vec<RiskFactor>,
    /// Additional notes
    pub notes: String,
}

/// Individual risk factors
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskFactor {
    /// Amount exceeds daily limit
    AmountExceedsDailyLimit,
    /// Amount exceeds single transaction limit
    AmountExceedsSingleLimit,
    /// Address is on watchlist
    AddressOnWatchlist,
    /// User account is new (high risk)
    NewAccount,
    /// Suspicious withdrawal pattern detected
    SuspiciousPattern,
    /// Other risk factor with description
    Other(String),
}

impl WithdrawalRequest {
    /// Create a new withdrawal request
    pub fn new(
        id: String,
        user_id: u64,
        asset: Asset,
        amount: u128,
        address: String,
        network_fee: u128,
    ) -> Self {
        Self {
            id,
            user_id,
            asset,
            amount,
            address,
            status: WithdrawalStatus::Pending,
            network_fee,
            created_at: Utc::now(),
            processed_at: None,
            tx_hash: None,
            risk_check: RiskCheckResult::default(),
        }
    }

    /// Mark withdrawal as processed with transaction hash
    pub fn mark_processed(&mut self, tx_hash: String) {
        self.tx_hash = Some(tx_hash);
        self.status = WithdrawalStatus::Broadcast;
        self.processed_at = Some(Utc::now());
    }

    /// Mark withdrawal as confirmed on blockchain
    pub fn mark_confirmed(&mut self) {
        self.status = WithdrawalStatus::Confirmed;
    }

    /// Check if withdrawal can be processed
    pub fn can_process(&self) -> bool {
        matches!(self.status, WithdrawalStatus::Reserved)
    }

    /// Get total amount deducted from user balance (amount + fee)
    pub fn total_deduction(&self) -> u128 {
        self.amount + self.network_fee
    }
}

impl Default for RiskCheckResult {
    fn default() -> Self {
        Self {
            passed: false,
            risk_score: 0.0,
            risk_factors: vec![],
            notes: String::new(),
        }
    }
}

/// Withdrawal manager interface
pub trait WithdrawalManager {
    /// Create new withdrawal request
    fn create_withdrawal_request(
        &mut self,
        user_id: u64,
        asset: Asset,
        amount: u128,
        address: String,
    ) -> anyhow::Result<String>;

    /// Process withdrawal request (risk checks, reserve funds)
    fn process_withdrawal(&mut self, withdrawal_id: &str) -> anyhow::Result<()>;

    /// Get pending withdrawals for user
    fn get_pending_withdrawals(&self, user_id: u64) -> Vec<&WithdrawalRequest>;

    /// Get all withdrawals
    fn get_all_withdrawals(&self) -> Vec<&WithdrawalRequest>;

    /// Update withdrawal status
    fn update_withdrawal_status(
        &mut self,
        withdrawal_id: &str,
        status: WithdrawalStatus,
        tx_hash: Option<String>,
    ) -> anyhow::Result<()>;
}
