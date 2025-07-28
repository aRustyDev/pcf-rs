/// GraphQL Subscription Patterns - Phase 3 Implementation Examples
///
/// This file demonstrates WebSocket subscription patterns for GraphQL including
/// event broadcasting, connection management, and resource protection.

use async_graphql::{Context, Result, Schema, Subscription};
use futures_util::{Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Event Types for Subscriptions
#[derive(Debug, Clone)]
pub enum DomainEvent {
    NoteCreated(Note),
    NoteUpdated { old: Note, new: Note },
    NoteDeleted(String),
    SystemEvent(SystemMessage),
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct SystemMessage {
    pub level: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Event Broadcaster with Channel Management
pub struct EventBroadcaster {
    /// Main broadcast channel for all events
    sender: broadcast::Sender<DomainEvent>,
    /// Channel capacity
    capacity: usize,
    /// Active subscriber count
    subscriber_count: Arc<RwLock<usize>>,
    /// Per-topic channels for filtered subscriptions
    topic_channels: Arc<RwLock<HashMap<String, broadcast::Sender<DomainEvent>>>>,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        
        Self {
            sender,
            capacity,
            subscriber_count: Arc::new(RwLock::new(0)),
            topic_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Subscribe to all events
    pub async fn subscribe(&self) -> Result<broadcast::Receiver<DomainEvent>> {
        let mut count = self.subscriber_count.write().await;
        *count += 1;
        
        Ok(self.sender.subscribe())
    }
    
    /// Subscribe to events for a specific topic
    pub async fn subscribe_to_topic(&self, topic: &str) -> Result<broadcast::Receiver<DomainEvent>> {
        let mut channels = self.topic_channels.write().await;
        
        let sender = channels.entry(topic.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(self.capacity);
                tx
            });
        
        Ok(sender.subscribe())
    }
    
    /// Broadcast an event to all subscribers
    pub async fn broadcast(&self, event: DomainEvent) -> Result<()> {
        // Send to main channel
        let _ = self.sender.send(event.clone());
        
        // Send to topic channels if applicable
        let topic = match &event {
            DomainEvent::NoteCreated(note) => Some(format!("note:{}", note.id)),
            DomainEvent::NoteUpdated { new, .. } => Some(format!("note:{}", new.id)),
            DomainEvent::NoteDeleted(id) => Some(format!("note:{}", id)),
            _ => None,
        };
        
        if let Some(topic) = topic {
            let channels = self.topic_channels.read().await;
            if let Some(sender) = channels.get(&topic) {
                let _ = sender.send(event);
            }
        }
        
        Ok(())
    }
    
    /// Get current subscriber count
    pub async fn subscriber_count(&self) -> usize {
        *self.subscriber_count.read().await
    }
    
    /// Clean up disconnected subscribers
    pub async fn cleanup_disconnected(&self) {
        let mut count = self.subscriber_count.write().await;
        *count = self.sender.receiver_count();
        
        // Clean up empty topic channels
        let mut channels = self.topic_channels.write().await;
        channels.retain(|_, sender| sender.receiver_count() > 0);
    }
}

/// Connection Manager for WebSocket Subscriptions
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    max_connections_per_client: usize,
    max_total_connections: usize,
    idle_timeout: Duration,
}

#[derive(Debug)]
struct ConnectionInfo {
    client_id: String,
    connected_at: Instant,
    last_activity: Instant,
    subscription_count: usize,
}

impl ConnectionManager {
    pub fn new(max_connections_per_client: usize, max_total_connections: usize) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections_per_client,
            max_total_connections,
            idle_timeout: Duration::from_secs(1800), // 30 minutes
        }
    }
    
    /// Register a new connection
    pub async fn register_connection(&self, connection_id: String, client_id: String) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        // Check total connection limit
        if connections.len() >= self.max_total_connections {
            return Err(async_graphql::Error::new("Maximum connections reached"));
        }
        
        // Check per-client limit
        let client_connections = connections.values()
            .filter(|info| info.client_id == client_id)
            .count();
        
        if client_connections >= self.max_connections_per_client {
            return Err(async_graphql::Error::new(
                format!("Maximum connections per client ({}) reached", self.max_connections_per_client)
            ));
        }
        
        connections.insert(connection_id, ConnectionInfo {
            client_id,
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            subscription_count: 0,
        });
        
        Ok(())
    }
    
    /// Update activity timestamp
    pub async fn update_activity(&self, connection_id: &str) {
        if let Some(mut connections) = self.connections.write().await.get_mut(connection_id) {
            connections.last_activity = Instant::now();
        }
    }
    
    /// Clean up idle connections
    pub async fn cleanup_idle_connections(&self) -> Vec<String> {
        let mut connections = self.connections.write().await;
        let now = Instant::now();
        let mut removed = Vec::new();
        
        connections.retain(|id, info| {
            if now.duration_since(info.last_activity) > self.idle_timeout {
                removed.push(id.clone());
                false
            } else {
                true
            }
        });
        
        removed
    }
    
    /// Remove a connection
    pub async fn remove_connection(&self, connection_id: &str) {
        self.connections.write().await.remove(connection_id);
    }
    
    /// Get connection statistics
    pub async fn get_stats(&self) -> ConnectionStats {
        let connections = self.connections.read().await;
        
        let mut stats = ConnectionStats {
            total_connections: connections.len(),
            connections_by_client: HashMap::new(),
            average_connection_duration: Duration::from_secs(0),
        };
        
        let now = Instant::now();
        let mut total_duration = Duration::from_secs(0);
        
        for info in connections.values() {
            *stats.connections_by_client
                .entry(info.client_id.clone())
                .or_insert(0) += 1;
            
            total_duration += now.duration_since(info.connected_at);
        }
        
        if !connections.is_empty() {
            stats.average_connection_duration = total_duration / connections.len() as u32;
        }
        
        stats
    }
}

