# WebSocket Subscriptions Guide

## Understanding GraphQL Subscriptions

Subscriptions are GraphQL's way of pushing real-time updates to clients. Unlike queries and mutations which follow a request/response pattern, subscriptions maintain a persistent connection.

## How Subscriptions Work

1. Client establishes WebSocket connection
2. Client sends subscription query
3. Server sends data whenever events occur
4. Connection stays open until client unsubscribes

```
Client                          Server
  |                               |
  |-------- WebSocket Connect --->|
  |<------- Connection Ack --------|
  |                               |
  |------ Subscribe to Events --->|
  |<------- Event 1 --------------|
  |<------- Event 2 --------------|
  |<------- Event 3 --------------|
  |                               |
  |-------- Unsubscribe --------->|
  |<------- Complete -------------|
```

## Basic Implementation

### 1. Define Subscription Type

```rust
use async_graphql::{Subscription, Context, Result};
use futures_util::stream::{Stream, StreamExt};
use tokio::sync::broadcast;

pub struct Subscription;

#[Subscription]
impl Subscription {
    /// Subscribe to new notes being created
    async fn note_created(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = Note>> {
        // Get the event broadcaster from context
        let broadcaster = ctx.data::<Arc<EventBroadcaster>>()?;
        
        // Create a stream that filters for NoteCreated events
        Ok(broadcaster
            .subscribe()
            .filter_map(|event| async move {
                match event {
                    Ok(Event::NoteCreated(note)) => Some(note),
                    _ => None,
                }
            }))
    }
}
```

### 2. Event Broadcasting System

```rust
use tokio::sync::broadcast;

#[derive(Clone)]
pub enum Event {
    NoteCreated(Note),
    NoteUpdated { old: Note, new: Note },
    NoteDeleted(String),
}

pub struct EventBroadcaster {
    sender: broadcast::Sender<Event>,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
    
    pub fn broadcast(&self, event: Event) -> Result<()> {
        self.sender.send(event)
            .map_err(|_| Error::new("No subscribers"))?;
        Ok(())
    }
}
```

### 3. Emit Events from Mutations

```rust
#[Object]
impl Mutation {
    async fn create_note(
        &self,
        ctx: &Context<'_>,
        input: CreateNoteInput,
    ) -> Result<Note> {
        // Create the note
        let note = create_note_in_database(input).await?;
        
        // Broadcast the event
        let broadcaster = ctx.data::<Arc<EventBroadcaster>>()?;
        broadcaster.broadcast(Event::NoteCreated(note.clone()))?;
        
        Ok(note)
    }
}
```

### 4. WebSocket Server Setup

```rust
use async_graphql_axum::{GraphQLSubscription, GraphQLProtocol};
use axum::routing::get;

// WebSocket handler
async fn graphql_ws_handler(
    ws: WebSocketUpgrade,
    Extension(schema): Extension<Schema<Query, Mutation, Subscription>>,
    protocol: GraphQLProtocol,
) -> impl IntoResponse {
    ws.protocols(["graphql-ws", "graphql-transport-ws"])
        .on_upgrade(move |socket| {
            GraphQLSubscription::new(socket, schema.clone(), protocol)
                .on_connection_init(on_connection_init)
                .serve()
        })
}

// Add to router
let app = Router::new()
    .route("/graphql", get(graphql_ws_handler).post(graphql_handler))
    .layer(Extension(schema));
```

## Advanced Patterns

### 1. Filtered Subscriptions

Allow clients to subscribe to specific events:

```rust
#[Subscription]
impl Subscription {
    /// Subscribe to notes by a specific author
    async fn notes_by_author(
        &self,
        ctx: &Context<'_>,
        author: String,
    ) -> Result<impl Stream<Item = Note>> {
        let broadcaster = ctx.data::<Arc<EventBroadcaster>>()?;
        
        Ok(broadcaster
            .subscribe()
            .filter_map(move |event| {
                let author = author.clone();
                async move {
                    match event {
                        Ok(Event::NoteCreated(note)) if note.author == author => Some(note),
                        Ok(Event::NoteUpdated { new, .. }) if new.author == author => Some(new),
                        _ => None,
                    }
                }
            }))
    }
}
```

