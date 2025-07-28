# Docker Deployment

Complete guide to containerizing and deploying the PCF API using Docker, including multi-stage builds, optimization techniques, and orchestration with Docker Compose.

<!-- toc -->

## Overview

Docker provides a consistent deployment environment for the PCF API across development, staging, and production. This guide covers building optimized containers, managing configurations, and deploying with Docker Compose.

## Dockerfile

### Multi-Stage Build

```dockerfile
# Build stage
FROM rust:1.75-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies (cached layer)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY . .

# Build application
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 -s /bin/bash pcf

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/pcf-api /usr/local/bin/pcf-api

# Copy configuration files
COPY --from=builder /usr/src/app/config /opt/pcf/config

# Set ownership
RUN chown -R pcf:pcf /opt/pcf

# Switch to non-root user
USER pcf

# Set working directory
WORKDIR /opt/pcf

# Expose ports
EXPOSE 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info \
    PCF_API__CONFIG_DIR=/opt/pcf/config \
    PCF_API__ENVIRONMENT=production

# Run the binary
CMD ["pcf-api"]
```

### Development Dockerfile

```dockerfile
# Dockerfile.dev
FROM rust:1.75

# Install development tools
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    git \
    vim \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Install cargo tools
RUN cargo install cargo-watch sqlx-cli

# Create app directory
WORKDIR /app

# Mount source code as volume
VOLUME ["/app"]

# Expose ports
EXPOSE 8080 9090

# Run with cargo watch
CMD ["cargo", "watch", "-x", "run"]
```

## Build Optimization

### 1. Layer Caching

```dockerfile
# Optimize dependency caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Source changes don't invalidate dependency cache
COPY src ./src
RUN cargo build --release
```

### 2. Binary Size Reduction

```toml
# Cargo.toml
[profile.release]
lto = true          # Link Time Optimization
codegen-units = 1   # Single codegen unit
strip = true        # Strip symbols
opt-level = "z"     # Optimize for size
```

### 3. Distroless Images

```dockerfile
# Use distroless for minimal attack surface
FROM gcr.io/distroless/cc-debian12

COPY --from=builder /usr/src/app/target/release/pcf-api /app/pcf-api

# Copy required libraries
COPY --from=builder /usr/lib/x86_64-linux-gnu/libpq.so.5 /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libssl.so.3 /usr/lib/x86_64-linux-gnu/

EXPOSE 8080
ENTRYPOINT ["/app/pcf-api"]
```

## Docker Compose

### Development Stack

```yaml
# docker-compose.yml
version: '3.8'

services:
  api:
    build:
      context: .
      dockerfile: Dockerfile.dev
    volumes:
      - .:/app
      - cargo-cache:/usr/local/cargo/registry
      - target-cache:/app/target
    ports:
      - "8080:8080"  # API port
      - "9090:9090"  # Metrics port
    environment:
      - DATABASE_URL=postgresql://pcf:password@postgres:5432/pcf_dev
      - REDIS_URL=redis://redis:6379
      - PCF_API__ENVIRONMENT=development
      - RUST_LOG=debug
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_started
    networks:
      - pcf-network

  postgres:
    image: postgres:16-alpine
    environment:
      - POSTGRES_USER=pcf
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=pcf_dev
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U pcf"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - pcf-network

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data
    ports:
      - "6379:6379"
    networks:
      - pcf-network

  # Monitoring stack
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    ports:
      - "9091:9090"
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    networks:
      - pcf-network

  grafana:
    image: grafana/grafana:latest
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - grafana-data:/var/lib/grafana
      - ./monitoring/grafana/provisioning:/etc/grafana/provisioning
    ports:
      - "3000:3000"
    depends_on:
      - prometheus
    networks:
      - pcf-network

  # Development tools
  adminer:
    image: adminer:latest
    ports:
      - "8081:8080"
    networks:
      - pcf-network

  mailhog:
    image: mailhog/mailhog:latest
    ports:
      - "1025:1025"  # SMTP
      - "8025:8025"  # Web UI
    networks:
      - pcf-network

volumes:
  postgres-data:
  redis-data:
  prometheus-data:
  grafana-data:
  cargo-cache:
  target-cache:

networks:
  pcf-network:
    driver: bridge
```

