//! Crypto Exchange Simulation & Benchmarking Tool
//! 
//! Realistic simulation of crypto exchange operations
//! Critical for performance testing and capacity planning

use clap::{Arg, Command};
use hdrhistogram::Histogram;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use crypto_exchange_common::{
    types::{OrderId, UserId, Quantity},
    order::{OrderSide, OrderType},
    assets::{Asset, TradingPair},
};
use crypto_exchange_events::{EventBus, ExchangeEvent, EventPayload};
use crypto_exchange_blockchain::{MockBlockchain, BlockchainAdapter};

/// Simulation configuration
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Number of users to simulate
    pub num_users: usize,
    /// Number of orders per user
    pub orders_per_user: usize,
    /// Simulation duration in seconds
    pub duration_seconds: u64,
    /// Order submission rate (orders per second)
    pub order_rate: f64,
    /// Deposit amount per user
    pub deposit_amount: u128,
    /// Trading pairs to simulate
    pub trading_pairs: Vec<TradingPair>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            num_users: 1000,
            orders_per_user: 10,
            duration_seconds: 60,
            order_rate: 100.0,
            deposit_amount: 1_000_000_000, // 10 BTC in satoshis
            trading_pairs: vec![
                TradingPair::new(Asset::BTC, Asset::USDT),
                TradingPair::new(Asset::ETH, Asset::USDT),
            ],
        }
    }
}

/// Simulation statistics
#[derive(Debug, Default)]
pub struct SimulationStats {
    /// Total orders submitted
    pub total_orders: u64,
    /// Total trades executed
    pub total_trades: u64,
    /// Total volume traded
    pub total_volume: u128,
    /// Order latency histogram
    pub order_latency: Histogram<u64>,
    /// Trade latency histogram
    pub trade_latency: Histogram<u64>,
    /// Orders per second
    pub orders_per_second: f64,
    /// Trades per second
    pub trades_per_second: f64,
    /// Average order book depth
    pub avg_order_book_depth: f64,
    /// Peak memory usage (MB)
    pub peak_memory_mb: f64,
}

/// User simulation state
#[derive(Debug)]
pub struct SimulatedUser {
    /// User ID
    pub user_id: UserId,
    /// Available balances
    pub balances: std::collections::HashMap<Asset, u128>,
    /// Active orders
    pub active_orders: std::collections::HashSet<OrderId>,
    /// Next order ID
    pub next_order_id: OrderId,
}

impl SimulatedUser {
    /// Create new simulated user
    pub fn new(user_id: UserId, initial_balances: Vec<(Asset, u128)>) -> Self {
        let mut balances = std::collections::HashMap::new();
        for (asset, amount) in initial_balances {
            balances.insert(asset, amount);
        }
        
        Self {
            user_id,
            balances,
            active_orders: std::collections::HashSet::new(),
            next_order_id: (user_id as OrderId) * 1000000, // Unique namespace
        }
    }
    
    /// Get balance for asset
    pub fn get_balance(&self, asset: &Asset) -> u128 {
        self.balances.get(asset).copied().unwrap_or(0)
    }
    
    /// Check if user can place order
    pub fn can_place_order(&self, side: OrderSide, asset: Asset, quantity: u128, price: u64) -> bool {
        match side {
            OrderSide::Buy => {
                let quote_asset = match asset {
                    Asset::BTC => Asset::USDT,
                    Asset::ETH => Asset::USDT,
                    _ => Asset::USDT,
                };
                let required_amount = (quantity as u128) * (price as u128);
                self.get_balance(&quote_asset) >= required_amount
            }
            OrderSide::Sell => {
                self.get_balance(&asset) >= quantity
            }
        }
    }
    
    /// Generate next order ID
    pub fn next_order_id(&mut self) -> OrderId {
        let id = self.next_order_id;
        self.next_order_id += 1;
        id
    }
}

/// Crypto exchange simulator
pub struct CryptoExchangeSimulator {
    /// Configuration
    config: SimulationConfig,
    /// Event bus
    event_bus: Arc<EventBus>,
    /// Blockchain adapters
    blockchains: std::collections::HashMap<Asset, Arc<dyn BlockchainAdapter>>,
    /// Simulated users
    users: Arc<RwLock<Vec<SimulatedUser>>>,
    /// Statistics
    stats: Arc<RwLock<SimulationStats>>,
}

