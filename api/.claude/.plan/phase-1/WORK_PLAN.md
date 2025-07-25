# Phase 1: Foundation & Core Infrastructure - Work Plan

## Prerequisites

Before starting Phase 1, ensure you have:
- **Rust Knowledge**: Basic understanding of Rust syntax, ownership, and cargo commands
- **Web Framework Experience**: Familiarity with HTTP servers, REST APIs, and middleware concepts
- **Configuration Management**: Understanding of environment variables and configuration files
- **Testing Fundamentals**: Experience with unit testing and test-driven development
- **Command Line Tools**: Comfort with terminal, git, and basic Unix commands

## Quick Reference - Essential Resources

### Example Files
All example files are located in `/api/.claude/.spec/examples/`:
- **[Configuration Default](../../.spec/examples/config-default.toml)** - Base configuration template
- **[Configuration Development](../../.spec/examples/config-development.toml)** - Development environment overrides
- **[TDD Test Structure](../../.spec/examples/tdd-test-structure.rs)** - Comprehensive test examples following TDD
- **[Sanitization Patterns](../../.spec/examples/sanitization-patterns.rs)** - Complete log sanitization implementation
- **[Error Messages](../../.spec/examples/error-messages.md)** - Guidelines for clear error messages

### Specification Documents
Key specifications in `/api/.claude/.spec/`:
- **[configuration.md](../../.spec/configuration.md)** - Full configuration system specification
- **[logging.md](../../.spec/logging.md)** - Logging and sanitization requirements
- **[error-handling.md](../../.spec/error-handling.md)** - Error type definitions and guidelines
- **[health-checks.md](../../.spec/health-checks.md)** - Health endpoint specifications

### Junior Developer Resources
Additional help in `/api/.claude/junior-dev-helper/`:
- **[Configuration Tutorial](../../junior-dev-helper/configuration-tutorial.md)** - Step-by-step Figment guide with visuals
- **Common Errors Guide** (coming soon) - Troubleshooting common issues
- **Interactive Examples** (coming soon) - Hands-on code samples

### Quick Links
- **Verification Script**: `scripts/verify-phase-1.sh`
- **Development Server**: `scripts/dev-server.sh`
- **Test Coverage**: `scripts/test-coverage.sh`

## Overview
This work plan breaks down Phase 1 into concrete, implementable tasks with explicit review checkpoints. Each checkpoint is placed at a natural boundary where work can be easily reviewed and corrections made without significant rework.

## Build and Test Commands

This project uses `just` as the command runner. Available commands:
- `just test` - Run all tests
- `just build` - Build the release binary
- `just clean` - Clean up processes and build artifacts

Always use these commands instead of direct cargo commands to ensure consistency.

## IMPORTANT: Review Process

**This plan includes 5 mandatory review checkpoints where work MUST stop for external review.**

At each checkpoint:
1. **STOP all work** and commit your code
2. **Request external review** by providing:
   - This WORK_PLAN.md file
   - The REVIEW_PLAN.md file  
   - The checkpoint number
   - All code and artifacts created
3. **Wait for approval** before continuing to next section

## Development Methodology: Test-Driven Development (TDD)

**IMPORTANT**: Follow TDD practices throughout Phase 1 (see `.claude/.spec/examples/tdd-test-structure.rs` for examples):
1. **Write tests FIRST** - Before any implementation
2. **Run tests to see them FAIL** - Confirms test is valid
3. **Write minimal code to make tests PASS** - No more than necessary
4. **REFACTOR** - Clean up while keeping tests green
5. **Document as you go** - Add rustdoc comments and inline explanations

The TDD example file includes:
- Proper test structure with Arrange-Act-Assert pattern
- Examples for configuration, error handling, and health checks
- Integration test patterns
- Test helper utilities

## Done Criteria Checklist
- [ ] Server starts successfully with Axum
- [ ] Configuration loads from all 4 tiers (defaults → files → env vars → CLI)
- [ ] Health check endpoints respond correctly
- [ ] Graceful shutdown implemented
- [ ] Structured logging with tracing operational
- [ ] All code has corresponding tests written first
- [ ] Documentation is complete and current
- [ ] No development artifacts remain

