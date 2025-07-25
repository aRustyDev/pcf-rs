# Container Deployment Directives for GraphQL API Servers

## Container Deployment Configuration Standards

Your containerized GraphQL API server must implement strict configuration hierarchy following the 12-factor app methodology. This approach ensures predictable behavior across environments and prevents configuration drift between development and production.

### Configuration Loading Order and Implementation

You must implement a four-tier configuration hierarchy with the following precedence order, where higher numbers override lower numbers:

1. **Default values** (lowest priority) - Hardcoded in your application
2. **Configuration files** - Loaded from the filesystem
3. **Environment variables** - Injected by the container runtime
4. **Command-line arguments** (highest priority) - Passed at startup

Here's the mandatory implementation pattern you must follow:

```rust
use figment::{Figment, providers::{Format, Toml, Env, Serialized}};
use clap::Parser;

#[derive(Parser)]
struct CliArgs {
    #[arg(long, env = "APP_CONFIG_PATH")]
    config_path: Option<String>,
    #[arg(long, env = "APP_PORT")]
    port: Option<u16>,
}

pub fn load_configuration() -> Result<AppConfig, ConfigError> {
    let cli_args = CliArgs::parse();

    // You MUST load in this exact order
    let mut figment = Figment::new()
        // 1. Start with hardcoded defaults
        .merge(Serialized::defaults(AppConfig::default()))
        // 2. Load base configuration file
        .merge(Toml::file("config/default.toml").nested())
        // 3. Override with environment-specific file
        .merge(Toml::file(format!("config/{}.toml",
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string())
        )).nested())
        // 4. Apply environment variables with APP_ prefix
        .merge(Env::prefixed("APP_").split("__"));

    // 5. Apply CLI arguments last
    if let Some(port) = cli_args.port {
        figment = figment.merge(("server.port", port));
    }

    figment.extract()
}
```

The double underscore (`__`) separator in environment variables maps to nested configuration. For example, `APP_SERVER__PORT=8080` sets `server.port` to 8080. You must use this exact separator pattern.

### Container Image Build Requirements

Your Dockerfile must separate configuration from code by following this exact structure:

```dockerfile
# Build stage
FROM rust:latest AS builder

# Install all necessary build dependencies
RUN apt-get update && apt-get install -y \
    musl-tools \
    musl-dev \
    build-essential \
    cmake \
    clang \
    llvm-dev \
    libclang-dev \
    pkg-config \
    protobuf-compiler \
    perl \
    && rm -rf /var/lib/apt/lists/*

# Add musl target
RUN rustup target add x86_64-unknown-linux-musl

# Create symlink for musl g++
RUN ln -s /usr/bin/g++ /usr/bin/x86_64-linux-musl-g++

WORKDIR /build

# Copy cargo configuration first
COPY ./build/.cargo .cargo

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs for dependency compilation
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release --target x86_64-unknown-linux-musl || true
RUN rm -rf src

# Copy actual source code
COPY src ./src

# Build the final binary
RUN cargo build --release --target x86_64-unknown-linux-musl

# ============================================
# Runtime stage
FROM scratch

# Copy the static binary from builder
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/api /api

# Expose the port
EXPOSE 4000

# Expose configuration through environment
ENV APP_ENVIRONMENT=production
ENV APP_CONFIG_PATH=/app/config

# Run the binary
ENTRYPOINT ["/api"]
```

Never include environment-specific configuration files (development.toml, staging.toml, production.toml) in the container image. These must be mounted at runtime.

## GraphQL Security Configuration

Your GraphQL server must implement these security controls without exception. These are not optional features but mandatory requirements for production deployment.

### Introspection and Development Features

In production environments, you must completely disable GraphQL introspection and any development tools. Implement this detection and configuration:

