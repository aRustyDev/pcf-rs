# Threat Model

This document outlines the threat model for the PCF API, identifying potential security threats and the mitigations implemented to address them.

## System Overview

The PCF API is a GraphQL-based service that:
- Accepts HTTP/HTTPS requests
- Processes GraphQL queries, mutations, and subscriptions
- Manages user sessions and authentication
- Interacts with backend databases
- Provides real-time updates via WebSockets

## Assets to Protect

### Primary Assets

1. **User Data**
   - Personal information (emails, user IDs)
   - User-generated content (notes, documents)
   - Session tokens and authentication credentials

2. **System Integrity**
   - API availability and performance
   - Configuration and secrets
   - Database connections

3. **Business Logic**
   - Authorization rules
   - Data validation logic
   - Query processing algorithms

### Secondary Assets

1. **Operational Data**
   - System logs
   - Performance metrics
   - Error messages

2. **Infrastructure**
   - Server resources
   - Network bandwidth
   - Database capacity

## Threat Actors

### External Attackers

**Motivation**: Data theft, service disruption, reputation damage

**Capabilities**: 
- Network access to API endpoints
- Ability to craft malicious requests
- Automated attack tools
- Social engineering

**Typical Attacks**:
- Injection attacks
- Brute force attempts
- DoS/DDoS attacks
- Data exfiltration

### Malicious Users

**Motivation**: Privilege escalation, unauthorized access

**Capabilities**:
- Valid user credentials
- Knowledge of API structure
- Time to probe for vulnerabilities

**Typical Attacks**:
- Authorization bypass attempts
- Resource exhaustion
- Data harvesting
- Account takeover

### Compromised Accounts

**Motivation**: Using legitimate access for malicious purposes

**Capabilities**:
- Full user privileges
- Legitimate-looking requests
- Access to user's data

**Typical Attacks**:
- Data theft
- Lateral movement
- Persistence establishment

## Threat Categories

### 1. Injection Attacks

#### GraphQL Injection

**Threat**: Malicious GraphQL queries designed to bypass security or extract data

**Attack Vectors**:
- Deeply nested queries
- Circular references
- Alias-based amplification
- Introspection abuse

**Mitigations Implemented**:
```rust
// Query depth limiting
pub struct DepthLimit {
    max_depth: usize,  // Production: 10
}

// Query complexity analysis
pub struct ComplexityLimit {
    max_complexity: usize,  // Production: 500
}

// Introspection disabled in production
introspection_enabled = false
```

**Residual Risk**: Low - Comprehensive query analysis in place

#### NoSQL Injection

**Threat**: Malicious input targeting the SurrealDB backend

**Attack Vectors**:
- User-provided IDs
- Search queries
- Filter parameters

**Mitigations Implemented**:
- Parameterized queries only
- Input validation via `garde`
- Type-safe query builders

**Residual Risk**: Low - No dynamic query construction

### 2. Authentication & Authorization

#### Authentication Bypass

**Threat**: Accessing protected resources without valid credentials

**Attack Vectors**:
- Missing auth checks
- Session hijacking
- Token replay

**Mitigations Implemented**:
```rust
// Explicit auth checks
pub fn require_auth(&self) -> Result<&Session> {
    self.session.as_ref()
        .ok_or_else(|| Error::new("Authentication required"))
}

// Per-request context isolation
let context = GraphQLContext::new(
    database,
    session,
    request_id,
);
```

**Residual Risk**: Medium - Full auth system not yet implemented

#### Privilege Escalation

**Threat**: Regular users gaining admin privileges

**Attack Vectors**:
- Session manipulation
- Authorization flaws
- Logic bugs

**Mitigations Implemented**:
```rust
// Explicit role checks
if !session.is_admin {
    return Err("Admin access required");
}

// Immutable session data
pub struct Session {
    pub user_id: String,
    pub is_admin: bool,
}
```

**Residual Risk**: Low - Clear role separation

### 3. Data Exposure

#### Information Leakage

**Threat**: Sensitive information exposed through errors or logs

**Attack Vectors**:
- Verbose error messages
- Stack traces
- Debug information
- Log files

**Mitigations Implemented**:
```rust
// Generic error messages
AppError::Internal(_) => "Internal error".to_string()

// Automatic log sanitization
pub fn sanitize_log_message(message: &str) -> String {
    // Redacts emails, IPs, tokens, etc.
}
```

**Residual Risk**: Low - Comprehensive sanitization

#### Unauthorized Data Access

**Threat**: Users accessing data they shouldn't see

**Attack Vectors**:
- Missing ownership checks
- GraphQL traversal
- Batch queries

**Mitigations Planned**:
- Row-level security
- Field-level permissions
- Query result filtering

**Residual Risk**: High - Pending implementation

### 4. Denial of Service

#### Resource Exhaustion

**Threat**: Consuming excessive server resources

**Attack Vectors**:
- Complex queries
- Large result sets
- Concurrent requests
- Memory exhaustion

**Mitigations Implemented**:
```rust
// Query complexity limits
if complexity > self.max_complexity {
    return Err("Query too complex");
}

// Graceful shutdown
pub shutdown_timeout: u64  // 30 seconds
```

**Mitigations Planned**:
- Rate limiting
- Request timeouts
- Memory limits
- Connection pooling

**Residual Risk**: High - Rate limiting not yet implemented

#### Algorithmic Complexity Attacks

**Threat**: Triggering O(n²) or worse algorithms

**Attack Vectors**:
- Nested queries
- Large pagination
- Complex filters

**Mitigations Implemented**:
- Query depth limits
- Complexity scoring
- Efficient algorithms

**Residual Risk**: Medium - Some operations need optimization

### 5. Session Management

#### Session Hijacking

**Threat**: Attacker using another user's session

