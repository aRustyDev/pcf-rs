# Logging Specification

## Core Principles

1. **Security First**: MUST NEVER log sensitive data, even at TRACE level
2. **Structured Format**: All logs MUST use structured JSON in production
3. **Trace Correlation**: Every log entry MUST include trace_id for distributed tracing
4. **Performance**: Logging MUST NOT block request processing
5. **Compliance**: Logs MUST be suitable for audit and compliance requirements

## Log Levels

### TRACE
- **Usage**: Detailed debugging information
- **Examples**: Function entry/exit, sanitized variable values
- **Production**: MUST be disabled
- **Security**: MUST NOT log any user data, even sanitized

### DEBUG  
- **Usage**: Debugging information helpful for development
- **Examples**: SQL queries (fully sanitized), cache operations, timing data
- **Production**: MUST be disabled
- **Security**: MUST replace all identifiers with placeholders

### INFO
- **Usage**: Normal operational events
- **Examples**: 
  - Server started on port 4000
  - GraphQL query 'getNotes' executed in 45ms
  - Database connection established
- **Production**: Default level

### WARN
- **Usage**: Warning conditions that don't prevent operation
- **Examples**:
  - Retry attempt 3/5 for database connection
  - Query took >1s to execute
  - Deprecated field 'oldField' used
- **Production**: Enabled

### ERROR
- **Usage**: Error conditions that failed an operation
- **Examples**:
  - Failed to connect to SpiceDB
  - GraphQL resolver error
  - Invalid configuration detected
- **Production**: Enabled

### FATAL
- **Usage**: Unrecoverable errors requiring shutdown
- **Examples**:
  - Cannot bind to port
  - Invalid configuration file
  - Out of memory
- **Production**: Enabled, causes exit

## Structured Log Format

### JSON Format (Production)
```json
{
  "timestamp": "2024-01-01T00:00:00.000Z",
  "level": "INFO",
  "target": "pcf_api::graphql::resolvers",
  "message": "GraphQL query executed",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "span_id": "e29b41d4",
  "fields": {
    "operation_type": "query",
    "operation_name": "getNotes", 
    "duration_ms": 45,
    "user_id": "user_123"
  }
}
```

### Pretty Format (Development)
```
2024-01-01T00:00:00.000Z INFO pcf_api::graphql: GraphQL query executed
  operation_type: query
  operation_name: getNotes
  duration_ms: 45
  trace_id: 550e8400-e29b-41d4-a716-446655440000
```

### Required Fields

Every log entry MUST include:
- `timestamp`: ISO 8601 format with milliseconds and timezone
- `level`: One of TRACE, DEBUG, INFO, WARN, ERROR, FATAL
- `target`: Module path where log originated
- `message`: Human-readable description
- `trace_id`: UUID for request correlation

Optional but recommended:
- `span_id`: For detailed tracing
- `user_id`: Hashed or bucketed, never raw
- `environment`: production/staging/development
- `version`: Application version

## Module-Specific Requirements

### Main/Startup
```
INFO: Starting PCF API v1.0.0
INFO: Configuration loaded for environment: development
INFO: Database connection pool initialized (size: 10)
INFO: SpiceDB client initialized
INFO: GraphQL schema loaded with 10 types
INFO: Server listening on 0.0.0.0:4000
```

### GraphQL Operations
```
INFO: GraphQL mutation 'createNote' started [trace_id: abc-123]
DEBUG: Validating input for createNote [trace_id: abc-123]
DEBUG: Authorization check: user_123 can create note [trace_id: abc-123]
INFO: GraphQL mutation 'createNote' completed in 67ms [trace_id: abc-123]
```

### Database Operations
```
INFO: Connecting to SurrealDB at ws://localhost:8000
DEBUG: Executing query: SELECT * FROM note WHERE author = $1 [params: sanitized]
WARN: Database query slow: 1,234ms for query 'getNotesByAuthor'
ERROR: Database connection lost: timeout after 30s
INFO: Retrying database connection (attempt 2/∞)
```

