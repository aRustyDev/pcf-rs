# Environment Variables

Complete reference for all environment variables supported by the PCF API, organized by category with examples and best practices.

<!-- toc -->

## Overview

The PCF API uses environment variables for runtime configuration, particularly for sensitive values that should not be stored in configuration files. All environment variables follow the `PCF_API__` prefix convention with double underscores (`__`) for nested values.

## Naming Convention

```bash
PCF_API__{SECTION}__{SUBSECTION}__{KEY}

# Examples:
PCF_API__SERVER__PORT=8080
PCF_API__GRAPHQL__MAX_DEPTH=10
PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=secret
```

## Core Variables

### Application Settings

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__ENVIRONMENT` | String | `development` | Environment mode: development, staging, production |
| `PCF_API__APP__NAME` | String | `pcf-api` | Application name for logging and monitoring |
| `PCF_API__APP__VERSION` | String | `0.1.0` | Application version |

```bash
# Example usage
export PCF_API__ENVIRONMENT=production
export PCF_API__APP__NAME=pcf-api-prod
export PCF_API__APP__VERSION=1.0.0
```

### Server Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__SERVER__PORT` | Integer | `8080` | HTTP server port (1024-65535) |
| `PCF_API__SERVER__BIND` | String | `127.0.0.1` | IP address to bind |
| `PCF_API__SERVER__SHUTDOWN_TIMEOUT` | Integer | `30` | Graceful shutdown timeout (seconds) |
| `PCF_API__SERVER__REQUEST_TIMEOUT` | Integer | `30` | Request timeout (seconds) |
| `PCF_API__SERVER__BODY_LIMIT` | String | `10MB` | Maximum request body size |
| `PCF_API__SERVER__KEEP_ALIVE` | Integer | `75` | Keep-alive timeout (seconds) |

```bash
# Production server settings
export PCF_API__SERVER__PORT=8080
export PCF_API__SERVER__BIND=0.0.0.0
export PCF_API__SERVER__SHUTDOWN_TIMEOUT=60
export PCF_API__SERVER__BODY_LIMIT=20MB
```

## GraphQL Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__GRAPHQL__PATH` | String | `/graphql` | GraphQL endpoint path |
| `PCF_API__GRAPHQL__PLAYGROUND_ENABLED` | Boolean | `true` | Enable GraphQL playground |
| `PCF_API__GRAPHQL__PLAYGROUND_PATH` | String | `/playground` | Playground UI path |
| `PCF_API__GRAPHQL__INTROSPECTION_ENABLED` | Boolean | `true` | Enable schema introspection |
| `PCF_API__GRAPHQL__MAX_DEPTH` | Integer | `15` | Maximum query depth |
| `PCF_API__GRAPHQL__MAX_COMPLEXITY` | Integer | `1000` | Maximum query complexity |
| `PCF_API__GRAPHQL__MAX_UPLOAD_SIZE` | String | `5MB` | Maximum file upload size |
| `PCF_API__GRAPHQL__TRACING_ENABLED` | Boolean | `true` | Enable query tracing |

```bash
# Production GraphQL settings (secure)
export PCF_API__GRAPHQL__PLAYGROUND_ENABLED=false
export PCF_API__GRAPHQL__INTROSPECTION_ENABLED=false
export PCF_API__GRAPHQL__MAX_DEPTH=10
export PCF_API__GRAPHQL__MAX_COMPLEXITY=500
```

## Logging Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__LOGGING__LEVEL` | String | `info` | Log level: trace, debug, info, warn, error |
| `PCF_API__LOGGING__FORMAT` | String | `pretty` | Format: pretty, json, compact |
| `PCF_API__LOGGING__INCLUDE_LOCATION` | Boolean | `true` | Include file/line in logs |
| `PCF_API__LOGGING__INCLUDE_THREAD` | Boolean | `false` | Include thread info |
| `PCF_API__LOGGING__INCLUDE_TIMESTAMP` | Boolean | `true` | Include timestamps |
| `PCF_API__LOGGING__TIMESTAMP_FORMAT` | String | `rfc3339` | Timestamp format |

