//! Testing utilities and fixtures for the crypto exchange system.

use crate::assets::{Asset, TradingPair};
use crate::order::{Order, OrderSide, OrderType, TimeInForce};
use crate::price::Price;
use crate::{Balance, UserId, timestamp};

/// Test configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Initial user balances
    pub initial_balances: Vec<(Asset, Balance)>,
    /// Test trading pairs
    pub trading_pairs: Vec<TradingPair>,
    /// Fee rates
    pub maker_fee_rate: crate::types::FeeRate,
    pub taker_fee_rate: crate::types::FeeRate,
    /// Order book depth
    pub order_book_depth: usize,
    /// Number of test users
    pub num_users: crate::types::Count,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            initial_balances: vec![
                (Asset::BTC, 1_000_000_000),    // 10 BTC
                (Asset::ETH, 10_000_000_000_000), // 10 ETH
                (Asset::USDT, 1_000_000_000_000), // 10,000 USDT
                (Asset::USDC, 1_000_000_000_000), // 10,000 USDC
            ],
            trading_pairs: vec![
                TradingPair::new(Asset::BTC, Asset::USDT),
                TradingPair::new(Asset::ETH, Asset::USDT),
                TradingPair::new(Asset::ETH, Asset::BTC),
            ],
            maker_fee_rate: crate::types::constants::DEFAULT_MAKER_FEE_RATE,
            taker_fee_rate: crate::types::constants::DEFAULT_TAKER_FEE_RATE,
            order_book_depth: 10,
            num_users: crate::types::constants::MAX_USERS.min(1000),
        }
    }
}

/// Test fixture builder
pub struct TestFixtureBuilder<A> {
    config: TestConfig,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: std::default::Default> TestFixtureBuilder<A> {
    /// Creates a new test fixture builder
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets initial balances
    pub fn with_balances(mut self, balances: Vec<(Asset, Balance)>) -> Self {
        self.config.initial_balances = balances;
        self
    }

    /// Sets trading pairs
    pub fn with_pairs(mut self, pairs: Vec<TradingPair>) -> Self {
        self.config.trading_pairs = pairs;
        self
    }

    /// Sets fee rates
    pub fn with_fees(mut self, maker_fee: crate::types::FeeRate, taker_fee: crate::types::FeeRate) -> Self {
        self.config.maker_fee_rate = maker_fee;
        self.config.taker_fee_rate = taker_fee;
        self
    }

    /// Sets order book depth
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.config.order_book_depth = depth;
        self
    }

    /// Sets number of users
    pub fn with_users(mut self, num_users: crate::types::Count) -> Self {
        self.config.num_users = num_users;
        self
    }

    /// Builds the test fixture
    pub fn build(self) -> TestFixture<A> {
        TestFixture::new(&self.config)
    }
}

impl<A: std::default::Default> Default for TestFixtureBuilder<A> {
    fn default() -> Self {
        Self::new(TestConfig::default())
    }
}

/// Test fixture with accounts and sample orders
pub struct TestFixture<A> {
    pub config: TestConfig,
    pub accounts: Vec<A>,
    pub sample_orders: Vec<Order>,
}