### 2. Connection Context

Track connection-specific data:

```rust
async fn on_connection_init(value: serde_json::Value) -> Result<Data> {
    // Extract auth token from connection params
    let mut data = Data::default();
    
    if let Some(token) = value.get("authToken").and_then(|v| v.as_str()) {
        // Verify token and add user context
        let user = verify_token(token).await?;
        data.insert(AuthContext { user_id: user.id });
    }
    
    Ok(data)
}
```

### 3. Resource Cleanup

Clean up when connections close:

```rust
pub struct SubscriptionConnection {
    id: Uuid,
    created_at: Instant,
}

#[Subscription]
impl Subscription {
    async fn note_updates(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = Note>> {
        // Track connection
        let connection_id = Uuid::new_v4();
        let tracker = ctx.data::<Arc<ConnectionTracker>>()?;
        tracker.add_connection(connection_id).await;
        
        let stream = create_note_stream(ctx).await?;
        
        // Clean up on drop
        Ok(CleanupStream::new(stream, move || {
            tokio::spawn(async move {
                tracker.remove_connection(connection_id).await;
            });
        }))
    }
}
```

### 4. Rate Limiting

Prevent subscription spam:

```rust
#[Subscription]
impl Subscription {
    async fn rate_limited_updates(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = Event>> {
        let rate_limiter = ctx.data::<Arc<RateLimiter>>()?;
        let user_id = ctx.data::<AuthContext>()?.user_id.clone();
        
        // Check rate limit
        if !rate_limiter.check_subscription(&user_id).await? {
            return Err(Error::new("Too many subscriptions")
                .extend_with(|_, e| e.set("code", "RATE_LIMITED")));
        }
        
        // Create throttled stream
        Ok(create_event_stream()
            .throttle(Duration::from_millis(100)))  // Max 10 events/second
    }
}
```

## Security Considerations

### 1. Authentication

Always verify authentication for subscriptions:

```rust
#[Subscription]
impl Subscription {
    async fn private_updates(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = Event>> {
        // Require authentication
        let auth = ctx.data::<AuthContext>()
            .map_err(|_| Error::new("Authentication required"))?;
        
        if !auth.is_authenticated() {
            return Err(Error::new("Must be logged in to subscribe"));
        }
        
        // Return user-specific events
        Ok(create_user_event_stream(auth.user_id))
    }
}
```

### 2. Authorization

Check permissions for each event:

```rust
async fn authorized_note_stream(
    ctx: &Context<'_>,
) -> Result<impl Stream<Item = Note>> {
    let auth = ctx.data::<AuthContext>()?.clone();
    let broadcaster = ctx.data::<Arc<EventBroadcaster>>()?;
    
    Ok(broadcaster
        .subscribe()
        .filter_map(move |event| {
            let auth = auth.clone();
            async move {
                match event {
                    Ok(Event::NoteCreated(note)) => {
                        // Check if user can view this note
                        if can_user_view_note(&auth, &note).await {
                            Some(note)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
        }))
}
```

### 3. Connection Limits

Prevent resource exhaustion:

```rust
const MAX_CONNECTIONS_PER_USER: usize = 5;

async fn on_connection_init(value: Value) -> Result<Data> {
    let token = extract_token(&value)?;
    let user = verify_token(token).await?;
    
    // Check connection limit
    let tracker = CONNECTION_TRACKER.read().await;
    if tracker.user_connections(&user.id) >= MAX_CONNECTIONS_PER_USER {
        return Err(Error::new("Too many connections"));
    }
    
    let mut data = Data::default();
    data.insert(AuthContext { user_id: user.id });
    Ok(data)
}
```

## Testing Subscriptions

### 1. Unit Testing Streams