```bash
# Production logging
export PCF_API__LOGGING__LEVEL=info
export PCF_API__LOGGING__FORMAT=json
export PCF_API__LOGGING__INCLUDE_LOCATION=false

# Development logging
export PCF_API__LOGGING__LEVEL=debug
export PCF_API__LOGGING__FORMAT=pretty
export PCF_API__LOGGING__INCLUDE_LOCATION=true
```

## Service Variables

### Database Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__SERVICES__DATABASE__ENABLED` | Boolean | `true` | Enable database service |
| `PCF_API__SERVICES__DATABASE__CONNECTION_TIMEOUT` | Integer | `5000` | Connection timeout (ms) |
| `PCF_API__SERVICES__DATABASE__MAX_CONNECTIONS` | Integer | `100` | Max connection pool size |
| `PCF_API__SERVICES__DATABASE__MIN_CONNECTIONS` | Integer | `10` | Min connection pool size |
| `PCF_API__SERVICES__DATABASE__ACQUIRE_TIMEOUT` | Integer | `30000` | Connection acquire timeout (ms) |
| `PCF_API__SERVICES__DATABASE__IDLE_TIMEOUT` | Integer | `600000` | Idle connection timeout (ms) |
| `PCF_API__SERVICES__DATABASE__MAX_LIFETIME` | Integer | `1800000` | Max connection lifetime (ms) |

### SurrealDB Specific

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__SERVICES__DATABASE__SURREALDB__URL` | String | `memory` | Database URL or "memory" |
| `PCF_API__SERVICES__DATABASE__SURREALDB__NAMESPACE` | String | `development` | Database namespace |
| `PCF_API__SERVICES__DATABASE__SURREALDB__DATABASE` | String | `pcf` | Database name |
| `PCF_API__SERVICES__DATABASE__SURREALDB__USERNAME` | String | - | Database username |
| `PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD` | String | - | Database password |
| `PCF_API__SERVICES__DATABASE__SURREALDB__STRICT` | Boolean | `false` | Strict mode |

```bash
# Production database
export PCF_API__SERVICES__DATABASE__SURREALDB__URL=surrealdb://db.example.com:8000
export PCF_API__SERVICES__DATABASE__SURREALDB__NAMESPACE=production
export PCF_API__SERVICES__DATABASE__SURREALDB__USERNAME=pcf_api_user
export PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD="${DB_PASSWORD}"
export PCF_API__SERVICES__DATABASE__MAX_CONNECTIONS=200
```

### SpiceDB Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__SERVICES__SPICEDB__ENABLED` | Boolean | `false` | Enable SpiceDB service |
| `PCF_API__SERVICES__SPICEDB__ENDPOINT` | String | `localhost:50051` | SpiceDB endpoint |
| `PCF_API__SERVICES__SPICEDB__INSECURE` | Boolean | `true` | Allow insecure connections |
| `PCF_API__SERVICES__SPICEDB__TOKEN` | String | - | Pre-shared key |
| `PCF_API__SERVICES__SPICEDB__TIMEOUT` | Integer | `30` | Request timeout (seconds) |
| `PCF_API__SERVICES__SPICEDB__MAX_RETRIES` | Integer | `3` | Maximum retry attempts |

```bash
# Production SpiceDB
export PCF_API__SERVICES__SPICEDB__ENABLED=true
export PCF_API__SERVICES__SPICEDB__ENDPOINT=spicedb.internal:50051
export PCF_API__SERVICES__SPICEDB__INSECURE=false
export PCF_API__SERVICES__SPICEDB__TOKEN="${SPICEDB_TOKEN}"
```

