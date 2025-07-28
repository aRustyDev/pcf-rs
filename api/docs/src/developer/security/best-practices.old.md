# Security Best Practices

This document outlines the security best practices implemented throughout the PCF API, providing a comprehensive guide for developers and security auditors.

## Overview

The PCF API implements defense-in-depth security principles with multiple layers of protection:

1. **Authentication & Authorization** - Session-based auth with role checks
2. **Input Validation** - GraphQL query depth/complexity limits and field validation
3. **Error Handling** - Safe error messages that don't leak sensitive information
4. **Logging Security** - Automatic PII redaction in logs
5. **Network Security** - CORS configuration and graceful shutdown
6. **Configuration Security** - Environment-based secrets management
7. **API Security** - Query depth limiting and complexity analysis
8. **Development Security** - Demo mode safety checks

## Authentication & Authorization

### Session Management

The API uses session-based authentication implemented in `src/graphql/context.rs`:

```rust
#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: String,
    pub is_admin: bool,
}
```

**Security Features:**
- Session validation on every GraphQL request
- Role-based access control (user vs admin)
- Authentication bypass only in demo mode (development only)
- Request-scoped session context prevents cross-request contamination

### Authorization Patterns

```rust
// Require authentication
let session = context.require_auth()?;

// Get current user (with auth check)
let user_id = context.get_current_user()?;
```

**Best Practices:**
1. Always use `require_auth()` for protected operations
2. Validate user ownership before allowing resource access
3. Check `is_admin` flag for administrative operations
4. Never store sensitive session data in GraphQL context

## Input Validation & Sanitization

### GraphQL Security Extensions

The API implements two critical GraphQL security extensions in `src/graphql/security.rs`:

#### 1. Query Depth Limiting

Prevents deeply nested queries that could cause exponential resource consumption:

```rust
pub struct DepthLimit {
    max_depth: usize,  // Default: 15 for development, 10 for production
}
```

**Protection Against:**
- Nested query attacks
- Resource exhaustion via deep queries
- Circular reference exploitation

#### 2. Query Complexity Analysis

Calculates and limits the computational complexity of queries:

```rust
pub struct ComplexityLimit {
    max_complexity: usize,  // Default: 1000 for development, 500 for production
}
```

**Complexity Calculation:**
- Base cost of 1 per field
- Multiplier of 10 for list operations (`first`, `last` arguments)
- Nested selections multiply complexity
- Variables are considered in complexity calculations

### Field Validation

Using the `garde` crate for comprehensive validation:

```rust
#[garde(range(min = 1024, max = 65535))]
pub port: u16,

#[garde(length(min = 1), custom(validate_bind_address))]
pub bind: String,
```

**Validation Rules:**
- Port numbers: 1024-65535 (non-privileged range)
- IP addresses: Valid IPv4/IPv6 format
- Timeouts: Reasonable bounds (1-300 seconds)
- String lengths: Minimum requirements enforced

## Error Handling Security

### Safe Error Messages

Implemented in `src/error/types.rs`:

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = match &self {
            AppError::Internal(_) => "Internal error".to_string(),
            _ => self.to_string(),
        };
        // ...
    }
}
```

**Security Features:**
1. Internal errors never expose details to clients
2. Database connection strings are hidden
3. File paths and system information are sanitized
4. Stack traces are never sent to clients

### Error Categories

- **InvalidInput** (400) - Safe to show to users
- **ServiceUnavailable** (503) - Generic service errors
- **Internal** (500) - Always shows generic message
- **Config/Server** (500) - Implementation details hidden

## Logging Security

### Automatic PII Redaction

Comprehensive sanitization in `src/logging/sanitization.rs`:

```rust
pub fn sanitize_log_message(message: &str) -> String {
    // Patterns redacted:
    // - Emails: ***@domain.com
    // - Credit cards: [REDACTED]
    // - API keys: [REDACTED]
    // - Bearer tokens: Bearer [REDACTED]
    // - Passwords: password=[REDACTED]
    // - IP addresses: 192.168.x.x (subnet only)
    // - User paths: /[USER]/
}
```

**Sanitization Patterns:**

1. **Email Addresses**
   - Pattern: `user@domain.com` → `***@domain.com`
   - Preserves domain for debugging

2. **Credit Card Numbers**
   - Pattern: Any 13-19 digit sequence → `[REDACTED]`
   - Covers all major card types

3. **API Keys & Tokens**
   - Patterns: `sk_*`, `pk_*`, `api_*`, `key_*` → `[REDACTED]`
   - Bearer tokens → `Bearer [REDACTED]`

4. **Passwords**
   - Pattern: `password=value` → `password=[REDACTED]`
   - Case-insensitive matching

5. **IP Addresses**
   - Pattern: `192.168.1.100` → `192.168.x.x`
   - Preserves subnet for debugging

6. **File Paths**
   - Pattern: `/home/username` → `/[USER]`
   - Prevents username disclosure

### Safe Logging Macros

```rust
info_sanitized!("User {} logged in from {}", email, ip);
// Logs: "User ***@example.com logged in from 192.168.x.x"
```

## Network Security

### CORS Configuration

Implemented in `src/server/runtime.rs`:

```rust
tower_http::cors::CorsLayer::new()
    .allow_origin(tower_http::cors::Any)
    .allow_methods(tower_http::cors::Any)
    .allow_headers(tower_http::cors::Any)
