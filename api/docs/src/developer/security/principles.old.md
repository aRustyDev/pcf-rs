# Security Principles

The PCF API is built on fundamental security principles that guide every design decision and implementation detail.

## Core Principles

### 1. Defense in Depth

Security is not a single feature but a layered approach:

```
┌─────────────────────────────────────┐
│         External Firewall           │
├─────────────────────────────────────┤
│          Load Balancer              │
├─────────────────────────────────────┤
│            TLS/HTTPS                │
├─────────────────────────────────────┤
│          CORS Policies              │
├─────────────────────────────────────┤
│       Rate Limiting (Future)        │
├─────────────────────────────────────┤
│    GraphQL Security Extensions      │
├─────────────────────────────────────┤
│     Authentication & Sessions       │
├─────────────────────────────────────┤
│      Authorization Checks           │
├─────────────────────────────────────┤
│       Input Validation              │
├─────────────────────────────────────┤
│    Sanitized Error Handling         │
├─────────────────────────────────────┤
│      Secure Logging                 │
├─────────────────────────────────────┤
│     Database Access Control         │
└─────────────────────────────────────┘
```

### 2. Principle of Least Privilege

- Users can only access their own resources
- Admin privileges are explicitly checked
- Database connections use minimal required permissions
- Configuration files contain no secrets
- Demo mode is compile-time disabled in production

### 3. Fail Secure

When something goes wrong, default to the secure state:

```rust
// Authentication defaults to denied
pub fn require_auth(&self) -> Result<&Session> {
    self.session.as_ref()
        .ok_or_else(|| {
            Error::new("Authentication required")
                .extend_with(|_, e| e.set("code", "UNAUTHENTICATED"))
        })
}

// Errors default to generic messages
match &self {
    AppError::Internal(_) => "Internal error".to_string(),
    _ => self.to_string(),
}
```

### 4. Complete Mediation

Every access to every resource is checked:

- GraphQL context validates auth on each request
- Query depth/complexity checked before execution
- Input validation on all user-provided data
- No bypasses or shortcuts in security checks

### 5. Economy of Mechanism

Keep security mechanisms simple and auditable:

- Clear session structure
- Straightforward error categories
- Simple validation rules
- Readable sanitization patterns
- Minimal security dependencies

### 6. Separation of Privilege

Multiple conditions required for sensitive operations:

- Authentication + Authorization
- Valid session + Resource ownership
- Proper environment + Valid configuration
- Request limits + Input validation

### 7. Least Common Mechanism

Minimize shared resources between users:

- Request-scoped contexts
- Isolated database connections
- No shared mutable state
- Independent session management

### 8. Psychological Acceptability

Security should not impede legitimate use:

- Clear error messages for validation failures
- Reasonable query depth/complexity limits
- Graceful degradation of features
- Developer-friendly demo mode
- Helpful authentication errors

## Implementation Guidelines

### Input Validation

**Principle**: Never trust user input

```rust
// Validate everything
#[garde(range(min = 1024, max = 65535))]
pub port: u16,

// Custom validators for complex rules
#[garde(custom(validate_bind_address))]
pub bind: String,
```

### Output Encoding

**Principle**: Prevent injection attacks

- GraphQL automatically escapes outputs
- JSON serialization prevents injection
- Log sanitization removes sensitive data

### Authentication & Authorization

**Principle**: Verify identity, then verify access

```rust
// Step 1: Verify identity
let session = context.require_auth()?;

// Step 2: Verify access
if note.author != session.user_id && !session.is_admin {
    return Err("Access denied");
}
```

### Error Handling

**Principle**: Fail safely and silently

```rust
// Good: Generic error for clients
AppError::Internal(_) => "Internal error"

// Bad: Exposing internal details
AppError::Internal(e) => format!("Database error: {}", e)
```

### Logging & Monitoring

**Principle**: Log everything, expose nothing

```rust
// Automatic sanitization
info_sanitized!("Login attempt from {} for user {}", ip, email);
// Logs: "Login attempt from 192.168.x.x for user ***@example.com"
```

### Cryptography

**Principle**: Don't roll your own

- Use standard libraries for hashing
- Leverage TLS for transport security
- Trust established cryptographic primitives
- Avoid custom encryption schemes

## Security Decision Framework

When making security decisions, ask:

1. **What am I protecting?**
   - User data
   - System integrity
   - Service availability

2. **What are the threats?**
   - Unauthorized access
   - Data leakage
   - Service disruption
   - Injection attacks

3. **What controls apply?**
   - Preventive (validation, authentication)
   - Detective (logging, monitoring)
   - Corrective (error handling, recovery)

4. **Is it simple enough?**
   - Can it be easily audited?
   - Will developers use it correctly?
   - Does it have minimal dependencies?

5. **Does it fail securely?**
   - What happens on error?
   - Is the default state secure?
   - Are there any bypasses?

## Security Anti-Patterns to Avoid

### 1. Security by Obscurity

❌ **Don't**: Hide endpoints or rely on secret URLs
✅ **Do**: Use proper authentication and authorization

### 2. Blacklist Validation

❌ **Don't**: Block known bad inputs
✅ **Do**: Allow only known good inputs

### 3. Verbose Error Messages

❌ **Don't**: `User admin@example.com not found in database`
✅ **Do**: `Invalid credentials`

### 4. Client-Side Security

❌ **Don't**: Trust client-side validation
✅ **Do**: Validate everything server-side

### 5. Hardcoded Secrets

❌ **Don't**: Put secrets in code or config files
✅ **Do**: Use environment variables or secret management

### 6. Excessive Privileges

❌ **Don't**: Run everything as admin
✅ **Do**: Use minimal required permissions

## Continuous Security

Security is not a feature but a continuous process:

1. **Regular Reviews**
   - Code reviews include security checks
   - Dependency updates for security patches
   - Configuration audits

2. **Testing**
   - Security-focused unit tests
   - Integration tests for auth flows
   - Penetration testing (planned)

3. **Monitoring**
   - Track authentication failures
   - Monitor query complexity
   - Alert on suspicious patterns

4. **Learning**
   - Stay updated on security best practices
   - Learn from security incidents
   - Share knowledge with the team

## Conclusion

These principles form the foundation of PCF API security. Every feature, every line of code, and every configuration decision should align with these principles. Security is not an afterthought but an integral part of our development process.
