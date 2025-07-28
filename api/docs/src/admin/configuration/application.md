# Configuration Files

Complete guide to configuration file formats, locations, and management for the PCF API.

<!-- toc -->

## Overview

The PCF API uses TOML configuration files organized in a hierarchical structure. Configuration files provide default values that can be overridden by environment variables or command-line arguments.

## File Locations

### Standard Directory Structure

```
pcf-api/
├── config/
│   ├── default.toml          # Base configuration for all environments
│   ├── development.toml      # Development-specific overrides
│   ├── staging.toml          # Staging-specific overrides
│   └── production.toml       # Production-specific overrides
├── config.toml               # Optional root config (overrides default.toml)
└── .env                      # Optional environment variables (development only)
```

### File Loading Order

1. **Embedded defaults** - Hardcoded in binary
2. **config/default.toml** - Base configuration
3. **config/{environment}.toml** - Environment-specific
4. **config.toml** - Root override (if exists)
5. **.env** - Environment variables (development only)

## File Format

### TOML Structure

Configuration files use TOML (Tom's Obvious, Minimal Language):

```toml
# Root-level settings
environment = "development"

# Sections use [brackets]
[server]
port = 8080
bind = "127.0.0.1"

# Nested sections use dots
[services.database]
enabled = true

# Or nested brackets
[services]
  [services.cache]
  enabled = false
```

### Data Types

```toml
# Strings
name = "pcf-api"
bind = '0.0.0.0'

# Numbers
port = 8080
timeout = 30.5

# Booleans
enabled = true
debug = false

# Arrays
cors_origins = ["https://example.com", "https://app.example.com"]
allowed_methods = ["GET", "POST", "PUT", "DELETE"]

# Inline tables
jwt = { algorithm = "HS256", issuer = "pcf-api" }

# Dates/Times
created_at = 2024-01-01T00:00:00Z
```

## Configuration Files

### default.toml

Base configuration with sensible defaults:

```toml
# config/default.toml
environment = "development"

[app]
name = "pcf-api"
version = "0.1.0"

[server]
port = 8080
bind = "127.0.0.1"
shutdown_timeout = 30
request_timeout = 30
body_limit = "10MB"

[graphql]
path = "/graphql"
playground_enabled = true
introspection_enabled = true
max_depth = 15
max_complexity = 1000

[logging]
level = "info"
format = "pretty"

[services.database]
enabled = true
max_connections = 100

[services.database.surrealdb]
url = "memory"
namespace = "development"
database = "pcf"
```

### development.toml

Development environment overrides:

```toml
# config/development.toml
environment = "development"

[server]
bind = "127.0.0.1"  # Localhost only

[graphql]
playground_enabled = true
introspection_enabled = true
max_depth = 20      # More permissive for development
max_complexity = 2000

[logging]
level = "debug"
format = "pretty"
include_location = true

[services.database.surrealdb]
url = "memory"      # In-memory database for development

[features]
demo_mode = true    # Enable demo features
```

### staging.toml

Staging environment configuration:

```toml
# config/staging.toml
environment = "staging"

[server]
bind = "0.0.0.0"    # Accept external connections

[graphql]
playground_enabled = true   # Still enabled for testing
introspection_enabled = true
max_depth = 12
max_complexity = 750

[logging]
level = "info"
format = "json"

[services.database.surrealdb]
url = "surrealdb://staging-db:8000"
namespace = "staging"

[security]
cors_origins = ["https://staging.example.com"]
rate_limit_enabled = true
rate_limit_requests = 500

[monitoring]
metrics_enabled = true
```

### production.toml

Production environment configuration:

```toml
# config/production.toml
environment = "production"

[server]
bind = "0.0.0.0"
shutdown_timeout = 60    # Longer for graceful shutdown

[graphql]
playground_enabled = false    # Security: disabled
introspection_enabled = false # Security: disabled
max_depth = 10               # Stricter limits
max_complexity = 500

[logging]
level = "warn"
format = "json"
include_location = false     # Performance optimization

[services.database]
max_connections = 200        # Higher for production load
min_connections = 20

[services.database.surrealdb]
namespace = "production"
strict = true               # Enable strict mode

[security]
cors_origins = ["https://app.example.com"]
cors_credentials = true
rate_limit_enabled = true
rate_limit_requests = 100
rate_limit_window = 60

[auth]
enabled = true
session_duration = 3600

[monitoring]
metrics_enabled = true
tracing_enabled = true
trace_sampling = 0.01       # Sample 1% of requests
```

## Advanced Configuration

### Multi-Environment Files

Support multiple deployment targets:

```toml
# config/production-us-east.toml
environment = "production"

[server]
bind = "10.0.1.0"

[services.database.surrealdb]
url = "surrealdb://us-east-db.internal:8000"

[monitoring.otel]
endpoint = "us-east-otel.internal:4317"
```

### Feature-Specific Configs

Organize by feature:

```toml
# config/features/caching.toml
[services.cache]
enabled = true
backend = "redis"
ttl = 3600

[services.cache.redis]
url = "redis://cache.internal:6379"
pool_size = 20
```

### Secret References

Reference external secrets:

```toml
# config/production.toml
[auth.jwt]
# Secret loaded from environment
secret = "${JWT_SECRET}"

[services.database.surrealdb]
# Credentials from environment
username = "${DB_USERNAME}"
password = "${DB_PASSWORD}"
```

## File Management

### Creating Configuration Files

```bash
# Initialize configuration structure
mkdir -p config
touch config/default.toml
touch config/development.toml
touch config/staging.toml
touch config/production.toml

# Set appropriate permissions
chmod 644 config/*.toml
chmod 755 config/
```

### Validating Configuration

```bash
# Validate TOML syntax
cat config/production.toml | toml-test

# Validate configuration schema
./pcf-api --validate-config --config config/production.toml

# Dry run with specific config
./pcf-api --dry-run --environment production
```

### Config Templates

Create templates for consistency:

```bash
#!/bin/bash
# create-config.sh

TEMPLATE="config/template.toml"
ENVIRONMENT=$1
OUTPUT="config/${ENVIRONMENT}.toml"

if [ -z "$ENVIRONMENT" ]; then
  echo "Usage: $0 <environment>"
  exit 1
fi

# Copy template and update environment
cp "$TEMPLATE" "$OUTPUT"
sed -i "s/environment = \".*\"/environment = \"$ENVIRONMENT\"/" "$OUTPUT"

echo "Created $OUTPUT"
```

## Security Best Practices

### 1. File Permissions

```bash
# Production config files should be readable only by app user
chown appuser:appgroup config/production.toml
chmod 640 config/production.toml

# Prevent accidental edits
chattr +i config/production.toml  # Make immutable
```

### 2. Sensitive Data

```toml
# ❌ NEVER store secrets in config files
[auth.jwt]
secret = "my-secret-key"  # BAD!

# ✅ Use environment variables
[auth.jwt]
secret = ""  # Set via PCF_API__AUTH__JWT__SECRET
```

### 3. Git Security

```gitignore
# .gitignore
config/production.toml
config/*-prod.toml
config/secrets/
*.key
*.pem
.env*
!.env.example
```

### 4. Config Encryption

```bash
# Encrypt sensitive configs
gpg --encrypt --recipient ops@example.com config/production.toml

# Decrypt when needed
gpg --decrypt config/production.toml.gpg > config/production.toml
```

## Configuration Examples

### Minimal Configuration

```toml
# config/minimal.toml
[server]
port = 8080

[services.database.surrealdb]
url = "surrealdb://localhost:8000"
```

### High Availability Configuration

```toml
# config/ha.toml
[server]
shutdown_timeout = 120  # Long timeout for draining

[services.database]
max_connections = 500
min_connections = 50
acquire_timeout = 5000  # Fail fast

[services.cache]
enabled = true
backend = "redis"

[health]
cache_duration = 1  # Frequent health checks
```

### Development with Docker

```toml
# config/docker-dev.toml
[server]
bind = "0.0.0.0"  # Accept from Docker network

[services.database.surrealdb]
url = "surrealdb://database:8000"  # Docker service name

[services.cache.redis]
url = "redis://cache:6379"  # Docker service name
```

## Troubleshooting

### Common Issues

1. **File Not Found**
   ```bash
   Error: Configuration file not found: config/production.toml
   
   # Fix: Ensure file exists
   ls -la config/
   ```

2. **Invalid TOML**
   ```bash
   Error: TOML parse error at line 10, column 5
   
   # Fix: Validate syntax
   toml-lint config/production.toml
   ```

3. **Permission Denied**
   ```bash
   Error: Permission denied reading config/production.toml
   
   # Fix: Check permissions
   ls -l config/production.toml
   chmod 644 config/production.toml
   ```

### Debug Configuration Loading

```bash
# Enable trace logging
PCF_API__LOGGING__LEVEL=trace ./pcf-api

# Output shows:
# - Files being loaded
# - Values being merged
# - Final configuration
```

### Configuration Precedence

```bash
# Test override precedence
echo "[server]
port = 9000" > config.toml

PCF_API__SERVER__PORT=9090 ./pcf-api --server.port 9095

# Result: port = 9095 (CLI wins)
```

## Migration Guide

### From JSON to TOML

```javascript
// OLD: config.json
{
  "server": {
    "port": 8080,
    "bind": "0.0.0.0"
  }
}
```

```toml
# NEW: config.toml
[server]
port = 8080
bind = "0.0.0.0"
```

### From YAML to TOML

```yaml
# OLD: config.yaml
server:
  port: 8080
  bind: 0.0.0.0
  
database:
  url: localhost:8000
  options:
    timeout: 30
```

```toml
# NEW: config.toml
[server]
port = 8080
bind = "0.0.0.0"

[database]
url = "localhost:8000"

[database.options]
timeout = 30
```

## Best Practices Summary

1. **Use environment-specific files** - Keep configurations organized
2. **Never commit secrets** - Use environment variables
3. **Validate before deployment** - Test configuration changes
4. **Document custom settings** - Add comments explaining non-obvious values
5. **Version control configs** - Track changes (except secrets)
6. **Use consistent structure** - Follow the same organization pattern
7. **Set secure permissions** - Protect production configs
8. **Regular audits** - Review configurations periodically