### Cache Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__SERVICES__CACHE__ENABLED` | Boolean | `false` | Enable caching service |
| `PCF_API__SERVICES__CACHE__BACKEND` | String | `memory` | Backend: memory, redis |
| `PCF_API__SERVICES__CACHE__TTL` | Integer | `3600` | Default TTL (seconds) |
| `PCF_API__SERVICES__CACHE__MAX_SIZE` | Integer | `1000` | Maximum cache entries |
| `PCF_API__SERVICES__CACHE__REDIS__URL` | String | `redis://localhost:6379` | Redis URL |
| `PCF_API__SERVICES__CACHE__REDIS__POOL_SIZE` | Integer | `10` | Connection pool size |

```bash
# Production cache
export PCF_API__SERVICES__CACHE__ENABLED=true
export PCF_API__SERVICES__CACHE__BACKEND=redis
export PCF_API__SERVICES__CACHE__REDIS__URL=redis://cache.internal:6379
export PCF_API__SERVICES__CACHE__REDIS__POOL_SIZE=20
```

## Security Variables

### CORS Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__SECURITY__CORS_ENABLED` | Boolean | `true` | Enable CORS |
| `PCF_API__SECURITY__CORS_ORIGINS` | Array | `["*"]` | Allowed origins |
| `PCF_API__SECURITY__CORS_METHODS` | Array | `["GET", "POST"]` | Allowed methods |
| `PCF_API__SECURITY__CORS_HEADERS` | Array | `["*"]` | Allowed headers |
| `PCF_API__SECURITY__CORS_CREDENTIALS` | Boolean | `false` | Allow credentials |

```bash
# Production CORS
export PCF_API__SECURITY__CORS_ORIGINS='["https://app.example.com","https://admin.example.com"]'
export PCF_API__SECURITY__CORS_CREDENTIALS=true
```

### Rate Limiting

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__SECURITY__RATE_LIMIT_ENABLED` | Boolean | `false` | Enable rate limiting |
| `PCF_API__SECURITY__RATE_LIMIT_REQUESTS` | Integer | `100` | Requests per window |
| `PCF_API__SECURITY__RATE_LIMIT_WINDOW` | Integer | `60` | Window duration (seconds) |
| `PCF_API__SECURITY__RATE_LIMIT_BURST` | Integer | `10` | Burst capacity |

```bash
# Production rate limiting
export PCF_API__SECURITY__RATE_LIMIT_ENABLED=true
export PCF_API__SECURITY__RATE_LIMIT_REQUESTS=1000
export PCF_API__SECURITY__RATE_LIMIT_WINDOW=60
export PCF_API__SECURITY__RATE_LIMIT_BURST=50
```

## Authentication Variables

### General Auth Settings

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__AUTH__ENABLED` | Boolean | `false` | Enable authentication |
| `PCF_API__AUTH__PROVIDER` | String | `jwt` | Auth provider: jwt, oauth2 |
| `PCF_API__AUTH__SESSION_DURATION` | Integer | `3600` | Session duration (seconds) |
| `PCF_API__AUTH__REFRESH_ENABLED` | Boolean | `true` | Enable token refresh |
| `PCF_API__AUTH__REFRESH_DURATION` | Integer | `86400` | Refresh token duration (seconds) |

### JWT Settings

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__AUTH__JWT__ALGORITHM` | String | `HS256` | JWT algorithm |
| `PCF_API__AUTH__JWT__SECRET` | String | - | JWT secret key |
| `PCF_API__AUTH__JWT__ISSUER` | String | `pcf-api` | Token issuer |
| `PCF_API__AUTH__JWT__AUDIENCE` | String | `pcf-users` | Token audience |

```bash
# Production JWT auth
export PCF_API__AUTH__ENABLED=true
export PCF_API__AUTH__JWT__SECRET="${JWT_SECRET}"
export PCF_API__AUTH__SESSION_DURATION=7200
export PCF_API__AUTH__JWT__ISSUER=api.example.com
```

### OAuth2 Settings

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__AUTH__OAUTH2__CLIENT_ID` | String | - | OAuth2 client ID |
| `PCF_API__AUTH__OAUTH2__CLIENT_SECRET` | String | - | OAuth2 client secret |
| `PCF_API__AUTH__OAUTH2__AUTHORIZE_URL` | String | - | Authorization endpoint |
| `PCF_API__AUTH__OAUTH2__TOKEN_URL` | String | - | Token endpoint |
| `PCF_API__AUTH__OAUTH2__REDIRECT_URL` | String | - | Redirect URL |

