# Phase 7 Gap Remediation Plan

## Overview

This plan addresses the minor gaps identified in the Phase 7 WORK_PLAN.md and REVIEW_PLAN.md analysis. Each gap is prioritized and includes specific implementation steps.

## Gap Priority

1. **Critical**: Health check timeout inconsistency (blocks SPEC compliance)
2. **High**: Missing troubleshooting section (impacts developer success)
3. **Medium**: Rollback procedures, structured logging setup
4. **Low**: HashiCorp Vault guidance, multi-architecture builds, monitoring integration

## Remediation Actions

### 1. Health Check Timeout Alignment (Critical)

**Gap**: WORK_PLAN uses 3-second timeout, SPEC requires 5-second timeout

**Actions**:
1. Update all health check timeout references from 3s to 5s
2. Add comment explaining the requirement
3. Update both Docker and Kubernetes examples

**Changes to WORK_PLAN.md**:
```dockerfile
# Line 283 - Update Docker HEALTHCHECK
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["/pcf-api", "healthcheck"]
# Timeout must be 5s per SPEC.md requirement
```

**Changes to REVIEW_PLAN.md**:
```bash
# Line 420 - Update test script
HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1
```

### 2. Add Troubleshooting Section (High)

**Gap**: No common errors and solutions guide

**Action**: Add new section after checkpoint 5 in WORK_PLAN.md

**New Section**:
```markdown
## Common Issues and Solutions

### Docker Build Failures

#### Problem: "no matching manifest for linux/arm64"
**Cause**: Building on M1 Mac without multi-arch support
**Solution**:
```bash
# Option 1: Build for AMD64
docker buildx build --platform linux/amd64 -t pcf-api:latest .

# Option 2: Use buildx for multi-arch
docker buildx create --use
docker buildx build --platform linux/amd64,linux/arm64 -t pcf-api:latest .
```

#### Problem: "cargo chef cook" fails with SSL errors
**Cause**: Corporate proxy or SSL certificate issues
**Solution**:
```dockerfile
# Add certificates before cargo chef cook
RUN apt-get update && apt-get install -y ca-certificates
# Or mount corporate certs
COPY corporate-ca.crt /usr/local/share/ca-certificates/
RUN update-ca-certificates
```

### Kubernetes Deployment Issues

#### Problem: CrashLoopBackOff
**Common Causes**:
1. Missing environment variables
2. Can't connect to database
3. Insufficient memory

**Debugging Steps**:
```bash
# Check logs
kubectl logs <pod-name> --previous

# Check events
kubectl describe pod <pod-name>

# Test with debug overrides
kubectl run debug-pod --image=pcf-api:latest --command -- sleep 3600
kubectl exec -it debug-pod -- /bin/sh
```

### HPA Not Scaling

#### Problem: HPA shows <unknown> for metrics
**Solution**:
```bash
# Verify metrics-server is installed
kubectl get deployment metrics-server -n kube-system

# Check resource requests are set
kubectl get deployment pcf-api -o yaml | grep -A5 resources:

# Install metrics-server if missing
kubectl apply -f https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml
```
```

### 3. Add Rollback Procedures (Medium)

**Gap**: No guidance on reverting failed deployments

**Action**: Add rollback section to checkpoint 5

**New Content**:
```markdown
#### Task 7.5.5: Implement Rollback Strategy

Create `scripts/rollback-deployment.sh`:
```bash
#!/bin/bash
set -euo pipefail

DEPLOYMENT_NAME="${1:-pcf-api}"
NAMESPACE="${2:-default}"

echo "🔄 Rolling back deployment: $DEPLOYMENT_NAME"

# Check current rollout status
kubectl rollout status deployment/$DEPLOYMENT_NAME -n $NAMESPACE || true

# Show rollout history
echo "📜 Rollout history:"
kubectl rollout history deployment/$DEPLOYMENT_NAME -n $NAMESPACE

# Perform rollback
if [ "${3:-}" = "--to-revision" ]; then
    REVISION="${4:-1}"
    echo "⏮️  Rolling back to revision $REVISION..."
    kubectl rollout undo deployment/$DEPLOYMENT_NAME -n $NAMESPACE --to-revision=$REVISION
else
    echo "⏮️  Rolling back to previous version..."
    kubectl rollout undo deployment/$DEPLOYMENT_NAME -n $NAMESPACE
fi

# Monitor rollback
echo "👀 Monitoring rollback..."
kubectl rollout status deployment/$DEPLOYMENT_NAME -n $NAMESPACE

# Verify pods are running
echo "✅ Rollback complete. Current pods:"
kubectl get pods -n $NAMESPACE -l app=$DEPLOYMENT_NAME
```

**Testing Rollback**:
```bash
# Save current version
kubectl annotate deployment pcf-api kubernetes.io/change-cause="v1.0.0 - Initial deployment"

