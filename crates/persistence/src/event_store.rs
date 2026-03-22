//! Event Store Implementation
//! 
//! Core component for event sourcing
//! Stores all exchange events for recovery and auditing

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use anyhow::Result;

use crypto_exchange_events::ExchangeEvent;

/// Event store trait for different implementations
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append event to store
    async fn append_event(&self, event: &ExchangeEvent) -> Result<()>;
    
    /// Get events for aggregate
    async fn get_events(&self, aggregate_id: &str, from_sequence: u64) -> Result<Vec<ExchangeEvent>>;
    
    /// Get events by type
    async fn get_events_by_type(&self, event_type: &str, limit: Option<usize>) -> Result<Vec<ExchangeEvent>>;
    
    /// Get events in time range
    async fn get_events_in_range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<ExchangeEvent>>;
    
    /// Get latest sequence number
    async fn get_latest_sequence(&self) -> Result<u64>;
    
    /// Replay events from given sequence
    async fn replay_from(&self, from_sequence: u64) -> Result<Vec<ExchangeEvent>>;
}

/// PostgreSQL event store implementation
pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    /// Create new PostgreSQL event store
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Self { pool })
    }
    
    /// Create tables
    async fn create_tables(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                event_type VARCHAR NOT NULL,
                sequence BIGINT NOT NULL UNIQUE,
                timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
                payload JSONB NOT NULL,
                correlation_id UUID,
                version INTEGER NOT NULL DEFAULT 1,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            );
            
            CREATE INDEX IF NOT EXISTS idx_events_sequence ON events(sequence);
            CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_correlation ON events(correlation_id);
            "#
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}

#[async_trait]
impl EventStore for PostgresEventStore {
    async fn append_event(&self, event: &ExchangeEvent) -> Result<()> {
        let payload_json = serde_json::to_value(&event.payload)?;
        
        sqlx::query(
            r#"
            INSERT INTO events (id, event_type, sequence, timestamp, payload, correlation_id, version)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (sequence) DO NOTHING
            "#
        )
        .bind(&event.id)
        .bind(event.type_name())
        .bind(event.sequence as i64)
        .bind(event.timestamp)
        .bind(payload_json)
        .bind(&event.correlation_id)
        .bind(event.version)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_events(&self, aggregate_id: &str, from_sequence: u64) -> Result<Vec<ExchangeEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_type, sequence, timestamp, payload, correlation_id, version
            FROM events
            WHERE payload->>'user_id' = $1 AND sequence >= $2
            ORDER BY sequence ASC
            "#
        )
        .bind(aggregate_id)
        .bind(from_sequence as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            let event = self.row_to_event(row)?;
            events.push(event);
        }
        
        Ok(events)
    }
    
    async fn get_events_by_type(&self, event_type: &str, limit: Option<usize>) -> Result<Vec<ExchangeEvent>> {
        let query = if let Some(limit) = limit {
            sqlx::query(
                r#"
                SELECT id, event_type, sequence, timestamp, payload, correlation_id, version
                FROM events
                WHERE event_type = $1
                ORDER BY sequence DESC
                LIMIT $2
                "#
            )
            .bind(event_type)
            .bind(limit as i64)
        } else {
            sqlx::query(
                r#"
                SELECT id, event_type, sequence, timestamp, payload, correlation_id, version
                FROM events
                WHERE event_type = $1
                ORDER BY sequence DESC
                "#
            )
            .bind(event_type)
        };
        
        let rows = query.fetch_all(&self.pool).await?;
        
        let mut events = Vec::new();
        for row in rows {
            let event = self.row_to_event(row)?;
            events.push(event);
        }
        
        Ok(events)
    }
    
    async fn get_events_in_range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<ExchangeEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_type, sequence, timestamp, payload, correlation_id, version
            FROM events
            WHERE timestamp BETWEEN $1 AND $2
            ORDER BY sequence ASC
            "#
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            let event = self.row_to_event(row)?;
            events.push(event);
        }
        
        Ok(events)
    }
    
    async fn get_latest_sequence(&self) -> Result<u64> {
        let row = sqlx::query(
            "SELECT COALESCE(MAX(sequence), 0) as max_sequence FROM events"
        )
        .fetch_one(&self.pool)
        .await?;
        
        let max_seq: i64 = row.get("max_sequence");
        Ok(max_seq as u64)
    }
    
    async fn replay_from(&self, from_sequence: u64) -> Result<Vec<ExchangeEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, event_type, sequence, timestamp, payload, correlation_id, version
            FROM events
            WHERE sequence >= $1
            ORDER BY sequence ASC
            "#
        )
        .bind(from_sequence as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let mut events = Vec::new();
        for row in rows {
            let event = self.row_to_event(row)?;
            events.push(event);
        }
        
        Ok(events)
    }
}