```bash
# OAuth2 configuration
export PCF_API__AUTH__PROVIDER=oauth2
export PCF_API__AUTH__OAUTH2__CLIENT_ID="${OAUTH_CLIENT_ID}"
export PCF_API__AUTH__OAUTH2__CLIENT_SECRET="${OAUTH_CLIENT_SECRET}"
export PCF_API__AUTH__OAUTH2__AUTHORIZE_URL=https://auth.example.com/authorize
export PCF_API__AUTH__OAUTH2__TOKEN_URL=https://auth.example.com/token
export PCF_API__AUTH__OAUTH2__REDIRECT_URL=https://api.example.com/auth/callback
```

## Monitoring Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__MONITORING__METRICS_ENABLED` | Boolean | `true` | Enable metrics collection |
| `PCF_API__MONITORING__METRICS_PATH` | String | `/metrics` | Metrics endpoint path |
| `PCF_API__MONITORING__TRACING_ENABLED` | Boolean | `false` | Enable distributed tracing |
| `PCF_API__MONITORING__TRACE_SAMPLING` | Float | `0.1` | Trace sampling rate (0.0-1.0) |

### OpenTelemetry Settings

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__MONITORING__OTEL__ENDPOINT` | String | `localhost:4317` | OTLP endpoint |
| `PCF_API__MONITORING__OTEL__PROTOCOL` | String | `grpc` | Protocol: grpc, http |
| `PCF_API__MONITORING__OTEL__TIMEOUT` | Integer | `10` | Export timeout (seconds) |
| `PCF_API__MONITORING__OTEL__SERVICE_NAME` | String | `pcf-api` | Service name for traces |

```bash
# Production monitoring
export PCF_API__MONITORING__METRICS_ENABLED=true
export PCF_API__MONITORING__TRACING_ENABLED=true
export PCF_API__MONITORING__TRACE_SAMPLING=0.01
export PCF_API__MONITORING__OTEL__ENDPOINT=otel-collector.internal:4317
```

## Feature Flags

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__FEATURES__DEMO_MODE` | Boolean | `false` | Enable demo mode |
| `PCF_API__FEATURES__MAINTENANCE_MODE` | Boolean | `false` | Enable maintenance mode |
| `PCF_API__FEATURES__READ_ONLY` | Boolean | `false` | Enable read-only mode |
| `PCF_API__FEATURES__EXPERIMENTAL` | Boolean | `false` | Enable experimental features |

```bash
# Feature flags
export PCF_API__FEATURES__DEMO_MODE=false
export PCF_API__FEATURES__MAINTENANCE_MODE=false
export PCF_API__FEATURES__READ_ONLY=false
export PCF_API__FEATURES__EXPERIMENTAL=false
```

## Health Check Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PCF_API__HEALTH__LIVENESS_PATH` | String | `/health` | Liveness probe path |
| `PCF_API__HEALTH__READINESS_PATH` | String | `/health/ready` | Readiness probe path |
| `PCF_API__HEALTH__STARTUP_PATH` | String | `/health/startup` | Startup probe path |
| `PCF_API__HEALTH__INCLUDE_DETAILS` | Boolean | `true` | Include detailed status |
| `PCF_API__HEALTH__CACHE_DURATION` | Integer | `5` | Cache duration (seconds) |

