//! Order book benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use crypto_exchange_common::{
    order::{Order, OrderSide, OrderType, TimeInForce},
    price::Price,
};
use crypto_exchange_orderbook::OrderBook;

/// Benchmark order book operations
pub fn bench_order_book_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("order_book");
    
    group.bench_function("add_limit_order", |b| {
        let mut order_book = OrderBook::new("BTC/USDT".to_string());
        
        b.iter(|| {
            let order = Order::new(
                1, 100, "BTC/USDT".to_string(),
                OrderSide::Buy, OrderType::Limit,
                Some(Price::new(50000)), 1000,
                TimeInForce::GTC, 1234567890,
            );
            
            black_box(order_book.add_limit_order(order).unwrap());
        });
    });
    
    group.bench_function("get_snapshot", |b| {
        let mut order_book = OrderBook::new("BTC/USDT".to_string());
        
        // Pre-populate with some orders
        for i in 0..1000 {
            let side = if i % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell };
            let price = 50000 + (i % 1000) as u64;
            
            let order = Order::new(
                i + 1, 100 + i, "BTC/USDT".to_string(),
                side, OrderType::Limit,
                Some(Price::new(price)), 1000,
                TimeInForce::GTC, 1234567890 + i,
            );
            
            order_book.add_limit_order(order).unwrap();
        }
        
        b.iter(|| {
            black_box(order_book.get_snapshot(20));
        });
    });
    
    group.bench_function("estimate_market_price", |b| {
        let mut order_book = OrderBook::new("BTC/USDT".to_string());
        
        // Pre-populate with orders
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
        
        order_book.add_limit_order(buy_order).unwrap();
        order_book.add_limit_order(sell_order).unwrap();
        
        b.iter(|| {
            black_box(order_book.estimate_market_price(OrderSide::Buy, 1000));
            black_box(order_book.estimate_market_price(OrderSide::Sell, 1000));
        });
    });
}

criterion_group!(
    name = "orderbook_bench";
    config = Criterion::default()
        .sample_size(100)
        .warm_up_time(std::time::Duration::from_secs(5))
        .measurement_time(std::time::Duration::from_secs(10));
    
    bench_order_book_operations,
);

criterion_main!(orderbook_bench);
