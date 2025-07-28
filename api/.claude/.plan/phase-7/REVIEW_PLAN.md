# Phase 7 Review Plan - Guidelines for Reviewing Agents

## Overview

This document provides comprehensive guidance for agents conducting reviews at Phase 7 checkpoints. As a reviewing agent, you are responsible for ensuring the containerization and deployment implementation meets all specifications and is production-ready.

## MANDATORY Review Scope

**IMPORTANT**: You MUST limit your review to:
1. The current checkpoint being reviewed
2. Any previously completed checkpoints in this phase
3. Do NOT review or comment on future checkpoints

This ensures focused, relevant feedback without overwhelming the implementation agent.

## Your Responsibilities as Reviewer

1. **First, check for questions** in `api/.claude/.reviews/checkpoint-X-questions.md` and answer them in the same file
2. **Thoroughly examine all provided artifacts**
3. **Build and test the container image**
4. **Verify security scan results**
5. **Test Kubernetes deployment in a test namespace**
6. **Verify TDD practices were followed**
7. **Check code documentation and comments** are comprehensive
8. **Look for code cleanliness** - no leftover stubs, TODOs, or test artifacts
9. **Verify junior dev resources were used** and check if they were helpful
10. **Test secret management** - ensure no leaks
11. **Verify resource usage** meets specifications
12. **Write feedback** to `api/.claude/.reviews/checkpoint-X-feedback.md`
13. **Document your review process** in `api/.claude/.reviews/checkpoint-X-review-vY.md` (where Y is version number)
14. **Give clear approval or rejection**

## Core Review Principles

### Container Best Practices Verification
At every checkpoint, verify container best practices:
1. **Multi-stage build efficiency** - Minimal final image
2. **Security hardening** - Non-root user, no capabilities
3. **Layer optimization** - Proper caching strategy
4. **Health checks** - Functional in container context
5. **No secrets in image** - Scan for exposed credentials

### Kubernetes Standards
All Kubernetes resources must follow best practices:
1. **Resource limits and requests** - Properly defined
2. **Health probes** - Liveness and readiness configured
3. **Security context** - Proper restrictions applied
4. **Labels and selectors** - Consistent and meaningful
5. **Configuration separation** - Secrets vs ConfigMaps

### Test-Driven Development (TDD) Verification
Continue verifying TDD practices:
1. **Tests exist before implementation** - Container tests, deployment tests
2. **Tests are comprehensive** - Cover security, size, functionality
3. **Integration tests** - Full deployment testing
4. **Documentation tests** - Examples work as written

### Documentation Standards
All deployment artifacts must be well-documented:
1. **Inline comments** in Dockerfiles and YAML
2. **README files** for deployment procedures
3. **Troubleshooting guides** for common issues
4. **Security documentation** for secret management
5. **No outdated information** from iterations

## Junior Developer Resources

Direct the implementing agent to these guides when you find issues:
- **[Docker Best Practices](../../junior-dev-helper/docker-best-practices.md)** - For image optimization
- **[Kubernetes Deployment Guide](../../junior-dev-helper/kubernetes-deployment-guide.md)** - For K8s resources
- **[Container Security Guide](../../junior-dev-helper/container-security-guide.md)** - For vulnerability fixes
- **[Secret Management Tutorial](../../junior-dev-helper/secret-management-tutorial.md)** - For secret handling
- **[Container Debugging Guide](../../junior-dev-helper/container-debugging-guide.md)** - For troubleshooting

## Review Process

For each checkpoint review:

1. **Check for questions**: Read `api/.claude/.reviews/checkpoint-X-questions.md` and provide answers

2. **Receive from implementing agent**:
   - Link to Phase 7 REVIEW_PLAN.md
   - Link to Phase 7 WORK_PLAN.md
   - Specific checkpoint number
   - All artifacts listed for that checkpoint
   - Build and deployment scripts
   - Any documented exceptions in `phase-7-exceptions.md`

3. **Perform the review** using checkpoint-specific checklist

4. **Run practical tests**:
   - Build the container (if Docker available)
   - Run security scans (if tools available)
   - Deploy to test environment (or validate with --dry-run)
   - Verify functionality
   
   **If tools unavailable**: Document theoretical review based on code inspection in review notes

