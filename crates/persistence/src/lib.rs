//! Persistence Layer with Event Sourcing
//! 
//! Critical for production crypto exchange
//! Enables recovery, auditing, and replay capabilities

pub mod event_store;
pub mod snapshot;
pub mod repository;

pub use event_store::*;
pub use snapshot::*;
pub use repository::*;
