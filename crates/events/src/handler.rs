//! Event Handler Trait
//! 
//! Defines how different components react to events
//! Essential for modular crypto exchange architecture

use async_trait::async_trait;
use anyhow::Result;

use super::ExchangeEvent;

/// Event handler trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an event
    async fn handle(&self, event: ExchangeEvent) -> Result<()>;
    
    /// Get handler name for debugging
    fn name(&self) -> &'static str;
}

/// Macro to implement EventHandler for closures
#[macro_export]
macro_rules! event_handler {
    ($name:expr, |$event:ident| $body:block) => {{
        struct Handler {
            name: &'static str,
        }
        
        impl Handler {
            fn new() -> Self {
                Self { name: $name }
            }
        }
        
        #[async_trait]
        impl EventHandler for Handler {
            async fn handle(&self, $event: ExchangeEvent) -> Result<()> {
                $body
            }
            
            fn name(&self) -> &'static str {
                self.name
            }
        }
        
        std::sync::Arc::new(Handler::new())
    }};
}

/// Balance update handler
pub struct BalanceUpdateHandler {
    // Dependencies would be injected here
}

impl BalanceUpdateHandler {
    /// Create new balance update handler
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for BalanceUpdateHandler {
    async fn handle(&self, event: ExchangeEvent) -> Result<()> {
        match event.payload {
            super::EventPayload::TradeExecuted { 
                maker_user_id, 
                taker_user_id, 
                quantity, 
                price,
                maker_side,
                taker_side,
                ..
            } => {
                // Update maker balance
                let maker_change = match maker_side {
                    crypto_exchange_common::order::OrderSide::Buy => -(price as i128 * quantity as i128),
                    crypto_exchange_common::order::OrderSide::Sell => quantity as i128,
                };
                
                // Update taker balance
                let taker_change = match taker_side {
                    crypto_exchange_common::order::OrderSide::Buy => quantity as i128,
                    crypto_exchange_common::order::OrderSide::Sell => -(price as i128 * quantity as i128),
                };
                
                println!("Balance update: Maker {} change {}, Taker {} change {}", 
                    maker_user_id, maker_change, taker_user_id, taker_change);
                
                // In real implementation, this would update database
                Ok(())
            }
            _ => Ok(()), // Not a trade event
        }
    }
    
    fn name(&self) -> &'static str {
        "BalanceUpdateHandler"
    }
}

/// Order book update handler
pub struct OrderBookUpdateHandler {
    // Dependencies would be injected here
}

impl OrderBookUpdateHandler {
    /// Create new order book handler
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for OrderBookUpdateHandler {
    async fn handle(&self, event: ExchangeEvent) -> Result<()> {
        match event.payload {
            super::EventPayload::OrderAccepted { 
                order_id, 
                pair, 
                side, 
                quantity, 
                price, 
                ..
            } => {
                println!("Order book update: {} {} {} {} @ {:?}", 
                    order_id, side, quantity, pair.symbol(), price);
                // In real implementation, this would update order book
                Ok(())
            }
            super::EventPayload::OrderCancelled { order_id, .. } => {
                println!("Order book update: Cancelled order {}", order_id);
                // In real implementation, this would remove from order book
                Ok(())
            }
            _ => Ok(()), // Not an order book event
        }
    }
    
    fn name(&self) -> &'static str {
        "OrderBookUpdateHandler"
    }
}

/// Market data handler
pub struct MarketDataHandler {
    // Dependencies would be injected here
}

impl MarketDataHandler {
    /// Create new market data handler
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for MarketDataHandler {
    async fn handle(&self, event: ExchangeEvent) -> Result<()> {
        match event.payload {
            super::EventPayload::TradeExecuted { 
                pair, 
                price, 
                quantity, 
                timestamp,
                ..
            } => {
                println!("Market data: {} trade {} @ {} at {}", 
                    pair.symbol(), quantity, price, timestamp);
                // In real implementation, this would update market data feeds
                Ok(())
            }
            _ => Ok(()), // Not a market data event
        }
    }
    
    fn name(&self) -> &'static str {
        "MarketDataHandler"
    }
}

impl Default for BalanceUpdateHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for OrderBookUpdateHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MarketDataHandler {
    fn default() -> Self {
        Self::new()
    }
}
