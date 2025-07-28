# Structured Logging Guide

## What is Structured Logging?

Instead of plain text messages, structured logging outputs JSON with consistent fields. This makes logs searchable, parseable, and analyzable.

### Plain Logging (Bad)
```
2024-01-15 10:30:45 INFO User john@example.com logged in from 192.168.1.1
```

### Structured Logging (Good)
```json
{
  "timestamp": "2024-01-15T10:30:45Z",
  "level": "INFO",
  "message": "User login",
  "user_email": "<REDACTED>",
  "ip_address": "192.168.x.x",
  "trace_id": "abc-123-def",
  "span_id": "456-789"
}
```

## Setting Up Tracing

We use the `tracing` crate for structured logging:

```rust
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

pub fn init_logging(is_production: bool) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    if is_production {
        // JSON format for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(true)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_line_number(true)
            )
            .init();
    } else {
        // Pretty format for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .pretty()
                    .with_span_events(FmtSpan::CLOSE)
                    .with_target(true)
                    .with_thread_names(true)
            )
            .init();
    }
    
    Ok(())
}
```

## Log Levels and When to Use Them

```rust
use tracing::{trace, debug, info, warn, error};

// TRACE - Very detailed, usually disabled
trace!("Entering function with args: {:?}", args);

// DEBUG - Useful for debugging, disabled in production
debug!("Cache lookup for key: {}", key);

// INFO - Normal operations
info!("Server started on port {}", port);

// WARN - Something unexpected but handled
warn!("Retry attempt {} of {}", attempt, max_retries);

// ERROR - Something failed
error!("Database connection failed: {}", error);
```

### Real Examples

```rust
// INFO - Important business events
info!(
    user_id = %user.id,
    action = "create_note",
    "User created a new note"
);

// WARN - Degraded performance
warn!(
    pool_size = pool.size(),
    waiting_count = pool.waiting(),
    "Connection pool near capacity"
);

// ERROR - With context
error!(
    error = %e,
    query = %query,
    elapsed = ?start.elapsed(),
    "Database query failed"
);
```

## Security: Log Sanitization

**CRITICAL**: Never log passwords, tokens, or personal data!

### The Problem

```rust
// NEVER DO THIS!
info!("Login attempt: email={}, password={}", email, password);
// Logs: Login attempt: email=user@example.com, password=secret123

// ALSO BAD!
debug!("User data: {:?}", user);
// Might log: User { id: 123, email: "user@example.com", ssn: "123-45-6789" }
```

### The Solution: Sanitization Layer

```rust
use tracing_subscriber::{Layer, layer::Context};
use tracing::{Event, field::{Field, Visit}};

pub struct SanitizationLayer {
    sensitive_fields: Vec<String>,
    patterns: Vec<(regex::Regex, String)>,
}

impl SanitizationLayer {
    pub fn new() -> Self {
        Self {
            sensitive_fields: vec![
                "password".to_string(),
                "token".to_string(),
                "api_key".to_string(),
                "secret".to_string(),
            ],
            patterns: vec![
                // Email addresses
                (regex::Regex::new(r"\b[\w.-]+@[\w.-]+\.\w+\b").unwrap(), 
                 "<EMAIL>".to_string()),
                // Credit cards
                (regex::Regex::new(r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b").unwrap(),
                 "<CARD>".to_string()),
                // SSN
                (regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
                 "<SSN>".to_string()),
                // User IDs
                (regex::Regex::new(r"user_\d+").unwrap(),
                 "<USER_ID>".to_string()),
            ],
        }
    }
}

struct SanitizingVisitor<'a> {
    fields: &'a mut Vec<(&'static str, String)>,
    layer: &'a SanitizationLayer,
}

impl<'a> Visit for SanitizingVisitor<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        let sanitized = if self.layer.sensitive_fields.contains(&field.name().to_string()) {
            "<REDACTED>".to_string()
        } else {
            let mut result = value.to_string();
            for (pattern, replacement) in &self.layer.patterns {
                result = pattern.replace_all(&result, replacement).to_string();
            }
            result
        };
        
        self.fields.push((field.name(), sanitized));
    }
    
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.record_str(field, &format!("{:?}", value));
    }
}
```

### Using Skip for Sensitive Parameters

```rust
use tracing::instrument;

// Skip sensitive parameters entirely
#[instrument(skip(password, token))]
async fn authenticate(
    username: &str,
    password: &str,
    token: &str,
) -> Result<User> {
    info!("Authentication attempt"); // password and token not logged
    // ...
}

// Selectively log safe fields
#[instrument(fields(
    user_id = %user.id,
    username = %user.username,
    // Don't log user.email or user.phone!
))]
async fn update_user(user: &User) -> Result<()> {
    info!("Updating user");
    // ...
}
```

## Trace Context Propagation

Every log should include a trace ID for correlation:

```rust
use uuid::Uuid;
use tracing::{info_span, Instrument};

pub async fn handle_request<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response {
    // Generate or extract trace ID
    let trace_id = req
        .headers()
        .get("x-trace-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    // Create span with trace ID
    let span = info_span!(
        "http_request",
        trace_id = %trace_id,
        method = %req.method(),
        path = %req.uri().path(),
    );
    
    // Process request within span
    async move {
        info!("Request started");
        let response = next.run(req).await;
        info!(
            status = response.status().as_u16(),
            "Request completed"
        );
        response
    }
    .instrument(span)
    .await
}
```

## Logging Best Practices

### 1. Use Structured Fields