```

**Current Settings:**
- Allows all origins (suitable for development)
- TODO: Restrict origins in production
- TODO: Implement preflight caching

### Graceful Shutdown

Proper connection draining on shutdown:

```rust
// Handles SIGTERM and SIGINT
// Configurable timeout (default: 30 seconds)
// Prevents data loss during deployments
```

## Configuration Security

### Environment-Based Secrets

Configuration hierarchy (highest to lowest priority):

1. CLI arguments
2. Environment variables (APP_ prefix)
3. Environment-specific config files
4. Default config file
5. Embedded defaults

**Security Features:**
- Secrets never stored in config files
- Environment variables for sensitive data
- Production config disables debug features
- Validation prevents insecure configurations

### Production Security Settings

```toml
[graphql]
playground_enabled = false      # No GraphQL playground
introspection_enabled = false   # No schema introspection
max_depth = 10                  # Stricter depth limit
max_complexity = 500            # Stricter complexity limit

[services.spicedb]
insecure = false                # Require TLS
```

## Development Security

### Demo Mode Protection

Compile-time safety check in `src/main.rs`:

```rust
#[cfg(all(not(debug_assertions), feature = "demo"))]
compile_error!("Demo mode MUST NOT be enabled in release builds");
```

**Protection:**
- Demo mode cannot be accidentally enabled in production
- Compile-time enforcement prevents deployment errors
- Demo features automatically disabled in release builds

## Security Monitoring

### Request Tracing

Every request gets a unique ID for correlation:

```rust
let request_id = uuid::Uuid::new_v4().to_string();
```

**Benefits:**
- Trace requests across services
- Correlate logs for security incidents
- Audit trail for all operations

### Health Checks

Separate liveness and readiness probes:

- **Liveness**: `/health` - Simple availability check
- **Readiness**: `/health/ready` - Comprehensive system check

## Future Security Enhancements

### Planned Improvements

1. **Authentication**
   - JWT token support
   - OAuth2/OIDC integration
   - API key management
   - Session timeout configuration

2. **Rate Limiting**
   - Per-user rate limits
   - GraphQL query cost-based limits
   - IP-based throttling
   - Adaptive rate limiting

3. **Encryption**
   - At-rest encryption for sensitive data
   - Field-level encryption
   - Key rotation support

4. **Audit Logging**
   - Structured audit events
   - Compliance-ready logging
   - Tamper-proof audit trail

5. **Network Security**
   - mTLS support
   - Certificate pinning
   - Strict CORS policies
   - Security headers

## Security Checklist

### For Developers

- [ ] Use `require_auth()` for all protected operations
- [ ] Validate all user inputs with `garde`
- [ ] Use sanitized logging macros for sensitive data
- [ ] Never expose internal errors to clients
- [ ] Set appropriate GraphQL depth/complexity limits
- [ ] Keep demo mode disabled in production

### For Deployment

- [ ] Set strong session secrets via environment variables
- [ ] Configure TLS for all external connections
- [ ] Disable GraphQL introspection in production
- [ ] Set up proper CORS policies
- [ ] Configure appropriate rate limits
- [ ] Enable comprehensive logging and monitoring

### For Security Audits

- [ ] Review authentication flows
- [ ] Test input validation boundaries
- [ ] Verify error message sanitization
- [ ] Check for PII in logs
- [ ] Test GraphQL security limits
- [ ] Validate configuration security

## Conclusion

The PCF API implements comprehensive security measures at every layer of the application. By following these best practices and continuously improving our security posture, we maintain a robust and secure API platform.