## Best Practices

### 1. Secret Management

```bash
# Use secret management tools
export PCF_API__AUTH__JWT__SECRET=$(vault kv get -field=secret secret/pcf-api/jwt)
export PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=$(aws secretsmanager get-secret-value --secret-id pcf-api/db-password --query SecretString --output text)

# Or use .env files (not in production!)
source .env.local
```

### 2. Environment-Specific Files

```bash
# .env.development
PCF_API__ENVIRONMENT=development
PCF_API__GRAPHQL__PLAYGROUND_ENABLED=true
PCF_API__LOGGING__LEVEL=debug

# .env.production  
PCF_API__ENVIRONMENT=production
PCF_API__GRAPHQL__PLAYGROUND_ENABLED=false
PCF_API__LOGGING__LEVEL=info
```

### 3. Docker Compose

```yaml
version: '3.8'
services:
  api:
    image: pcf-api:latest
    environment:
      PCF_API__ENVIRONMENT: production
      PCF_API__SERVER__PORT: 8080
      PCF_API__GRAPHQL__PLAYGROUND_ENABLED: false
      PCF_API__AUTH__JWT__SECRET: ${JWT_SECRET}
      PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD: ${DB_PASSWORD}
```

### 4. Kubernetes Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: pcf-api-secrets
type: Opaque
stringData:
  PCF_API__AUTH__JWT__SECRET: "your-secret-key"
  PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD: "db-password"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pcf-api
spec:
  template:
    spec:
      containers:
      - name: api
        envFrom:
        - secretRef:
            name: pcf-api-secrets
```

### 5. Validation Script

```bash
#!/bin/bash
# validate-env.sh

required_vars=(
  "PCF_API__ENVIRONMENT"
  "PCF_API__AUTH__JWT__SECRET"
  "PCF_API__SERVICES__DATABASE__SURREALDB__URL"
  "PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD"
)

for var in "${required_vars[@]}"; do
  if [ -z "${!var}" ]; then
    echo "Error: Required variable $var is not set"
    exit 1
  fi
done

echo "All required environment variables are set"
```

## Debugging

### List All Variables

```bash
# Show all PCF_API variables
env | grep PCF_API__ | sort

# Show configuration hierarchy
PCF_API__LOGGING__LEVEL=trace ./pcf-api --print-config
```

### Override Order

```bash
# CLI args override everything
./pcf-api --server.port 9090

# Env vars override config files  
PCF_API__SERVER__PORT=9090 ./pcf-api

# Config files override defaults
# config/production.toml: server.port = 9090
```

### Common Errors

```bash
# Wrong separator (single underscore)
PCF_API_SERVER_PORT=8080  # ❌ Won't work

# Correct separator (double underscore)
PCF_API__SERVER__PORT=8080  # ✅ Works

# Array values need proper formatting
PCF_API__SECURITY__CORS_ORIGINS=https://example.com  # ❌ Not an array
PCF_API__SECURITY__CORS_ORIGINS='["https://example.com"]'  # ✅ Valid array
```

## Security Considerations

1. **Never log sensitive variables**
   ```bash
   # Bad: Logs password
   echo "Database: $PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD"
   
   # Good: Mask sensitive values
   echo "Database: ${PCF_API__SERVICES__DATABASE__SURREALDB__URL} (password hidden)"
   ```

2. **Use read-only variables in production**
   ```bash
   readonly PCF_API__AUTH__JWT__SECRET
   readonly PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD
   ```

3. **Rotate secrets regularly**
   ```bash
   # Implement secret rotation
   NEW_SECRET=$(generate-secret)
   export PCF_API__AUTH__JWT__SECRET="$NEW_SECRET"
   # Graceful restart to pick up new secret
   ```

4. **Audit environment access**
   ```bash
   # Log who accesses production environment
   echo "$(date): $USER accessed production environment" >> /var/log/pcf-api/env-access.log
   ```