**Attack Vectors**:
- Session token theft
- Man-in-the-middle
- XSS (if web UI added)

**Mitigations Planned**:
- Secure session tokens
- Token rotation
- IP binding
- Timeout policies

**Residual Risk**: High - Basic session management only

#### Session Fixation

**Threat**: Forcing a user to use a known session ID

**Attack Vectors**:
- Pre-set session IDs
- Session persistence

**Mitigations Planned**:
- New session on login
- Secure random IDs
- Session invalidation

**Residual Risk**: High - Not yet implemented

### 6. Infrastructure Attacks

#### Network Attacks

**Threat**: Interception or manipulation of network traffic

**Attack Vectors**:
- Man-in-the-middle
- Packet sniffing
- DNS hijacking

**Mitigations**:
- HTTPS enforcement (deployment)
- Certificate validation
- Secure headers

**Residual Risk**: Medium - Depends on deployment

#### Configuration Exposure

**Threat**: Sensitive configuration data exposed

**Attack Vectors**:
- Config file access
- Environment variables
- Error messages

**Mitigations Implemented**:
```toml
# No secrets in config files
[server]
port = 8080
# API keys come from environment
```

**Residual Risk**: Low - Proper secret management

## Risk Matrix

| Threat | Likelihood | Impact | Risk Level | Status |
|--------|------------|--------|------------|--------|
| GraphQL Injection | Medium | High | Medium | Mitigated |
| Auth Bypass | Low | Critical | Medium | Partial |
| Info Leakage | High | Medium | Medium | Mitigated |
| DoS Attacks | High | High | High | Partial |
| Session Hijacking | Medium | High | High | Planned |
| Config Exposure | Low | High | Medium | Mitigated |

## Attack Scenarios

### Scenario 1: Query Depth Attack

**Attack**: Attacker sends deeply nested query
```graphql
query Evil {
  notes {
    edges {
      node {
        author {
          notes {
            edges {
              node {
                author {
                  # ... nested 20 levels deep
                }
              }
            }
          }
        }
      }
    }
  }
}
```

**Detection**: Query depth checker
**Response**: Request rejected with error
**Mitigation**: Working as designed ✅

### Scenario 2: Complexity Bomb

**Attack**: Multiple expensive operations
```graphql
query Bomb {
  a1: notes(first: 1000) { ... }
  a2: notes(first: 1000) { ... }
  a3: notes(first: 1000) { ... }
  # ... 100 aliases
}
```

**Detection**: Complexity calculator
**Response**: Request rejected
**Mitigation**: Working as designed ✅

### Scenario 3: Log Injection

**Attack**: Attempting to corrupt logs
```
Email: admin@evil.com\n[INFO] User logged in as admin
Password: '; DROP TABLE users; --
```

**Detection**: Input validation
**Response**: Sanitized in logs
**Mitigation**: Working as designed ✅

### Scenario 4: Unauthorized Access

**Attack**: Accessing another user's data
```graphql
mutation {
  updateNote(id: "other-user-note", title: "Hacked") {
    success
  }
}
```

**Detection**: Authorization checks
**Response**: Access denied
**Mitigation**: Needs implementation ⚠️

## Security Controls Summary

### Implemented Controls

1. **Query Security**
   - ✅ Depth limiting (max: 10)
   - ✅ Complexity analysis (max: 500)
   - ✅ Introspection disabled in production

2. **Error Handling**
   - ✅ Generic error messages
   - ✅ No stack traces to clients
   - ✅ Safe error categories

3. **Logging Security**
   - ✅ PII redaction
   - ✅ Sanitized output
   - ✅ Request correlation

4. **Configuration**
   - ✅ No hardcoded secrets
   - ✅ Environment-based config
   - ✅ Validation on startup

5. **Development Security**
   - ✅ Demo mode protection
   - ✅ Compile-time checks
   - ✅ Security tests

### Planned Controls

1. **Authentication**
   - ⏳ JWT tokens
   - ⏳ OAuth2/OIDC
   - ⏳ MFA support

2. **Rate Limiting**
   - ⏳ Per-user limits
   - ⏳ IP-based throttling
   - ⏳ Adaptive limits

3. **Authorization**
   - ⏳ Row-level security
   - ⏳ Field permissions
   - ⏳ Policy engine

4. **Network Security**
   - ⏳ Strict CORS
   - ⏳ Security headers
   - ⏳ Certificate pinning

## Recommendations

### Immediate Actions

1. **Implement Rate Limiting**
   - High risk of DoS attacks
   - Use token bucket algorithm
   - Configure per-user limits

2. **Complete Auth System**
   - Session management incomplete
   - Add token-based auth
   - Implement timeout policies

3. **Add Authorization Layer**
   - Row-level security critical
   - Implement ownership checks
   - Add admin overrides

### Medium-Term Improvements

1. **Security Headers**
   - Add HSTS, CSP, etc.
   - Configure CORS properly
   - Implement request signing

2. **Audit Logging**
   - Track security events
   - Monitor failed auth
   - Alert on anomalies

3. **Dependency Scanning**
   - Regular updates
   - Vulnerability scanning
   - License compliance

### Long-Term Goals

1. **Zero Trust Architecture**
   - mTLS between services
   - Service mesh integration
   - Policy-based access

2. **Advanced Threat Detection**
   - Behavioral analysis
   - ML-based anomaly detection
   - Real-time response

3. **Compliance Readiness**
   - GDPR compliance
   - SOC2 preparation
   - Security certifications

## Conclusion

The PCF API has strong foundational security controls, particularly in query validation and log sanitization. However, critical gaps exist in authentication, authorization, and rate limiting that must be addressed before production deployment. Regular security reviews and updates to this threat model are essential as the system evolves.
