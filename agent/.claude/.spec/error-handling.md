# Error Handling Specification

## Core Principles

1. **Never Panic**: The server MUST NOT panic. All potential panics MUST be caught and converted to appropriate errors.
2. **Fail Gracefully**: When errors occur, the system SHOULD continue operating in a degraded mode when safe to do so.
3. **Log Everything**: All errors MUST be logged with full context internally while returning safe messages to clients.
4. **Trace Correlation**: Every error MUST include a trace ID for debugging across distributed systems.
5. **No Information Disclosure**: Error messages MUST NOT leak internal implementation details or sensitive data.

## Error Categories

### Input Validation Errors
- **Code**: `INVALID_INPUT`
- **HTTP Status**: 400
- **When**: Client provides data that fails validation
- **Example**: Empty required field, invalid email format, string exceeds max length
- **Recovery**: Not recoverable - client must fix input
- **Client Action**: Display validation error to user

### Resource Errors
- **Code**: `NOT_FOUND`
- **HTTP Status**: 404
- **When**: Requested resource doesn't exist
- **Example**: Note with given ID not found

- **Code**: `ALREADY_EXISTS`
- **HTTP Status**: 409
- **When**: Resource with unique constraint exists
- **Example**: User with email already registered

### Authorization Errors
- **Code**: `UNAUTHORIZED`
- **HTTP Status**: 401
- **When**: No valid authentication provided
- **Example**: Missing or invalid auth token

- **Code**: `FORBIDDEN`
- **HTTP Status**: 403
- **When**: Authenticated but not authorized
- **Example**: User cannot delete another user's note

### System Errors
- **Code**: `DATABASE_ERROR`
- **HTTP Status**: 503
- **When**: Database operation fails after retries exhausted
- **Example**: Connection lost, query timeout, transaction deadlock
- **Recovery**: Automatic retry with exponential backoff
- **Client Action**: Retry with backoff based on Retry-After header

- **Code**: `SERVICE_UNAVAILABLE`
- **HTTP Status**: 503
- **When**: External service is down or circuit breaker open
- **Example**: SpiceDB not responding, auth service timeout
- **Recovery**: Circuit breaker pattern, use cache if available
- **Client Action**: Retry with backoff or try alternative endpoint

- **Code**: `INTERNAL_ERROR`
- **HTTP Status**: 500
- **When**: Unexpected server error that should not happen
- **Example**: Panic recovered, assertion failure, unhandled error type
- **Recovery**: Log and alert, convert to safe error
- **Client Action**: Report issue, do not retry

- **Code**: `TIMEOUT`
- **HTTP Status**: 504
- **When**: Operation exceeds configured time limit
- **Example**: Complex query timeout, slow external service
- **Recovery**: Cancel operation, return partial results if possible
- **Client Action**: Simplify request or retry with smaller scope

- **Code**: `RATE_LIMITED`
- **HTTP Status**: 429
- **When**: Client exceeds rate limits
- **Example**: Too many requests per second
- **Recovery**: Not applicable
- **Client Action**: Respect Retry-After header, implement backoff

## Error Response Format

### GraphQL Error Structure
```json
{
  "errors": [{
    "message": "Title cannot be empty",
    "path": ["createNote", "input", "title"],
    "extensions": {
      "code": "INVALID_INPUT",
      "timestamp": "2024-01-01T00:00:00.000Z",
      "traceId": "550e8400-e29b-41d4-a716-446655440000",
      "details": {
        "field": "title",
        "constraint": "required",
        "providedValue": ""
      }
    }
  }],
  "data": null
}
```

**Required Fields:**
- `message`: Human-readable error description (MUST be safe for display)
- `path`: GraphQL query path to error location (MAY be null for request-level errors)
- `extensions.code`: Machine-readable error code (MUST be from defined categories)
- `extensions.timestamp`: ISO 8601 timestamp of error (MUST include timezone)
- `extensions.traceId`: UUID for request correlation (MUST match trace in logs)

### REST Error Structure (for health checks)
```json
{
  "error": {
    "code": "SERVICE_UNAVAILABLE",
    "message": "Database connection failed",
    "timestamp": "2024-01-01T00:00:00.000Z",
    "traceId": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

## Error Handling Implementation

### 1. Panic Prevention
```rust
// MUST NOT use unwrap() or expect() in production code paths
// Instead, use proper error handling:

// BAD - This will panic
let value = some_option.unwrap();

// GOOD - Handle the None case
let value = some_option.ok_or_else(|| {
    FieldError::new("Required value not found")
        .extend_with(|_, e| e.set("code", "INTERNAL_ERROR"))
})?;