## Work Breakdown with Review Checkpoints

### 1.1 Project Setup & Module Structure (2-3 work units)

**Work Unit Context:**
- **Complexity**: Low - Mostly boilerplate and configuration
- **Scope**: ~10 files, ~200 lines of configuration
- **Key Components**: 
  - Cargo.toml with 15+ dependencies
  - 7 module directories
  - Basic mod.rs files (5-10 lines each)
- **No algorithms required** - Just project structure setup

#### Task 1.1.1: Initialize Project Structure
```bash
cargo init --name pcf-api
```
- Create initial Cargo.toml with required dependencies
- Set up workspace structure if needed
- Configure Rust edition 2024
- Add .gitignore and .dockerignore

#### Task 1.1.2: Add Core Dependencies
Update Cargo.toml with:
```toml
[dependencies]
# Core server
axum = { version = "0.8", features = ["macros"] }
tokio = { version = "1.0", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Configuration
figment = { version = "0.10", features = ["toml", "env"] }
garde = "0.22.0"
clap = { version = "4.0", features = ["derive", "env"] }
serde = { version = "1.0", features = ["derive"] }

# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

#### Task 1.1.3: Create Module Structure
```
src/
├── main.rs
├── config/
│   ├── mod.rs
│   ├── models.rs      # Configuration structs
│   └── validation.rs  # Garde validation rules
├── health/
│   ├── mod.rs
│   ├── handlers.rs    # HTTP handlers
│   └── state.rs       # Health state management
├── error/
│   ├── mod.rs
│   └── types.rs       # Error type definitions
└── server/
    ├── mod.rs
    └── shutdown.rs    # Graceful shutdown logic
```

---
## 🛑 CHECKPOINT 1: Project Foundation Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Created all module directories as specified
2. Added all dependencies to Cargo.toml
3. Verified `just build` succeeds
4. Verified `just test` runs (even with no tests)
5. Created .gitignore with appropriate entries
6. Committed all work with message: "Checkpoint 1: Project foundation complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 1
- Your git commit hash

**DO NOT PROCEED** until you receive explicit approval.

---

### 1.2 Error Types and Handling (1-2 work units)

**Work Unit Context:**
- **Complexity**: Medium - Requires understanding of error propagation
- **Scope**: ~300 lines across 2-3 files
- **Key Components**:
  - 6 error enum variants with thiserror derive
  - IntoResponse trait implementation (~50 lines)
  - Panic handler setup (~30 lines)
  - 10+ unit tests covering all error paths
- **Patterns**: Error conversion, HTTP status mapping

#### Task 1.2.1: Write Error Type Tests First
Create `src/error/mod.rs` with comprehensive test module following TDD practices:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    
    #[test]
    fn test_error_display_messages() {
        // Based on error-handling.md specification
        let err = AppError::Config("Port out of range".to_string());
        assert_eq!(err.to_string(), "Configuration error: Port out of range");
        
        let err = AppError::InvalidInput("Email required".to_string());
        assert_eq!(err.to_string(), "Invalid input: Email required");
    }
    
    #[test]
    fn test_error_to_response_conversion() {
        // Verify HTTP status codes match specification
        let err = AppError::InvalidInput("test".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        
        let err = AppError::ServiceUnavailable("DB down".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
    
    #[test]
    fn test_error_safe_messages() {
        // Ensure internal errors don't leak details
        let internal = anyhow::anyhow!("Connection to 192.168.1.100:5432 failed");
        let err = AppError::Internal(internal);
        let response = err.into_response();
        // Response body should NOT contain IP address
        // Should only show generic "Internal error" message
    }
}
```

#### Task 1.2.2: Implement Error Types to Pass Tests
Create `src/error/types.rs` following error message guidelines (see `.claude/.spec/examples/error-messages.md`):
```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Server error: {0}")]
    Server(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Internal error")]
    Internal(#[from] anyhow::Error),
}

// Implement IntoResponse for Axum
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code) = match &self {
            AppError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR"),
            AppError::InvalidInput(_) => (StatusCode::BAD_REQUEST, "INVALID_INPUT"),
            AppError::ServiceUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            AppError::Server(_) => (StatusCode::INTERNAL_SERVER_ERROR, "SERVER_ERROR"),
        };
        
        // Get safe message (never expose internal details)
        let message = match &self {
            AppError::Internal(_) => "An internal error occurred".to_string(),
            other => other.to_string(),
        };
        
        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "trace_id": get_trace_id(), // From request extension
            }
        }));
        
        (status, body).into_response()
    }
}
```

