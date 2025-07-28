# SpiceDB Setup Guide for Phase 4

## What is SpiceDB?

SpiceDB is an authorization system based on Google's Zanzibar. Think of it as a specialized database just for permissions - it answers questions like "Can Alice edit Document 123?"

## Quick Start

### 1. Run SpiceDB Locally

The easiest way is using Docker:

```bash
# Using the project's just command
just spicedb-up

# Or manually with Docker
docker run -d \
  --name spicedb \
  -p 50051:50051 \
  -p 8080:8080 \
  authzed/spicedb serve \
  --grpc-preshared-key "somerandomkeyhere" \
  --datastore-engine memory
```

### 2. Install zed CLI Tool

```bash
# macOS
brew install authzed/tap/zed

# Linux
curl https://raw.githubusercontent.com/authzed/zed/main/install.sh | bash

# Or download from https://github.com/authzed/zed/releases
```

### 3. Configure zed

```bash
# Create a context for local development
zed context set local localhost:50051 "somerandomkeyhere" --insecure
```

## Understanding the SpiceDB Model

### Core Concepts

1. **Definitions** - Types of objects (user, note, organization)
2. **Relations** - How objects relate (owner, member, viewer)
3. **Permissions** - What actions are allowed (read, write, delete)

### Visual Example

```
┌─────────────┐     owner      ┌─────────────┐
│ user:alice  │ ─────────────> │  note:123   │
└─────────────┘                 └─────────────┘
                                       │
                                    allows
                                       ↓
                               read, write, delete
```

## Writing Your First Schema

### 1. Basic Schema Structure

Create `schema/spicedb/schema.zed`:

```zed
// Define a user type (subject)
definition user {}

// Define a note type with relations and permissions
definition note {
    // Relations define connections
    relation owner: user
    relation viewer: user
    
    // Permissions compute what's allowed
    permission read = viewer + owner
    permission write = owner
    permission delete = owner
}
```

### 2. What This Means

- `owner` can read, write, and delete
- `viewer` can only read
- The `+` operator means "OR" (union)

### 3. Apply the Schema

```bash
# Validate the schema
zed schema validate schema/spicedb/schema.zed

# Apply to SpiceDB
zed schema write schema/spicedb/schema.zed --context local
```

## Creating Relationships

### 1. Basic Relationship

When a user creates a note, establish ownership:

```bash
# Alice owns note 123
zed relationship create note:123 owner user:alice --context local
```

### 2. In Rust Code

```rust
use authzed::api::v1::*;
use tonic::transport::Channel;

pub struct SpiceDBClient {
    client: PermissionsServiceClient<Channel>,
    token: String,
}

impl SpiceDBClient {
    pub async fn create_relationship(
        &self,
        resource: &str,
        relation: &str,
        subject: &str,
    ) -> Result<(), Error> {
        let mut request = WriteRelationshipsRequest {
            updates: vec![RelationshipUpdate {
                operation: relationship_update::Operation::Touch as i32,
                relationship: Some(Relationship {
                    resource: Some(ObjectReference {
                        object_type: resource.split(':').next().unwrap().to_string(),
                        object_id: resource.split(':').nth(1).unwrap().to_string(),
                    }),
                    relation: relation.to_string(),
                    subject: Some(SubjectReference {
                        object: Some(ObjectReference {
                            object_type: subject.split(':').next().unwrap().to_string(),
                            object_id: subject.split(':').nth(1).unwrap().to_string(),
                        }),
                        optional_relation: String::new(),
                    }),
                }),
            }],
            optional_preconditions: vec![],
        };
        
        // Add auth token
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", self.token).parse().unwrap(),
        );
        
        self.client.write_relationships(request).await?;
        Ok(())
    }
}
```

## Checking Permissions

### 1. Using zed CLI

```bash
# Can Alice read note 123?
zed permission check note:123 read user:alice --context local
# Output: true

# Can Bob read note 123?
zed permission check note:123 read user:bob --context local
# Output: false
```

### 2. In Rust Code