```rust
#[derive(Deserialize, Clone)]
pub struct GraphQLSecurityConfig {
    // These MUST be false in production
    pub enable_introspection: bool,
    pub enable_playground: bool,
    pub enable_graphiql: bool,

    // These MUST be set to reasonable limits
    pub max_depth: u32,
    pub max_complexity: u32,
    pub max_aliases: u32,
}

impl GraphQLSecurityConfig {
    pub fn validate_for_production(&self) -> Result<(), SecurityError> {
        if std::env::var("ENVIRONMENT").unwrap_or_default() == "production" {
            if self.enable_introspection {
                return Err(SecurityError::IntrospectionEnabled);
            }
            if self.enable_playground || self.enable_graphiql {
                return Err(SecurityError::DevelopmentToolsEnabled);
            }
        }

        // Enforce maximum limits
        if self.max_depth > 15 {
            return Err(SecurityError::DepthLimitTooHigh(self.max_depth));
        }
        if self.max_complexity > 1000 {
            return Err(SecurityError::ComplexityLimitTooHigh(self.max_complexity));
        }

        Ok(())
    }
}
```

### Query Depth and Complexity Limits

You must implement query depth limiting to prevent malicious nested queries. Set your depth limit between 5 and 15 levels based on your schema design. Here's the mandatory implementation:

```rust
use async_graphql::{extensions::DepthLimit, Schema};

pub fn build_secure_schema(config: &GraphQLSecurityConfig) -> Schema<Query, Mutation, Subscription> {
    let mut builder = Schema::build(query_root, mutation_root, subscription_root)
        .extension(DepthLimit::new(config.max_depth as usize))
        .limit_complexity(config.max_complexity as usize);

    // Production mode enforcement
    if std::env::var("ENVIRONMENT").unwrap_or_default() == "production" {
        builder = builder
            .disable_introspection()
            .disable_suggestions();  // Also disable field suggestions in errors
    }

    builder.finish()
}
```

For complexity calculation, you must implement a cost analysis system that assigns points to each field based on its computational expense. Database queries should cost more than scalar field access:

```rust
// Implement this for each resolver
impl Query {
    #[graphql(complexity = "limit * 2 + 10")]
    async fn users(&self, limit: i32) -> Result<Vec<User>> {
        // The complexity formula accounts for both the base cost (10)
        // and the multiplier effect of the limit parameter
    }
}
```

## Secret Management Architecture

You must never embed secrets in container images or source code. This is an absolute requirement with no exceptions.

### Kubernetes Secrets Implementation

When deploying to Kubernetes, you must use Secrets for all sensitive configuration:

```yaml
# secret.yaml - Create this separately, never commit to version control
apiVersion: v1
kind: Secret
metadata:
  name: graphql-api-secrets
type: Opaque
stringData:
  database-url: "postgresql://user:pass@host:5432/db"
  jwt-secret: "your-256-bit-secret-key"
  api-keys: |
    service1=key1
    service2=key2
```

Mount these secrets as environment variables in your deployment:

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: graphql-api
spec:
  template:
    spec:
      containers:
      - name: api
        image: your-registry/graphql-api:latest
        envFrom:
        - secretRef:
            name: graphql-api-secrets
        env:
        - name: APP_DATABASE__URL
          valueFrom:
            secretKeyRef:
              name: graphql-api-secrets
              key: database-url
```

### External Vault Integration

For production systems, you must integrate with a proper secret management system. Here's the required pattern using HashiCorp Vault as an example:

```rust
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

pub struct SecretManager {
    client: VaultClient,
    mount_path: String,
}

impl SecretManager {
    pub async fn initialize() -> Result<Self, SecretError> {
        // Vault token MUST come from environment or service account
        let token = std::env::var("VAULT_TOKEN")
            .map_err(|_| SecretError::MissingVaultToken)?;

        let client = VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(std::env::var("VAULT_ADDR")?)
                .token(token)
                .build()?
        )?;

        Ok(SecretManager {
            client,
            mount_path: "secret/data/graphql-api".to_string(),
        })
    }

    pub async fn get_database_url(&self) -> Result<String, SecretError> {
        // Secrets are fetched at runtime, never cached to disk
        let secret: HashMap<String, String> = self.client
            .kv2()
            .read(&self.mount_path)
            .await?;

        secret.get("database_url")
            .ok_or(SecretError::MissingSecret("database_url"))
            .map(|s| s.to_string())
    }
}
```

You must rotate secrets regularly and implement proper error handling for secret retrieval failures. The application must fail to start if critical secrets are unavailable.

## Performance Configuration Requirements

Your GraphQL API must implement these performance optimizations to handle production loads effectively.

### Database Connection Pooling

You must configure connection pooling with these specific parameters:

```rust
use sqlx::postgres::{PgPoolOptions, PgPool};
use std::time::Duration;

