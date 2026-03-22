//! Event Bus Implementation
//! 
//! High-performance event distribution system
//! Critical for scalable crypto exchange architecture

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use async_trait::async_trait;
use anyhow::Result;

use super::{ExchangeEvent, EventHandler};

/// Event bus for distributing events to handlers
pub struct EventBus {
    /// Broadcast channel for events
    sender: broadcast::Sender<ExchangeEvent>,
    /// Event handlers registry
    handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
    /// Event sequence counter
    sequence: Arc<RwLock<u64>>,
}

impl EventBus {
    /// Create new event bus
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        
        Self {
            sender,
            handlers: Arc::new(RwLock::new(Vec::new())),
            sequence: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Publish event to all handlers
    pub async fn publish(&self, mut event: ExchangeEvent) -> Result<()> {
        // Set sequence number
        {
            let mut seq = self.sequence.write().await;
            event.sequence = *seq;
            *seq += 1;
        }
        
        // Send to broadcast channel
        match self.sender.send(event.clone()) {
            Ok(_) => {
                // Also call registered handlers directly for immediate processing
                let handlers = self.handlers.read().await;
                for handler in handlers.iter() {
                    if let Err(e) = handler.handle(event.clone()).await {
                        eprintln!("Event handler error: {}", e);
                    }
                }
                Ok(())
            }
            Err(broadcast::error::SendError(_)) => {
                anyhow::bail!("No active event receivers");
            }
        }
    }
    
    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ExchangeEvent> {
        self.sender.subscribe()
    }
    
    /// Register event handler
    pub async fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push(handler);
    }
    
    /// Get current sequence number
    pub async fn current_sequence(&self) -> u64 {
        *self.sequence.read().await
    }
    
    /// Create event stream for specific event types
    pub async fn event_stream(&self) -> EventStream {
        EventStream::new(self.sender.subscribe())
    }
}

/// Event stream for filtering events
pub struct EventStream {
    receiver: broadcast::Receiver<ExchangeEvent>,
}

impl EventStream {
    /// Create new event stream
    pub fn new(receiver: broadcast::Receiver<ExchangeEvent>) -> Self {
        Self { receiver }
    }
    
    /// Get next event
    pub async fn recv(&mut self) -> Result<ExchangeEvent> {
        match self.receiver.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                anyhow::bail!("Event stream lagged, skipped {} events", skipped);
            }
            Err(broadcast::error::RecvError::Closed) => {
                anyhow::bail!("Event stream closed");
            }
        }
    }
    
    /// Filter events by type
    pub fn filter_by_type<F>(self, filter: F) -> FilteredEventStream<F>
    where
        F: Fn(&ExchangeEvent) -> bool,
    {
        FilteredEventStream {
            stream: self,
            filter,
        }
    }
}

/// Filtered event stream
pub struct FilteredEventStream<F> {
    stream: EventStream,
    filter: F,
}

impl<F> FilteredEventStream<F>
where
    F: Fn(&ExchangeEvent) -> bool,
{
    /// Get next filtered event
    pub async fn recv(&mut self) -> Result<ExchangeEvent> {
        loop {
            let event = self.stream.recv().await?;
            if (self.filter)(&event) {
                return Ok(event);
            }
        }
    }
}

/// In-memory event bus for testing
pub struct InMemoryEventBus {
    events: Arc<RwLock<Vec<ExchangeEvent>>>,
    handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
    sequence: Arc<RwLock<u64>>,
}

impl InMemoryEventBus {
    /// Create new in-memory event bus
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            handlers: Arc::new(RwLock::new(Vec::new())),
            sequence: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Publish event
    pub async fn publish(&mut self, mut event: ExchangeEvent) -> Result<()> {
        // Set sequence number
        {
            let mut seq = self.sequence.write().await;
            event.sequence = *seq;
            *seq += 1;
        }
        
        // Store event
        {
            let mut events = self.events.write().await;
            events.push(event.clone());
        }
        
        // Call handlers
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            if let Err(e) = handler.handle(event.clone()).await {
                eprintln!("Event handler error: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Get all events
    pub async fn get_events(&self) -> Vec<ExchangeEvent> {
        self.events.read().await.clone()
    }
    
    /// Get events after sequence
    pub async fn get_events_since(&self, since: u64) -> Vec<ExchangeEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.sequence > since)
            .cloned()
            .collect()
    }
    
    /// Clear all events
    pub async fn clear(&self) {
        let mut events = self.events.write().await;
        events.clear();
    }
    
    /// Register handler
    pub async fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push(handler);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(10000) // Default capacity of 10k events
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}
