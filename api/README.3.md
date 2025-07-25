# Complete Setup: Rust GraphQL + ORY + SpiceDB

## Project Structure
```
my-app/
├── docker-compose.yml
├── setup-spicedb.sh
├── kratos/
│   ├── kratos.yml
│   ├── identity.schema.json
│   └── webhook.jsonnet
└── api/
    ├── Cargo.toml
    ├── Dockerfile
    └── src/
        ├── main.rs
        ├── auth.rs
        └── webhooks.rs
```

## Step-by-Step Setup

### 1. Initialize Project
```bash
mkdir my-app && cd my-app
mkdir -p kratos api/src

# Copy all configuration files from the artifacts above
```

### 2. Update Your Existing Rust Code

Your existing GraphQL server needs these modifications:

**Add to Cargo.toml:**
- Authentication/authorization dependencies (see rust-cargo-toml artifact)

**Create new modules:**
- `src/auth.rs` - Authentication/authorization logic
- `src/webhooks.rs` - Kratos webhook handlers

**Update main.rs:**
- Add authentication context to GraphQL handler
- Inject SpiceDB client into schema
- Add webhook routes
- Update resolvers to check permissions

### 3. Environment Configuration

Create `.env` file:
```env
RUST_LOG=debug
KRATOS_PUBLIC_URL=http://kratos:4433
KRATOS_ADMIN_URL=http://kratos:4434
SPICEDB_URL=spicedb:50051
SPICEDB_TOKEN=somerandomkeyhere
SURREALDB_URL=ws://surrealdb:8000
```

### 4. Start Services
```bash
# Start infrastructure
docker-compose up -d kratos hydra spicedb surrealdb

# Wait for services
sleep 10

# Initialize SpiceDB schema
chmod +x setup-spicedb.sh
./setup-spicedb.sh

# Build and start Rust API
docker-compose up --build graphql-api
```

## Key Integration Points

### 1. Session Management
```rust
// Every GraphQL request extracts session from cookie
async fn graphql_handler(headers: HeaderMap, req: GraphQLRequest) -> GraphQLResponse {
    let auth_context = get_auth_context(&headers).await;
    // auth_context contains user_id if authenticated
}
```

### 2. Permission Checks
```rust
// In resolvers, check permissions before operations
async fn sensitive_operation(&self, ctx: &Context<'_>) -> Result<Data> {
    let user_id = require_auth(ctx)?;  // Ensures authenticated

    let mut spicedb = ctx.data::<Arc<Mutex<SpiceDBClient>>>()?.lock().await;
    let allowed = spicedb.check_permission(
        "resource", "id", "action", &user_id
    ).await?;

    if !allowed {
        return Err(FieldError::new("Not authorized"));
    }
    // ... proceed with operation
}
```

### 3. User Synchronization
When users register via Kratos:
1. Kratos creates identity
2. Webhook fires to `/webhook/user-created`
3. Rust handler creates user in SurrealDB
4. Initial permissions set in SpiceDB

## Testing the Integration

### 1. Create Test User
```bash
# Via Kratos API
curl -X POST http://localhost:4433/self-service/registration/api \
  -H "Content-Type: application/json" \
  -d '{
    "method": "password",
    "traits": {
      "email": "test@example.com",
      "name": { "first": "Test", "last": "User" }
    },
    "password": "TestPassword123!"
  }'
```

### 2. Login and Get Session
```bash
# Get login flow
FLOW=$(curl -s http://localhost:4433/self-service/login/api)
FLOW_ID=$(echo $FLOW | jq -r '.id')

# Submit credentials
RESPONSE=$(curl -s -X POST "http://localhost:4433/self-service/login?flow=$FLOW_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "method": "password",
    "identifier": "test@example.com",
    "password": "TestPassword123!"
  }' -c cookies.txt)

# Use cookies.txt for authenticated requests
```

### 3. Test GraphQL with Authentication
```bash
# Authenticated query
curl -X POST http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{"query": "{ me { id email name } }"}'
```

## Production Considerations

### 1. Security
- Use TLS for all connections
- Rotate SpiceDB pre-shared key
- Implement rate limiting
- Add request signing for webhooks

### 2. Performance
- Implement connection pooling for SpiceDB
- Cache Kratos session validation
- Use DataLoader for batching permission checks
- Add Redis for session storage

### 3. Monitoring
- Export Prometheus metrics
- Add distributed tracing with OpenTelemetry
- Log all authorization decisions
- Set up alerts for auth failures

### 4. Deployment
```yaml
# kubernetes/spicedb.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: spicedb
spec:
  replicas: 3  # HA setup
  template:
    spec:
      containers:
      - name: spicedb
        image: authzed/spicedb:v1.29.2
        args:
        - serve
        - --grpc-preshared-key=$(SPICEDB_KEY)
        - --datastore-engine=postgres
        - --datastore-conn-uri=$(POSTGRES_URI)
```

## Troubleshooting

### Common Issues

1. **"Not authenticated" errors**
   - Check cookie is being sent
   - Verify Kratos session is valid
   - Check network connectivity between services

2. **"Not authorized" errors**
   - Verify SpiceDB relationships exist
   - Check permission schema is correct
   - Use zed CLI to debug permissions

3. **Webhook failures**
   - Check graphql-api is accessible from kratos container
   - Verify webhook URL in kratos.yml
   - Check webhook logs in Rust app

### Debug Commands
```bash
# Check Kratos session
curl http://localhost:4433/sessions/whoami -b cookies.txt

# Test SpiceDB directly
docker exec spicedb zed permission check \
  document:123 view user:test-id

# View container logs
docker-compose logs -f graphql-api kratos spicedb
```

This completes the Rust-specific implementation of ORY Kratos + SpiceDB!