pub async fn create_database_pool(config: &DatabaseConfig) -> Result<PgPool, DatabaseError> {
    PgPoolOptions::new()
        // Minimum connections to maintain
        .min_connections(5)
        // Maximum connections allowed
        .max_connections(config.max_connections.unwrap_or(100))
        // How long to wait for a connection
        .acquire_timeout(Duration::from_secs(3))
        // How long connections can be idle
        .idle_timeout(Duration::from_secs(600))
        // Maximum lifetime of any connection
        .max_lifetime(Duration::from_secs(1800))
        // Test connections before use
        .test_before_acquire(true)
        .connect(&config.url)
        .await
}
```

Monitor pool saturation and adjust max_connections based on your database's connection limits minus overhead for maintenance operations.

### DataLoader Implementation

You must implement DataLoader for all N+1 query scenarios. This is not optional for any resolver that fetches related data:

```rust
use async_graphql::dataloader::{DataLoader, Loader};
use std::collections::HashMap;

pub struct UserLoader {
    pool: PgPool,
}

#[async_trait]
impl Loader<i32> for UserLoader {
    type Value = User;
    type Error = DataLoaderError;

    async fn load(&self, keys: &[i32]) -> Result<HashMap<i32, Self::Value>, Self::Error> {
        // Batch load all requested users in a single query
        let users = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = ANY($1)",
            keys
        )
        .fetch_all(&self.pool)
        .await?;

        // Return hashmap keyed by user ID
        Ok(users.into_iter().map(|u| (u.id, u)).collect())
    }
}

// In your GraphQL context
pub struct GraphQLContext {
    pub user_loader: DataLoader<UserLoader>,
    pub post_loader: DataLoader<PostLoader>,
    // Add a loader for EVERY related entity type
}
```

Configure DataLoader batch sizes based on your database's IN clause limitations:

```rust
DataLoader::new(user_loader)
    .max_batch_size(1000)  // PostgreSQL handles up to ~1000 IN values efficiently
```

### Request Timeout Configuration

You must implement request timeouts at multiple levels to prevent resource exhaustion:

```rust
use tower::timeout::TimeoutLayer;
use std::time::Duration;

// HTTP server timeout
let app = Router::new()
    .route("/graphql", post(graphql_handler))
    .layer(TimeoutLayer::new(Duration::from_secs(30)));

// GraphQL execution timeout
let schema = schema_builder
    .extension(async_graphql::extensions::TimeoutExtension::new(
        Duration::from_secs(25)  // 5 seconds less than HTTP timeout
    ))
    .finish();

// Database query timeout
let pool = PgPoolOptions::new()
    .acquire_timeout(Duration::from_secs(3))
    .connect_with(
        config.url.parse::<PgConnectOptions>()?
            .statement_cache_capacity(100)
            .application_name("graphql-api")
            // Individual query timeout
            .options([("statement_timeout", "20s")])
    )
    .await?;
```

The timeout hierarchy must follow this pattern: Database Query Timeout (20s) < GraphQL Execution Timeout (25s) < HTTP Request Timeout (30s). This ensures clean error propagation and prevents partial response states.

### Response Caching Strategy

Implement caching for expensive computations, but you must never cache user-specific data without proper cache key isolation:

```rust
use std::sync::Arc;
use moka::sync::Cache;

pub struct CacheManager {
    // Schema introspection cache (safe to share globally)
    schema_cache: Cache<String, String>,
    // User-specific cache (must include user ID in key)
    user_cache: Cache<String, Vec<u8>>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            schema_cache: Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(3600))
                .build(),
            user_cache: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    pub fn cache_user_query(&self, user_id: i32, query_hash: u64, result: &[u8]) {
        // User ID MUST be part of the cache key
        let key = format!("user:{}:query:{}", user_id, query_hash);
        self.user_cache.insert(key, result.to_vec());
    }
}
```

These directives form the mandatory baseline for production GraphQL API deployment. Deviation from these patterns will result in security vulnerabilities, performance degradation, or operational failures. Implement exactly as specified without shortcuts or simplifications.
