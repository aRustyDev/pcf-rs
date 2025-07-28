# Docker Best Practices for Rust APIs - Junior Developer Guide

## What You'll Learn

This guide teaches you how to create optimized Docker images for Rust applications, focusing on multi-stage builds, minimal image sizes, and security best practices.

## Why This Matters

- **Size Impact**: A naive Rust Docker image can be 2GB+, but with best practices, you can achieve <50MB
- **Security**: Smaller images = smaller attack surface and fewer vulnerabilities
- **Performance**: Smaller images deploy faster and use less memory
- **Cost**: Smaller images reduce storage and bandwidth costs

## Core Concepts

### Multi-Stage Builds

Multi-stage builds let you use multiple FROM statements in your Dockerfile. Each stage can use a different base image and copy artifacts from previous stages.

```dockerfile
# Stage 1: Build environment (can be large)
FROM rust:1.75 AS builder
# ... build your app ...

# Stage 2: Runtime environment (minimal)
FROM scratch AS runtime
# ... copy only the binary ...
```

### Static Linking with MUSL

MUSL is an alternative to glibc that enables truly static binaries that can run on `scratch` images:

```bash
# Add MUSL target
rustup target add x86_64-unknown-linux-musl
```

## Step-by-Step Implementation

### 1. Basic Multi-Stage Dockerfile

**❌ BAD: Single-stage Dockerfile**
```dockerfile
FROM rust:1.75
WORKDIR /app
COPY . .
RUN cargo build --release
CMD ["./target/release/pcf-api"]
# Result: ~2GB image with build tools, source code, and dependencies
```

**✅ GOOD: Multi-stage Dockerfile**
```dockerfile
# Build stage
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/pcf-api /usr/local/bin/
CMD ["pcf-api"]
# Result: ~100MB image with just runtime dependencies
```

### 2. Dependency Caching with cargo-chef

cargo-chef pre-builds dependencies separately, dramatically speeding up rebuilds:

```dockerfile
# Stage 1: Chef preparation
FROM lukemathwalker/cargo-chef:latest-rust-1.75 AS chef
WORKDIR /app

# Stage 2: Plan the build (extracts dependency info)
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build dependencies (cached if unchanged)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Now copy source and build (only rebuilds if source changes)
COPY . .
RUN cargo build --release
```

### 3. Minimal Runtime with Scratch

For the absolute smallest image, use `scratch` with a statically linked binary:

```dockerfile
# Build stage with MUSL
FROM rust:1.75 AS builder
RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage - just the binary!
FROM scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/pcf-api /pcf-api
EXPOSE 8080
ENTRYPOINT ["/pcf-api"]
# Result: <50MB image with ZERO attack surface
```

### 4. Security Hardening

**Always run as non-root user:**
```dockerfile
# Create user in builder
FROM rust:1.75 AS builder
RUN useradd -u 1000 -U -s /bin/sh pcf

# In runtime stage
FROM scratch
COPY --from=builder /etc/passwd /etc/passwd
USER 1000
```

**Drop all capabilities:**
```dockerfile
# In Kubernetes deployment.yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  capabilities:
    drop:
    - ALL
```

### 5. Layer Optimization

Order your COPY statements from least to most frequently changed:

```dockerfile
# Dependencies change rarely - cache them
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# Source changes often - put last
COPY src ./src
RUN cargo build --release
```

### 6. Health Checks

Implement container health checks for orchestrators:

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/pcf-api", "healthcheck"]
```

## Complete Optimized Example

Here's the complete Phase 7 Dockerfile combining all best practices:

```dockerfile
# Stage 1: Dependency caching with cargo-chef
FROM lukemathwalker/cargo-chef:latest-rust-1.75 AS chef
WORKDIR /app

# Stage 2: Generate dependency recipe
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build dependencies and application
FROM chef AS builder
# Install MUSL for static linking
RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add x86_64-unknown-linux-musl

# Build dependencies (cached)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

# Create non-root user
RUN useradd -u 1000 -U -s /bin/sh pcf

# Stage 4: Minimal runtime
FROM scratch AS runtime

# Copy required files from builder
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/pcf-api /pcf-api

# Use non-root user
USER 1000

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/pcf-api", "healthcheck"]

# Run binary
ENTRYPOINT ["/pcf-api"]
```

## Docker Ignore Patterns

Always use `.dockerignore` to exclude unnecessary files:

```
# Build artifacts
target/
Dockerfile
.dockerignore

# Development files
.git/
.gitignore
*.md
.claude/
scripts/
tests/

# IDE files
.vscode/
.idea/
*.swp

# Local environment
.env
.env.*
!.env.example
```

## BuildKit Optimizations

Enable BuildKit for better caching and parallel builds:

```bash
# Enable BuildKit
export DOCKER_BUILDKIT=1

# Build with inline cache
docker build \
    --build-arg BUILDKIT_INLINE_CACHE=1 \
    --cache-from myapp:cache \
    --tag myapp:latest \
    --tag myapp:cache \
    .
```

## Common Pitfalls and Solutions

### 1. Binary Won't Start in Scratch
**Problem**: "No such file or directory" when running binary
**Solution**: Use MUSL for static linking or include required libraries

### 2. SSL/TLS Errors
**Problem**: HTTPS requests fail in scratch image
**Solution**: Copy CA certificates:
```dockerfile
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
```

### 3. Timezone Issues
**Problem**: Time-related functions fail
**Solution**: Copy timezone data:
```dockerfile
COPY --from=builder /usr/share/zoneinfo /usr/share/zoneinfo
ENV TZ=UTC
```

### 4. Slow Rebuilds
**Problem**: Every code change rebuilds all dependencies
**Solution**: Use cargo-chef or separate dependency copying

### 5. Large Image Size
**Problem**: Image exceeds size limits
**Solution**: Check for:
- Debug symbols (use `--release`)
- Unnecessary dependencies
- Build artifacts in final image
- Using correct target stage

## Testing Your Container

### 1. Size Verification
```bash
docker images myapp:latest --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"
```

### 2. Security Scanning
```bash
# Using Trivy
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
    aquasec/trivy image myapp:latest

# Using Grype
grype myapp:latest
```

### 3. Running Tests
```bash
# Test basic functionality
docker run --rm myapp:latest
docker run --rm myapp:latest healthcheck

# Test as non-root
docker run --rm myapp:latest whoami
# Should output: pcf (or your username)
```

## Quick Reference Commands

```bash
# Build with progress
DOCKER_BUILDKIT=1 docker build --progress=plain -t myapp .

# Analyze image layers
docker history myapp:latest --no-trunc

# Export image for analysis
docker save myapp:latest | tar -tv | head -20

# Multi-platform build
docker buildx build --platform linux/amd64,linux/arm64 -t myapp .
```

## Integration with CI/CD

Example GitHub Actions workflow:
```yaml
- name: Build and push Docker image
  uses: docker/build-push-action@v4
  with:
    context: .
    push: true
    tags: myapp:latest
    cache-from: type=gha
    cache-to: type=gha,mode=max
    platforms: linux/amd64,linux/arm64
```

## Next Steps

1. After creating your Dockerfile, verify it passes size requirements
2. Run security scans to ensure zero vulnerabilities
3. Test the health check functionality
4. Move on to Kubernetes deployment configuration

## Additional Resources

- [Docker's official Rust guide](https://docs.docker.com/language/rust/)
- [cargo-chef documentation](https://github.com/LukeMathWalker/cargo-chef)
- [Docker security best practices](https://docs.docker.com/develop/security-best-practices/)
- [BuildKit documentation](https://docs.docker.com/develop/develop-images/build_enhancements/)