5. **Document your findings**:
   - Write feedback to `api/.claude/.reviews/checkpoint-X-feedback.md`
   - Save your review notes to `api/.claude/.reviews/checkpoint-X-review-vY.md`

6. **Provide clear decision**: APPROVED, APPROVED WITH CONDITIONS, or CHANGES REQUIRED

7. **Stop and wait** for the implementing agent to address feedback before continuing

## Checkpoint-Specific Review Guidelines

### 🛑 CHECKPOINT 1: Docker Foundation Review

**What You're Reviewing**: Multi-stage Dockerfile and basic container functionality (DO NOT review future checkpoints)

**Key Specifications to Verify**:
- Multi-stage build pattern used correctly
- Static binary compilation successful
- Health check integrated
- Container starts and responds to health checks
- Base image is appropriate (scratch/distroless)

**Required Tests**:
```bash
# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "Docker not available - perform code review only"
    echo "Check Dockerfile syntax and structure manually"
    exit 0
fi

# Build the container
docker build -t pcf-api:test . || {
    echo "Build failed - check Dockerfile syntax"
    exit 1
}

# Verify build success
docker images pcf-api:test

# Test container runs
docker run -d --name test-api -p 8080:8080 pcf-api:test

# Check health status
sleep 10
docker inspect test-api --format='{{.State.Health.Status}}' || echo "Health check not configured"

# Test health endpoint
curl http://localhost:8080/health || echo "Health endpoint not responding"

# Check for running as non-root
docker exec test-api whoami || echo "Cannot verify user"

# Cleanup
docker stop test-api && docker rm test-api || true
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 1 Review Results

### Multi-stage Build Analysis
- [ ] Dependency caching stage: [PRESENT/PARTIAL/MISSING]
- [ ] Build stage optimization: [GOOD/ADEQUATE/NEEDS WORK]
- [ ] Runtime stage minimal: [YES/PARTIAL/NO]
- [ ] Static binary verified: [YES/NO/UNABLE TO TEST]

### Container Functionality
- [ ] Container builds successfully: [YES/NO]
- [ ] Health check command works: [YES/NO]
- [ ] Runs as non-root user: [YES/NO]
- [ ] Proper signal handling: [YES/NO]

### TDD Verification
- [ ] Container tests written first: [YES/NO]
- [ ] Tests cover all requirements: [YES/NO]
- [ ] Tests are meaningful: [YES/NO]

### Issues Found
[List specific issues with file locations]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]

If APPROVED WITH CONDITIONS, specify:
- What conditions must be met
- Timeline for addressing conditions
- Whether work can continue
```

### 🛑 CHECKPOINT 2: Image Optimization Review

**What You're Reviewing**: Image size, security scanning, and build optimization

**Key Specifications to Verify**:
- Image size < 50MB (or documented reason for larger size)
- Zero CRITICAL and HIGH security vulnerabilities (MEDIUM require justification)
- Build caching implemented
- Layer optimization applied
- Security hardening complete

**Required Tests**:
```bash
# Check image size
docker images pcf-api:test --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"

# Run security scan
if command -v trivy &> /dev/null; then
    trivy image --severity CRITICAL,HIGH pcf-api:test
else
    docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
      aquasec/trivy image --severity CRITICAL,HIGH pcf-api:test || {
        echo "Unable to scan - check for known vulnerabilities manually"
    }
fi

# Analyze image layers
docker history pcf-api:test --no-trunc

# Check for secrets in image
docker run --rm pcf-api:test sh -c 'grep -r "SECRET\|PASSWORD\|KEY" / 2>/dev/null || true'

# Verify build cache usage
DOCKER_BUILDKIT=1 docker build -t pcf-api:test2 . \
  --progress=plain 2>&1 | grep -i "cache"
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 2 Review Results

### Image Size Analysis
- [ ] Current size: ___MB (target < 50MB)
- [ ] Largest layers identified: [LIST]
- [ ] Optimization opportunities: [LIST]

### Security Scan Results
- [ ] Critical vulnerabilities: ___ (must be 0)
- [ ] High vulnerabilities: ___ (must be 0)
- [ ] Medium vulnerabilities: ___ (document if > 0)
- [ ] Low vulnerabilities: ___ (informational)
- [ ] Exceptions documented: [YES/NO] (if any vulnerabilities)

### Build Optimization
- [ ] Dependency caching works: [YES/NO]
- [ ] Layer reuse on rebuild: [YES/NO]
- [ ] BuildKit features used: [YES/NO]

### Security Hardening
- [ ] No exposed secrets: [YES/NO]
- [ ] Non-root user verified: [YES/NO]
- [ ] Minimal attack surface: [YES/NO]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]

If APPROVED WITH CONDITIONS, specify:
- What conditions must be met
- Timeline for addressing conditions
- Whether work can continue
```

