# Configuration Management Specification

## Core Requirements

### Configuration System
- MUST use Figment for configuration management with exactly 4-tier precedence:
  1. Hardcoded defaults (lowest priority)
  2. Configuration files (TOML format)
  3. Environment variables
  4. Command-line arguments (highest priority)
- MUST validate all configuration with Garde before use
- MUST NOT start the server with invalid configuration
- MUST provide clear error messages for configuration failures

### Configuration Loading

**File Loading Order:**
```rust
// MUST load configurations in this exact order:
1. Embedded defaults from AppConfig::default()
2. config/default.toml (if exists)
3. config/{environment}.toml where environment from $ENVIRONMENT or "production"
4. Environment variables with APP_ prefix
5. Command-line arguments
```

**Environment Detection:**
- MUST read `ENVIRONMENT` variable to determine config file
- MUST default to "production" if not set
- SHOULD support: development, staging, production
- MAY support custom environment names

### Path Handling

**Flexible Path Resolution:**
```toml
# All paths MUST support both relative and absolute paths
[database]
# Relative path (resolved from working directory)
path = "./data/surrealdb"
# OR absolute path
path = "/var/lib/pcf/surrealdb"

# Environment variable override
# APP_DATABASE__PATH=/custom/path
```

**Path Resolution Rules:**
- Relative paths MUST be resolved from the current working directory
- Absolute paths MUST be used as-is
- Paths MUST be validated for existence and permissions at startup
- Missing directories SHOULD be created if possible (with proper permissions)
- If directory creation fails, MUST log clear error and exit

### Port Configuration

**Port Binding Flexibility:**
```toml
[server]
# Default port
port = 8080
# MUST support PORT environment variable for container environments
# MUST support APP_SERVER__PORT for Figment consistency
# MUST support --port CLI argument

# Bind address
bind = "0.0.0.0"  # Default for containers
# Development may override to "127.0.0.1"
```

**Port Conflict Handling:**
- If port is already in use, MUST log clear error message
- MUST include the attempted port number in error
- SHOULD suggest checking for other processes
- MAY attempt next port if configured to do so (dev only)

### Service Endpoints

**External Service Configuration:**
```toml
[services.surrealdb]
endpoint = "ws://localhost:8000"
namespace = "pcf"
database = "api"
username = "${SURREALDB_USER}"  # MUST support env var interpolation
password = "${SURREALDB_PASS}"  # MUST NOT log passwords

[services.spicedb]
endpoint = "grpc://localhost:50051"
preshared_key = "${SPICEDB_KEY}"  # MUST redact in logs
timeout_seconds = 5
# SHOULD support endpoint discovery in Kubernetes:
# endpoint = "${SPICEDB_SERVICE_HOST}:${SPICEDB_SERVICE_PORT}"

[services.auth]
kratos_public = "http://localhost:4433"
kratos_admin = "http://localhost:4434"  
hydra_public = "http://localhost:4444"
hydra_admin = "http://localhost:4445"
```

### Configuration Validation

**Required Validations with Garde:**
```rust
use garde::Validate;

#[derive(Validate)]
struct ServerConfig {
    #[garde(range(min = 1024, max = 65535))]
    port: u16,
    
    #[garde(length(min = 1))]
    bind: String,
    
    #[garde(custom(validate_bind_address))]
    bind_parsed: std::net::IpAddr,
}

#[derive(Validate)]
struct DatabaseConfig {
    #[garde(length(min = 1), custom(validate_url))]
    endpoint: String,
    
    #[garde(length(min = 1))]
    namespace: String,
    
    #[garde(length(min = 1))]
    database: String,
}
```

**Validation Rules:**
- MUST validate all configuration before use
- MUST provide specific error messages for validation failures
- MUST NOT start with invalid configuration
- SHOULD validate early in startup sequence

### Environment-Specific Defaults

**Production Defaults (Secure):**
```toml
[server]
port = 8080
bind = "0.0.0.0"

[security]
tls_enabled = true
cors_allow_origins = ["https://app.example.com"]

[limits]
max_query_depth = 10
max_query_complexity = 500
rate_limit_rps = 100
```

**Development Defaults (Convenient):**
```toml
[server]
port = 8080
bind = "127.0.0.1"

[security]
tls_enabled = false
cors_allow_origins = ["*"]

[limits]
max_query_depth = 20
max_query_complexity = 2000
rate_limit_rps = 0  # Disabled
```

### Secret Management

**Secret Handling Rules:**
- MUST NEVER log secrets or passwords at any level
- MUST redact sensitive values in configuration dumps
- SHOULD support reading secrets from files:
  ```toml
  password = "${file:/var/run/secrets/db_password}"
  ```
- MAY integrate with secret management systems (Vault, K8s secrets)

### Configuration Reloading

**Hot Reload Support (Optional):**
- MAY support configuration reload without restart
- If supported, MUST validate new config before applying
- MUST NOT reload if validation fails
- SHOULD log configuration changes
- Critical settings (ports, database) MAY require restart

### CLI Arguments

**Required CLI Support:**
```rust
#[derive(Parser)]
struct Cli {
    /// Path to configuration file
    #[arg(long, env = "APP_CONFIG")]
    config: Option<PathBuf>,
    
    /// Server port
    #[arg(long, env = "PORT")]
    port: Option<u16>,
    
    /// Environment name
    #[arg(long, env = "ENVIRONMENT")]
    environment: Option<String>,
    
    /// Enable debug logging
    #[arg(long)]
    debug: bool,
    
    /// Run health check and exit
    #[arg(long)]
    healthcheck: bool,
}
```

### Error Messages

**Configuration Error Format:**
```
Error: Failed to load configuration

  × Invalid value for 'server.port'
  ├─▶ Value must be between 1024 and 65535
  ├─▶ Found: 80
  ├─▶ Source: config/production.toml line 3
  ╰─▶ help: Use a port number above 1024 or run with elevated privileges
```

**MUST include:**
- What failed
- Why it failed  
- Where it came from (file, env var, CLI)
- How to fix it

### Docker/Kubernetes Support

**Container-Friendly Defaults:**
- MUST respect standard PORT environment variable
- MUST bind to 0.0.0.0 by default in containers
- SHOULD auto-detect container environment
- MUST support K8s service discovery patterns
- SHOULD provide example ConfigMaps and Secrets

**Health Check Configuration:**
```toml
[health]
liveness_path = "/health"
readiness_path = "/health/ready"
startup_timeout_seconds = 300  # For slow initial DB setup
startup_check_interval_seconds = 5
```

## Configuration Schema Export

**Demo Mode Feature:**
- When demo feature is enabled, MUST expose `/config/schema` endpoint
- Returns JSON Schema of all configuration options
- Includes descriptions and validation rules
- MUST redact secret field schemas