```rust
impl SpiceDBClient {
    pub async fn check_permission(
        &self,
        subject: String,
        resource: String,
        permission: String,
    ) -> Result<bool, Error> {
        let (resource_type, resource_id) = resource.split_once(':').unwrap();
        let (subject_type, subject_id) = subject.split_once(':').unwrap();
        
        let mut request = CheckPermissionRequest {
            resource: Some(ObjectReference {
                object_type: resource_type.to_string(),
                object_id: resource_id.to_string(),
            }),
            permission: permission.to_string(),
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: subject_type.to_string(),
                    object_id: subject_id.to_string(),
                }),
                optional_relation: String::new(),
            }),
            consistency: None,
        };
        
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", self.token).parse().unwrap(),
        );
        
        let response = self.client.check_permission(request).await?;
        
        Ok(response.get_ref().permissionship == 
           check_permission_response::Permissionship::HasPermission as i32)
    }
}
```

## Advanced Schema Patterns

### 1. Organizational Hierarchy

```zed
definition user {}

definition organization {
    relation admin: user
    relation member: user
    
    permission manage = admin
    permission view = member + admin
}

definition note {
    relation owner: user
    relation parent_org: organization
    
    // Notes inherit permissions from organization
    permission read = owner + parent_org->view
    permission write = owner + parent_org->manage
    permission delete = owner + parent_org->admin
}
```

### 2. Sharing and Collaboration

```zed
definition user {}

definition folder {
    relation owner: user
    relation editor: user
    relation viewer: user
    
    permission read = viewer + editor + owner
    permission write = editor + owner
    permission share = owner
}

definition note {
    relation owner: user
    relation parent: folder
    
    // Inherit from folder
    permission read = owner + parent->read
    permission write = owner + parent->write
}
```

### 3. Time-Based Permissions

```zed
definition user {}

definition document {
    relation owner: user
    relation temp_viewer: user // Handled with caveats
    
    permission read = owner + temp_viewer
    permission write = owner
}
```

With caveats for expiration:
```rust
// Check with time constraint
let caveat = ContextualizedCaveat {
    caveat: Some(Caveat {
        name: "expires_at".to_string(),
        context: Some(google::protobuf::Struct {
            fields: hashmap! {
                "now".to_string() => Value {
                    kind: Some(value::Kind::NumberValue(
                        chrono::Utc::now().timestamp() as f64
                    )),
                },
            },
        }),
    }),
};
```

## Testing Your Schema

### 1. Using Assertions

Create `schema/spicedb/assertions.yaml`:

```yaml
assertions:
  - user: user:alice
    subject: note:123
    permission: write
    expected: true
    
  - user: user:bob
    subject: note:123
    permission: write
    expected: false
    
  - user: user:alice
    subject: note:456
    permission: read
    expected: false
```

Test with:
```bash
zed validate schema.zed assertions.yaml --context local
```

### 2. Integration Tests

```rust
#[tokio::test]
async fn test_spicedb_permissions() {
    let client = SpiceDBClient::connect("localhost:50051", "testkey").await.unwrap();
    
    // Create test data
    client.create_relationship("note:test1", "owner", "user:alice").await.unwrap();
    
    // Test owner can write
    assert!(client.check_permission(
        "user:alice".into(),
        "note:test1".into(),
        "write".into()
    ).await.unwrap());
    
    // Test non-owner cannot write
    assert!(!client.check_permission(
        "user:bob".into(),
        "note:test1".into(),
        "write".into()
    ).await.unwrap());
}
```

## Production Configuration

### 1. PostgreSQL Datastore

```yaml
# docker-compose.yaml
services:
  spicedb:
    image: authzed/spicedb:latest
    command: serve
    environment:
      - SPICEDB_GRPC_PRESHARED_KEY=${SPICEDB_KEY}
      - SPICEDB_DATASTORE_ENGINE=postgres
      - SPICEDB_DATASTORE_CONN_URI=postgres://spicedb:password@postgres:5432/spicedb
    ports:
      - "50051:50051"
```

### 2. Configuration Best Practices

```rust
pub struct SpiceDBConfig {
    pub endpoint: String,
    pub key: String,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl SpiceDBConfig {
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            endpoint: env::var("SPICEDB_ENDPOINT")
                .unwrap_or_else(|_| "localhost:50051".to_string()),
            key: env::var("SPICEDB_PRESHARED_KEY")
                .map_err(|_| Error::new("SPICEDB_PRESHARED_KEY not set"))?,
            timeout: Duration::from_secs(5),
            max_retries: 3,
        })
    }
}
```