### Health Checks
```
INFO: Health check started
DEBUG: Checking database health
DEBUG: Checking SpiceDB health
INFO: Health status: healthy (db: 5ms, spicedb: 8ms)
WARN: Health status changed from 'healthy' to 'degraded'
```

### Subscription Lifecycle
```
INFO: WebSocket connection established [client_id: ws_123]
DEBUG: Subscription 'noteCreated' registered [client_id: ws_123]
INFO: Subscription 'noteCreated' delivered event [client_id: ws_123]
INFO: WebSocket connection closed [client_id: ws_123, duration: 3,456s]
```

## Security Requirements

### MUST NEVER Log (Automatic Detection Required)

**Credentials and Secrets:**
- Passwords, password hashes, or password hints
- API keys, tokens, or session IDs  
- Private keys or certificates
- Database connection strings with credentials
- OAuth tokens or refresh tokens

**Personal Identifiable Information (PII):**
- Email addresses (use hash or domain only)
- Phone numbers
- Social security numbers or government IDs
- Credit card or payment information
- IP addresses (may log subnet)
- Real names (use user ID or hash)

**System Information:**
- Full stack traces in production
- Internal file paths
- Internal service URLs
- Database schema details

### Sanitization Rules

```rust
// Implement automatic sanitization
struct LogSanitizer {
    patterns: Vec<Regex>,
}

impl LogSanitizer {
    fn new() -> Self {
        Self {
            patterns: vec![
                // Email addresses
                Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
                // Bearer tokens
                Regex::new(r"Bearer\s+[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+").unwrap(),
                // API keys (common patterns)
                Regex::new(r"(api[_-]?key|apikey|auth|token)[\s:=]+['\"]?([A-Za-z0-9\-_]{20,})['\"]?").unwrap(),
                // Credit cards
                Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap(),
            ],
        }
    }
    
    fn sanitize(&self, text: &str) -> String {
        let mut result = text.to_string();
        for pattern in &self.patterns {
            result = pattern.replace_all(&result, "[REDACTED]").to_string();
        }
        result
    }
}

// Use in logging macro
macro_rules! safe_log {
    ($level:expr, $($arg:tt)*) => {
        let message = format!($($arg)*);
        let sanitized = LOG_SANITIZER.sanitize(&message);
        log::log!($level, "{}", sanitized);
    };
}
```

### Sensitive Field Handling

```rust
use serde::Serialize;

// Custom serialization for logging
#[derive(Serialize)]
struct User {
    id: String,
    #[serde(serialize_with = "hash_email")]
    email: String,
    #[serde(skip_serializing)]
    password_hash: String,
    #[serde(serialize_with = "redact")]
    phone: Option<String>,
}

fn hash_email<S>(email: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let parts: Vec<&str> = email.split('@').collect();
    let domain = parts.get(1).unwrap_or(&"unknown");
    serializer.serialize_str(&format!("***@{}", domain))
}

fn redact<S, T>(_value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str("[REDACTED]")
}

// Implement safe Debug
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("email", &hash_email_string(&self.email))
            .finish_non_exhaustive()
    }
}
```

## Performance Requirements

### Async and Non-Blocking

```rust
use tokio::sync::mpsc;
use std::time::Duration;

struct AsyncLogger {
    sender: mpsc::Sender<LogEntry>,
}

impl AsyncLogger {
    fn new(buffer_size: usize) -> (Self, mpsc::Receiver<LogEntry>) {
        let (sender, receiver) = mpsc::channel(buffer_size);
        (Self { sender }, receiver)
    }
    
    async fn log(&self, entry: LogEntry) {
        // Non-blocking send, drop log if buffer full
        let _ = self.sender.try_send(entry);
    }
}

// Background worker
async fn log_writer(mut receiver: mpsc::Receiver<LogEntry>) {
    let mut buffer = Vec::with_capacity(1000);
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    
    loop {
        tokio::select! {
            Some(entry) = receiver.recv() => {
                buffer.push(entry);
                if buffer.len() >= 1000 {
                    flush_logs(&mut buffer).await;
                }
            }
            _ = interval.tick() => {
                if !buffer.is_empty() {
                    flush_logs(&mut buffer).await;
                }
            }
        }
    }
}
```