### 🛑 CHECKPOINT 3: Kubernetes Manifests Review

**What You're Reviewing**: Kubernetes deployment resources and configuration

**Key Specifications to Verify**:
- Deployment manifest complete and correct
- Resource limits and requests defined
- Health probes properly configured
- Service exposes correct ports
- ConfigMap contains only non-sensitive data

**Required Tests**:
```bash
# Check kubectl availability
if ! command -v kubectl &> /dev/null; then
    echo "kubectl not available - validate YAML syntax manually"
    echo "Use online YAML validators or review structure"
    exit 0
fi

# Validate YAML syntax (works without cluster)
kubectl apply --dry-run=client -f k8s/deployment.yaml || echo "Deployment YAML invalid"
kubectl apply --dry-run=client -f k8s/service.yaml || echo "Service YAML invalid"
kubectl apply --dry-run=client -f k8s/configmap.yaml || echo "ConfigMap YAML invalid"

# If cluster available, deploy to test namespace
if kubectl cluster-info &> /dev/null; then
    kubectl create namespace test-review || true
    kubectl apply -f k8s/configmap.yaml -n test-review
    kubectl apply -f k8s/deployment.yaml -n test-review
    kubectl apply -f k8s/service.yaml -n test-review
    
    # Wait for rollout
    kubectl rollout status deployment/pcf-api -n test-review --timeout=2m || echo "Rollout timeout"
    
    # Check pod status
    kubectl get pods -n test-review -l app=pcf-api
    
    # Cleanup
    kubectl delete namespace test-review || true
else
    echo "No cluster access - review based on YAML validation only"
fi
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 3 Review Results

### Deployment Configuration
- [ ] Replicas defined: ___ (recommended 3+)
- [ ] Update strategy: [RollingUpdate/Recreate]
- [ ] Pod disruption budget: [PRESENT/MISSING/NOT REQUIRED]

### Resource Management
- [ ] CPU requests: ___m
- [ ] CPU limits: ___m
- [ ] Memory requests: ___Mi
- [ ] Memory limits: ___Mi
- [ ] Limits appropriate: [YES/NO]

### Health Probes
- [ ] Liveness probe configured: [YES/NO]
- [ ] Readiness probe configured: [YES/NO]
- [ ] Probe paths correct: [YES/NO]
- [ ] Timeouts reasonable: [YES/NO]

### Security Context
- [ ] Non-root user: [YES/NO]
- [ ] Read-only root filesystem: [YES/NO]
- [ ] Capabilities dropped: [YES/NO]
- [ ] Privilege escalation denied: [YES/NO]

### Configuration
- [ ] ConfigMap valid: [YES/NO]
- [ ] No secrets in ConfigMap: [YES/NO]
- [ ] Environment variables correct: [YES/NO]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]

If APPROVED WITH CONDITIONS, specify:
- What conditions must be met
- Timeline for addressing conditions
- Whether work can continue
```

### 🛑 CHECKPOINT 4: Secret Management Review

**What You're Reviewing**: Secret handling, injection, and security

**Key Specifications to Verify**:
- Secrets properly templated
- Injection script works correctly
- No secrets in code or configs
- Application handles secrets safely
- Secret rotation supported