# Deploy bad version
kubectl set image deployment/pcf-api pcf-api=pcf-api:bad-version

# When it fails, rollback
./scripts/rollback-deployment.sh pcf-api
```
```

### 4. Add Structured Logging Setup (Medium)

**Gap**: SPEC requires structured JSON logs but not implemented

**Action**: Add logging configuration to Task 7.1.2

**Addition to Dockerfile section**:
```markdown
#### Configure Structured Logging

Add environment variables for JSON logging:
```dockerfile
# In the runtime stage
ENV RUST_LOG=info
ENV LOG_FORMAT=json
ENV LOG_TIMESTAMP_FORMAT=rfc3339
```

Update your Rust application to support JSON logging:
```rust
// In main.rs or config.rs
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() {
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());
    
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env());
    
    if log_format == "json" {
        subscriber.with(tracing_subscriber::fmt::layer().json()).init();
    } else {
        subscriber.with(tracing_subscriber::fmt::layer()).init();
    }
}
```
```

### 5. Add HashiCorp Vault Guidance (Low)

**Gap**: ROADMAP mentions optional Vault integration

**Action**: Add optional advanced section

**New Content**:
```markdown
### Optional: HashiCorp Vault Integration

**Note**: This is an advanced topic for future enhancement. Skip if using Kubernetes secrets.

For production environments requiring advanced secret management:

1. **Install Vault Agent Injector**:
```bash
helm repo add hashicorp https://helm.releases.hashicorp.com
helm install vault hashicorp/vault --set "injector.enabled=true"
```

2. **Annotate Deployment**:
```yaml
metadata:
  annotations:
    vault.hashicorp.com/agent-inject: "true"
    vault.hashicorp.com/role: "pcf-api"
    vault.hashicorp.com/agent-inject-secret-config: "secret/data/pcf-api/config"
```

3. **Access Secrets**:
Vault will inject secrets to `/vault/secrets/config`

For detailed implementation, see Phase 9 production features.
```

### 6. Add Multi-Architecture Build Support (Low)

**Gap**: No ARM64 support for M1 Macs

**Action**: Add buildx instructions to Docker section

**Addition**:
```markdown
#### Multi-Architecture Builds (Optional)

For M1 Mac support and cloud ARM instances:

```bash
# Setup buildx
docker buildx create --name multiarch --use

# Build for multiple architectures
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --tag pcf-api:latest \
  --push .
```

**Note**: Multi-arch builds take longer but support more deployment targets.
```

### 7. Add Monitoring Integration (Low)

**Gap**: No Prometheus/Grafana connection guidance

**Action**: Add monitoring section to checkpoint 5

**New Content**:
```markdown
#### Connect to Monitoring Stack

1. **Add Prometheus Annotations**:
```yaml
metadata:
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/path: "/metrics"
    prometheus.io/port: "8080"
```

2. **ServiceMonitor for Prometheus Operator**:
```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: pcf-api-metrics
spec:
  selector:
    matchLabels:
      app: pcf-api
  endpoints:
  - port: http
    path: /metrics
    interval: 30s
```

3. **Basic Grafana Dashboard**:
Import dashboard ID 12345 or create custom dashboard for:
- Request rate
- Error rate
- Response time (p50, p95, p99)
- Resource usage
```

## Implementation Schedule

### Immediate (Before Phase 7 Start)
1. Fix health check timeout (5 minutes)
2. Add troubleshooting section (30 minutes)

### During Phase 7 Implementation
3. Add rollback procedures with checkpoint 5 (included in work)
4. Add structured logging setup with checkpoint 1 (included in work)

### Post Phase 7 (Documentation Enhancement)
5. Add Vault guidance as appendix
6. Add multi-arch build notes
7. Add monitoring integration guide

## Validation

After implementing changes:
1. Re-run verification script
2. Confirm SPEC alignment
3. Test examples work correctly
4. Review with team for clarity

## File Update Summary

**Files to Update**:
- `WORK_PLAN.md`: 5 sections to add/modify
- `REVIEW_PLAN.md`: 2 timeout references to fix
- No changes needed to junior dev helper files

**Estimated Time**: 2 hours for critical/high priority items

## Success Criteria

- [ ] All health check timeouts match SPEC (5s)
- [ ] Troubleshooting section helps resolve 80% of common issues
- [ ] Rollback procedure tested and documented
- [ ] Structured logging configuration included
- [ ] Optional enhancements clearly marked
- [ ] No new mandatory requirements added
- [ ] Maintains balance between guidance and flexibility