### 3. Connection Management

```rust
use tonic::transport::{Channel, ClientTlsConfig};

impl SpiceDBClient {
    pub async fn connect(config: SpiceDBConfig) -> Result<Self, Error> {
        let channel = if config.endpoint.starts_with("https://") {
            Channel::from_shared(config.endpoint)?
                .tls_config(ClientTlsConfig::new())?
                .timeout(config.timeout)
                .connect()
                .await?
        } else {
            Channel::from_shared(config.endpoint)?
                .timeout(config.timeout)
                .connect()
                .await?
        };
        
        let mut client = PermissionsServiceClient::new(channel);
        
        // Add auth interceptor
        client = client.with_interceptor(move |mut req: Request<()>| {
            req.metadata_mut().insert(
                "authorization",
                format!("Bearer {}", config.key).parse().unwrap(),
            );
            Ok(req)
        });
        
        Ok(Self { client })
    }
}
```

## Monitoring and Debugging

### 1. Enable Debug Logging

```rust
// Set RUST_LOG=authzed=debug
tracing::debug!("Checking permission: {} {} {}", subject, resource, permission);
```

### 2. SpiceDB Dashboard

Access at `http://localhost:8080` when running locally. Shows:
- Schema visualization
- Relationship graphs
- Permission computation traces

### 3. Common Issues

**"Permission denied" errors:**
```bash
# Check if relationship exists
zed relationship read note:123 --context local

# Trace permission computation
zed permission check note:123 read user:alice --explain --context local
```

**Connection failures:**
```rust
// Add retry logic
use backoff::{ExponentialBackoff, retry};

pub async fn check_with_retry(&self, subject: String, resource: String, permission: String) -> Result<bool, Error> {
    retry(ExponentialBackoff::default(), || async {
        self.check_permission(subject.clone(), resource.clone(), permission.clone())
            .await
            .map_err(backoff::Error::transient)
    }).await
}
```

## Schema Evolution

### 1. Adding New Permissions

```zed
// Before
definition note {
    relation owner: user
    permission read = owner
    permission write = owner
}

// After - backward compatible
definition note {
    relation owner: user
    relation editor: user  // New relation
    
    permission read = owner + editor
    permission write = owner + editor
    permission delete = owner  // New permission
}
```

### 2. Migration Strategy

```bash
# 1. Test new schema
zed schema validate new-schema.zed

# 2. Check compatibility
zed schema diff current-schema.zed new-schema.zed

# 3. Apply in stages
zed schema write new-schema.zed --context staging
# Test thoroughly
zed schema write new-schema.zed --context production
```

## Best Practices

### 1. Resource Naming
- Always use `type:id` format
- Keep IDs consistent with your database
- Use lowercase for types

### 2. Schema Design
- Start simple, evolve as needed
- Use inheritance sparingly
- Document permission logic

### 3. Performance
- Batch permission checks when possible
- Use caching (positive results only!)
- Monitor query latency

### 4. Security
- Never expose SpiceDB directly
- Always validate input
- Use TLS in production
- Rotate keys regularly

## Troubleshooting Checklist

When permissions aren't working:

1. **Check the relationship exists**
   ```bash
   zed relationship read resource:id --context local
   ```

2. **Verify schema is loaded**
   ```bash
   zed schema read --context local
   ```

3. **Test with zed CLI**
   ```bash
   zed permission check resource:id permission user:id --explain
   ```

4. **Check resource format**
   - Must be `type:id`
   - No spaces or special characters

5. **Enable debug logging**
   - See full request/response
   - Check for auth errors

## Next Steps

1. Set up local SpiceDB instance
2. Create your schema
3. Test with zed CLI
4. Integrate with Rust code
5. Add monitoring

For more context, see:
- [Authorization Tutorial](./authorization-tutorial.md) - Full implementation
- [Circuit Breaker Guide](./circuit-breaker-guide.md) - Handling failures
- [Authorization TDD Examples](./authorization-tdd-examples.md) - Test patterns