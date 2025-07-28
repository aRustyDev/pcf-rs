# Container Debugging Guide - Junior Developer Guide

## What You'll Learn

This guide teaches you how to troubleshoot containerized applications, debug issues in Kubernetes, analyze resource usage, and fix common problems with HPA (Horizontal Pod Autoscaler).

## Common Container Issues

Understanding common problems helps you debug faster:

1. **Container Won't Start**: Image issues, missing dependencies
2. **CrashLoopBackOff**: Application crashes repeatedly  
3. **OOMKilled**: Out of memory errors
4. **Resource Starvation**: CPU/memory limits too low
5. **Network Issues**: Can't connect to services
6. **Permission Errors**: File system or user issues
7. **HPA Not Scaling**: Metrics or configuration problems

## Essential Debugging Commands

### Docker Debugging

```bash
# Check if container is running
docker ps -a

# View container logs
docker logs <container-id> --tail 50 -f

# Execute commands in running container
docker exec -it <container-id> /bin/sh

# Inspect container configuration
docker inspect <container-id>

# Check resource usage
docker stats <container-id>

# View image layers and size
docker history <image-name>

# Debug build issues
DOCKER_BUILDKIT=1 docker build --progress=plain -t debug .
```

### Kubernetes Debugging

```bash
# Get pod status and events
kubectl describe pod <pod-name>

# View pod logs
kubectl logs <pod-name> --tail=50 -f
kubectl logs <pod-name> --previous  # Previous container logs

# Execute commands in pod
kubectl exec -it <pod-name> -- /bin/sh

# Get events for debugging
kubectl get events --sort-by=.metadata.creationTimestamp

# Check resource usage
kubectl top pod <pod-name>
kubectl top nodes

# Port forward for testing
kubectl port-forward <pod-name> 8080:8080
```

## Debugging Scenarios

### 1. Container Won't Start

**Symptoms:**
- Container exits immediately
- Status: `Completed` or `Error`

**Debugging Steps:**

```bash
# 1. Check exit code
docker ps -a
# Look for Exit Code: 0 (success), 1 (general error), 125 (docker daemon error), 126 (container command not executable), 127 (container command not found)

# 2. View logs
docker logs <container-id>

# 3. Try running with shell
docker run -it --entrypoint /bin/sh <image-name>

# 4. Check if binary exists and is executable
docker run --rm <image-name> ls -la /pcf-api

# 5. Verify command syntax
docker inspect <image-name> | jq '.[0].Config.Entrypoint, .[0].Config.Cmd'
```

**Common Fixes:**
```dockerfile
# Ensure binary is executable
RUN chmod +x /pcf-api

# Use correct path
ENTRYPOINT ["/pcf-api"]  # Not "pcf-api"

# For scratch images, ensure all dependencies
COPY --from=builder /lib/x86_64-linux-gnu/libc.so.6 /lib/x86_64-linux-gnu/
```

### 2. CrashLoopBackOff in Kubernetes

**Symptoms:**
- Pod restarts repeatedly
- Status: `CrashLoopBackOff`

**Debugging Steps:**

```bash
# 1. Describe pod for events
kubectl describe pod <pod-name>
# Look for:
# - Exit Code
# - Reason
# - Message
# - Last State

# 2. Check logs from crashed container
kubectl logs <pod-name> --previous

# 3. Check resource limits
kubectl get pod <pod-name> -o yaml | grep -A5 resources:

# 4. Test with increased resources
kubectl patch deployment <deployment-name> -p '
{
  "spec": {
    "template": {
      "spec": {
        "containers": [{
          "name": "pcf-api",
          "resources": {
            "limits": {
              "memory": "512Mi",
              "cpu": "1000m"
            }
          }
        }]
      }
    }
  }
}'
```

**Debug Container Method:**
```yaml
# Add debug container to pod spec
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: pcf-api
    image: pcf-api:latest
    command: ["/bin/sh"]  # Override to prevent crash
    args: ["-c", "sleep 3600"]  # Keep alive for debugging
```

### 3. OOMKilled (Out of Memory)

**Symptoms:**
- Container terminated with `OOMKilled`
- Exit code 137

**Debugging Steps:**

```bash
# 1. Check memory usage before OOM
kubectl describe pod <pod-name>
# Look for "Last State: Terminated Reason: OOMKilled"

# 2. View memory limits
kubectl get pod <pod-name> -o yaml | grep -A3 "memory:"

# 3. Monitor memory usage
kubectl top pod <pod-name> --containers

# 4. Check for memory leaks
kubectl exec <pod-name> -- /bin/sh -c 'ps aux | sort -k4 -r | head'
```

**Memory Profiling in Container:**
```rust
// Add memory reporting endpoint
async fn memory_stats() -> Json<MemoryStats> {
    let mut stats = MemoryStats::default();
    
    if let Ok(status) = procinfo::pid::status() {
        stats.vm_rss = status.vm_rss;
        stats.vm_size = status.vm_size;
    }
    
    Json(stats)
}
```

