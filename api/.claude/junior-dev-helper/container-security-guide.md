# Container Security Guide - Junior Developer Guide

## What You'll Learn

This guide teaches you how to secure your containerized applications through vulnerability scanning, hardening techniques, and security best practices.

## Why Container Security Matters

- **Attack Surface**: Containers share the host kernel - vulnerabilities can escape
- **Supply Chain**: Base images may contain vulnerabilities
- **Secrets**: Improper handling exposes sensitive data
- **Compliance**: Many regulations require security scanning
- **Trust**: Users need confidence in your application

## Security Layers

Container security involves multiple layers:
1. **Base Image Security** - Start with minimal, trusted images
2. **Build-Time Security** - Scan during build process
3. **Runtime Security** - Enforce security policies
4. **Secret Management** - Protect sensitive data
5. **Network Security** - Control communication

## Vulnerability Scanning

### Using Trivy (Recommended)

Trivy is a comprehensive scanner that finds vulnerabilities in:
- OS packages (Alpine, RHEL, CentOS, etc.)
- Language-specific packages (npm, pip, gem, etc.)
- Container images

**Basic Usage:**
```bash
# Scan an image
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
    aquasec/trivy image pcf-api:latest

# Scan with specific severity levels
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
    aquasec/trivy image --severity CRITICAL,HIGH pcf-api:latest

# Exit with error code if vulnerabilities found
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
    aquasec/trivy image --exit-code 1 --severity CRITICAL pcf-api:latest
```

**Scan Output Example:**
```
pcf-api:latest (alpine 3.18.4)
===============================
Total: 0 (CRITICAL: 0)

rust-app/Cargo.lock
===================
Total: 2 (CRITICAL: 1, HIGH: 1)

┌─────────────┬────────────────┬──────────┬───────────────────┬─────────────────┐
│   Library   │ Vulnerability  │ Severity │ Installed Version │ Fixed Version   │
├─────────────┼────────────────┼──────────┼───────────────────┼─────────────────┤
│ openssl     │ CVE-2023-12345 │ CRITICAL │ 1.1.1            │ 1.1.1q         │
│ tokio       │ CVE-2023-54321 │ HIGH     │ 1.28.0           │ 1.28.2         │
└─────────────┴────────────────┴──────────┴───────────────────┴─────────────────┘
```

### Using Grype

Alternative scanner with different vulnerability database:

```bash
# Install grype
curl -sSfL https://raw.githubusercontent.com/anchore/grype/main/install.sh | sh -s -- -b /usr/local/bin

# Scan image
grype pcf-api:latest

# Output in JSON for CI/CD
grype pcf-api:latest -o json
```

### Scanning in CI/CD

**GitHub Actions Example:**
```yaml
name: Security Scan
on: [push, pull_request]

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Build image
      run: docker build -t pcf-api:${{ github.sha }} .
    
    - name: Run Trivy vulnerability scanner
      uses: aquasecurity/trivy-action@master
      with:
        image-ref: pcf-api:${{ github.sha }}
        format: 'sarif'
        output: 'trivy-results.sarif'
        severity: 'CRITICAL,HIGH'
        exit-code: '1'
    
    - name: Upload Trivy scan results
      uses: github/codeql-action/upload-sarif@v2
      if: always()
      with:
        sarif_file: 'trivy-results.sarif'
```

## Container Hardening

### 1. Use Minimal Base Images

**Image Size Comparison:**
```
rust:latest        2.5GB   ❌ Includes build tools, package managers
debian:bookworm    124MB   ⚠️  Includes shell, package manager  
alpine:latest      7MB     ✅ Minimal Linux
distroless         20MB    ✅ No shell, no package manager
scratch            0MB     ✅ Nothing but your app
```

### 2. Run as Non-Root User

**In Dockerfile:**
```dockerfile
# Create user during build
RUN adduser -u 1000 -D -s /sbin/nologin appuser

# Switch to non-root user
USER 1000

# Or use numeric UID directly
USER 1000:1000
```

**Verify non-root:**
```bash
docker run --rm pcf-api:latest whoami
# Should output: appuser (not root)

docker run --rm pcf-api:latest id
# Should show: uid=1000(appuser) gid=1000(appuser)
```

### 3. Read-Only Root Filesystem

