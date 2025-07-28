# Administrator Quick Start

This guide helps system administrators quickly deploy and manage the PCF API server in production environments.

<!-- toc -->

## Prerequisites

Before deploying PCF API, ensure you have:

- **Container Runtime**: Docker 20.10+ or Kubernetes 1.24+
- **Hardware**: Minimum 2 CPU cores, 4GB RAM
- **Network**: Ports 8080 (API) and 9090 (metrics) available
- **Optional**: PostgreSQL 14+ for production database

## Quick Deployment with Docker

### Using Docker Compose (Recommended)

```bash
# Clone the repository
git clone {{ github_url }}
cd pcf-api

# Create environment file
cp .env.example .env

# Start all services (API + dependencies)
docker-compose up -d

# Verify deployment
docker-compose ps
docker-compose logs -f api

# Health check
curl http://localhost:8080/health
```

### Using Docker Directly

```bash
# Pull the latest image
docker pull ghcr.io/pcf/api:latest

# Run with basic configuration
docker run -d \
  -p 8080:8080 \
  -p 9090:9090 \
  -e PCF_API__SERVER__HOST=0.0.0.0 \
  -e PCF_API__SERVER__PORT=8080 \
  -e PCF_API__LOG__LEVEL=info \
  -e PCF_API__LOG__FORMAT=json \
  --name pcf-api \
  --restart unless-stopped \
  ghcr.io/pcf/api:latest

# Check logs
docker logs -f pcf-api
```

### Production Docker Deployment

```bash
# Create Docker network
docker network create pcf-network

# Run PostgreSQL
docker run -d \
  --name pcf-postgres \
  --network pcf-network \
  -e POSTGRES_DB=pcf_api \
  -e POSTGRES_USER=pcf_user \
  -e POSTGRES_PASSWORD=secure_password \
  -v postgres_data:/var/lib/postgresql/data \
  --restart unless-stopped \
  postgres:14-alpine

# Run Redis
docker run -d \
  --name pcf-redis \
  --network pcf-network \
  -v redis_data:/data \
  --restart unless-stopped \
  redis:7-alpine

# Run PCF API
docker run -d \
  --name pcf-api \
  --network pcf-network \
  -p 8080:8080 \
  -p 9090:9090 \
  -e PCF_API__SERVER__HOST=0.0.0.0 \
  -e PCF_API__DATABASE__URL=postgresql://pcf_user:secure_password@pcf-postgres/pcf_api \
  -e PCF_API__CACHE__URL=redis://pcf-redis:6379 \
  -e PCF_API__LOG__LEVEL=info \
  -e PCF_API__LOG__FORMAT=json \
  --restart unless-stopped \
  ghcr.io/pcf/api:latest
```

## Health Checks and Monitoring

### Health Check Endpoints

PCF API provides Kubernetes-compatible health checks:

```bash
# Liveness probe - Is the application running?
curl -f http://localhost:8080/health
# Response: {"status":"healthy","version":"1.0.0"}

# Readiness probe - Is the application ready to serve traffic?
curl -f http://localhost:8080/health/ready
# Response: {"status":"ready","checks":{"database":"healthy","cache":"healthy"}}

# Detailed health information
curl http://localhost:8080/health/detailed
```

### Prometheus Metrics

```bash
# Access metrics endpoint
curl http://localhost:9090/metrics

# Key metrics to monitor:
# - pcf_graphql_requests_total
# - pcf_graphql_request_duration_seconds
# - pcf_http_requests_total
# - pcf_http_request_duration_seconds
# - pcf_active_connections
# - pcf_database_connections_active
# - pcf_cache_hits_total
# - pcf_cache_misses_total
```

### Setting Up Monitoring Stack

```yaml
# docker-compose.monitoring.yml
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    ports:
      - "9091:9090"

  grafana:
    image: grafana/grafana:latest
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    ports:
      - "3000:3000"

  alertmanager:
    image: prom/alertmanager:latest
    volumes:
      - ./alertmanager.yml:/etc/alertmanager/alertmanager.yml
    ports:
      - "9093:9093"

volumes:
  prometheus_data:
  grafana_data:
```

## Environment Configuration

PCF API uses a hierarchical configuration system. Environment variables follow the pattern `PCF_API__<SECTION>__<KEY>`:

### Core Configuration

```bash
# Server Settings
PCF_API__SERVER__HOST=0.0.0.0           # Bind address
PCF_API__SERVER__PORT=8080             # API port
PCF_API__SERVER__METRICS_PORT=9090     # Metrics port
PCF_API__SERVER__SHUTDOWN_TIMEOUT=30s  # Graceful shutdown timeout

# Logging
PCF_API__LOG__LEVEL=info               # trace, debug, info, warn, error
PCF_API__LOG__FORMAT=json              # json or pretty
PCF_API__LOG__OUTPUT=stdout            # stdout, stderr, or file path

# Database (Production)
PCF_API__DATABASE__URL=postgresql://user:pass@host:5432/dbname
PCF_API__DATABASE__MAX_CONNECTIONS=100
PCF_API__DATABASE__MIN_CONNECTIONS=10
PCF_API__DATABASE__CONNECT_TIMEOUT=30s
PCF_API__DATABASE__IDLE_TIMEOUT=10m

# Cache
PCF_API__CACHE__URL=redis://host:6379/0
PCF_API__CACHE__TTL=3600               # Default TTL in seconds
PCF_API__CACHE__MAX_CONNECTIONS=50

# GraphQL
PCF_API__GRAPHQL__PLAYGROUND=false     # Disable in production
PCF_API__GRAPHQL__INTROSPECTION=false  # Disable in production
PCF_API__GRAPHQL__MAX_DEPTH=10         # Query depth limit
PCF_API__GRAPHQL__MAX_COMPLEXITY=1000  # Query complexity limit

# Security
PCF_API__SECURITY__CORS_ORIGINS=https://app.example.com
PCF_API__SECURITY__RATE_LIMIT_REQUESTS=100
PCF_API__SECURITY__RATE_LIMIT_WINDOW=60s
PCF_API__SECURITY__TLS_CERT=/path/to/cert.pem
PCF_API__SECURITY__TLS_KEY=/path/to/key.pem
```