impl PostgresEventStore {
    /// Convert database row to event
    fn row_to_event(&self, row: sqlx::postgres::PgRow) -> Result<ExchangeEvent> {
        use crypto_exchange_events::{EventType, EventPayload};
        
        let id: String = row.get("id");
        let event_type_str: String = row.get("event_type");
        let sequence: i64 = row.get("sequence");
        let timestamp: DateTime<Utc> = row.get("timestamp");
        let payload_json: serde_json::Value = row.get("payload");
        let correlation_id: Option<String> = row.get("correlation_id");
        let version: i32 = row.get("version");
        
        let event_type = match event_type_str.as_str() {
            "OrderAccepted" => EventType::OrderAccepted,
            "OrderRejected" => EventType::OrderRejected,
            "OrderCancelled" => EventType::OrderCancelled,
            "OrderPartiallyFilled" => EventType::OrderPartiallyFilled,
            "OrderFullyFilled" => EventType::OrderFullyFilled,
            "TradeExecuted" => EventType::TradeExecuted,
            "BalanceUpdated" => EventType::BalanceUpdated,
            "BalanceReserved" => EventType::BalanceReserved,
            "BalanceReleased" => EventType::BalanceReleased,
            "DepositDetected" => EventType::DepositDetected,
            "DepositConfirmed" => EventType::DepositConfirmed,
            "DepositFailed" => EventType::DepositFailed,
            "WithdrawalRequested" => EventType::WithdrawalRequested,
            "WithdrawalProcessed" => EventType::WithdrawalProcessed,
            "WithdrawalFailed" => EventType::WithdrawalFailed,
            "WithdrawalConfirmed" => EventType::WithdrawalConfirmed,
            "MarketDataUpdated" => EventType::MarketDataUpdated,
            "SystemStatusChanged" => EventType::SystemStatusChanged,
            "RiskAlert" => EventType::RiskAlert,
            _ => anyhow::bail!("Unknown event type: {}", event_type_str),
        };
        
        let payload: EventPayload = serde_json::from_value(payload_json)?;
        
        Ok(ExchangeEvent {
            id,
            event_type,
            timestamp,
            sequence: sequence as u64,
            payload,
            correlation_id,
            version: version as u32,
        })
    }
}

/// In-memory event store for testing
pub struct InMemoryEventStore {
    events: std::sync::Arc<tokio::sync::RwLock<Vec<ExchangeEvent>>>,
}

impl InMemoryEventStore {
    /// Create new in-memory event store
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn append_event(&self, event: &ExchangeEvent) -> Result<()> {
        let mut events = self.events.write().await;
        events.push(event.clone());
        Ok(())
    }
    
    async fn get_events(&self, _aggregate_id: &str, _from_sequence: u64) -> Result<Vec<ExchangeEvent>> {
        // Simplified implementation for testing
        let events = self.events.read().await;
        Ok(events.clone())
    }
    
    async fn get_events_by_type(&self, event_type: &str, _limit: Option<usize>) -> Result<Vec<ExchangeEvent>> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.type_name() == event_type)
            .cloned()
            .collect())
    }
    
    async fn get_events_in_range(&self, _from: DateTime<Utc>, _to: DateTime<Utc>) -> Result<Vec<ExchangeEvent>> {
        let events = self.events.read().await;
        Ok(events.clone())
    }
    
    async fn get_latest_sequence(&self) -> Result<u64> {
        let events = self.events.read().await;
        Ok(events.last().map(|e| e.sequence).unwrap_or(0))
    }
    
    async fn replay_from(&self, from_sequence: u64) -> Result<Vec<ExchangeEvent>> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.sequence >= from_sequence)
            .cloned()
            .collect())
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}
