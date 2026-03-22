//! High-Performance Metrics Collection
//! 
//! Low-overhead latency tracking for crypto exchange
//! Uses HDR histograms for accurate percentile measurements

use hdrhistogram::{Histogram, Recorder};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use metrics::{counter, histogram, gauge};

/// Exchange metrics collector
pub struct ExchangeMetrics {
    /// Order processing latency histogram
    order_latency: Arc<RwLock<Histogram<u64>>>,
    /// Trade execution latency histogram
    trade_latency: Arc<RwLock<Histogram<u64>>>,
    /// WebSocket message latency histogram
    websocket_latency: Arc<RwLock<Histogram<u64>>>,
    /// Database query latency histogram
    db_latency: Arc<RwLock<Histogram<u64>>>,
    /// Blockchain transaction latency histogram
    blockchain_latency: Arc<RwLock<Histogram<u64>>>,
    
    /// Performance counters
    orders_total: Arc<RwLock<u64>>,
    trades_total: Arc<RwLock<u64>>,
    rejects_total: Arc<RwLock<u64>>,
    errors_total: Arc<RwLock<u64>>,
    
    /// System metrics
    memory_usage_mb: Arc<RwLock<f64>>,
    cpu_usage_percent: Arc<RwLock<f64>>,
    active_connections: Arc<RwLock<u64>>,
    order_book_depth: Arc<RwLock<u64>>,
}

impl ExchangeMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            order_latency: Arc::new(RwLock::new(Histogram::new_with_bounds(1, 1_000_000, 3).unwrap())),
            trade_latency: Arc::new(RwLock::new(Histogram::new_with_bounds(1, 1_000_000, 3).unwrap())),
            websocket_latency: Arc::new(RwLock::new(Histogram::new_with_bounds(1, 1_000_000, 3).unwrap())),
            db_latency: Arc::new(RwLock::new(Histogram::new_with_bounds(1, 10_000_000, 3).unwrap())),
            blockchain_latency: Arc::new(RwLock::new(Histogram::new_with_bounds(1, 100_000_000, 3).unwrap())),
            
            orders_total: Arc::new(RwLock::new(0)),
            trades_total: Arc::new(RwLock::new(0)),
            rejects_total: Arc::new(RwLock::new(0)),
            errors_total: Arc::new(RwLock::new(0)),
            
            memory_usage_mb: Arc::new(RwLock::new(0.0)),
            cpu_usage_percent: Arc::new(RwLock::new(0.0)),
            active_connections: Arc::new(RwLock::new(0)),
            order_book_depth: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Record order processing latency
    pub fn record_order_latency(&self, latency: Duration) {
        let micros = latency.as_micros() as u64;
        self.order_latency.write().record(micros).unwrap();
        histogram!("exchange_order_latency_micros", micros as f64);
        counter!("exchange_orders_total").increment(1);
        *self.orders_total.write() += 1;
    }
    
    /// Record trade execution latency
    pub fn record_trade_latency(&self, latency: Duration) {
        let micros = latency.as_micros() as u64;
        self.trade_latency.write().record(micros).unwrap();
        histogram!("exchange_trade_latency_micros", micros as f64);
        counter!("exchange_trades_total").increment(1);
        *self.trades_total.write() += 1;
    }
    
    /// Record WebSocket message latency
    pub fn record_websocket_latency(&self, latency: Duration) {
        let micros = latency.as_micros() as u64;
        self.websocket_latency.write().record(micros).unwrap();
        histogram!("exchange_websocket_latency_micros", micros as f64);
    }
    
    /// Record database query latency
    pub fn record_db_latency(&self, latency: Duration) {
        let micros = latency.as_micros() as u64;
        self.db_latency.write().record(micros).unwrap();
        histogram!("exchange_db_latency_micros", micros as f64);
    }
    
    /// Record blockchain transaction latency
    pub fn record_blockchain_latency(&self, latency: Duration) {
        let micros = latency.as_micros() as u64;
        self.blockchain_latency.write().record(micros).unwrap();
        histogram!("exchange_blockchain_latency_micros", micros as f64);
    }
    
    /// Increment order rejects counter
    pub fn increment_order_rejects(&self) {
        counter!("exchange_order_rejects_total").increment(1);
        *self.rejects_total.write() += 1;
    }
    
    /// Increment errors counter
    pub fn increment_errors(&self) {
        counter!("exchange_errors_total").increment(1);
        *self.errors_total.write() += 1;
    }
    
    /// Update memory usage
    pub fn update_memory_usage(&self, mb: f64) {
        gauge!("exchange_memory_usage_mb", mb);
        *self.memory_usage_mb.write() = mb;
    }
    
    /// Update CPU usage
    pub fn update_cpu_usage(&self, percent: f64) {
        gauge!("exchange_cpu_usage_percent", percent);
        *self.cpu_usage_percent.write() = percent;
    }
    
    /// Update active connections
    pub fn update_active_connections(&self, count: u64) {
        gauge!("exchange_active_connections", count as f64);
        *self.active_connections.write() = count;
    }
    
    /// Update order book depth
    pub fn update_order_book_depth(&self, depth: u64) {
        gauge!("exchange_order_book_depth", depth as f64);
        *self.order_book_depth.write() = depth;
    }
    
    /// Get percentile for order latency
    pub fn get_order_latency_percentile(&self, percentile: f64) -> u64 {
        self.order_latency.read().value_at_quantile(percentile)
    }
    
    /// Get percentile for trade latency
    pub fn get_trade_latency_percentile(&self, percentile: f64) -> u64 {
        self.trade_latency.read().value_at_quantile(percentile)
    }
    
    /// Get current statistics snapshot
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        let order_hist = self.order_latency.read();
        let trade_hist = self.trade_latency.read();
        
        MetricsSnapshot {
            orders_total: *self.orders_total.read(),
            trades_total: *self.trades_total.read(),
            rejects_total: *self.rejects_total.read(),
            errors_total: *self.errors_total.read(),
            
            order_latency_p50: order_hist.value_at_quantile(0.5),
            order_latency_p95: order_hist.value_at_quantile(0.95),
            order_latency_p99: order_hist.value_at_quantile(0.99),
            order_latency_p999: order_hist.value_at_quantile(0.999),
            
            trade_latency_p50: trade_hist.value_at_quantile(0.5),
            trade_latency_p95: trade_hist.value_at_quantile(0.95),
            trade_latency_p99: trade_hist.value_at_quantile(0.99),
            trade_latency_p999: trade_hist.value_at_quantile(0.999),
            
            memory_usage_mb: *self.memory_usage_mb.read(),
            cpu_usage_percent: *self.cpu_usage_percent.read(),
            active_connections: *self.active_connections.read(),
            order_book_depth: *self.order_book_depth.read(),
        }
    }
    
    /// Print detailed metrics report
    pub fn print_report(&self) {
        let snapshot = self.get_snapshot();
        
        println!("\n📊 Exchange Performance Metrics:");
        println!("🔄 Order Processing:");
        println!("  Total Orders: {}", snapshot.orders_total);
        println!("  Rejects: {} ({:.2}%)", 
            snapshot.rejects_total,
            (snapshot.rejects_total as f64 / snapshot.orders_total as f64) * 100.0
        );
        println!("  Latency p50: {} μs", snapshot.order_latency_p50);
        println!("  Latency p95: {} μs", snapshot.order_latency_p95);
        println!("  Latency p99: {} μs", snapshot.order_latency_p99);
        println!("  Latency p999: {} μs", snapshot.order_latency_p999);
        
        println!("\n💰 Trade Execution:");
        println!("  Total Trades: {}", snapshot.trades_total);
        println!("  Latency p50: {} μs", snapshot.trade_latency_p50);
        println!("  Latency p95: {} μs", snapshot.trade_latency_p95);
        println!("  Latency p99: {} μs", snapshot.trade_latency_p99);
        println!("  Latency p999: {} μs", snapshot.trade_latency_p999);
        
        println!("\n🖥️  System:");
        println!("  Memory Usage: {:.2} MB", snapshot.memory_usage_mb);
        println!("  CPU Usage: {:.2}%", snapshot.cpu_usage_percent);
        println!("  Active Connections: {}", snapshot.active_connections);
        println!("  Order Book Depth: {}", snapshot.order_book_depth);
        println!("  Errors: {}", snapshot.errors_total);
    }
}