```rust
// BAD - Hard to parse
info!("User {} performed {} on {}", user_id, action, resource);

// GOOD - Easy to query
info!(
    user_id = %user_id,
    action = %action,
    resource = %resource,
    "Action performed"
);
```

### 2. Be Consistent with Field Names

```rust
// Define constants for field names
const USER_ID: &str = "user_id";
const REQUEST_ID: &str = "request_id";
const DURATION_MS: &str = "duration_ms";

// Use consistently
info!(
    { USER_ID } = %user.id,
    { REQUEST_ID } = %req_id,
    { DURATION_MS } = elapsed.as_millis(),
    "Request processed"
);
```

### 3. Add Context at Boundaries

```rust
#[instrument(
    name = "graphql_query",
    fields(
        operation_name = %query.operation_name().unwrap_or("unknown"),
        operation_type = %query.operation_type(),
    )
)]
async fn execute_graphql_query(query: GraphQLQuery) -> Result<Response> {
    info!("Executing GraphQL query");
    
    // Authorization check
    let _auth_span = info_span!("authorization").entered();
    check_authorization(&query)?;
    drop(_auth_span);
    
    // Database query
    let _db_span = info_span!("database_query").entered();
    let result = fetch_data(&query).await?;
    drop(_db_span);
    
    info!("Query completed successfully");
    Ok(result)
}
```

### 4. Log Errors with Context

```rust
async fn process_payment(payment: Payment) -> Result<Receipt> {
    match payment_gateway.charge(&payment).await {
        Ok(receipt) => {
            info!(
                payment_id = %payment.id,
                amount = payment.amount,
                "Payment processed successfully"
            );
            Ok(receipt)
        }
        Err(e) => {
            error!(
                payment_id = %payment.id,
                amount = payment.amount,
                error = %e,
                error_type = %classify_payment_error(&e),
                "Payment processing failed"
            );
            Err(e)
        }
    }
}
```

## Performance Considerations

### 1. Avoid Expensive Formatting in Hot Paths

```rust
// BAD - Formats even if trace level is disabled
trace!("Processing item: {:?}", expensive_debug_format(&item));

// GOOD - Only formats if trace is enabled
if tracing::enabled!(tracing::Level::TRACE) {
    trace!("Processing item: {:?}", expensive_debug_format(&item));
}

// BETTER - Use closure
trace!(item = ?item, "Processing item");
```

### 2. Use Async-Safe Logging

```rust
// For high-throughput scenarios
use tracing_appender::non_blocking;

let (non_blocking_writer, _guard) = non_blocking(std::io::stdout());

tracing_subscriber::fmt()
    .with_writer(non_blocking_writer)
    .init();
```

### 3. Sample Verbose Logs

```rust
use rand::Rng;

// Log only 1% of trace-level events
if rand::thread_rng().gen_ratio(1, 100) {
    trace!("Detailed trace information");
}

// Always log errors
error!("This error is always logged");
```

## Testing Logs

```rust
#[cfg(test)]
mod tests {
    use tracing_test::traced_test;
    
    #[traced_test]
    #[test]
    fn test_log_sanitization() {
        // Your code that logs
        info!(
            email = "test@example.com",
            user_id = "user_12345",
            "Test log"
        );
        
        // Verify sanitization
        assert!(logs_contain("<EMAIL>"));
        assert!(logs_contain("<USER_ID>"));
        assert!(!logs_contain("test@example.com"));
        assert!(!logs_contain("user_12345"));
    }
    
    #[traced_test]
    #[test]
    fn test_error_logging() {
        // Trigger an error
        let result = risky_operation();
        
        if result.is_err() {
            error!(error = %result.unwrap_err(), "Operation failed");
        }
        
        // Verify error was logged
        assert!(logs_contain("ERROR"));
        assert!(logs_contain("Operation failed"));
    }
}
```

## Common Patterns

### Request Logging Middleware

```rust
pub async fn logging_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();
    
    let span = info_span!(
        "http_request",
        method = %method,
        path = %path,
    );
    
    let response = next.run(req).instrument(span.clone()).await;
    
    let elapsed = start.elapsed();
    let status = response.status();
    
    span.in_scope(|| {
        info!(
            status = status.as_u16(),
            duration_ms = elapsed.as_millis(),
            "Request completed"
        );
    });
    
    Ok(response)
}
```

### Database Query Logging

```rust
#[instrument(skip(conn), fields(query_type = "select"))]
async fn get_user_by_id(
    conn: &DbConnection,
    user_id: &str,
) -> Result<User> {
    debug!("Fetching user from database");
    
    let start = Instant::now();
    let result = conn
        .query_one("SELECT * FROM users WHERE id = $1", &[&user_id])
        .await;
    
    match result {
        Ok(user) => {
            debug!(
                elapsed_ms = start.elapsed().as_millis(),
                "User fetched successfully"
            );
            Ok(user)
        }
        Err(e) => {
            error!(
                error = %e,
                elapsed_ms = start.elapsed().as_millis(),
                "Failed to fetch user"
            );
            Err(e)
        }
    }
}
```

## Production Checklist

1. ✅ JSON format enabled in production
2. ✅ Sensitive data sanitization active
3. ✅ Trace IDs on all requests
4. ✅ Error logs include context
5. ✅ Log levels appropriate (INFO in prod)
6. ✅ No debug! or trace! in hot paths
7. ✅ Async logging for high throughput
8. ✅ Log rotation configured
9. ✅ No println! or dbg! macros
10. ✅ Performance impact measured

Remember: Logs are your best friend during incidents. Make them useful!