Make the container's root filesystem read-only:

```yaml
# In Kubernetes
securityContext:
  readOnlyRootFilesystem: true
```

**Handle writable directories:**
```dockerfile
# Create writable directories
RUN mkdir /tmp /app/cache && \
    chown -R 1000:1000 /tmp /app/cache

USER 1000
```

```yaml
# Mount temporary volumes in K8s
volumeMounts:
- name: tmp
  mountPath: /tmp
- name: cache
  mountPath: /app/cache
volumes:
- name: tmp
  emptyDir: {}
- name: cache
  emptyDir: {}
```

### 4. Drop Linux Capabilities

Remove all unnecessary Linux capabilities:

```yaml
# In Kubernetes
securityContext:
  capabilities:
    drop:
    - ALL
  # Only add back what's absolutely needed
  # capabilities:
  #   add:
  #   - NET_BIND_SERVICE  # Only if binding to port < 1024
```

### 5. Security Scanning Script

Create `scripts/scan-container.sh`:

```bash
#!/bin/bash
set -euo pipefail

IMAGE="${1:-pcf-api:latest}"

echo "🔍 Security Scan Report for $IMAGE"
echo "=================================="

# 1. Vulnerability Scan
echo -e "\n📋 Vulnerability Scan (Trivy):"
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
    aquasec/trivy image --severity CRITICAL,HIGH,MEDIUM \
    --exit-code 0 $IMAGE

# 2. Check for secrets
echo -e "\n🔐 Checking for exposed secrets:"
docker run --rm $IMAGE sh -c \
    'grep -r "PASSWORD\|SECRET\|KEY\|TOKEN" / 2>/dev/null || true' | \
    grep -v "Binary file" | head -20

if [ $? -eq 0 ]; then
    echo "⚠️  WARNING: Potential secrets found!"
else
    echo "✅ No obvious secrets detected"
fi

# 3. Check user
echo -e "\n👤 Container user check:"
USER=$(docker run --rm $IMAGE whoami 2>/dev/null || echo "unknown")
if [ "$USER" = "root" ]; then
    echo "❌ Container runs as root!"
    exit 1
else
    echo "✅ Container runs as non-root user: $USER"
fi

# 4. Check installed packages
echo -e "\n📦 Installed packages (security risk):"
docker run --rm $IMAGE sh -c \
    'which apt-get yum apk 2>/dev/null' || echo "✅ No package managers found"

# 5. Network tools check
echo -e "\n🔧 Network tools (attack vectors):"
TOOLS="curl wget nc netcat nmap"
for tool in $TOOLS; do
    docker run --rm $IMAGE sh -c "which $tool 2>/dev/null" && \
        echo "⚠️  Found: $tool" || true
done

# 6. File permissions
echo -e "\n📁 Checking file permissions:"
docker run --rm $IMAGE find / -perm -4000 2>/dev/null | head -10 || \
    echo "✅ No SUID files found"
```

## Secret Management Best Practices

### 1. Never Include Secrets in Images

**❌ BAD:**
```dockerfile
ENV DATABASE_PASSWORD=supersecret
COPY .env /app/.env
```

**✅ GOOD:**
```dockerfile
# Secrets injected at runtime
ENV DATABASE_PASSWORD=""
```

### 2. Use BuildKit Secrets for Build-Time

For secrets needed during build:

```dockerfile
# syntax=docker/dockerfile:1
FROM rust:1.75 AS builder

# Mount secret during build only
RUN --mount=type=secret,id=cargo_token \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    CARGO_NET_GIT_HTTPS_USER=oauth2 \
    CARGO_NET_GIT_HTTPS_PASSWORD=$(cat /run/secrets/cargo_token) \
    cargo build --release
```

Build with:
```bash
DOCKER_BUILDKIT=1 docker build \
    --secret id=cargo_token,src=$HOME/.cargo/credentials \
    -t pcf-api .
```

### 3. Runtime Secret Injection

**Environment Variables:**
```bash
docker run -e DATABASE_PASSWORD=$DB_PASS pcf-api
```

**Kubernetes Secrets:**
```yaml
envFrom:
- secretRef:
    name: pcf-api-secrets
```

### 4. Scanning for Secrets

Use tools to detect accidentally committed secrets:

```bash
# Using git-secrets
git secrets --install
git secrets --register-aws
git secrets --scan

# Using truffleHog
docker run --rm -v "$PWD:/repo" \
    trufflesecurity/trufflehog:latest \
    filesystem /repo
```

## Network Security

### 1. Minimize Attack Surface

Only expose necessary ports:

```dockerfile
# Only expose what's needed
EXPOSE 8080

# Don't expose debugging ports in production
# EXPOSE 9229  # Node.js debug
# EXPOSE 6060  # Go pprof
```

### 2. Network Policies in Kubernetes

Control traffic flow:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: pcf-api-network-policy
spec:
  podSelector:
    matchLabels:
      app: pcf-api
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: production
    - podSelector:
        matchLabels:
          app: frontend
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: database
    ports:
    - protocol: TCP
      port: 5432
  - to:  # Allow DNS
    ports:
    - protocol: UDP
      port: 53
```

## Security Policies

### Pod Security Standards

Apply security policies at namespace level:

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: production
  labels:
    pod-security.kubernetes.io/enforce: restricted
    pod-security.kubernetes.io/audit: restricted
    pod-security.kubernetes.io/warn: restricted
```

### Complete Security Context

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: security-context-demo
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
    runAsGroup: 3000
    fsGroup: 2000
    seccompProfile:
      type: RuntimeDefault
  containers:
  - name: pcf-api
    image: pcf-api:latest
    securityContext:
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop:
        - ALL
      privileged: false
      procMount: Default
```

## Common Security Issues

### 1. CVE in Base Image
**Solution**: Update base image or switch to more minimal base

### 2. Outdated Dependencies
**Solution**: Regular dependency updates
```bash
cargo update
cargo audit fix
```

### 3. Secrets in Logs
**Solution**: Implement log sanitization
```rust
// Sanitize before logging
let sanitized = url.replace(&password, "***");
info!("Connecting to: {}", sanitized);
```

### 4. Excessive Permissions
**Solution**: Apply principle of least privilege

### 5. Missing Security Headers
**Solution**: Add security middleware
```rust
use axum::middleware::security_headers;
app.layer(security_headers());
```

## Security Checklist

Before deploying:

- [ ] Base image is minimal and up-to-date
- [ ] No vulnerabilities from security scan
- [ ] Container runs as non-root
- [ ] Root filesystem is read-only
- [ ] All capabilities dropped
- [ ] No secrets in image
- [ ] No unnecessary tools installed
- [ ] Network policies configured
- [ ] Security context applied
- [ ] Logs don't contain sensitive data
- [ ] Image signed (if using registry)
- [ ] SBOM (Software Bill of Materials) generated

## Continuous Security

### 1. Regular Scanning
```yaml
# Scheduled scan job
apiVersion: batch/v1
kind: CronJob
metadata:
  name: security-scan
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: scanner
            image: aquasec/trivy:latest
            command:
            - sh
            - -c
            - trivy image pcf-api:latest
```

### 2. Dependency Updates
- Use Dependabot or Renovate
- Regular `cargo audit`
- Monitor security advisories

### 3. Image Signing
```bash
# Sign with cosign
cosign sign pcf-api:latest

# Verify signature
cosign verify pcf-api:latest
```

## Tools and Resources

### Security Scanners
- **Trivy**: Fast, comprehensive scanning
- **Grype**: Good false-positive handling  
- **Snyk**: Commercial with free tier
- **Clair**: For registry integration

### Runtime Security
- **Falco**: Runtime threat detection
- **Sysdig**: Commercial security platform
- **AppArmor/SELinux**: Mandatory access control

### Secret Scanners
- **git-secrets**: Pre-commit hook
- **TruffleHog**: Deep repository scanning
- **detect-secrets**: Python-based scanner

## Next Steps

1. Implement security scanning in CI/CD
2. Apply all hardening measures
3. Set up runtime monitoring
4. Document security procedures
5. Regular security audits

## Additional Resources

- [NIST Container Security Guide](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-190.pdf)
- [CIS Docker Benchmark](https://www.cisecurity.org/benchmark/docker)
- [OWASP Container Security](https://cheatsheetseries.owasp.org/cheatsheets/Docker_Security_Cheat_Sheet.html)
- [Kubernetes Security Best Practices](https://kubernetes.io/docs/concepts/security/)