#### Task 1.2.2: Implement Panic Handler
- Set up panic hook to log FATAL and exit cleanly
- Ensure no panics in production code paths
- Add compile-time check for demo mode:
```rust
#[cfg(all(not(debug_assertions), feature = "demo"))]
compile_error!("Demo mode MUST NOT be enabled in release builds");
```

### 1.3 Configuration System (2-3 work units)

**Work Unit Context:**
- **Complexity**: High - Multi-tier configuration merging logic
- **Scope**: ~500 lines across 3-4 files
- **Key Components**:
  - 4 configuration structs with 15+ fields total
  - Garde validation rules for each field
  - Figment provider setup (~100 lines)
  - 15+ tests covering hierarchy and validation
- **Algorithms**: Configuration merging precedence, path resolution

**📚 Junior Developer Resource**: See the [Configuration Tutorial](../../junior-dev-helper/configuration-tutorial.md) for a step-by-step guide with visual diagrams and debugging tips.

#### Task 1.3.1: Write Configuration Tests First
Create `src/config/mod.rs` with comprehensive tests based on configuration.md:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use figment::{Figment, providers::{Env, Format, Toml}};
    use garde::Validate;
    
    #[test]
    fn test_valid_config_loads() {
        // Test from configuration.md examples
        let config_toml = r#"
            [server]
            port = 8080
            bind = "0.0.0.0"
            
            [logging]
            level = "info"
            format = "json"
        "#;
        
        let config: AppConfig = Figment::new()
            .merge(Toml::string(config_toml))
            .extract()
            .expect("Should parse valid config");
            
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.bind, "0.0.0.0");
    }
    
    #[test]
    fn test_invalid_port_rejected() {
        // Port validation from configuration.md
        let config_toml = r#"
            [server]
            port = 80  # Below 1024, should fail
            bind = "0.0.0.0"
        "#;
        
        let config: Result<AppConfig, _> = Figment::new()
            .merge(Toml::string(config_toml))
            .extract();
            
        let config = config.expect("Should parse");
        let validation = config.validate();
        assert!(validation.is_err());
        assert!(validation.unwrap_err().to_string().contains("port"));
    }
    
    #[test] 
    fn test_config_hierarchy() {
        // 4-tier precedence from configuration.md
        std::env::set_var("APP_SERVER__PORT", "3000");
        
        let default = r#"[server]
        port = 8080"#;
        
        let env_specific = r#"[server]
        port = 9090"#;
        
        let config: AppConfig = Figment::new()
            .merge(Toml::string(default))     // Tier 1: defaults
            .merge(Toml::string(env_specific)) // Tier 2: env file
            .merge(Env::prefixed("APP_"))      // Tier 3: env vars
            .extract()
            .expect("Should merge configs");
            
        // Environment variable (tier 3) should win
        assert_eq!(config.server.port, 3000);
        
        std::env::remove_var("APP_SERVER__PORT");
    }
}
```

#### Task 1.3.2: Define Configuration Models to Satisfy Tests
Create `src/config/models.rs` following configuration.md specification:
```rust
use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Validate, Default)]
pub struct AppConfig {
    #[validate(nested)]
    pub server: ServerConfig,
    
    #[validate(nested)]
    pub logging: LoggingConfig,
    
    #[validate(nested)]
    pub health: HealthConfig,
    
