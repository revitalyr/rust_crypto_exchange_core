//! Mock Blockchain Implementation
//! 
//! Simulated blockchain for testing and development
//! Provides realistic behavior without external dependencies

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use anyhow::Result;

use super::{
    BlockchainAdapter, Transaction, NetworkInfo, AddressValidation, 
    TransactionStatus, TxStatus, FeeEstimates, FeePriority
};
use crypto_exchange_common::types::Asset;

/// Mock blockchain implementation
pub struct MockBlockchain {
    asset: Asset,
    network_name: String,
    current_block: Arc<RwLock<u64>>,
    transactions: Arc<RwLock<HashMap<String, Transaction>>>,
    deposit_addresses: Arc<RwLock<HashMap<u64, String>>>,
    balances: Arc<RwLock<HashMap<String, u128>>>,
}

impl MockBlockchain {
    /// Create new mock blockchain
    pub fn new(asset: Asset) -> Self {
        let network_name = match asset {
            Asset::BTC => "bitcoin",
            Asset::ETH => "ethereum",
            Asset::USDT => "ethereum",
            Asset::USDC => "ethereum",
            Asset::Custom(name) => name,
        }.to_string();

        Self {
            asset,
            network_name,
            current_block: Arc::new(RwLock::new(1000000)),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            deposit_addresses: Arc::new(RwLock::new(HashMap::new())),
            balances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Simulate block generation
    pub async fn start_block_generation(&self) {
        let current_block = self.current_block.clone();
        let transactions = self.transactions.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                // Increment block
                {
                    let mut block = current_block.write().await;
                    *block += 1;
                    
                    // Update confirmations for existing transactions
                    let mut txs = transactions.write().await;
                    for tx in txs.values_mut() {
                        if tx.confirmations < 6 {
                            tx.confirmations += 1;
                        }
                    }
                }
            }
        });
    }

    /// Generate mock deposit transaction
    pub async fn generate_deposit(&self, user_id: u64, amount: u128) -> Result<String> {
        let addresses = self.deposit_addresses.read().await;
        let to_address = addresses.get(&user_id).cloned()
            .unwrap_or_else(|| format!("mock_address_{}", user_id));
        
        let tx_hash = format!("mock_tx_{}", Uuid::new_v4());
        let current_height = *self.current_block.read().await;
        
        let transaction = Transaction {
            tx_hash: tx_hash.clone(),
            asset: self.asset,
            amount,
            from_address: "mock_sender_address".to_string(),
            to_address,
            network_fee: 1000, // Mock fee
            confirmations: 0,
            block_height: Some(current_height + 1),
            timestamp: Utc::now(),
            metadata: serde_json::json!({"mock": true}),
        };
        
        let mut transactions = self.transactions.write().await;
        transactions.insert(tx_hash.clone(), transaction);
        
        Ok(tx_hash)
    }
}

#[async_trait]
impl BlockchainAdapter for MockBlockchain {
    async fn get_network_info(&self) -> Result<NetworkInfo> {
        let current_height = *self.current_block.read().await;
        
        Ok(NetworkInfo {
            name: self.network_name.clone(),
            block_height: current_height,
            average_block_time: 10, // 10 seconds for mock
            min_confirmations: 3,
            fee_estimates: FeeEstimates {
                low: 1000,
                medium: 2000,
                high: 5000,
            },
        })
    }

    async fn get_new_deposits(&self, since_block: Option<u64>) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read().await;
        let current_height = *self.current_block.read().await;
        
        let since = since_block.unwrap_or(0);
        
        Ok(transactions
            .values()
            .filter(|tx| {
                // Filter for deposits (incoming transactions)
                tx.block_height.map_or(false, |height| height > since)
            })
            .cloned()
            .collect())
    }

    async fn send_transaction(&self, tx: &Transaction) -> Result<String> {
        // Simulate transaction broadcasting
        let current_height = *self.current_block.read().await;
        
        let mut tx_copy = tx.clone();
        tx_copy.block_height = Some(current_height + 1);
        tx_copy.confirmations = 0;
        
        let mut transactions = self.transactions.write().await;
        transactions.insert(tx.tx_hash.clone(), tx_copy);
        
        Ok(tx.tx_hash.clone())
    }

    async fn get_transaction(&self, tx_hash: &str) -> Result<Option<Transaction>> {
        let transactions = self.transactions.read().await;
        Ok(transactions.get(tx_hash).cloned())
    }

    async fn get_block_height(&self) -> Result<u64> {
        Ok(*self.current_block.read().await)
    }

    fn validate_address(&self, address: &str) -> AddressValidation {
        // Simple validation for mock addresses
        if address.starts_with("mock_address_") || address.len() > 10 {
            AddressValidation {
                is_valid: true,
                address_type: Some("mock".to_string()),
                error: None,
            }
        } else {
            AddressValidation {
                is_valid: false,
                address_type: None,
                error: Some("Invalid mock address format".to_string()),
            }
        }
    }

    async fn generate_deposit_address(&self, user_id: u64) -> Result<String> {
        let address = format!("mock_address_{}", user_id);
        let mut addresses = self.deposit_addresses.write().await;
        addresses.insert(user_id, address.clone());
        Ok(address)
    }

    async fn get_address_balance(&self, address: &str) -> Result<u128> {
        let balances = self.balances.read().await;
        Ok(balances.get(address).copied().unwrap_or(0))
    }

    async fn estimate_fee(&self, priority: FeePriority) -> Result<u128> {
        let fee = match priority {
            FeePriority::Low => 1000,
            FeePriority::Medium => 2000,
            FeePriority::High => 5000,
        };
        Ok(fee)
    }

    async fn get_transaction_status(&self, tx_hash: &str) -> Result<Option<TransactionStatus>> {
        let transactions = self.transactions.read().await;
        
        if let Some(tx) = transactions.get(tx_hash) {
            let status = if tx.confirmations >= 3 {
                TxStatus::Confirmed
            } else {
                TxStatus::Pending
            };
            
            Ok(Some(TransactionStatus {
                tx_hash: tx_hash.to_string(),
                confirmed: tx.confirmations >= 3,
                confirmations: tx.confirmations,
                block_height: tx.block_height,
                status,
            }))
        } else {
            Ok(None)
        }
    }
}