/// Metrics snapshot for reporting
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub orders_total: u64,
    pub trades_total: u64,
    pub rejects_total: u64,
    pub errors_total: u64,
    
    pub order_latency_p50: u64,
    pub order_latency_p95: u64,
    pub order_latency_p99: u64,
    pub order_latency_p999: u64,
    
    pub trade_latency_p50: u64,
    pub trade_latency_p95: u64,
    pub trade_latency_p99: u64,
    pub trade_latency_p999: u64,
    
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub active_connections: u64,
    pub order_book_depth: u64,
}

/// Latency measurement guard
pub struct LatencyGuard {
    start_time: Instant,
    metrics: Arc<ExchangeMetrics>,
    operation_type: OperationType,
}

#[derive(Debug, Clone, Copy)]
pub enum OperationType {
    Order,
    Trade,
    WebSocket,
    Database,
    Blockchain,
}

impl LatencyGuard {
    /// Create new latency guard
    pub fn new(metrics: Arc<ExchangeMetrics>, operation_type: OperationType) -> Self {
        Self {
            start_time: Instant::now(),
            metrics,
            operation_type,
        }
    }
    
    /// Record latency and consume guard
    pub fn record(self) {
        let latency = self.start_time.elapsed();
        
        match self.operation_type {
            OperationType::Order => self.metrics.record_order_latency(latency),
            OperationType::Trade => self.metrics.record_trade_latency(latency),
            OperationType::WebSocket => self.metrics.record_websocket_latency(latency),
            OperationType::Database => self.metrics.record_db_latency(latency),
            OperationType::Blockchain => self.metrics.record_blockchain_latency(latency),
        }
    }
}

impl Drop for LatencyGuard {
    fn drop(&mut self) {
        let latency = self.start_time.elapsed();
        
        match self.operation_type {
            OperationType::Order => self.metrics.record_order_latency(latency),
            OperationType::Trade => self.metrics.record_trade_latency(latency),
            OperationType::WebSocket => self.metrics.record_websocket_latency(latency),
            OperationType::Database => self.metrics.record_db_latency(latency),
            OperationType::Blockchain => self.metrics.record_blockchain_latency(latency),
        }
    }
}

impl Default for ExchangeMetrics {
    fn default() -> Self {
        Self::new()
    }
}
