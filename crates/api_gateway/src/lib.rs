//! REST API and WebSocket gateway.

#[path = "main.rs"]
mod main;

// Re-export commonly used types for library usage
pub use main::{
    WsMessage, Platform, ConnectedClient, ExchangeServer, OrderBook
};