```rust
#[tokio::test]
async fn test_note_created_subscription() {
    let broadcaster = Arc::new(EventBroadcaster::new(100));
    let schema = create_test_schema(broadcaster.clone());
    
    // Start subscription
    let stream = schema.execute_stream(r#"
        subscription {
            noteCreated {
                id
                title
            }
        }
    "#);
    
    // Pin the stream for polling
    tokio::pin!(stream);
    
    // Broadcast an event
    broadcaster.broadcast(Event::NoteCreated(Note {
        id: "123".into(),
        title: "Test Note".into(),
    })).unwrap();
    
    // Check we received it
    if let Some(response) = stream.next().await {
        assert!(response.errors.is_empty());
        let note = response.data.get("noteCreated").unwrap();
        assert_eq!(note.get("id"), "123");
    }
}
```

### 2. Integration Testing

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::test]
async fn test_websocket_subscription() {
    // Start server
    let server = start_test_server().await;
    
    // Connect WebSocket
    let (ws_stream, _) = connect_async("ws://localhost:8080/graphql").await?;
    let (write, read) = ws_stream.split();
    
    // Send connection init
    write.send(Message::Text(json!({
        "type": "connection_init",
        "payload": { "authToken": "test-token" }
    }).to_string())).await?;
    
    // Send subscription
    write.send(Message::Text(json!({
        "id": "1",
        "type": "subscribe",
        "payload": {
            "query": "subscription { noteCreated { id } }"
        }
    }).to_string())).await?;
    
    // Trigger event and verify received
    create_test_note().await;
    
    let msg = read.next().await.unwrap()?;
    assert!(msg.to_text()?.contains("noteCreated"));
}
```

## Common Issues and Solutions

### 1. Events Not Received

**Problem**: Subscription connected but no events received

**Checklist**:
- Is the broadcaster being shared correctly?
- Are events being sent after subscription starts?
- Is filtering too restrictive?
- Check error logs for broadcast failures

```rust
// Debug logging
broadcaster.broadcast(event.clone())
    .map_err(|e| {
        tracing::warn!("Failed to broadcast: {:?}", e);
        e
    })?;
```

### 2. Memory Leaks

**Problem**: Memory usage grows with subscriptions

**Solutions**:
- Set channel capacity limits
- Implement connection tracking
- Add automatic cleanup

```rust
// Bounded channel prevents unbounded growth
let (sender, _) = broadcast::channel(1000);  // Max 1000 pending events

// Track and clean up stale connections
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        cleanup_stale_connections().await;
    }
});
```

### 3. Connection Drops

**Problem**: WebSocket connections drop unexpectedly

**Solutions**:
- Implement heartbeat/ping-pong
- Set appropriate timeouts
- Handle reconnection gracefully

```rust
// Configure WebSocket with keep-alive
let ws_config = WebSocketConfig {
    keep_alive_interval: Some(Duration::from_secs(30)),
    max_frame_size: 65536,
    max_message_size: 1048576,  // 1MB
    ..Default::default()
};
```

## Best Practices

1. **Use Bounded Channels**: Prevent memory issues
2. **Filter Early**: Don't send unnecessary events
3. **Batch Events**: Group rapid updates
4. **Monitor Connections**: Track active subscriptions
5. **Test Disconnections**: Ensure cleanup happens
6. **Document Event Types**: Clear event schemas
7. **Version Events**: Plan for schema evolution

## Performance Tips

### 1. Event Deduplication

```rust
// Prevent duplicate events in short time windows
let mut last_event: Option<(EventType, Instant)> = None;

stream.filter(move |event| {
    let now = Instant::now();
    let is_duplicate = last_event
        .as_ref()
        .map(|(last, time)| last == event && now.duration_since(*time) < Duration::from_millis(100))
        .unwrap_or(false);
    
    if !is_duplicate {
        last_event = Some((event.clone(), now));
        true
    } else {
        false
    }
})
```

### 2. Subscription Metrics

```rust
// Track subscription performance
metrics::gauge!("websocket.active_connections", tracker.count() as f64);
metrics::histogram!("websocket.message_size", message.len() as f64);
metrics::counter!("websocket.events_sent", 1);
```

Remember: Subscriptions are powerful but need careful implementation to avoid performance issues and resource leaks!