// MUST catch panics at thread boundaries
let result = std::panic::catch_unwind(|| {
    potentially_panicking_code()
}).map_err(|_| {
    error!("Panic caught in resolver");
    FieldError::new("An unexpected error occurred")
        .extend_with(|_, e| e.set("code", "INTERNAL_ERROR"))
})?;
```

### 2. Log Internally, Return Safely
```rust
// Log the real error
error!(
    error = ?e,
    user_id = %auth.user_id,
    operation = "create_note",
    "Database query failed"
);

// Return safe error to client
return Err(FieldError::new("Unable to create note at this time")
    .extend_with(|_, e| e.set("code", "DATABASE_ERROR")));
```

### 3. Trace Everything
Every error includes a trace ID from the request context for correlation across logs.

### 4. Graceful Degradation

**Required Degradation Paths:**

1. **Cache Unavailable**:
   - MUST proceed with direct database queries
   - SHOULD log cache failure at WARN level
   - MAY increase cache timeout for reconnection

2. **Metrics Collection Failure**:
   - MUST continue normal operations
   - SHOULD buffer metrics locally up to 10k entries
   - MAY drop metrics after buffer full

3. **Non-Critical Service Failure**:
   - Authorization cache miss: MUST query SpiceDB directly
   - Webhook delivery failure: MUST queue for retry (max 1000 items)
   - Search service down: MAY return filtered results from primary DB

4. **Partial Success Handling**:
   ```rust
   // For batch operations, MUST return partial results
   let results = items.iter().map(|item| {
       match process_item(item).await {
           Ok(result) => BatchResult::Success(result),
           Err(e) => {
               warn!(error = ?e, item_id = %item.id, "Item processing failed");
               BatchResult::Failed { 
                   id: item.id,
                   error: safe_error_message(&e)
               }
           }
       }
   }).collect();
   ```

## Error Recovery Patterns

### Retry Logic
```rust
// For transient errors only
match error_code {
    "DATABASE_ERROR" | "SERVICE_UNAVAILABLE" => {
        // Retry with exponential backoff
        retry_with_backoff(operation).await
    }
    _ => {
        // Don't retry client errors
        Err(error)
    }
}
```

### Circuit Breaker Implementation

**MUST implement circuit breakers for all external services:**

```rust
// Circuit breaker states and thresholds
struct CircuitBreakerConfig {
    failure_threshold: u32,      // Default: 5 consecutive failures
    success_threshold: u32,      // Default: 2 consecutive successes to close
    timeout: Duration,           // Default: 60 seconds in open state
    half_open_max_requests: u32, // Default: 3 test requests
}

// State transitions:
// CLOSED -> OPEN: After failure_threshold consecutive failures
// OPEN -> HALF_OPEN: After timeout expires
// HALF_OPEN -> CLOSED: After success_threshold successes
// HALF_OPEN -> OPEN: After any failure
```

**Per-Service Configuration:**
- **Database**: 10 failures, 120s timeout (critical service)
- **SpiceDB**: 5 failures, 60s timeout (has cache fallback)
- **Auth Service**: 3 failures, 30s timeout (has session cache)
- **Webhooks**: 3 failures, 300s timeout (not critical)

**Circuit Breaker Behavior:**
- When OPEN: MUST return cached data if available, otherwise 503
- When HALF_OPEN: MUST only send limited test traffic
- MUST emit metrics for state changes
- MUST log state changes at WARN level

### Batch Operation Error Handling

**MUST return partial results for all batch operations:**

```graphql
# GraphQL Schema for batch results
type BatchDeleteResult {
  succeeded: [ID!]!
  failed: [BatchError!]!
  totalRequested: Int!
  totalSucceeded: Int!
  totalFailed: Int!
}

type BatchError {
  id: ID!
  code: ErrorCode!
  message: String!
}
```

**Example Response:**
```json
{
  "data": {
    "deleteNotes": {
      "succeeded": ["id1", "id2"],
      "failed": [{
        "id": "id3",
        "code": "NOT_FOUND",
        "message": "Note not found"
      }],
      "totalRequested": 3,
      "totalSucceeded": 2,
      "totalFailed": 1
    }
  }
}
```

**Batch Processing Rules:**
- MUST NOT stop processing on first error
- MUST process items in parallel where safe
- MUST respect transaction boundaries
- SHOULD process up to 1000 items per batch
- MUST return clear summary statistics

## Error Monitoring and Alerting

**Required Error Metrics:**
```prometheus
# Error rate by category
api_errors_total{code="INTERNAL_ERROR",path="/graphql"} 

# Circuit breaker state
circuit_breaker_state{service="spicedb",state="open"} 1

# Error budget tracking
error_budget_remaining{slo="99.9"} 0.98
```

**Alert Thresholds:**
- INTERNAL_ERROR rate > 1/minute: Page on-call
- Circuit breaker open > 5 minutes: Alert team
- Error rate > 5% of traffic: Investigate
- Panic recovery detected: Immediate investigation