#[derive(Debug)]
pub struct ConnectionStats {
    pub total_connections: usize,
    pub connections_by_client: HashMap<String, usize>,
    pub average_connection_duration: Duration,
}

/// Subscription Filter Builder
pub struct SubscriptionFilter {
    filters: Vec<Box<dyn Fn(&DomainEvent) -> bool + Send + Sync>>,
}

impl SubscriptionFilter {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }
    
    /// Add a filter for specific note ID
    pub fn with_note_id(mut self, note_id: String) -> Self {
        self.filters.push(Box::new(move |event| {
            match event {
                DomainEvent::NoteCreated(note) => note.id == note_id,
                DomainEvent::NoteUpdated { new, .. } => new.id == note_id,
                DomainEvent::NoteDeleted(id) => id == &note_id,
                _ => false,
            }
        }));
        self
    }
    
    /// Add a filter for specific author
    pub fn with_author(mut self, author: String) -> Self {
        self.filters.push(Box::new(move |event| {
            match event {
                DomainEvent::NoteCreated(note) => note.author == author,
                DomainEvent::NoteUpdated { new, .. } => new.author == author,
                _ => false,
            }
        }));
        self
    }
    
    /// Add a filter for event type
    pub fn with_event_type(mut self, event_type: EventType) -> Self {
        self.filters.push(Box::new(move |event| {
            matches!(
                (event, &event_type),
                (DomainEvent::NoteCreated(_), EventType::Created) |
                (DomainEvent::NoteUpdated { .. }, EventType::Updated) |
                (DomainEvent::NoteDeleted(_), EventType::Deleted)
            )
        }));
        self
    }
    
    /// Apply all filters to an event
    pub fn matches(&self, event: &DomainEvent) -> bool {
        self.filters.iter().all(|filter| filter(event))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Created,
    Updated,
    Deleted,
}

/// GraphQL Subscription Implementation
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to note creation events
    async fn note_created(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = Note>> {
        let broadcaster = ctx.data::<Arc<EventBroadcaster>>()?;
        let connection_manager = ctx.data::<Arc<ConnectionManager>>()?;
        let connection_id = ctx.data::<String>()?.clone();
        
        // Update activity
        connection_manager.update_activity(&connection_id).await;
        
        // Check subscription limit
        if broadcaster.subscriber_count().await >= 1000 {
            return Err(async_graphql::Error::new("Subscription limit reached"));
        }
        
        let receiver = broadcaster.subscribe().await?;
        
        Ok(async_stream::stream! {
            let mut receiver = receiver;
            
            while let Ok(event) = receiver.recv().await {
                if let DomainEvent::NoteCreated(note) = event {
                    yield note;
                }
            }
        })
    }
    
    /// Subscribe to note updates with optional filtering
    async fn note_updated(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Optional note ID to filter updates")] note_id: Option<String>,
        #[graphql(desc = "Optional author to filter updates")] author: Option<String>,
    ) -> Result<impl Stream<Item = NoteUpdate>> {
        let broadcaster = ctx.data::<Arc<EventBroadcaster>>()?;
        let connection_manager = ctx.data::<Arc<ConnectionManager>>()?;
        let connection_id = ctx.data::<String>()?.clone();
        
        connection_manager.update_activity(&connection_id).await;
        
        // Build filter
        let mut filter = SubscriptionFilter::new();
        if let Some(id) = note_id {
            filter = filter.with_note_id(id);
        }
        if let Some(author) = author {
            filter = filter.with_author(author);
        }
        
        let receiver = broadcaster.subscribe().await?;
        
        Ok(async_stream::stream! {
            let mut receiver = receiver;
            
            while let Ok(event) = receiver.recv().await {
                if let DomainEvent::NoteUpdated { old, new } = event {
                    if filter.matches(&DomainEvent::NoteUpdated { 
                        old: old.clone(), 
                        new: new.clone() 
                    }) {
                        yield NoteUpdate {
                            previous: old,
                            current: new,
                        };
                    }
                }
            }
        })
    }
    
    /// Subscribe to note deletion events
    async fn note_deleted(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = String>> {
        let broadcaster = ctx.data::<Arc<EventBroadcaster>>()?;
        let receiver = broadcaster.subscribe().await?;
        
        Ok(async_stream::stream! {
            let mut receiver = receiver;
            
            while let Ok(event) = receiver.recv().await {
                if let DomainEvent::NoteDeleted(id) = event {
                    yield id;
                }
            }
        })
    }
    
    /// Subscribe to system events (admin only)
    async fn system_events(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = SystemMessage>> {
        // Check admin permissions
        let session = ctx.data::<Session>()?;
        if !session.is_admin {
            return Err(async_graphql::Error::new("Admin access required"));
        }
        
        let broadcaster = ctx.data::<Arc<EventBroadcaster>>()?;
        let receiver = broadcaster.subscribe().await?;
        
        Ok(async_stream::stream! {
            let mut receiver = receiver;
            
            while let Ok(event) = receiver.recv().await {
                if let DomainEvent::SystemEvent(msg) = event {
                    yield msg;
                }
            }
        })
    }
}

#[derive(async_graphql::SimpleObject)]
struct NoteUpdate {
    previous: Note,
    current: Note,
}

/// Background Tasks for Subscription Management
pub struct SubscriptionManager {
    broadcaster: Arc<EventBroadcaster>,
    connection_manager: Arc<ConnectionManager>,
}

impl SubscriptionManager {
    pub fn new(broadcaster: Arc<EventBroadcaster>, connection_manager: Arc<ConnectionManager>) -> Self {
        Self {
            broadcaster,
            connection_manager,
        }
    }
    
    /// Start background cleanup task
    pub fn start_cleanup_task(self) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Clean up disconnected subscribers
                self.broadcaster.cleanup_disconnected().await;
                
                // Clean up idle connections
                let removed = self.connection_manager.cleanup_idle_connections().await;
                if !removed.is_empty() {
                    tracing::info!("Cleaned up {} idle connections", removed.len());
                }
                
                // Log statistics
                let stats = self.connection_manager.get_stats().await;
                tracing::info!(
                    "Subscription stats: {} total connections, {} subscribers",
                    stats.total_connections,
                    self.broadcaster.subscriber_count().await
                );
            }
        });
    }
}