**Fix Memory Issues:**
```yaml
resources:
  requests:
    memory: "256Mi"  # Increase from 64Mi
  limits:
    memory: "512Mi"  # Increase from 256Mi
```

### 4. Container Can't Connect to Services

**Symptoms:**
- Connection refused errors
- DNS resolution failures
- Timeouts

**Debugging Network Issues:**

```bash
# 1. Test DNS resolution
kubectl exec <pod-name> -- nslookup kubernetes.default
kubectl exec <pod-name> -- nslookup <service-name>

# 2. Test connectivity
kubectl exec <pod-name> -- nc -zv <service-name> <port>
kubectl exec <pod-name> -- curl -v http://<service-name>:<port>/health

# 3. Check service endpoints
kubectl get endpoints <service-name>

# 4. Verify network policies
kubectl get networkpolicies

# 5. Debug with tcpdump
kubectl exec <pod-name> -- tcpdump -i any -n host <target-ip>
```

**Network Debug Container:**
```yaml
apiVersion: v1
kind: Pod
metadata:
  name: network-debug
spec:
  containers:
  - name: debug
    image: nicolaka/netshoot
    command: ["/bin/bash"]
    args: ["-c", "sleep 3600"]
```

### 5. Permission and File System Errors

**Symptoms:**
- Permission denied errors
- Read-only file system errors
- Cannot create directory/file

**Debugging Steps:**

```bash
# 1. Check user and permissions
kubectl exec <pod-name> -- id
kubectl exec <pod-name> -- ls -la /

# 2. Test write permissions
kubectl exec <pod-name> -- touch /tmp/test
kubectl exec <pod-name> -- mkdir -p /app/data

# 3. Check security context
kubectl get pod <pod-name> -o yaml | grep -A10 securityContext:
```

**Common Fixes:**

```yaml
# Add writable volumes
spec:
  containers:
  - name: app
    volumeMounts:
    - name: tmp
      mountPath: /tmp
    - name: data
      mountPath: /app/data
  volumes:
  - name: tmp
    emptyDir: {}
  - name: data
    emptyDir: {}
    
  # Fix user permissions
  securityContext:
    runAsUser: 1000
    runAsGroup: 1000
    fsGroup: 1000
```

### 6. HPA Not Scaling

**Symptoms:**
- HPA shows `<unknown>` for metrics
- Pods don't scale despite load
- Scaling delays

**Debugging HPA:**

```bash
# 1. Check HPA status
kubectl get hpa <hpa-name>
kubectl describe hpa <hpa-name>

# 2. Verify metrics-server
kubectl get deployment metrics-server -n kube-system
kubectl top nodes  # Should return data

# 3. Check pod metrics
kubectl top pod -l app=pcf-api

# 4. Verify resource requests are set
kubectl get deployment <deployment-name> -o yaml | grep -A5 resources:

# 5. Check HPA events
kubectl describe hpa <hpa-name> | grep -A20 Events:
```

**Fix HPA Issues:**

```yaml
# Ensure resource requests are set (required for HPA)
spec:
  containers:
  - name: pcf-api
    resources:
      requests:  # REQUIRED for HPA!
        cpu: 100m
        memory: 128Mi
      limits:
        cpu: 500m
        memory: 512Mi
```

**Test HPA Scaling:**
```bash
# Generate load
kubectl run -i --tty load-generator --rm --image=busybox --restart=Never -- /bin/sh -c "while sleep 0.01; do wget -q -O- http://pcf-api/health; done"

# Watch HPA
kubectl get hpa <hpa-name> --watch
```

## Advanced Debugging Techniques

### 1. Debug Containers (Ephemeral Containers)

```bash
# Add debug container to running pod (K8s 1.23+)
kubectl debug <pod-name> -it --image=busybox --target=<container-name>

# Copy pod with debug modifications
kubectl debug <pod-name> -it --copy-to=debug-pod --container=app -- /bin/sh
```

### 2. Core Dumps

Enable core dumps for crash analysis:

```yaml
spec:
  containers:
  - name: app
    securityContext:
      capabilities:
        add:
        - SYS_PTRACE  # For debugging
    env:
    - name: RUST_BACKTRACE
      value: "full"
```

### 3. Profiling in Containers

**CPU Profiling:**
```dockerfile
# Install profiling tools
RUN apt-get update && apt-get install -y linux-perf

# Run with profiling
CMD ["perf", "record", "-g", "--", "/pcf-api"]
```

**Memory Profiling:**
```rust
// Add profiling endpoint
#[cfg(feature = "profiling")]
async fn heap_profile() -> Result<Vec<u8>, Error> {
    let profile = jeprof::heap_profile()?;
    Ok(profile)
}
```

### 4. Distributed Tracing

Debug request flow across services:

