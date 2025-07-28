# GraphQL Security Best Practices

## Overview

GraphQL's flexibility can be a security risk if not properly controlled. This guide covers essential security measures for your GraphQL API.

## 1. Query Depth Limiting

### The Problem

Malicious queries can nest infinitely deep:

```graphql
# This could crash your server!
query Evil {
  note(id: "1") {
    author {
      notes {
        author {
          notes {
            author {
              notes {
                # ... 100 levels deep
              }
            }
          }
        }
      }
    }
  }
}
```

### The Solution

Implement depth limiting:

```rust
use async_graphql::extensions::DepthLimit;

let schema = Schema::build(Query, Mutation, Subscription)
    .extension(DepthLimit::new(15))  // Maximum depth of 15
    .finish();
```

### Custom Depth Counter

For more control:

```rust
pub struct DepthAnalyzer {
    max_depth: usize,
}

impl DepthAnalyzer {
    pub fn check_query(&self, query: &str) -> Result<(), String> {
        let doc = parse_query(query)?;
        let depth = self.calculate_depth(&doc);
        
        if depth > self.max_depth {
            return Err(format!(
                "Query depth {} exceeds maximum allowed depth of {}",
                depth, self.max_depth
            ));
        }
        
        Ok(())
    }
}
```

## 2. Query Complexity Analysis

### The Problem

Even shallow queries can be expensive:

```graphql
# Not deep, but fetches massive data
query ExpensiveQuery {
  allUsers(first: 10000) {
    posts(first: 1000) {
      comments(first: 100) {
        # 10,000 × 1,000 × 100 = 1 billion items!
      }
    }
  }
}
```

### The Solution

Implement complexity scoring:

```rust
use async_graphql::extensions::ComplexityLimit;

// Simple scoring
let schema = Schema::build(Query, Mutation, Subscription)
    .extension(ComplexityLimit::new(1000))  // Max complexity score
    .finish();

// Custom complexity calculation
#[Object]
impl Query {
    #[graphql(complexity = "limit.unwrap_or(10) as usize")]
    async fn users(&self, limit: Option<i32>) -> Vec<User> {
        // Complexity based on limit
    }
    
    #[graphql(complexity = "1")]  // Simple field
    async fn me(&self) -> User {
        // Low complexity
    }
}
```

### Advanced Complexity Rules

```rust
// Calculate complexity based on arguments
fn calculate_complexity(
    limit: Option<i32>,
    depth: Option<i32>,
    include_relations: bool,
) -> usize {
    let base = limit.unwrap_or(10) as usize;
    let depth_multiplier = depth.unwrap_or(1) as usize;
    let relation_cost = if include_relations { 10 } else { 1 };
    
    base * depth_multiplier * relation_cost
}
```

## 3. Rate Limiting

### Per-User Rate Limiting

```rust
use std::sync::Arc;
use dashmap::DashMap;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    limits: Arc<DashMap<String, UserLimit>>,
    max_requests: usize,
    window: Duration,
}

struct UserLimit {
    count: usize,
    window_start: Instant,
}

impl RateLimiter {
    pub async fn check_limit(&self, user_id: &str) -> Result<(), Error> {
        let mut entry = self.limits.entry(user_id.to_string()).or_insert(UserLimit {
            count: 0,
            window_start: Instant::now(),
        });
        
        // Reset window if expired
        if entry.window_start.elapsed() > self.window {
            entry.count = 0;
            entry.window_start = Instant::now();
        }
        
        // Check limit
        if entry.count >= self.max_requests {
            return Err(Error::new("Rate limit exceeded")
                .extend_with(|_, e| {
                    e.set("code", "RATE_LIMITED");
                    e.set("retry_after", self.window.as_secs());
                }));
        }
        
        entry.count += 1;
        Ok(())
    }
}

// Use in resolver
async fn expensive_query(&self, ctx: &Context<'_>) -> Result<Data> {
    let rate_limiter = ctx.data::<Arc<RateLimiter>>()?;
    let user_id = ctx.data::<AuthContext>()?.user_id.as_ref().unwrap();
    
    rate_limiter.check_limit(user_id).await?;
    
    // Proceed with query
}
```