/// WebSocket Connection Handler
pub mod websocket {
    use super::*;
    use async_graphql::http::{WebSocketProtocols, WsMessage};
    use async_graphql_axum::GraphQLProtocol;
    use axum::extract::ws::{Message, WebSocket};
    use futures_util::sink::SinkExt;
    
    pub async fn handle_websocket(
        socket: WebSocket,
        schema: Schema<Query, Mutation, SubscriptionRoot>,
        connection_manager: Arc<ConnectionManager>,
        client_id: String,
    ) {
        let connection_id = uuid::Uuid::new_v4().to_string();
        
        // Register connection
        if let Err(e) = connection_manager.register_connection(connection_id.clone(), client_id).await {
            let _ = socket.close().await;
            tracing::warn!("Connection rejected: {}", e);
            return;
        }
        
        // Handle WebSocket protocol
        let protocol = GraphQLProtocol::new(schema)
            .on_connection_init(|value| async move {
                // Validate connection params
                // Extract auth token, etc.
                Ok(Data::default())
            });
        
        // Process messages
        let (mut sink, mut stream) = socket.split();
        
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Update activity
                    connection_manager.update_activity(&connection_id).await;
                    
                    // Process GraphQL message
                    if let Ok(msg) = serde_json::from_str::<WsMessage>(&text) {
                        // Handle subscription
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
        
        // Clean up connection
        connection_manager.remove_connection(&connection_id).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_event_broadcasting() {
        let broadcaster = EventBroadcaster::new(100);
        let mut receiver = broadcaster.subscribe().await.unwrap();
        
        let note = Note {
            id: "test-1".to_string(),
            title: "Test Note".to_string(),
            content: "Test content".to_string(),
            author: "test_user".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        broadcaster.broadcast(DomainEvent::NoteCreated(note.clone())).await.unwrap();
        
        match receiver.recv().await {
            Ok(DomainEvent::NoteCreated(received)) => {
                assert_eq!(received.id, note.id);
            }
            _ => panic!("Expected NoteCreated event"),
        }
    }
    
    #[tokio::test]
    async fn test_connection_limits() {
        let manager = ConnectionManager::new(2, 10);
        
        // Register connections for same client
        assert!(manager.register_connection("conn1".to_string(), "client1".to_string()).await.is_ok());
        assert!(manager.register_connection("conn2".to_string(), "client1".to_string()).await.is_ok());
        
        // Third connection should fail
        assert!(manager.register_connection("conn3".to_string(), "client1".to_string()).await.is_err());
        
        // Different client should work
        assert!(manager.register_connection("conn3".to_string(), "client2".to_string()).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_subscription_filtering() {
        let filter = SubscriptionFilter::new()
            .with_note_id("note-123".to_string())
            .with_event_type(EventType::Updated);
        
        let note = Note {
            id: "note-123".to_string(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            author: "user".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        let event = DomainEvent::NoteUpdated {
            old: note.clone(),
            new: note.clone(),
        };
        
        assert!(filter.matches(&event));
        
        let wrong_event = DomainEvent::NoteCreated(note);
        assert!(!filter.matches(&wrong_event));
    }
}