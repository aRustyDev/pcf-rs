# Migration Guide: Adding ORY + SpiceDB to Your Existing Rust GraphQL Server

## Overview
This guide helps you add authentication (ORY Kratos) and authorization (SpiceDB) to your existing Rust GraphQL server with minimal changes to your current code.

## Step 1: Add Dependencies

Add these to your existing `Cargo.toml`:

```toml
# Authentication & Authorization
reqwest = { version = "0.12", features = ["json", "cookies"] }
tonic = "0.12"
prost = "0.13"
ory-kratos-client = "1.0"
authzed = { git = "https://github.com/authzed/authzed-rust" }
axum-extra = { version = "0.9", features = ["cookie"] }
cookie = "0.18"
lazy_static = "1.5"
```

## Step 2: Add New Modules

Create two new files:

1. **`src/auth.rs`** - Copy from the rust-auth-module artifact
2. **`src/webhooks.rs`** - Copy from the rust-webhooks artifact

## Step 3: Update Your main.rs

### Minimal Changes Required:

1. **Add imports:**
```rust
mod auth;
mod webhooks;

use auth::{AuthContext, SpiceDBClient, get_auth_context, require_auth};
use std::sync::Arc;
use tokio::sync::Mutex;
```

2. **Update your GraphQL handler:**
```rust
// Replace your existing handler
async fn graphql_handler(
    Extension(schema): Extension<Schema<QueryRoot, MutationRoot, EmptySubscription>>,
    headers: HeaderMap,  // Add this parameter
    req: GraphQLRequest,
) -> GraphQLResponse {
    // Add these two lines
    let auth_context = get_auth_context(&headers).await;
    let request = req.into_inner().data(auth_context);

    // Execute with auth context
    schema.execute(request).await.into()
}
```

3. **Add SpiceDB to your schema:**
```rust
// In main(), after database initialization
let spicedb_client = SpiceDBClient::new(
    std::env::var("SPICEDB_URL").unwrap_or_else(|_| "localhost:50051".to_string()),
    std::env::var("SPICEDB_TOKEN").unwrap_or_else(|_| "somerandomkeyhere".to_string()),
).await?;

let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
    .data(database)
    .data(Arc::new(Mutex::new(spicedb_client)))  // Add this
    .finish();
```

4. **Add webhook routes:**
```rust
let app = Router::new()
    .route("/", get(graphql_playground))
    .route("/graphql", post(graphql_handler))
    .route("/health", get(health))
    .nest("/webhook", webhooks::routes())  // Add this line
    .layer(Extension(schema))
    .layer(CorsLayer::permissive());
```

## Step 4: Protect Your Existing Queries

For each query/mutation you want to protect:

### Simple Authentication (user must be logged in):
```rust
async fn users(&self, ctx: &Context<'_>) -> Result<Vec<User>, FieldError> {
    require_auth(ctx)?;  // Add this line
    // ... rest of your existing code
}
```

### Authorization (check specific permissions):
```rust
async fn sensitive_data(&self, ctx: &Context<'_>, resource_id: String) -> Result<Data, FieldError> {
    let user_id = require_auth(ctx)?;

    // Check permission with SpiceDB
    let spicedb = ctx.data::<Arc<Mutex<SpiceDBClient>>>()?;
    let mut client = spicedb.lock().await;
    let allowed = client.check_permission(
        "resource_type", &resource_id, "view", &user_id
    ).await?;

    if !allowed {
        return Err(FieldError::new("Not authorized"));
    }

    // ... rest of your existing code
}
```

## Step 5: Update Your Existing Types

No changes needed to your existing GraphQL types! Just ensure your `User` type has an `id` field that matches Kratos identity IDs.

## Step 6: Docker Compose Integration

Add these services to your existing setup:

```yaml
services:
  # Your existing services...

  kratos:
    image: oryd/kratos:v1.0.0
    # ... (copy from docker-compose.yml artifact)

  spicedb:
    image: authzed/spicedb:v1.29.2
    # ... (copy from docker-compose.yml artifact)
```

## Migration Checklist

- [ ] Add new dependencies to Cargo.toml
- [ ] Create auth.rs and webhooks.rs modules
- [ ] Update main.rs imports
- [ ] Modify GraphQL handler to include auth context
- [ ] Add SpiceDB client to schema
- [ ] Add webhook routes
- [ ] Add `require_auth()` to protected queries/mutations
- [ ] Update docker-compose.yml
- [ ] Create kratos configuration files
- [ ] Run SpiceDB schema setup

## Testing Your Migration

1. **Start services:**
```bash
docker-compose up -d kratos spicedb
```

2. **Create a test user:**
```bash
curl -X POST http://localhost:4433/self-service/registration/api \
  -H "Content-Type: application/json" \
  -d '{"method": "password", "traits": {"email": "test@example.com"}, "password": "Test123!"}'
```

3. **Test unauthenticated access (should fail):**
```bash
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ users { id } }"}'
# Should return "Not authenticated"
```

4. **Test authenticated access:**
```bash
# Login and save cookies
# Then use cookies for authenticated request
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{"query": "{ users { id } }"}'
# Should return data
```

## Gradual Migration Strategy

You don't need to protect everything at once:

### Phase 1: Authentication Only
- Add auth infrastructure
- Protect sensitive queries with `require_auth()`
- Leave public queries unchanged

### Phase 2: Basic Authorization
- Add SpiceDB relationships for existing data
- Implement ownership checks (users can only modify their own data)

### Phase 3: Advanced Permissions
- Implement role-based access (admin, user, etc.)
- Add resource sharing capabilities
- Implement hierarchical permissions

## Rollback Plan

If you need to disable auth temporarily:

1. Remove `require_auth()` calls
2. Skip auth context extraction in handler
3. Auth infrastructure can remain running without affecting app

This approach lets you add authentication and authorization incrementally without breaking existing functionality!