impl CryptoExchangeSimulator {
    /// Create new simulator
    pub fn new(config: SimulationConfig) -> Self {
        let event_bus = Arc::new(EventBus::new(10000));
        let mut blockchains = std::collections::HashMap::new();
        
        // Create mock blockchains
        for pair in &config.trading_pairs {
            let btc_blockchain = Arc::new(MockBlockchain::new(Asset::BTC));
            let eth_blockchain = Arc::new(MockBlockchain::new(Asset::ETH));
            blockchains.insert(Asset::BTC, btc_blockchain as Arc<dyn BlockchainAdapter>);
            blockchains.insert(Asset::ETH, eth_blockchain as Arc<dyn BlockchainAdapter>);
        }
        
        Self {
            config,
            event_bus,
            blockchains,
            users: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(SimulationStats::default())),
        }
    }
    
    /// Initialize users with deposits
    async fn initialize_users(&self) -> anyhow::Result<()> {
        let mut users = self.users.write().await;
        
        for i in 0..self.config.num_users {
            let user_id = (i + 1) as UserId;
            
            // Give users initial balances
            let initial_balances = vec![
                (Asset::BTC, self.config.deposit_amount),
                (Asset::USDT, self.config.deposit_amount * 50000), // 50k USDT
                (Asset::ETH, self.config.deposit_amount / 10), // 0.1 ETH
            ];
            
            let user = SimulatedUser::new(user_id, initial_balances);
            users.push(user);
            
            // Simulate deposit events
            let deposit_event = ExchangeEvent::new(
                crypto_exchange_events::EventType::DepositConfirmed,
                EventPayload::DepositConfirmed {
                    deposit_id: format!("deposit_{}", user_id),
                    user_id,
                    asset: Asset::BTC,
                    amount: self.config.deposit_amount,
                    tx_hash: format!("tx_deposit_{}", user_id),
                    credited_at: chrono::Utc::now(),
                },
                i as u64,
            );
            
            self.event_bus.publish(deposit_event).await?;
        }
        
        println!("✅ Initialized {} users with deposits", self.config.num_users);
        Ok(())
    }
    
    /// Run the simulation
    pub async fn run(&self) -> anyhow::Result<SimulationStats> {
        println!("🚀 Starting crypto exchange simulation...");
        println!("📊 Config: {} users, {} orders/user, {}s duration", 
            self.config.num_users, 
            self.config.orders_per_user, 
            self.config.duration_seconds);
        
        // Initialize users
        self.initialize_users().await?;
        
        // Start simulation
        let start_time = Instant::now();
        let mut order_count = 0u64;
        let mut trade_count = 0u64;
        
        let simulation_duration = Duration::from_secs(self.config.duration_seconds);
        let interval_between_orders = Duration::from_secs_f64(1.0 / self.config.order_rate);
        
        let mut next_order_time = Instant::now();
        
        while start_time.elapsed() < simulation_duration {
            // Submit orders at configured rate
            if Instant::now() >= next_order_time {
                if let Some(order_result) = self.submit_random_order().await? {
                    order_count += 1;
                    if order_result.is_trade {
                        trade_count += 1;
                    }
                }
                
                next_order_time = Instant::now() + interval_between_orders;
            }
            
            // Process blockchain events
            self.process_blockchain_events().await?;
            
            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        
        // Calculate final statistics
        let elapsed = start_time.elapsed();
        let mut stats = self.stats.write().await;
        
        stats.orders_per_second = order_count as f64 / elapsed.as_secs_f64();
        stats.trades_per_second = trade_count as f64 / elapsed.as_secs_f64();
        
        println!("✅ Simulation completed!");
        println!("📈 Results: {:.2} orders/sec, {:.2} trades/sec", 
            stats.orders_per_second, stats.trades_per_second);
        
        Ok(stats.clone())
    }
    
    /// Submit random order
    async fn submit_random_order(&self) -> anyhow::Result<Option<OrderResult>> {
        let users = self.users.read().await;
        let user_idx = rand::random::<usize>() % users.len();
        let user = &users[user_idx];
        
        // Select random trading pair
        let pair_idx = rand::random::<usize>() % self.config.trading_pairs.len();
        let pair = &self.config.trading_pairs[pair_idx];
        
        // Generate random order parameters
        let side = if rand::random() { OrderSide::Buy } else { OrderSide::Sell };
        let order_type = if rand::random() { OrderType::Limit } else { OrderType::Market };
        
        let quantity = (rand::random::<u64>() % 1000) + 1; // 1-1000 units
        let price = if order_type == OrderType::Limit {
            let base_price = match pair.base {
                Asset::BTC => 50000,
                Asset::ETH => 3000,
                _ => 1000,
            };
            let variation = (rand::random::<i64>() % 1000) - 500; // ±500
            (base_price as i64 + variation).max(1) as u64
        } else {
            0 // Market order
        };
        
        // Check if user can place order
        if !user.can_place_order(side, pair.base, quantity as u128, price) {
            return Ok(None);
        }
        
        // Submit order
        let order_start = Instant::now();
        let order_id = user.next_order_id();
        
        let order_event = ExchangeEvent::new(
            crypto_exchange_events::EventType::OrderAccepted,
            EventPayload::OrderAccepted {
                order_id,
                user_id: user.user_id,
                pair: pair.clone(),
                side,
                order_type,
                quantity,
                price: if price > 0 { Some(price) } else { None },
            },
            order_count,
        );
        
        self.event_bus.publish(order_event).await?;
        
        // Record latency
        let latency = order_start.elapsed().as_micros() as u64;
        let mut stats = self.stats.write().await;
        stats.order_latency.record(latency)?;
        stats.total_orders += 1;
        
        // Simulate trade execution (simplified)
        let is_trade = rand::random::<f64>() < 0.3; // 30% chance of immediate trade
        if is_trade {
            let trade_latency = order_start.elapsed().as_micros() as u64;
            stats.trade_latency.record(trade_latency)?;
            stats.total_trades += 1;
            stats.total_volume += quantity as u128;
        }
        
        Ok(Some(OrderResult { is_trade }))
    }
    
    /// Process blockchain events
    async fn process_blockchain_events(&self) -> anyhow::Result<()> {
        // Simulate deposit confirmations
        for (asset, blockchain) in &self.blockchains {
            let deposits = blockchain.get_new_deposits(None).await?;
            for deposit in deposits {
                if deposit.confirmations >= 3 {
                    let deposit_event = ExchangeEvent::new(
                        crypto_exchange_events::EventType::DepositConfirmed,
                        EventPayload::DepositConfirmed {
                            deposit_id: format!("deposit_{}", deposit.tx_hash),
                            user_id: rand::random::<UserId>() % 1000 + 1,
                            asset: asset.clone(),
                            amount: deposit.amount,
                            tx_hash: deposit.tx_hash,
                            credited_at: chrono::Utc::now(),
                        },
                        0,
                    );
                    
                    self.event_bus.publish(deposit_event).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Print detailed statistics
    pub fn print_statistics(&stats: &SimulationStats) {
        println!("\n📊 Simulation Statistics:");
        println!("  Total Orders: {}", stats.total_orders);
        println!("  Total Trades: {}", stats.total_trades);
        println!("  Total Volume: {} units", stats.total_volume);
        println!("  Orders/sec: {:.2}", stats.orders_per_second);
        println!("  Trades/sec: {:.2}", stats.trades_per_second);
        
        if stats.total_orders > 0 {
            println!("  Order Latency:");
            println!("    p50: {} μs", stats.order_latency.value_at_quantile(0.5));
            println!("    p95: {} μs", stats.order_latency.value_at_quantile(0.95));
            println!("    p99: {} μs", stats.order_latency.value_at_quantile(0.99));
        }
        
        if stats.total_trades > 0 {
            println!("  Trade Latency:");
            println!("    p50: {} μs", stats.trade_latency.value_at_quantile(0.5));
            println!("    p95: {} μs", stats.trade_latency.value_at_quantile(0.95));
            println!("    p99: {} μs", stats.trade_latency.value_at_quantile(0.99));
        }
        
        println!("  Peak Memory: {:.2} MB", stats.peak_memory_mb);
    }
}

/// Order submission result
#[derive(Debug)]
struct OrderResult {
    is_trade: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = Command::new("crypto-exchange-simulation")
        .version("1.0")
        .about("Crypto Exchange Simulation & Benchmarking Tool")
        .arg(
            Arg::new("users")
                .long("users")
                .short('u')
                .value_name("COUNT")
                .help("Number of users to simulate")
                .default_value("1000"),
        )
        .arg(
            Arg::new("orders")
                .long("orders")
                .short('o')
                .value_name("COUNT")
                .help("Orders per user")
                .default_value("10"),
        )
        .arg(
            Arg::new("duration")
                .long("duration")
                .short('d')
                .value_name("SECONDS")
                .help("Simulation duration in seconds")
                .default_value("60"),
        )
        .arg(
            Arg::new("rate")
                .long("rate")
                .short('r')
                .value_name("ORDERS_PER_SEC")
                .help("Order submission rate")
                .default_value("100"),
        )
        .get_matches();
    
    let config = SimulationConfig {
        num_users: matches.get_one::<String>("users").unwrap().parse().unwrap_or(1000),
        orders_per_user: matches.get_one::<String>("orders").unwrap().parse().unwrap_or(10),
        duration_seconds: matches.get_one::<String>("duration").unwrap().parse().unwrap_or(60),
        order_rate: matches.get_one::<String>("rate").unwrap().parse().unwrap_or(100.0),
        ..Default::default()
    };
    
    let simulator = CryptoExchangeSimulator::new(config);
    let stats = simulator.run().await?;
    
    CryptoExchangeSimulator::print_statistics(&stats);
    
    Ok(())
}