### Query-Specific Limits

```rust
// Different limits for different operations
pub struct QueryLimits {
    pub default: usize,
    pub expensive_queries: usize,
    pub mutations: usize,
    pub subscriptions: usize,
}

// Apply based on operation type
match operation_type {
    OperationType::Query if is_expensive => limits.expensive_queries,
    OperationType::Query => limits.default,
    OperationType::Mutation => limits.mutations,
    OperationType::Subscription => limits.subscriptions,
}
```

## 4. Authentication & Authorization

### Always Require Authentication

```rust
// Global auth check in middleware
pub async fn auth_middleware(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<Response<Body>, Error> {
    // Extract token from header
    let token = req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    
    match token {
        Some(token) => {
            let user = verify_token(token).await?;
            req.extensions_mut().insert(AuthContext { user });
            Ok(next.run(req).await)
        }
        None => {
            Err(Error::new("Authentication required"))
        }
    }
}
```

### Field-Level Authorization

```rust
#[Object]
impl User {
    async fn id(&self) -> &str {
        &self.id
    }
    
    async fn name(&self) -> &str {
        &self.name
    }
    
    // Sensitive field - only visible to self or admin
    #[graphql(guard = "SelfOrAdminGuard::new(self.id.clone())")]
    async fn email(&self, ctx: &Context<'_>) -> Result<&str> {
        Ok(&self.email)
    }
    
    // Admin only field
    #[graphql(guard = "RoleGuard::new(Role::Admin)")]
    async fn internal_notes(&self) -> &str {
        &self.internal_notes
    }
}
```

### Custom Guards

```rust
pub struct SelfOrAdminGuard {
    user_id: String,
}

#[async_trait]
impl Guard for SelfOrAdminGuard {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let auth = ctx.data::<AuthContext>()?;
        
        if auth.user_id == self.user_id || auth.is_admin {
            Ok(())
        } else {
            Err(Error::new("Not authorized to view this field"))
        }
    }
}
```

## 5. Input Validation

### Validate All Inputs

```rust
#[derive(InputObject)]
pub struct CreateNoteInput {
    #[graphql(validator(min_length = 1, max_length = 200))]
    title: String,
    
    #[graphql(validator(min_length = 1, max_length = 10000))]
    content: String,
    
    #[graphql(validator(custom = "validate_tags"))]
    tags: Vec<String>,
}

fn validate_tags(tags: &[String]) -> Result<(), String> {
    if tags.len() > 10 {
        return Err("Maximum 10 tags allowed".into());
    }
    
    for tag in tags {
        if tag.len() > 50 {
            return Err("Tag too long".into());
        }
        if !tag.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err("Invalid tag format".into());
        }
    }
    
    Ok(())
}
```

### Sanitize String Inputs

```rust
pub fn sanitize_input(input: &str) -> String {
    // Remove null bytes
    let cleaned = input.replace('\0', "");
    
    // Trim whitespace
    let trimmed = cleaned.trim();
    
    // Limit length
    let limited = if trimmed.len() > 1000 {
        &trimmed[..1000]
    } else {
        trimmed
    };
    
    limited.to_string()
}
```

## 6. Error Handling

### Don't Leak Sensitive Information

```rust
// Bad - exposes internal details
Err(Error::new(format!(
    "Database error: Failed to connect to postgres://user:pass@internal-db:5432"
)))

// Good - generic error
Err(Error::new("Failed to fetch data"))

// Better - with error code
Err(Error::new("Failed to fetch data")
    .extend_with(|_, e| e.set("code", "DATABASE_ERROR")))

// Best - log details, return generic
tracing::error!(
    error = ?db_error,
    user_id = %user_id,
    "Database query failed"
);
Err(Error::new("An error occurred while processing your request"))
```

### Structured Error Responses