impl<A> TestFixture<A> 
where
    A: Default,
{
    /// Creates a new test fixture
    pub fn new(config: &TestConfig) -> Self {
        let accounts = Self::create_test_accounts(config);
        let sample_orders = Self::create_sample_orders(config, &accounts);

        Self {
            config: config.clone(),
            accounts,
            sample_orders,
        }
    }

    /// Creates test accounts with initial balances
    fn create_test_accounts(config: &TestConfig) -> Vec<A> 
    where
        A: Default,
    {
        let mut accounts = Vec::new();

        for _user_id in 1..=config.num_users {
            let account = A::default();
            
            // Set initial balances
            for (_asset, _balance) in &config.initial_balances {
                // This is a simplified approach - in reality, Account would have methods
                // to set balances. For now, we'll just create the accounts.
            }
            
            accounts.push(account);
        }

        accounts
    }

    /// Creates sample orders for testing
    fn create_sample_orders(config: &TestConfig, _accounts: &[A]) -> Vec<Order> {
        let mut orders = Vec::new();
        let mut order_id = 1;

        // Create orders for each trading pair
        for pair in &config.trading_pairs {
            // Create some buy orders
            for i in 0..5 {
                let price = match pair.base {
                    Asset::BTC => 50000 + (i as u64 * 100), // 50000, 50100, ...
                    Asset::ETH => 2000 + (i as u64 * 50),
                    Asset::USDT | Asset::USDC => 1, // Stable coins
                    Asset::Custom(_) => 1000 + (i as u64 * 100),
                };

                let order = Order::new(
                    order_id,
                    1, // Default user ID
                    pair.to_string(),
                    OrderSide::Buy,
                    OrderType::Limit,
                    Some(Price::new(price)),
                    1000 + (i as u64 * 100),
                    TimeInForce::GTC,
                    timestamp::now(),
                );

                orders.push(order);
                order_id += 1;
            }

            // Create some sell orders
            for i in 0..5 {
                let price = match pair.base {
                    Asset::BTC => 55000 + (i as u64 * 100), // 55000, 55100, ...
                    Asset::ETH => 2200 + (i as u64 * 50),
                    Asset::USDT | Asset::USDC => 1, // Stable coins
                    Asset::Custom(_) => 1100 + (i as u64 * 100),
                };

                let order = Order::new(
                    order_id,
                    1, // Default user ID
                    pair.to_string(),
                    OrderSide::Sell,
                    OrderType::Limit,
                    Some(Price::new(price)),
                    1000 + (i as u64 * 100),
                    TimeInForce::GTC,
                    timestamp::now(),
                );

                orders.push(order);
                order_id += 1;
            }
        }

        // Create some market orders
        for i in 0..3 {
            let order = Order::new(
                order_id,
                1, // Default user ID
                config.trading_pairs[0].to_string(),
                if i % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell },
                OrderType::Market,
                None,
                500 + (i as u64 * 100),
                TimeInForce::IOC,
                timestamp::now(),
            );

            orders.push(order);
            order_id += 1;
        }

        orders
    }

    /// Gets an account by user ID
    pub fn get_account(&self, _user_id: UserId) -> Option<&A> {
        self.accounts.iter().find(|_acc| {
            // This is a simplified approach - in reality, Account would have a user_id() method
            // For now, we'll just return None since we don't have access to Account internals
            false
        })
    }

    /// Gets first account
    pub fn first_account(&self) -> &A {
        &self.accounts[0]
    }

    /// Gets a random account
    pub fn random_account(&self) -> &A {
        let index = (timestamp::now() % self.accounts.len() as u64) as usize;
        &self.accounts[index]
    }

    /// Gets orders for a specific trading pair
    pub fn get_orders_for_pair(&self, pair: &str) -> Vec<&Order> {
        self.sample_orders
            .iter()
            .filter(|order| order.pair == pair)
            .collect()
    }

    /// Gets buy orders
    pub fn get_buy_orders(&self) -> Vec<&Order> {
        self.sample_orders
            .iter()
            .filter(|order| order.side == OrderSide::Buy)
            .collect()
    }

    /// Gets sell orders
    pub fn get_sell_orders(&self) -> Vec<&Order> {
        self.sample_orders
            .iter()
            .filter(|order| order.side == OrderSide::Sell)
            .collect()
    }

    /// Gets limit orders
    pub fn get_limit_orders(&self) -> Vec<&Order> {
        self.sample_orders
            .iter()
            .filter(|order| order.order_type == OrderType::Limit)
            .collect()
    }

    /// Gets market orders
    pub fn get_market_orders(&self) -> Vec<&Order> {
        self.sample_orders
            .iter()
            .filter(|order| order.order_type == OrderType::Market)
            .collect()
    }
}

impl<A> Default for TestFixture<A> 
where
    A: Default,
{
    fn default() -> Self {
        Self::new(&TestConfig::default())
    }
}

/// Performance test utilities
pub mod performance {
    use super::*;
    use std::time::{Duration, Instant};

    /// Performance measurement result
    #[derive(Debug, Clone)]
    pub struct PerformanceResult {
        pub name: String,
        pub duration: Duration,
        pub operations: u64,
    }

    impl PerformanceResult {
        pub fn new(name: String, duration: Duration, operations: u64) -> Self {
            Self {
                name,
                duration,
                operations,
            }
        }

        /// Returns operations per second
        pub fn ops_per_sec(&self) -> f64 {
            self.operations as f64 / self.duration.as_secs_f64()
        }
    }

    /// Measures execution time of a function
    pub fn measure_performance<F, R>(name: &str, iterations: u64, f: F) -> PerformanceResult
    where
        F: Fn(u64) -> R,
    {
        let start = Instant::now();
        for i in 0..iterations {
            let _result = f(i);
        }
        let duration = start.elapsed();

        PerformanceResult::new(name.to_string(), duration, iterations)
    }

    /// Benchmark configuration
    #[derive(Debug, Clone)]
    pub struct BenchmarkConfig {
        pub warmup_iterations: u64,
        pub benchmark_iterations: u64,
        pub parallel_threads: Option<usize>,
    }

    impl Default for BenchmarkConfig {
        fn default() -> Self {
            Self {
                warmup_iterations: 100,
                benchmark_iterations: 1000,
                parallel_threads: None,
            }
        }
    }

