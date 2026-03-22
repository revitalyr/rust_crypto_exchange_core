//! Core types and utilities for the crypto exchange system.
//!
//! This crate provides shared data structures, error types, and utilities
//! used across all components of the exchange.

pub mod assets;
pub mod config;
pub mod error;
pub mod events;
pub mod order;
pub mod price;
pub mod trade;
pub mod types;

pub use assets::*;
pub use config::*;
pub use error::*;
pub use events::*;
pub use order::*;
pub use price::*;
pub use trade::*;
pub use types::*;

#[cfg(test)]
pub mod testing;