### Production Stack

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  api:
    image: pcf-api:latest
    deploy:
      replicas: 3
      update_config:
        parallelism: 1
        delay: 10s
        failure_action: rollback
      restart_policy:
        condition: any
        delay: 5s
        max_attempts: 3
        window: 120s
    environment:
      - PCF_API__ENVIRONMENT=production
      - DATABASE_URL_FILE=/run/secrets/db_url
      - JWT_SECRET_FILE=/run/secrets/jwt_secret
    secrets:
      - db_url
      - jwt_secret
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    networks:
      - pcf-network
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "3"

  nginx:
    image: nginx:alpine
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/ssl:/etc/nginx/ssl:ro
    ports:
      - "80:80"
      - "443:443"
    depends_on:
      - api
    networks:
      - pcf-network

secrets:
  db_url:
    external: true
  jwt_secret:
    external: true

networks:
  pcf-network:
    driver: overlay
    attachable: true
```

## Configuration Management

### Environment Variables

```bash
# .env file for docker-compose
PCF_API__SERVER__PORT=8080
PCF_API__SERVER__WORKERS=0
PCF_API__DATABASE__MAX_CONNECTIONS=100
PCF_API__REDIS__POOL_SIZE=20
PCF_API__GRAPHQL__MAX_DEPTH=10
PCF_API__GRAPHQL__MAX_COMPLEXITY=500
```

### Docker Secrets

```bash
# Create secrets
echo "postgresql://user:pass@host/db" | docker secret create db_url -
echo "super-secret-jwt-key" | docker secret create jwt_secret -

# Use in service
docker service create \
  --name pcf-api \
  --secret db_url \
  --secret jwt_secret \
  pcf-api:latest
```

### Config Files

```dockerfile
# Mount configuration
volumes:
  - ./config/production.toml:/opt/pcf/config/production.toml:ro
```

## Networking

### Service Communication

```yaml
# Internal service discovery
services:
  api:
    networks:
      - internal
    environment:
      - DATABASE_HOST=postgres
      - REDIS_HOST=redis

networks:
  internal:
    driver: bridge
    internal: true
```

### Load Balancing

```nginx
# nginx.conf
upstream pcf_api {
    least_conn;
    server api_1:8080 max_fails=3 fail_timeout=30s;
    server api_2:8080 max_fails=3 fail_timeout=30s;
    server api_3:8080 max_fails=3 fail_timeout=30s;
}

