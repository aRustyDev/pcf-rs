# Error Message Examples

This document provides examples of clear vs unclear error messages, following the principle that errors should be helpful without leaking internal implementation details.

## Good Error Messages

These messages are clear, actionable, and safe:

### Configuration Errors
```
✅ GOOD: "Server port must be between 1024 and 65535. Found: 80"
❌ BAD:  "Invalid u16 value at config.server.port"

✅ GOOD: "Configuration file not found: config/production.toml. Create this file or set ENVIRONMENT to use a different config."
❌ BAD:  "std::io::Error: No such file or directory (os error 2)"

✅ GOOD: "Invalid IP address in server.bind. Expected format: '0.0.0.0' or '127.0.0.1'"
❌ BAD:  "AddrParseError"
```

### Database Errors
```
✅ GOOD: "Unable to connect to database. Please check if the database service is running."
❌ BAD:  "Connection refused: 192.168.1.100:5432"

✅ GOOD: "Database operation failed. The service is temporarily unavailable."
❌ BAD:  "SurrealDB error: Transaction deadlock detected on table 'notes' shard 3"
```

### Authentication Errors
```
✅ GOOD: "Invalid authentication token. Please log in again."
❌ BAD:  "JWT validation failed: exp claim is expired"

✅ GOOD: "You don't have permission to access this resource."
❌ BAD:  "SpiceDB check failed: user:john#reader@note:123#view = false"
```

### Input Validation Errors
```
✅ GOOD: "Title cannot be empty"
❌ BAD:  "Validation error at path $.input.title: required field"

✅ GOOD: "Email address is not valid. Expected format: user@example.com"
❌ BAD:  "Regex match failed: ^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
```

## Error Message Guidelines

1. **Be Specific About the Problem**
   - What failed
   - Why it failed (in user terms)
   - What value was invalid (if applicable)

2. **Provide Actionable Solutions**
   - What the user can do to fix it
   - What format/range is expected
   - Where to find more help

3. **Never Expose**
   - Internal IP addresses or hostnames
   - Database schema or table names
   - Internal service names
   - Stack traces or line numbers
   - Library-specific error types

4. **Include Context**
   - Which field or parameter was invalid
   - What operation was being performed
   - Link to documentation if complex

## Implementation Example

```rust
// Transform internal errors to safe external messages
match internal_error {
    DbError::ConnectionFailed(addr) => {
        // Log the details internally
        error!("Database connection failed to {}", addr);
        // Return safe message to user
        AppError::ServiceUnavailable("Unable to connect to database. Please try again later.".to_string())
    }
    DbError::QueryTimeout(query) => {
        // Log the query internally
        error!("Query timeout: {}", query);
        // Return safe message
        AppError::Timeout("The operation took too long. Please try with fewer items.".to_string())
    }
    _ => {
        // Log unexpected errors
        error!("Unexpected database error: {:?}", internal_error);
        // Return generic message
        AppError::Internal("An unexpected error occurred. Please try again.".to_string())
    }
}
```