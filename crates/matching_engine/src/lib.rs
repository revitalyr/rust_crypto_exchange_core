//! High-performance matching engine with price-time priority.
//!
//! This crate provides a single-threaded matching engine optimized for
//! low-latency execution with deterministic behavior.

pub mod engine;
pub mod executor;
pub mod matcher;
pub mod processor;
pub mod types;

pub use engine::*;
pub use executor::*;
pub use matcher::*;
pub use processor::*;
pub use types::*;