**Required Tests**:
```bash
# Check for secrets in ConfigMap
grep -i "password\|secret\|key\|token" k8s/configmap.yaml

# Verify secret template
cat k8s/secret-template.yaml | grep "REPLACE_"

# Test secret creation script (with test values)
if [ -f "./scripts/create-secrets.sh" ]; then
    cat > .env.test << EOF
DATABASE_USER=testuser
DATABASE_PASSWORD=testpass
DATABASE_HOST=testhost
DATABASE_NAME=testdb
JWT_SECRET=testsecret123
REFRESH_SECRET=testrefresh123
SPICEDB_PRESHARED_KEY=testspicedb123
METRICS_PASSWORD=testmetrics123
EOF

    # Test script execution
    ./scripts/create-secrets.sh .env.test || echo "Script failed - check error handling"
    
    # Verify secret if kubectl available
    if command -v kubectl &> /dev/null; then
        kubectl get secret pcf-api-secrets -o yaml | grep -v "data:" || echo "Secret not created"
        kubectl delete secret pcf-api-secrets || true
    fi
    
    rm -f .env.test
else
    echo "Secret creation script not found - verify manual process documented"
fi

# Check application doesn't log secrets (if Docker available)
if command -v docker &> /dev/null; then
    docker run --rm pcf-api:test 2>&1 | grep -i "password\|secret" && echo "WARNING: Possible secret leak" || echo "No secrets in logs"
fi
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 4 Review Results

### Secret Template
- [ ] All required secrets defined: [YES/NO]
- [ ] Template uses stringData: [YES/NO]
- [ ] Clear replacement markers: [YES/NO]

### Secret Injection
- [ ] Script validates inputs: [YES/NO]
- [ ] Script creates valid secret: [YES/NO]
- [ ] Script cleans up temp files: [YES/NO]

### Application Security
- [ ] Secrets not logged: [YES/NO]
- [ ] Debug output sanitized: [YES/NO]
- [ ] Config redaction works: [YES/NO]

### Secret Management
- [ ] Rotation documented: [YES/NO]
- [ ] No hardcoded secrets: [YES/NO]
- [ ] Environment handling correct: [YES/NO]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]

If APPROVED WITH CONDITIONS, specify:
- What conditions must be met
- Timeline for addressing conditions
- Whether work can continue
```

### 🛑 CHECKPOINT 5: Production Readiness Review

**What You're Reviewing**: Complete deployment automation and production features

**Key Specifications to Verify**:
- HPA configuration correct
- Deployment script automates process
- Integration tests comprehensive
- Documentation complete
- Production deployment successful

**Required Tests**:
```bash
# Test full deployment script
./scripts/deploy-k8s.sh test-v1 development

# Verify HPA configuration
kubectl get hpa pcf-api -o yaml

# Check HPA metrics
kubectl get hpa pcf-api --watch
# Generate some load to see scaling

# Run integration tests
./tests/integration_test.sh

# Verify monitoring metrics
kubectl port-forward svc/pcf-api 8080:80 &
curl http://localhost:8080/metrics | grep -E "http_requests_total|go_memstats"

# Test rolling update
sed -i 's/test-v1/test-v2/g' k8s/deployment.yaml
kubectl apply -f k8s/deployment.yaml
kubectl rollout status deployment/pcf-api

# Check zero-downtime deployment
# (Monitor service availability during rollout)
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 5 Review Results

### HPA Configuration
- [ ] CPU scaling threshold: ___%
- [ ] Memory scaling threshold: ___%
- [ ] Custom metrics defined: [YES/NO]
- [ ] Scale down stabilization: ___s
- [ ] Behavior policies reasonable: [YES/NO]

### Deployment Automation
- [ ] Script handles all resources: [YES/NO]
- [ ] Proper deployment order: [YES/NO]
- [ ] Rollback capability: [YES/NO]
- [ ] Health verification: [YES/NO]

### Integration Testing
- [ ] Build verification: [PASS/FAIL]
- [ ] Security scanning: [PASS/FAIL]
- [ ] Deployment test: [PASS/FAIL]
- [ ] Metrics verification: [PASS/FAIL]

### Documentation
- [ ] Deployment guide complete: [YES/NO]
- [ ] Prerequisites listed: [YES/NO]
- [ ] Troubleshooting section: [YES/NO]
- [ ] Examples provided: [YES/NO]

### Production Readiness
- [ ] Zero-downtime updates: [YES/NO]
- [ ] Monitoring integrated: [YES/NO]
- [ ] Scaling tested: [YES/NO]
- [ ] Secrets managed properly: [YES/NO]

### Junior Developer Resources
- [ ] Guides referenced appropriately: [YES/NO]
- [ ] Guides were helpful: [YES/NO]
- [ ] Missing topics identified: [LIST]

### Decision: [APPROVED / APPROVED WITH CONDITIONS / CHANGES REQUIRED]

If APPROVED WITH CONDITIONS, specify:
- What conditions must be met
- Timeline for addressing conditions
- Whether work can continue

### Sign-off
Reviewed by: [Agent Name]
Date: [Date]
Phase 7 Status: [COMPLETE / INCOMPLETE]
```