    pub environment: Environment,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct ServerConfig {
    #[garde(range(min = 1024, max = 65535))]
    pub port: u16,
    
    #[garde(length(min = 1), custom(validate_bind_address))]
    pub bind: String,
    
    #[garde(range(min = 1, max = 300))]
    pub shutdown_timeout: u64, // seconds
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct LoggingConfig {
    #[garde(length(min = 1))]
    pub level: String,  // trace, debug, info, warn, error
    
    #[garde(pattern(r"^(json|pretty)$"))]
    pub format: String,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct HealthConfig {
    #[garde(length(min = 1))]
    pub liveness_path: String,
    
    #[garde(length(min = 1))]
    pub readiness_path: String,
    
    #[garde(range(min = 1, max = 3600))]
    pub startup_timeout_seconds: u32,
}

// Custom validator example from configuration.md
fn validate_bind_address(value: &str, _: &()) -> garde::Result {
    value.parse::<std::net::IpAddr>()
        .map(|_| ())
        .map_err(|_| garde::Error::new("Invalid IP address"))
}
```

#### Task 1.3.3: Implement Figment Loader
Create configuration loading with 4-tier hierarchy (see examples in `.claude/.spec/examples/`):
```rust
// Configuration loading order from configuration.md
pub fn load_config() -> Result<AppConfig> {
    let cli = Cli::parse();
    
    let mut figment = Figment::new()
        // 1. Embedded defaults
        .merge(Serialized::defaults(AppConfig::default()))
        // 2. Default config file
        .merge(Toml::file("config/default.toml").nested())
        // 3. Environment-specific config
        .merge(Toml::file(format!("config/{}.toml", env_name)).nested())
        // 4. Environment variables with APP_ prefix
        .merge(Env::prefixed("APP_").split("__"))
        // 5. CLI arguments (highest priority)
        .merge(Serialized::defaults(&cli));
        
    let config: AppConfig = figment.extract()?;
    
    // Validate with Garde
    config.validate()?;
    
    Ok(config)
}
```

Example configuration files are provided in:
- `.claude/.spec/examples/config-default.toml` - Base configuration
- `.claude/.spec/examples/config-development.toml` - Development overrides

#### Task 1.3.3: Add Garde Validation
- Implement custom validators for paths, URLs
- Validate configuration before server start
- Provide clear error messages for validation failures

---
## 🛑 CHECKPOINT 2: Core Infrastructure Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Implemented all error types with proper Display and IntoResponse traits
2. Created configuration models with Garde validation
3. Implemented 4-tier configuration loading with Figment
4. Added panic handler and demo mode compile check
5. Written tests for:
   - Valid configuration loading
   - Configuration hierarchy precedence 
   - Error type conversions
   - Invalid configuration rejection
   - Validation error messages
6. All tests pass
7. Code is documented with rustdoc comments
8. Committed all work with message: "Checkpoint 2: Core infrastructure complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 2  
- Your git commit hash
- Test output showing all tests pass

**DO NOT PROCEED** until you receive explicit approval.

---

### 1.4 Logging and Tracing Setup (1-2 work units)

**Work Unit Context:**
- **Complexity**: Medium - Custom log formatting and sanitization
- **Scope**: ~400 lines across 3 files
- **Key Components**:
  - Tracing subscriber setup with environment-based formatting (~100 lines)
  - Log sanitizer with 10+ regex patterns (~150 lines)
  - Request ID middleware (~50 lines)
  - 20+ tests for sanitization patterns
- **Algorithms**: Regex-based pattern matching for sensitive data

#### Task 1.4.1: Initialize Tracing Subscriber
- JSON format for production
- Pretty format for development
- Configure log levels per module
- Add trace_id to all spans

#### Task 1.4.2: Implement Request Tracing
- Generate trace_id for each request
- Propagate through request lifecycle
- Include in all log entries
- Add to error responses

#### Task 1.4.3: Security Sanitization
Based on logging.md specification, implement these sanitization patterns:

**Core Regex Patterns to Implement:**
```rust
use regex::Regex;
use std::sync::OnceLock;

pub struct SanitizationPatterns {
    email: Regex,
    credit_card: Regex,
    api_key: Regex,
    bearer_token: Regex,
    password_field: Regex,
    ipv4_address: Regex,
    user_path: Regex,
}

static PATTERNS: OnceLock<SanitizationPatterns> = OnceLock::new();

pub fn get_patterns() -> &'static SanitizationPatterns {
    PATTERNS.get_or_init(|| SanitizationPatterns {
        // Email addresses - keep domain visible
        email: Regex::new(r"\b([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})\b").unwrap(),
        
        // Credit card numbers - any 13-19 digit sequence
        credit_card: Regex::new(r"\b\d{13,19}\b").unwrap(),
        
        // API keys - common patterns
        api_key: Regex::new(r"\b(sk_|pk_|api_|key_)[a-zA-Z0-9]{20,}\b").unwrap(),
        
        // Bearer tokens
        bearer_token: Regex::new(r"Bearer\s+[a-zA-Z0-9\-_\.]+").unwrap(),
        
        // Password fields in various formats
        password_field: Regex::new(r"(?i)(password|passwd|pwd)\s*[:=]\s*\S+").unwrap(),
        
        // IPv4 addresses - show subnet only
        ipv4_address: Regex::new(r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b").unwrap(),
        
        // User home directories
        user_path: Regex::new(r"/(?:home|Users)/([^/]+)").unwrap(),
    })
}
```

**Sanitization Function:**
```rust
pub fn sanitize_log_message(message: &str) -> String {
    let patterns = get_patterns();
    let mut result = message.to_string();
    
    // Replace emails with ***@domain
    result = patterns.email.replace_all(&result, "***@$2").to_string();
    
    // Replace credit cards
    result = patterns.credit_card.replace_all(&result, "[REDACTED]").to_string();
    
    // Replace API keys
    result = patterns.api_key.replace_all(&result, "[REDACTED]").to_string();
    
    // Replace bearer tokens
    result = patterns.bearer_token.replace_all(&result, "Bearer [REDACTED]").to_string();
    
    // Replace password fields
    result = patterns.password_field.replace_all(&result, "$1=[REDACTED]").to_string();
    
    // Replace IP addresses with subnet
    result = patterns.ipv4_address.replace_all(&result, "$1.$2.x.x").to_string();
    
    // Replace user paths
    result = patterns.user_path.replace_all(&result, "/[USER]").to_string();
    
    result
}
```

**Test Cases to Verify Sanitization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_all_sanitization_patterns() {
        let test_cases = vec![
            // Email
            ("User john.doe@example.com logged in", "User ***@example.com logged in"),
            
            // Credit card
            ("Payment with card 4111111111111111", "Payment with card [REDACTED]"),
            
            // API key
            ("Using key sk_test_1234567890abcdefghij", "Using key [REDACTED]"),
            
            // Bearer token
            ("Authorization: Bearer eyJhbGciOiJIUzI1NiIs", "Authorization: Bearer [REDACTED]"),
            
            // Password
            ("password=secret123", "password=[REDACTED]"),
            ("pwd: mysecret", "pwd=[REDACTED]"),
            
            // IP address
            ("Connected from 192.168.1.100", "Connected from 192.168.x.x"),
            
            // User path
            ("Reading /home/john/config", "Reading /[USER]/config"),
            ("File at /Users/jane/Documents", "File at /[USER]/Documents"),
        ];
        
        for (input, expected) in test_cases {
            assert_eq!(sanitize_log_message(input), expected);
        }
    }
}
```

**Sanitization Verification Checklist:**
- [ ] Emails show as ***@domain.com
- [ ] Passwords never appear in logs (password=[REDACTED])
- [ ] API keys show as [REDACTED] (sk_, pk_, api_, key_ prefixes)
- [ ] Credit cards show as [REDACTED] (13-19 digit sequences)
- [ ] IP addresses show subnet only (192.168.x.x)
- [ ] User paths anonymized (/home/john → /[USER])
- [ ] Bearer tokens redacted (Bearer [REDACTED])

For the complete example with additional patterns, see `.claude/.spec/examples/sanitization-patterns.rs`

---
## 🛑 CHECKPOINT 3: Logging Infrastructure Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Implemented tracing subscriber with JSON/pretty format switching
2. Created log sanitizer that removes all sensitive patterns
3. Added trace_id generation and propagation
4. Implemented async, non-blocking logging
5. Written comprehensive tests for:
   - Password sanitization
   - Email redaction 
   - Token/API key removal
   - Credit card masking
   - File path sanitization
6. Verified production logs are valid JSON
7. Verified development logs are human-readable
8. All logging code is documented
9. Committed all work with message: "Checkpoint 3: Logging infrastructure complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 3
- Your git commit hash
- Example log outputs from both environments
- Test output showing sanitization works

**DO NOT PROCEED** until you receive explicit approval.

---

### 1.5 Server Bootstrap (2 work units)

**Work Unit Context:**
- **Complexity**: Medium - Coordinating multiple systems
- **Scope**: ~300 lines in main.rs and server module
- **Key Components**:
  - Main function with proper initialization order (~100 lines)
  - Axum router setup with middleware (~50 lines)
  - Signal handler for graceful shutdown (~80 lines)
  - Integration tests for lifecycle (~100 lines)
- **Patterns**: Tokio async runtime, signal handling, resource cleanup

#### Task 1.5.1: Write Integration Tests First
Create `tests/server_integration.rs`:
```rust
#[tokio::test]
async fn test_server_starts_and_binds() {
    // Test server starts on configured port
}

#[tokio::test]
async fn test_graceful_shutdown() {
    // Test server shuts down cleanly within timeout
}

#[tokio::test]
async fn test_port_conflict_error() {
    // Test clear error when port in use
}
```

#### Task 1.5.2: Implement Main Function to Pass Tests
Create `src/main.rs`:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse CLI args
    // 2. Load and validate configuration
    // 3. Initialize logging
    // 4. Set up panic handler
    // 5. Start server with graceful shutdown
}
```

#### Task 1.5.2: Axum Server Setup
- Bind to configured address/port
- Add middleware (tracing, CORS)
- Mount health check routes
- Handle port binding errors clearly

#### Task 1.5.3: Graceful Shutdown
- Listen for SIGTERM/SIGINT
- Implement 30-second shutdown timeout
- Drain in-flight requests
- Clean shutdown of all resources

---
## 🛑 CHECKPOINT 4: Basic Server Review

**STOP HERE FOR EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Implemented complete main.rs with proper initialization order
2. Created Axum server with middleware stack
3. Implemented graceful shutdown with 30-second timeout
4. Added signal handlers for SIGTERM/SIGINT
5. Mounted basic health endpoint returning "OK"
6. Written integration tests for:
   - Server startup and binding
   - Health endpoint response
   - Graceful shutdown behavior
   - Port conflict handling
7. Verified server logs include trace_ids
8. Documented server lifecycle and configuration
9. Committed all work with message: "Checkpoint 4: Basic server complete"

**Request review by providing:**
- Link to this checkpoint in WORK_PLAN.md
- Link to REVIEW_PLAN.md section for Checkpoint 4
- Your git commit hash
- Recording/logs of server startup, health check, and shutdown
- Test output showing integration tests pass

**DO NOT PROCEED** until you receive explicit approval.

---

### 1.6 Health Check Implementation (2 work units)

**Work Unit Context:**
- **Complexity**: Low to Medium - State management and caching
- **Scope**: ~400 lines across health module
- **Key Components**:
  - Two HTTP handlers (liveness and readiness) (~100 lines)
  - Health state enum and transitions (~80 lines)
  - Cache implementation with TTL (~100 lines)
  - CLI subcommand for health checks (~50 lines)
  - 15+ tests for state transitions and caching
- **Patterns**: State machine, time-based caching, HTTP client for CLI

#### Task 1.6.1: Liveness Endpoint
Implement `/health`:
- Return 200 OK with "OK" body
- No external checks
- Complete within 1 second
- No authentication required

#### Task 1.6.2: Readiness Endpoint
Implement `/health/ready`:
- Return JSON with service statuses
- Track startup phase (first 30 seconds)
- Implement caching (5 second TTL)
- Support stale data (30 seconds)

#### Task 1.6.3: Health State Management
- Define service health states (healthy/degraded/unhealthy/starting)
- Track service dependencies
- Implement state transitions
- Add metrics for health status

#### Task 1.6.4: CLI Health Check Command
- Add `healthcheck` subcommand
- Query readiness endpoint
- Return appropriate exit codes
- Pretty print health status

### 1.7 Basic Observability (1 work unit)

**Work Unit Context:**
- **Complexity**: Low - Basic metrics collection
- **Scope**: ~200 lines for metrics endpoint
- **Key Components**:
  - Prometheus metrics handler (~50 lines)
  - 5-10 basic metric collectors (request count, duration, etc.)
  - Metrics registration (~50 lines)
  - Tests for metrics format validation
- **No complex algorithms** - Just metric collection and formatting

#### Task 1.7.1: Write Metrics Tests First
Create metrics tests:
```rust
#[test]
fn test_metrics_endpoint_format() {
    // Verify Prometheus format
}

#[test]
fn test_required_metrics_exist() {
    // Check all required metrics are present
}
```

#### Task 1.7.2: Implement Metrics Endpoint
- Add `/metrics` endpoint (no auth)
- Basic HTTP metrics (request count, duration)
- Process metrics (memory, CPU)
- Health check metrics

#### Task 1.7.3: Verify Structured Logging
- Ensure all operations log with trace_id
- Add operation timing logs
- Log server lifecycle events
- Implement log sampling for high-frequency ops
- Write tests to verify log output format

---
## 🛑 CHECKPOINT 5: Complete Phase 1 System Review

**STOP HERE FOR FINAL EXTERNAL REVIEW**

**Before requesting review, ensure you have:**
1. Implemented all health check endpoints (/health and /health/ready)
2. Added health state management and caching
3. Created CLI healthcheck subcommand
4. Implemented /metrics endpoint with basic metrics
5. Written comprehensive test suite achieving:
   - ≥80% overall coverage
   - 100% coverage on critical paths
6. Created all required scripts:
   - `scripts/verify-phase-1.sh`
   - `scripts/dev-server.sh`  
   - `scripts/test-coverage.sh`
7. Written complete documentation:
   - README.md with setup/run instructions
   - Example configuration files
   - API documentation
8. Cleaned up all code:
   - Removed all TODO/FIXME comments
   - Removed debug prints and test stubs
   - Ensured consistent formatting
9. Committed all work with message: "Checkpoint 5: Phase 1 complete"

**Request review by providing:**
```bash
#!/bin/bash
set -e

echo "=== Phase 1 Verification ==="

# 1. Compilation
echo "✓ Checking compilation..."
just build

# 2. Test Coverage
echo "✓ Running tests with coverage..."
cargo tarpaulin --out Html --output-dir target/coverage

# 3. Clean any running processes
echo "✓ Cleaning existing processes..."
just clean || true

# 4. Start server for endpoint tests
echo "✓ Starting server..."
cargo run &
SERVER_PID=$!
sleep 5

# 5. Test endpoints
echo "✓ Testing endpoints..."
curl -f http://localhost:8080/health
curl -f http://localhost:8080/health/ready
curl -f http://localhost:8080/metrics

# 6. Test CLI health check
echo "✓ Testing CLI healthcheck..."
cargo run -- healthcheck

# 7. Clean shutdown
echo "✓ Testing graceful shutdown..."
kill -TERM $SERVER_PID
wait $SERVER_PID

echo "=== All Phase 1 checks passed! ==="
```

2. **Test Coverage Report**:
   ```bash
   cargo tarpaulin --out Html --output-dir target/coverage
   # Provide link to coverage report or summary statistics
   ```

3. **Complete Source Code**:
   - Provide repository link or zip file with all Phase 1 code
   - Include all configuration examples

4. **Documentation**:
   - README.md with setup instructions
   - Example configuration files

**Review Checklist for Reviewer**:

### Code Quality
- [ ] No `.unwrap()` or `.expect()` in production code paths
- [ ] All public functions have documentation comments
- [ ] Error messages are helpful but don't leak internal details
- [ ] Code follows Rust idioms and conventions

### Configuration System
- [ ] 4-tier hierarchy works (defaults → files → env → CLI)
- [ ] Invalid configuration fails with clear errors
- [ ] Secrets are never logged at any level
- [ ] Example configs provided for all environments

### Health & Observability  
- [ ] `/health` returns "OK" with 200 status
- [ ] `/health/ready` returns proper JSON status
- [ ] `/metrics` returns valid Prometheus format
- [ ] Every log entry includes trace_id
- [ ] No sensitive data in logs or metrics

### Testing & Coverage
- [ ] Overall coverage ≥ 80%
- [ ] Critical paths have 100% coverage
- [ ] All tests pass consistently (run 3 times)
- [ ] No flaky tests
- [ ] Integration tests cover server lifecycle

### Operational Readiness
- [ ] Server starts and stops cleanly
- [ ] Handles signals properly (SIGTERM/SIGINT)
- [ ] Clear error messages for common issues
- [ ] CLI healthcheck command works
- [ ] All Phase 1 "Done Criteria" are met

**Final Approval Required**: The reviewer must explicitly approve before Phase 2 can begin.

**Implementation Agent**: Do NOT proceed to Phase 2 until review is complete and approved.

---

## Final Phase 1 Deliverables

Before marking Phase 1 complete, ensure these artifacts exist:

1. **Documentation**
   - [ ] README.md with setup and run instructions
   - [ ] Configuration example files (config/default.toml, config/development.toml)
   - [ ] API documentation for health endpoints

2. **Tests**
   - [ ] Unit tests for all modules
   - [ ] Integration tests for server lifecycle
   - [ ] E2E test script that validates all endpoints

3. **Scripts**
   - [ ] `scripts/verify-phase-1.sh` - Automated verification
   - [ ] `scripts/dev-server.sh` - Development server with hot reload
   - [ ] `scripts/test-coverage.sh` - Generate coverage reports

## Next Steps

Once all checkpoints pass:
1. Commit with message: "Complete Phase 1: Foundation & Core Infrastructure"
2. Tag as `v0.1.0-phase1`
3. Create PR for review if working in team
4. Document any deviations from original plan
5. Begin Phase 2 planning

## Important Notes

- **DO NOT PROCEED** past a checkpoint until all verification steps pass
- **CLEAN UP** any experimental code before moving to next section
- **DOCUMENT** any decisions that deviate from the plan
- **TEST CONTINUOUSLY** - run tests after every significant change

## Troubleshooting Guide

### Common Issues and Solutions

#### Compilation Errors

**Issue**: `error[E0433]: failed to resolve: use of undeclared crate`
**Solution**: Ensure all dependencies are added to Cargo.toml. Run `cargo build` to download dependencies.

**Issue**: `error[E0432]: unresolved import`
**Solution**: Check module structure matches the plan. Ensure mod.rs files export public items.

#### Test Failures

**Issue**: Tests fail with "address already in use"
**Solution**: Use `just clean` to kill any running processes, or use random ports in tests:
```rust
let listener = TcpListener::bind("127.0.0.1:0").await?;
let port = listener.local_addr()?.port();
```

**Issue**: Integration tests timeout
**Solution**: Increase timeout values or check for deadlocks in shutdown logic.

#### Configuration Issues

**Issue**: "Configuration error: invalid type: string, expected u16"
**Solution**: Check environment variable format. Numbers must be unquoted: `APP_SERVER__PORT=8080`

**Issue**: Validation fails but error message unclear
**Solution**: Use `dbg!(&config)` to inspect loaded values. Check Garde validation rules.

#### Logging Issues

**Issue**: Logs not appearing
**Solution**: 
1. Check `RUST_LOG` environment variable (e.g., `RUST_LOG=debug`)
2. Ensure tracing subscriber is initialized early in main()
3. Check if logs are being sanitized too aggressively

**Issue**: JSON logs are malformed
**Solution**: Ensure you're using tracing macros correctly. Avoid string interpolation in messages.

### Debugging Tips

1. **Use `just test` instead of `cargo test`** - Ensures consistent environment
2. **Enable debug logging**: `RUST_LOG=debug just test`
3. **Check git status frequently** - Ensure you're not missing files
4. **Read error messages carefully** - Rust errors often suggest fixes
5. **Use `cargo clippy`** - Catches common mistakes early

### Useful Resources

- [Axum Documentation](https://docs.rs/axum/latest/axum/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Figment Configuration Guide](https://docs.rs/figment/latest/figment/)
- [Garde Validation Examples](https://docs.rs/garde/latest/garde/)
- [Tracing Subscriber Setup](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/)