### Buffer Management
- MUST buffer logs in memory (default: 10,000 entries)
- MUST flush on: buffer full, time interval (100ms), or shutdown
- MUST drop logs if buffer full rather than block
- SHOULD track dropped log count in metrics

### Sampling Configuration

```rust
#[derive(Clone)]
struct SamplingConfig {
    rules: HashMap<String, SamplingRule>,
}

#[derive(Clone)]
struct SamplingRule {
    operation: String,
    status: Option<String>,
    sample_rate: f32,
    max_per_second: Option<u32>,
}

impl SamplingConfig {
    fn should_log(&self, operation: &str, status: &str) -> bool {
        if let Some(rule) = self.rules.get(operation) {
            // Check status match
            if let Some(rule_status) = &rule.status {
                if rule_status != status {
                    return true; // Different status, always log
                }
            }
            
            // Apply sampling rate
            if rand::random::<f32>() > rule.sample_rate {
                return false;
            }
            
            // Apply rate limiting
            if let Some(max_per_sec) = rule.max_per_second {
                return self.check_rate_limit(operation, max_per_sec);
            }
            
            true
        } else {
            true // No rule, always log
        }
    }
}

// Configuration example
let sampling = SamplingConfig {
    rules: HashMap::from([
        ("health_check".to_string(), SamplingRule {
            operation: "health_check".to_string(),
            status: Some("success".to_string()),
            sample_rate: 0.01,  // 1%
            max_per_second: Some(1),
        }),
        ("graphql_query".to_string(), SamplingRule {
            operation: "graphql_query".to_string(),
            status: Some("success".to_string()),
            sample_rate: 0.1,   // 10%
            max_per_second: Some(100),
        }),
    ]),
};
```

### Context Propagation and Compliance

```rust
use tracing::{info_span, Instrument};
use uuid::Uuid;

// Request context with compliance fields
#[derive(Clone)]
struct LogContext {
    trace_id: Uuid,
    user_id_hash: String,
    session_id_hash: String,
    request_path: String,
    client_ip_subnet: String,  // e.g., "192.168.0.0/24"
    compliance_tags: Vec<String>,
}

// Middleware to inject context
async fn logging_middleware(
    req: Request,
    next: Next,
) -> Response {
    let trace_id = extract_or_generate_trace_id(&req);
    let context = LogContext {
        trace_id,
        user_id_hash: hash_user_id(req.user_id()),
        session_id_hash: hash_session_id(req.session_id()),
        request_path: sanitize_path(req.path()),
        client_ip_subnet: extract_subnet(req.client_ip()),
        compliance_tags: vec![],
    };
    
    let span = info_span!(
        "request",
        trace_id = %context.trace_id,
        user_id = %context.user_id_hash,
        path = %context.request_path,
    );
    
    async move {
        info!("Request started");
        let response = next.run(req).await;
        info!(
            status = response.status().as_u16(),
            duration_ms = span.elapsed().as_millis(),
            "Request completed"
        );
        response
    }.instrument(span).await
}

// Hash functions for compliance
fn hash_user_id(user_id: Option<&str>) -> String {
    match user_id {
        Some(id) => {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(id.as_bytes());
            format!("{:x}", hasher.finalize())[..8].to_string()
        }
        None => "anonymous".to_string(),
    }
}
```

## Log Retention and Compliance

### Retention Policy
- **Production Logs**: MUST retain for 30 days minimum
- **Security Events**: MUST retain for 1 year
- **Debug Logs**: MAY retain for 7 days
- **PII Logs**: MUST NOT create (automatic prevention)

### Compliance Tags
Logs MUST include tags for compliance filtering:
```rust
enum ComplianceTag {
    SecurityEvent,      // Login attempts, permission changes
    DataAccess,        // CRUD operations on user data
    ConfigChange,      // System configuration modifications
    ErrorEvent,        // System errors and failures
    PerformanceEvent,  // Slow queries, timeouts
}
```