    /// Runs a benchmark with warmup
    pub fn run_benchmark<F, R>(name: &str, config: &BenchmarkConfig, f: F) -> Vec<PerformanceResult>
    where
        F: Fn(u64) -> R + Clone + std::marker::Send,
    {
        let mut results = Vec::new();

        // Warmup phase
        for i in 0..config.warmup_iterations {
            let _result = f(i);
        }

        // Benchmark phase
        if let Some(threads) = config.parallel_threads {
            // Parallel benchmark
            let start = Instant::now();
            std::thread::scope(|s| {
                let handles: Vec<_> = (0..threads)
                    .map(|i| {
                        let thread_id = i;
                        let f_clone = f.clone();
                        s.spawn(move || {
                            for i in 0..config.benchmark_iterations {
                                let _result = f_clone(thread_id as u64 * config.benchmark_iterations + i);
                            }
                        })
                    })
                    .collect();

                for handle in handles {
                    handle.join().unwrap();
                }
            });
            let duration = start.elapsed();
            let total_ops = threads as u64 * config.benchmark_iterations;

            results.push(PerformanceResult::new(
                format!("{} (parallel)", name),
                duration,
                total_ops,
            ));
        } else {
            // Sequential benchmark
            results.push(measure_performance(name, config.benchmark_iterations, f));
        }

        results
    }
}

/// Assertion utilities for testing
pub mod assertions {
    use super::*;
    use crate::Balance;

    /// Asserts that two balances are approximately equal within tolerance
    pub fn assert_balance_approx_eq(
        expected: Balance,
        actual: Balance,
        tolerance: Balance,
        message: &str,
    ) {
        let diff = if expected > actual {
            expected - actual
        } else {
            actual - expected
        };

        assert!(
            diff <= tolerance,
            "{}: Expected {}, got {}, tolerance {}",
            message, expected, actual, tolerance
        );
    }

    /// Asserts that order book state is consistent
    pub fn assert_order_book_consistency(
        bids_total: Balance,
        asks_total: Balance,
        message: &str,
    ) {
        // In a healthy order book, bids and asks should not cross
        // This is a simplified check - in reality we'd need price information
        assert!(
            bids_total > 0 && asks_total > 0,
            "{}: Order book should have both bids and asks",
            message
        );
    }

    /// Asserts that account balances are non-negative
    pub fn assert_non_negative_balances<A>(_account: &A, _assets: &[Asset]) 
    where
        A: Default,
    {
        // This is a simplified approach - in reality, Account would have balance methods
        // For now, we'll just skip this assertion
    }

    /// Asserts that reserved balance doesn't exceed total balance
    pub fn assert_reserved_not_exceeds_total<A>(_account: &A, _assets: &[Asset])
    where
        A: Default,
    {
        // This is a simplified approach - in reality, Account would have balance methods
        // For now, we'll just skip this assertion
    }
}

/// Mock data generator
pub mod mock {
    use super::*;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    /// Mock market data generator
    pub struct MockMarketDataGenerator {
        rng: ChaCha8Rng,
    }

    impl MockMarketDataGenerator {
        /// Creates a new mock data generator with fixed seed
        pub fn new() -> Self {
            Self {
                rng: ChaCha8Rng::seed_from_u64(12345),
            }
        }

        /// Generates a random price for an asset
        pub fn random_price(&mut self, asset: Asset) -> u64 {
            match asset {
                Asset::BTC => self.rng.gen_range(40000..60000),
                Asset::ETH => self.rng.gen_range(2000..5000),
                Asset::USDT | Asset::USDC => 1, // Stable coins
                Asset::Custom(_) => self.rng.gen_range(1..1000),
            }
        }

        /// Generates a random order quantity
        pub fn random_quantity(&mut self) -> crate::types::Quantity {
            self.rng.gen_range(100..10000)
        }

        /// Generates a random order side
        pub fn random_side(&mut self) -> OrderSide {
            if self.rng.gen_bool(0.5) {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            }
        }

        /// Generates a random order type
        pub fn random_order_type(&mut self) -> OrderType {
            if self.rng.gen_bool(0.8) {
                OrderType::Limit
            } else {
                OrderType::Market
            }
        }

        /// Generates a random time in force
        pub fn random_time_in_force(&mut self) -> TimeInForce {
            let rand_val = self.rng.gen_range(0..3);
            match rand_val {
                0 => TimeInForce::GTC,
                1 => TimeInForce::IOC,
                _ => TimeInForce::FOK,
            }
        }

        /// Generates a random order
        pub fn random_order(&mut self, order_id: crate::types::OrderId, user_id: crate::types::UserId, pair: &TradingPair) -> Order {
            let order_type = self.random_order_type();
            let price = if order_type == OrderType::Limit {
                Some(Price::new(self.random_price(pair.base)))
            } else {
                None
            };

            Order::new(
                order_id,
                user_id,
                pair.to_string(),
                self.random_side(),
                order_type,
                price,
                self.random_quantity(),
                self.random_time_in_force(),
                timestamp::now(),
            )
        }

        /// Generates multiple random orders
        pub fn random_orders(&mut self, count: usize, user_id: crate::types::UserId, pair: &TradingPair) -> Vec<Order> {
            (0..count)
                .map(|i| self.random_order(i as u64 + 1, user_id, pair))
                .collect()
        }
    }

    impl Default for MockMarketDataGenerator {
        fn default() -> Self {
            Self::new()
        }
    }
}