```rust
use opentelemetry::trace::Tracer;

#[instrument(skip(db))]
async fn handle_request(db: &Database) -> Result<Response, Error> {
    let span = tracer.start("database_query");
    let result = db.query().await;
    span.end();
    
    Ok(result)
}
```

View traces:
```bash
# Port forward to Jaeger
kubectl port-forward svc/jaeger-query 16686:80

# Open http://localhost:16686
```

## Debugging Checklists

### Container Won't Start Checklist
- [ ] Check exit code and logs
- [ ] Verify image exists and is pullable
- [ ] Test entrypoint/command manually
- [ ] Check for missing dependencies
- [ ] Verify file permissions
- [ ] Test with different user

### CrashLoopBackOff Checklist
- [ ] Check application logs
- [ ] Verify configuration
- [ ] Test with increased resources
- [ ] Check health probe settings
- [ ] Verify all environment variables
- [ ] Test locally with same settings

### Performance Issues Checklist
- [ ] Monitor CPU and memory usage
- [ ] Check for throttling
- [ ] Review resource limits
- [ ] Analyze slow queries/operations
- [ ] Check network latency
- [ ] Review concurrent connections

### HPA Issues Checklist
- [ ] Verify metrics-server is running
- [ ] Check resource requests are set
- [ ] Validate HPA target metrics
- [ ] Review scaling policies
- [ ] Check for pod disruption budgets
- [ ] Monitor HPA events

## Useful Debug Scripts

### Container Health Check Script
```bash
#!/bin/bash
# debug-health.sh
POD=$1
NAMESPACE=${2:-default}

echo "=== Pod Status ==="
kubectl get pod $POD -n $NAMESPACE

echo -e "\n=== Recent Events ==="
kubectl get events -n $NAMESPACE --field-selector involvedObject.name=$POD

echo -e "\n=== Container Logs (last 50 lines) ==="
kubectl logs $POD -n $NAMESPACE --tail=50

echo -e "\n=== Resource Usage ==="
kubectl top pod $POD -n $NAMESPACE

echo -e "\n=== Pod Description ==="
kubectl describe pod $POD -n $NAMESPACE | grep -A5 "Conditions:\|Events:"
```

### Resource Analysis Script
```bash
#!/bin/bash
# analyze-resources.sh
DEPLOYMENT=$1
NAMESPACE=${2:-default}

echo "=== Current Resource Settings ==="
kubectl get deployment $DEPLOYMENT -n $NAMESPACE -o json | jq '.spec.template.spec.containers[].resources'

echo -e "\n=== Actual Usage (all pods) ==="
kubectl top pods -n $NAMESPACE -l app=$DEPLOYMENT

echo -e "\n=== HPA Status ==="
kubectl get hpa -n $NAMESPACE | grep $DEPLOYMENT

echo -e "\n=== Recommendations ==="
# Simple recommendation based on usage
USAGE=$(kubectl top pods -n $NAMESPACE -l app=$DEPLOYMENT --no-headers | awk '{print $2}' | sed 's/m//' | awk '{sum+=$1} END {print sum/NR}')
echo "Average CPU usage: ${USAGE}m"
if (( $(echo "$USAGE > 80" | bc -l) )); then
    echo "⚠️  High CPU usage - consider increasing limits"
fi
```

## Prevention Strategies

### 1. Comprehensive Health Checks
```rust
// Detailed readiness check
async fn readiness_check(state: State<AppState>) -> Result<Json<Health>, Error> {
    let mut checks = HashMap::new();
    
    // Database check
    match state.db.ping().await {
        Ok(latency) => checks.insert("database", HealthStatus::Ok(latency)),
        Err(e) => checks.insert("database", HealthStatus::Error(e.to_string())),
    };
    
    // Memory check
    if let Ok(usage) = get_memory_usage() {
        if usage > 0.9 {
            checks.insert("memory", HealthStatus::Warning("High memory usage"));
        }
    }
    
    Ok(Json(Health { checks }))
}
```

### 2. Gradual Rollouts
```yaml
spec:
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1        # Only 1 new pod at a time
      maxUnavailable: 0  # Keep all old pods during update
```

### 3. Circuit Breakers
```rust
use circuit_breaker::CircuitBreaker;

let breaker = CircuitBreaker::new()
    .error_threshold(5)
    .success_threshold(2)
    .timeout(Duration::from_secs(30));

match breaker.call(|| external_service.request()).await {
    Ok(response) => process(response),
    Err(_) => fallback_response(),
}
```

## Next Steps

1. Practice debugging in a test environment
2. Set up monitoring and alerting
3. Create runbooks for common issues
4. Implement better observability
5. Regular chaos testing

## Additional Resources

- [Kubernetes Debugging Documentation](https://kubernetes.io/docs/tasks/debug/)
- [Docker Debugging Guide](https://docs.docker.com/config/containers/logging/)
- [kubectl Cheat Sheet](https://kubernetes.io/docs/reference/kubectl/cheatsheet/)
- [Linux Performance Tools](http://www.brendangregg.com/linuxperf.html)