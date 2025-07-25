# Testing ORY + SpiceDB with Rust GraphQL

## Complete Testing Flow

### 1. Register a User via Kratos
```bash
# Register user through Kratos
curl -X POST http://localhost:4433/self-service/registration/api \
  -H "Content-Type: application/json" \
  -d '{
    "method": "password",
    "traits": {
      "email": "alice@example.com",
      "name": {
        "first": "Alice",
        "last": "Smith"
      }
    },
    "password": "SecurePassword123!"
  }'

# This triggers the webhook to create user in SurrealDB and initial permissions in SpiceDB
```

### 2. Login and Get Session
```bash
# Initialize login flow
FLOW_ID=$(curl -s -X GET http://localhost:4433/self-service/login/api | jq -r '.id')

# Submit credentials
SESSION_TOKEN=$(curl -s -X POST "http://localhost:4433/self-service/login?flow=$FLOW_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "method": "password",
    "identifier": "alice@example.com",
    "password": "SecurePassword123!"
  }' | jq -r '.session_token')

# Extract session cookie (you'll need to parse the Set-Cookie header)
```

### 3. Test Authenticated GraphQL Queries

```graphql
# Get current user (with session cookie)
query Me {
  me {
    id
    email
    name
  }
}

# Create a document
mutation CreateDocument {
  createDocument(
    title: "Project Proposal"
    content: "This is a confidential document..."
  ) {
    id
    title
    canView
    canEdit
  }
}

# Share document with another user
mutation ShareDocument {
  shareDocument(
    documentId: "abc123"
    userId: "bob-kratos-id"
  ) {
    id
    title
  }
}

# Query documents I can access
query MyDocuments {
  myDocuments {
    id
    title
    owner
    canView
    canEdit
  }
}
```

### 4. Test Authorization Scenarios

#### Scenario A: Document Ownership
```rust
// In GraphQL Playground with Alice's session
mutation {
  createDocument(title: "Alice's Doc", content: "...") {
    id
    canView  // true - owner can view
    canEdit  // true - owner can edit
  }
}

// Switch to Bob's session
query {
  myDocuments {
    id  // Alice's doc won't appear
  }
}
```

#### Scenario B: Document Sharing
```rust
// As Alice, share with Bob
mutation {
  shareDocument(documentId: "doc123", userId: "bob-id") {
    id
  }
}

// As Bob, now can see the document
query {
  myDocuments {
    id
    title
    canView  // true - shared with Bob
    canEdit  // true - shared users can edit
  }
}
```

### 5. Direct SpiceDB Permission Checks

```bash
# Check if Bob can view Alice's document
docker exec spicedb zed permission check \
  document:doc123 view user:bob-id \
  --endpoint localhost:50051 \
  --insecure \
  --token "somerandomkeyhere"

# List all documents Bob can view
docker exec spicedb zed permission lookup-resources \
  document - view user:bob-id \
  --endpoint localhost:50051 \
  --insecure \
  --token "somerandomkeyhere"
```

## Rust-Specific Implementation Notes

### 1. Authentication Middleware
The Rust implementation extracts session cookies from headers and validates with Kratos:

```rust
// In auth.rs
pub async fn get_auth_context(headers: &HeaderMap) -> AuthContext {
    // Extract cookie from headers
    // Validate session with Kratos
    // Return authenticated user context
}
```

### 2. GraphQL Context Injection
Each resolver has access to:
- Authenticated user ID
- Database connection
- SpiceDB client

```rust
async fn my_resolver(&self, ctx: &Context<'_>) -> Result<Data> {
    let user_id = require_auth(ctx)?;  // Ensures authenticated
    let db = ctx.data::<Database>()?;
    let spicedb = ctx.data::<Arc<Mutex<SpiceDBClient>>>()?;
    // ... resolver logic
}
```

### 3. Permission Checks in Resolvers
```rust
// Check permission before returning data
let mut client = spicedb.lock().await;
let has_permission = client.check_permission(
    "document", &doc_id, "view", &user_id
).await?;

if !has_permission {
    return Err(FieldError::new("Not authorized"));
}
```

## Error Handling

### Common Issues and Solutions

1. **SpiceDB Connection Failed**
```bash
# Check SpiceDB is running
docker logs spicedb

# Test gRPC connection
grpcurl -plaintext \
  -H "authorization: Bearer somerandomkeyhere" \
  localhost:50051 list
```

2. **Kratos Session Invalid**
```bash
# Check Kratos logs
docker logs kratos

# Verify cookie format
curl http://localhost:4433/sessions/whoami \
  -H "Cookie: ory_kratos_session=..."
```

3. **Webhook Not Firing**
```bash
# Check webhook endpoint is accessible
docker exec kratos curl http://graphql-api:4000/webhook/user-created

# Check Kratos webhook config
docker exec kratos cat /etc/config/kratos/kratos.yml | grep webhook
```

## Performance Optimization

### 1. Use DataLoader Pattern
```rust
// Batch permission checks
use async_graphql::dataloader::{DataLoader, Loader};

struct PermissionLoader {
    spicedb: Arc<Mutex<SpiceDBClient>>,
}

#[async_trait::async_trait]
impl Loader<PermissionKey> for PermissionLoader {
    type Value = bool;
    type Error = FieldError;

    async fn load(&self, keys: &[PermissionKey]) -> Result<HashMap<PermissionKey, bool>> {
        // Batch check permissions
    }
}
```

### 2. Cache Session Validation
```rust
use moka::sync::Cache;

lazy_static! {
    static ref SESSION_CACHE: Cache<String, Session> = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .build();
}
```

### 3. Connection Pooling
```rust
// Use connection pool for SpiceDB
use deadpool::managed::{Pool, Manager};

struct SpiceDBManager {
    endpoint: String,
    token: String,
}

impl Manager for SpiceDBManager {
    // Implement connection pooling
}
```

## Monitoring and Observability

### 1. Add Tracing
```rust
use tracing::{info, span, Level};

#[tracing::instrument(skip(ctx))]
async fn my_resolver(&self, ctx: &Context<'_>) -> Result<Data> {
    let span = span!(Level::INFO, "authorization_check");
    let _enter = span.enter();

    info!("Checking permissions for user");
    // ... resolver logic
}
```

### 2. Metrics Collection
```rust
use prometheus::{Counter, Histogram};

lazy_static! {
    static ref AUTH_FAILURES: Counter = Counter::new(
        "auth_failures_total", "Total authentication failures"
    ).unwrap();

    static ref PERMISSION_CHECK_DURATION: Histogram = Histogram::new(
        "permission_check_duration_seconds", "SpiceDB check duration"
    ).unwrap();
}
```

This completes the Rust-specific implementation of ORY + SpiceDB integration!