```rust
pub enum ApiError {
    NotFound { resource: String },
    Unauthorized,
    Forbidden { reason: String },
    ValidationError { field: String, message: String },
    InternalError,
}

impl From<ApiError> for Error {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::NotFound { resource } => {
                Error::new(format!("{} not found", resource))
                    .extend_with(|_, e| e.set("code", "NOT_FOUND"))
            }
            ApiError::Unauthorized => {
                Error::new("Authentication required")
                    .extend_with(|_, e| e.set("code", "UNAUTHORIZED"))
            }
            ApiError::InternalError => {
                Error::new("Internal server error")
                    .extend_with(|_, e| e.set("code", "INTERNAL_ERROR"))
            }
            // ... other cases
        }
    }
}
```

## 7. Query Whitelisting (Production)

### Allow Only Known Queries

```rust
pub struct QueryWhitelist {
    allowed_queries: HashMap<String, String>,
}

impl QueryWhitelist {
    pub fn check(&self, query_hash: &str) -> Result<&str, Error> {
        self.allowed_queries
            .get(query_hash)
            .ok_or_else(|| Error::new("Query not whitelisted"))
    }
}

// In production, only accept query hashes
async fn graphql_handler_production(
    Json(request): Json<Request>,
    Extension(whitelist): Extension<Arc<QueryWhitelist>>,
) -> Result<Json<Response>, Error> {
    let query = if let Some(hash) = request.extensions.get("queryHash") {
        whitelist.check(hash.as_str())?
    } else {
        return Err(Error::new("Query hash required"));
    };
    
    // Execute whitelisted query
}
```

## 8. Timeout Protection

### Set Query Timeouts

```rust
use tokio::time::timeout;

async fn execute_with_timeout(
    schema: &Schema<Query, Mutation, Subscription>,
    request: Request,
) -> Result<Response> {
    match timeout(Duration::from_secs(30), schema.execute(request)).await {
        Ok(response) => Ok(response),
        Err(_) => Err(Error::new("Query timeout")
            .extend_with(|_, e| e.set("code", "TIMEOUT"))),
    }
}
```

## 9. CORS Configuration

### Configure CORS Properly

```rust
use tower_http::cors::{CorsLayer, AllowOrigin};

let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::exact("https://trusted-domain.com".parse().unwrap()))
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    .allow_credentials(true);

let app = Router::new()
    .route("/graphql", post(graphql_handler))
    .layer(cors);
```

## 10. Monitoring & Alerting

### Track Security Metrics

```rust
// Track suspicious activity
metrics::counter!("graphql.query.depth_limit_exceeded", 1);
metrics::counter!("graphql.query.complexity_limit_exceeded", 1);
metrics::counter!("graphql.auth.failed_attempts", 1);
metrics::counter!("graphql.rate_limit.exceeded", 1);

// Alert on patterns
if failed_auth_attempts > 10 {
    alert!("Possible brute force attack from user {}", user_id);
}
```

## Security Checklist

- [ ] Query depth limiting enabled (max 15)
- [ ] Query complexity limiting enabled (max 1000)
- [ ] Rate limiting implemented
- [ ] Authentication required for all operations
- [ ] Authorization checks in every resolver
- [ ] Input validation on all inputs
- [ ] Error messages don't leak sensitive data
- [ ] Query timeouts configured (30s)
- [ ] CORS properly configured
- [ ] Monitoring and alerting in place
- [ ] Query whitelisting for production
- [ ] No introspection in production
- [ ] Security headers configured
- [ ] TLS/HTTPS required

## Testing Security

```rust
#[cfg(test)]
mod security_tests {
    #[tokio::test]
    async fn test_depth_limit() {
        let schema = create_schema();
        let deep_query = generate_deep_query(20);  // Too deep
        
        let response = schema.execute(deep_query).await;
        assert!(response.errors.iter().any(|e| 
            e.message.contains("depth")
        ));
    }
    
    #[tokio::test]
    async fn test_complexity_limit() {
        let schema = create_schema();
        let complex_query = r#"
            query {
                users(first: 1000) {
                    posts(first: 1000) {
                        comments(first: 100) { id }
                    }
                }
            }
        "#;
        
        let response = schema.execute(complex_query).await;
        assert!(response.errors.iter().any(|e| 
            e.message.contains("complexity")
        ));
    }
}
```

Remember: Security is not optional. Every GraphQL API must implement these protections!