## How to Handle Issues

When you find issues during review:

1. **Categorize by severity**:
   - **CRITICAL**: Security vulnerabilities, exposed secrets
   - **HIGH**: Image > 50MB, missing health checks
   - **MEDIUM**: Suboptimal configuration, missing docs
   - **LOW**: Style issues, minor optimizations

2. **Test thoroughly**:
   - Always build and run the container
   - Deploy to a test namespace
   - Verify all functionality works

3. **Provide specific fixes**:
   ```markdown
   Issue: Container runs as root
   File: Dockerfile, line 45
   Fix: Add USER directive:
   ```dockerfile
   USER 1000
   ```

## Review Decision Framework

### APPROVED
Grant approval when:
- Image size < 50MB
- Zero security vulnerabilities
- All health checks pass
- Kubernetes deployment successful
- Secrets properly managed
- Only LOW severity issues

### CHANGES REQUIRED
Require changes when:
- Image size exceeds limit
- Security vulnerabilities found
- Health checks fail
- Deployment errors occur
- Secrets exposed
- Any CRITICAL or HIGH issues

## Container Testing Guide

Essential container tests to run:

1. **Size Verification**
   ```bash
   docker images --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"
   ```

2. **Security Scanning**
   ```bash
   # Using Trivy
   docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
     aquasec/trivy image pcf-api:latest
   
   # Using Grype
   grype pcf-api:latest
   ```

3. **Runtime Testing**
   ```bash
   # Start container
   docker run -d --name test -p 8080:8080 pcf-api:latest
   
   # Check logs
   docker logs test
   
   # Test endpoints
   curl http://localhost:8080/health
   curl http://localhost:8080/health/ready
   ```

4. **Kubernetes Testing**
   ```bash
   # Deploy all resources
   kubectl apply -f k8s/
   
   # Watch rollout
   kubectl rollout status deployment/pcf-api
   
   # Check events for issues
   kubectl get events --sort-by='.lastTimestamp'
   ```

## Final Review Checklist

Before submitting your review:
- [ ] All tests executed successfully
- [ ] Security scan shows zero vulnerabilities
- [ ] Container runs as non-root user
- [ ] Image size verified < 50MB
- [ ] Kubernetes deployment tested
- [ ] Secrets verified secure
- [ ] Documentation reviewed
- [ ] Made clear APPROVED/CHANGES REQUIRED decision
- [ ] Provided specific remediation if needed

## Feedback File Template

When creating `api/.claude/.reviews/checkpoint-X-feedback.md`:

```markdown
# Checkpoint X Feedback

## Overall Assessment
[Brief summary of the checkpoint implementation quality]

## Strengths
- [What was done well]
- [Good practices observed]

## Issues Requiring Attention
### High Priority
1. [Critical issue with specific location and suggested fix]

### Medium Priority
1. [Important but not blocking issue]

### Low Priority
1. [Nice to have improvements]

## Test Results
- Container Build: [PASS/FAIL]
- Security Scan: [PASS/FAIL] 
- Size Requirement: [PASS/FAIL] - ___MB
- Kubernetes Deploy: [PASS/FAIL]

## Junior Developer Resources Assessment
- Resources Used: [List which guides were consulted]
- Effectiveness: [How helpful were they]
- Gaps Identified: [What additional guidance would help]

## Recommendations
1. [Specific actionable recommendations]

## Questions Answered
[If any questions were in checkpoint-X-questions.md, note that you answered them]
```

Remember: Container security is paramount. Be thorough in security scanning and secret management verification.