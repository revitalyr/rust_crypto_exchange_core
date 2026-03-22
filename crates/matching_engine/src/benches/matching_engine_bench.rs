//! Matching engine benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use crypto_exchange_common::{
    order::{Order, OrderSide, OrderType, TimeInForce},
    price::Price,
};
use crypto_exchange_matching_engine::{MatchingEngine, MatchingEngineBuilder};

/// Benchmark matching engine operations
pub fn bench_matching_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("matching_engine");
    
    group.bench_function("submit_order", |b| {
        let config = MatchingEngineBuilder::new("BTC/USDT".to_string())
            .max_price_levels(10000)
            .build();
        let mut engine = MatchingEngine::new("BTC/USDT".to_string(), config);
        
        b.iter(|| {
            let order = Order::new(
                1, 100, "BTC/USDT".to_string(),
                OrderSide::Buy, OrderType::Limit,
                Some(Price::new(50000)), 1000,
                TimeInForce::GTC, 1234567890,
            );
            
            black_box(engine.submit_order(order));
        });
    });
    
    group.bench_function("submit_market_order", |b| {
        let mut engine = MatchingEngine::default();
        
        // Pre-populate order book
        let buy_order = Order::new(
            1, 100, "BTC/USDT".to_string(),
            OrderSide::Buy, OrderType::Limit,
            Some(Price::new(49000)), 1000,
            TimeInForce::GTC, 1234567890,
        );
        
        let sell_order = Order::new(
            2, 101, "BTC/USDT".to_string(),
            OrderSide::Sell, OrderType::Limit,
            Some(Price::new(51000)), 1000,
            TimeInForce::GTC, 1234567891,
        );
        
        engine.submit_order(buy_order).unwrap();
        engine.submit_order(sell_order).unwrap();
        
        b.iter(|| {
            let order = Order::new(
                3, 102, "BTC/USDT".to_string(),
                OrderSide::Buy, OrderType::Market,
                None, 100,
                TimeInForce::IOC, 1234567892,
            );
            
            black_box(engine.submit_order(order));
        });
    });
    
    group.bench_function("get_order_book", |b| {
        let mut engine = MatchingEngine::default();
        
        // Pre-populate with orders
        for i in 0..100 {
            let side = if i % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell };
            let order_type = if i % 3 == 0 { OrderType::Market } else { OrderType::Limit };
            let price = if order_type == OrderType::Limit {
                Some(Price::new(48000 + (i % 4000) as u64))
            } else {
                None
            };
            
            let order = Order::new(
                i + 1, 100 + i, "BTC/USDT".to_string(),
                side, order_type, price,
                100 + (i % 500) as u64,
                TimeInForce::IOC, 1234567890 + i,
            );
            
            engine.submit_order(order);
        }
        
        b.iter(|| {
            black_box(engine.get_order_book(10));
        });
    });
}

criterion_group!(
    name = "matching_engine_bench";
    config = Criterion::default()
        .sample_size(100)
        .warm_up_time(std::time::Duration::from_secs(5))
        .measurement_time(std::time::Duration::from_secs(10));
    
    bench_matching_engine,
);

criterion_main!(matching_engine_bench);