server {
    listen 80;
    location / {
        proxy_pass http://pcf_api;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Security Best Practices

### 1. Non-Root User

```dockerfile
# Create and use non-root user
RUN useradd -m -u 1001 -s /bin/bash pcf
USER pcf
```

### 2. Read-Only Root Filesystem

```yaml
# docker-compose.yml
services:
  api:
    read_only: true
    tmpfs:
      - /tmp
      - /var/run
```

### 3. Security Scanning

```bash
# Scan for vulnerabilities
docker scout cves pcf-api:latest

# Use Trivy
trivy image pcf-api:latest

# Snyk scanning
snyk container test pcf-api:latest
```

### 4. Minimal Base Images

```dockerfile
# Use Alpine or distroless
FROM alpine:3.19
# or
FROM gcr.io/distroless/static-debian12
```

## Resource Management

### CPU and Memory Limits

```yaml
# docker-compose.yml
services:
  api:
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 2G
        reservations:
          cpus: '1.0'
          memory: 1G
```

### Monitoring Resources

```bash
# Check resource usage
docker stats

# Detailed inspection
docker inspect pcf-api | jq '.[0].HostConfig.Memory'

# Container metrics
docker exec pcf-api cat /proc/1/status | grep -E '(VmSize|VmRSS)'
```

## Logging

### Log Configuration

```yaml
# Centralized logging
services:
  api:
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "3"
        labels: "service=pcf-api,environment=production"
        
  # Log aggregation
  loki:
    image: grafana/loki:latest
    ports:
      - "3100:3100"
    volumes:
      - ./loki-config.yaml:/etc/loki/local-config.yaml
```

### Log Drivers

```bash
# Syslog driver
docker run -d \
  --log-driver=syslog \
  --log-opt syslog-address=tcp://192.168.0.42:514 \
  --log-opt tag="pcf-api" \
  pcf-api:latest

# Fluentd driver
docker run -d \
  --log-driver=fluentd \
  --log-opt fluentd-address=localhost:24224 \
  --log-opt tag="docker.pcf-api" \
  pcf-api:latest
```

## Health Checks

### Docker Health Check

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1
```

### Compose Health Check

```yaml
healthcheck:
  test: ["CMD-SHELL", "curl -f http://localhost:8080/health || exit 1"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 40s
```

## Deployment Commands

### Building Images

```bash
# Build with specific tag
docker build -t pcf-api:v1.0.0 .

# Build with build args
docker build \
  --build-arg RUST_VERSION=1.75 \
  --build-arg APP_VERSION=1.0.0 \
  -t pcf-api:latest .

# Multi-platform build
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t pcf-api:latest \
  --push .
```

### Running Containers

```bash
# Basic run
docker run -d \
  --name pcf-api \
  -p 8080:8080 \
  -e PCF_API__ENVIRONMENT=production \
  pcf-api:latest

# With volume mounts
docker run -d \
  --name pcf-api \
  -v $(pwd)/config:/opt/pcf/config:ro \
  -v pcf-data:/opt/pcf/data \
  pcf-api:latest

# With resource limits
docker run -d \
  --name pcf-api \
  --memory="2g" \
  --cpus="2.0" \
  pcf-api:latest
```

### Docker Compose Operations

```bash
# Start services
docker-compose up -d

# Scale service
docker-compose up -d --scale api=3

# View logs
docker-compose logs -f api

# Execute commands
docker-compose exec api /bin/bash

# Update services
docker-compose pull
docker-compose up -d

# Clean up
docker-compose down -v
```

## Troubleshooting

### Common Issues

1. **Container exits immediately**
   ```bash
   # Check exit code
   docker ps -a
   
   # View logs
   docker logs pcf-api
   
   # Debug interactively
   docker run -it --entrypoint /bin/bash pcf-api:latest
   ```

2. **Cannot connect to database**
   ```bash
   # Check network
   docker network ls
   docker network inspect pcf-network
   
   # Test connectivity
   docker exec pcf-api ping postgres
   ```

3. **Permission denied errors**
   ```bash
   # Check file ownership
   docker exec pcf-api ls -la /opt/pcf
   
   # Fix permissions
   docker exec -u root pcf-api chown -R pcf:pcf /opt/pcf
   ```

### Debugging Tools

```bash
# Execute shell in running container
docker exec -it pcf-api /bin/bash

# Copy files from container
docker cp pcf-api:/opt/pcf/logs/app.log ./

# Inspect container
docker inspect pcf-api

# View container processes
docker top pcf-api

# Export container filesystem
docker export pcf-api > pcf-api.tar
```

## Best Practices Summary

1. **Use multi-stage builds** to minimize image size
2. **Run as non-root user** for security
3. **Implement health checks** for reliability
4. **Use specific image tags** (not `latest`) in production
5. **Limit resources** to prevent resource exhaustion
6. **Scan images** for vulnerabilities
7. **Use secrets** for sensitive data
8. **Enable logging** with appropriate drivers
9. **Document** all environment variables
10. **Test** container behavior in CI/CD