### Configuration Files

You can also use configuration files:

```toml
# config/production.toml
[server]
host = "0.0.0.0"
port = 8080
metrics_port = 9090

[log]
level = "info"
format = "json"

[database]
url = "postgresql://user:pass@localhost/pcf_api"
max_connections = 100

[cache]
url = "redis://localhost:6379"
```

Load with: `PCF_API__CONFIG_FILE=/path/to/config.toml`

## Security Checklist

### Container Security

- [ ] Run as non-root user (UID 1000)
- [ ] Use read-only root filesystem where possible
- [ ] Drop all unnecessary capabilities
- [ ] Scan images for vulnerabilities regularly
- [ ] Use specific image tags, not `latest`

```bash
# Secure Docker run example
docker run -d \
  --name pcf-api \
  --user 1000:1000 \
  --read-only \
  --tmpfs /tmp \
  --cap-drop ALL \
  --cap-add NET_BIND_SERVICE \
  --security-opt no-new-privileges:true \
  -p 8080:8080 \
  ghcr.io/pcf/api:1.0.0
```

### Network Security

- [ ] Enable TLS/HTTPS in production
- [ ] Use network policies to restrict traffic
- [ ] Configure firewall rules
- [ ] Enable rate limiting
- [ ] Set up DDoS protection

### Application Security

- [ ] Disable GraphQL introspection in production
- [ ] Disable GraphQL playground in production
- [ ] Configure CORS appropriately
- [ ] Enable audit logging
- [ ] Rotate secrets regularly
- [ ] Use strong database passwords

## Kubernetes Deployment

### Basic Deployment

```yaml
# pcf-api-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pcf-api
  labels:
    app: pcf-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: pcf-api
  template:
    metadata:
      labels:
        app: pcf-api
    spec:
      containers:
      - name: api
        image: ghcr.io/pcf/api:1.0.0
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: PCF_API__SERVER__HOST
          value: "0.0.0.0"
        - name: PCF_API__DATABASE__URL
          valueFrom:
            secretKeyRef:
              name: pcf-api-secrets
              key: database-url
        livenessProbe:
          httpGet:
            path: /health
            port: http
          initialDelaySeconds: 10
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: http
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
---
apiVersion: v1
kind: Service
metadata:
  name: pcf-api
spec:
  selector:
    app: pcf-api
  ports:
  - name: http
    port: 80
    targetPort: 8080
  - name: metrics
    port: 9090
    targetPort: 9090
```

### Production Kubernetes Setup

See our complete [Kubernetes Deployment Guide](../admin/deployment/kubernetes.md) for:
- High availability configuration
- Auto-scaling setup
- Ingress configuration
- Secret management
- Network policies
- Pod security policies

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker logs pcf-api --tail 50

# Common issues:
# - Port already in use
# - Invalid configuration
# - Database connection failed

# Debug with shell
docker run -it --entrypoint /bin/sh ghcr.io/pcf/api:latest
```

### High Memory Usage

```bash
# Check memory stats
docker stats pcf-api

# Analyze memory profile
curl http://localhost:9090/debug/pprof/heap > heap.pprof

# Common causes:
# - Connection pool too large
# - Cache size unbounded
# - Memory leak (report bug)
```

### Performance Issues

```bash
# Check response times
curl -w "@curl-format.txt" http://localhost:8080/health

# Enable debug logging
docker run -e PCF_API__LOG__LEVEL=debug ...

# Common solutions:
# - Enable caching
# - Increase connection pool
# - Scale horizontally
```

### Connection Issues

```bash
# Test connectivity
nc -zv localhost 8080

# Check Docker networking
docker network ls
docker port pcf-api

# Verify firewall rules
sudo iptables -L -n | grep 8080
```

## Next Steps

Now that you have PCF API running:

1. **Secure your deployment**: Follow our [Security Hardening Guide](../admin/security/hardening.md)
2. **Set up monitoring**: Implement [Prometheus & Grafana](../admin/monitoring/prometheus.md)
3. **Configure for production**: Review [Configuration Reference](../admin/configuration/README.md)
4. **Plan for scale**: Read [Scaling Guide](../admin/deployment/scaling.md)
5. **Prepare for disasters**: Implement [Backup & Recovery](../admin/deployment/backup.md)

## Getting Help

- 📖 [Troubleshooting Guide](../admin/troubleshooting/common-issues.md)
- 🔍 [Performance Tuning](../admin/troubleshooting/performance.md)
- 💬 [GitHub Discussions]({{ github_url }}/discussions)
- 🐛 [Report Issues]({{ github_url }}/issues)
- 🔒 [Security Issues](../shared/security/reporting.md)

---

**Need enterprise support?** Contact us at support@pcf-api.org for SLA-backed assistance.