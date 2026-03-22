//! Blockchain Adapter Trait
//! 
//! Abstract interface for blockchain operations
//! Supports multiple networks (Bitcoin, Ethereum, etc.)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crypto_exchange_common::types::Asset;

/// Blockchain transaction representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction identifier
    pub tx_hash: String,
    /// Asset being transferred
    pub asset: Asset,
    /// Amount being transferred (in smallest units)
    pub amount: u128,
    /// Source address
    pub from_address: String,
    /// Destination address
    pub to_address: String,
    /// Network fee paid
    pub network_fee: u128,
    /// Number of confirmations
    pub confirmations: u32,
    /// Block height (if confirmed)
    pub block_height: Option<u64>,
    /// Transaction timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional transaction data
    pub metadata: serde_json::Value,
}

/// Blockchain network information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Network name (e.g., "bitcoin", "ethereum")
    pub name: String,
    /// Current block height
    pub block_height: u64,
    /// Average block time in seconds
    pub average_block_time: u64,
    /// Minimum confirmations for security
    pub min_confirmations: u32,
    /// Network fee estimates
    pub fee_estimates: FeeEstimates,
}

/// Fee estimates for different priority levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeEstimates {
    /// Low priority fee
    pub low: u128,
    /// Medium priority fee
    pub medium: u128,
    /// High priority fee
    pub high: u128,
}

/// Address validation result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressValidation {
    /// Whether address is valid for the network
    pub is_valid: bool,
    /// Address format type
    pub address_type: Option<String>,
    /// Validation error message
    pub error: Option<String>,
}

/// Blockchain adapter trait for different networks
#[async_trait]
pub trait BlockchainAdapter: Send + Sync {
    /// Get network information
    async fn get_network_info(&self) -> anyhow::Result<NetworkInfo>;
    
    /// Get new deposits for monitored addresses
    async fn get_new_deposits(&self, since_block: Option<u64>) -> anyhow::Result<Vec<Transaction>>;
    
    /// Send transaction to network
    async fn send_transaction(&self, tx: &Transaction) -> anyhow::Result<String>;
    
    /// Get transaction by hash
    async fn get_transaction(&self, tx_hash: &str) -> anyhow::Result<Option<Transaction>>;
    
    /// Get current block height
    async fn get_block_height(&self) -> anyhow::Result<u64>;
    
    /// Validate address format for this network
    fn validate_address(&self, address: &str) -> AddressValidation;
    
    /// Generate new deposit address for user
    async fn generate_deposit_address(&self, user_id: u64) -> anyhow::Result<String>;
    
    /// Get balance of an address
    async fn get_address_balance(&self, address: &str) -> anyhow::Result<u128>;
    
    /// Estimate transaction fee
    async fn estimate_fee(&self, priority: FeePriority) -> anyhow::Result<u128>;
    
    /// Get transaction status and confirmations
    async fn get_transaction_status(&self, tx_hash: &str) -> anyhow::Result<Option<TransactionStatus>>;
}

/// Fee priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeePriority {
    Low,
    Medium,
    High,
}

/// Transaction status information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionStatus {
    /// Transaction hash
    pub tx_hash: String,
    /// Whether transaction is confirmed
    pub confirmed: bool,
    /// Number of confirmations
    pub confirmations: u32,
    /// Block height if confirmed
    pub block_height: Option<u64>,
    /// Transaction status
    pub status: TxStatus,
}

/// Transaction status enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    /// Transaction in mempool
    Pending,
    /// Transaction confirmed in block
    Confirmed,
    /// Transaction failed
    Failed,
    /// Transaction not found
    NotFound,
}

/// Blockchain registry for managing multiple adapters
pub struct BlockchainRegistry {
    adapters: std::collections::HashMap<String, Box<dyn BlockchainAdapter>>,
}

impl BlockchainRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
        }
    }

    /// Register blockchain adapter for asset
    pub fn register_adapter(&mut self, asset: Asset, adapter: Box<dyn BlockchainAdapter>) {
        let key = asset.to_string();
        self.adapters.insert(key, adapter);
    }

    /// Get adapter for asset
    pub fn get_adapter(&self, asset: &Asset) -> Option<&dyn BlockchainAdapter> {
        let key = asset.to_string();
        self.adapters.get(&key).map(|adapter| adapter.as_ref())
    }

    /// Get all registered assets
    pub fn supported_assets(&self) -> Vec<&Asset> {
        self.adapters.keys()
            .filter_map(|key| key.parse::<Asset>().ok())
            .collect()
    }
}

impl Default for BlockchainRegistry {
    fn default() -> Self {
        Self::new()
    }
}
