# Kubernetes Deployment Guide for APIs - Junior Developer Guide

## What You'll Learn

This guide teaches you how to deploy a Rust API to Kubernetes, including creating deployments, services, configuring health probes, managing secrets, and enabling auto-scaling.

## Why Kubernetes?

- **Scalability**: Automatically scale based on load
- **Reliability**: Self-healing with automatic restarts
- **Load Balancing**: Built-in request distribution
- **Rolling Updates**: Zero-downtime deployments
- **Secret Management**: Secure handling of sensitive data

## Core Kubernetes Concepts

### Pods
- Smallest deployable unit
- Contains one or more containers
- Shares network and storage
- Ephemeral (comes and goes)

### Deployments
- Manages a set of identical pods
- Ensures desired number of replicas
- Handles rolling updates
- Provides rollback capability

### Services
- Stable network endpoint for pods
- Load balances between pod replicas
- Service discovery via DNS
- Types: ClusterIP, NodePort, LoadBalancer

### ConfigMaps & Secrets
- **ConfigMap**: Non-sensitive configuration
- **Secret**: Sensitive data (base64 encoded)
- Both can be mounted as files or environment variables

## Step-by-Step Deployment

### 1. Basic Deployment Manifest

Start with a minimal deployment:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pcf-api
  labels:
    app: pcf-api
spec:
  replicas: 3  # Number of pod instances
  selector:
    matchLabels:
      app: pcf-api
  template:
    metadata:
      labels:
        app: pcf-api
    spec:
      containers:
      - name: pcf-api
        image: pcf-api:latest
        ports:
        - containerPort: 8080
```

### 2. Adding Resource Management

**Always define resource requests and limits:**

```yaml
spec:
  containers:
  - name: pcf-api
    resources:
      requests:  # Guaranteed resources
        memory: "64Mi"
        cpu: "100m"     # 0.1 CPU core
      limits:    # Maximum allowed
        memory: "256Mi"
        cpu: "500m"     # 0.5 CPU core
```

**Resource Guidelines:**
- **Requests**: What your app needs to run normally
- **Limits**: Maximum to prevent resource hogging
- Start conservative and adjust based on monitoring

### 3. Health Probes Configuration

Kubernetes uses probes to manage pod lifecycle:

```yaml
spec:
  containers:
  - name: pcf-api
    livenessProbe:
      httpGet:
        path: /health
        port: 8080
      initialDelaySeconds: 10  # Wait before first check
      periodSeconds: 30        # Check interval
      timeoutSeconds: 3        # Probe timeout
      failureThreshold: 3      # Failures before restart
    
    readinessProbe:
      httpGet:
        path: /health/ready
        port: 8080
      initialDelaySeconds: 5   # Faster than liveness
      periodSeconds: 10
      timeoutSeconds: 5
      failureThreshold: 3
    
    startupProbe:  # For slow-starting apps
      httpGet:
        path: /health/ready
        port: 8080
      failureThreshold: 30     # 30 * 10s = 5 minutes
      periodSeconds: 10
```

**Probe Types:**
- **Liveness**: Is the container alive? (restart if not)
- **Readiness**: Can it handle traffic? (remove from service if not)
- **Startup**: Give extra time for initial startup

### 4. Service Definition

Expose your deployment with a service:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: pcf-api
  labels:
    app: pcf-api
spec:
  type: ClusterIP  # Internal only
  selector:
    app: pcf-api   # Matches deployment labels
  ports:
  - name: http
    port: 80       # Service port
    targetPort: 8080  # Container port
    protocol: TCP
```

**Service Types:**
- **ClusterIP**: Internal cluster access only (default)
- **NodePort**: Exposes on each node's IP
- **LoadBalancer**: Cloud provider load balancer

### 5. ConfigMap for Configuration

Store non-sensitive configuration:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: pcf-api-config
data:
  # String values
  RUST_LOG: "info,pcf_api=debug"
  SERVER__HOST: "0.0.0.0"
  
  # Structured data
  app-config.yaml: |
    graphql:
      depth_limit: 15
      complexity_limit: 1000
    cache:
      ttl_seconds: 300
```

**Using ConfigMap in Deployment:**
```yaml
spec:
  containers:
  - name: pcf-api
    envFrom:
    - configMapRef:
        name: pcf-api-config
    # Or mount as file
    volumeMounts:
    - name: config
      mountPath: /etc/config
  volumes:
  - name: config
    configMap:
      name: pcf-api-config
```

### 6. Security Context

Run containers securely:

```yaml
spec:
  securityContext:  # Pod-level
    runAsNonRoot: true
    runAsUser: 1000
    fsGroup: 1000
  
  containers:
  - name: pcf-api
    securityContext:  # Container-level
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop:
        - ALL
```

### 7. Complete Production Deployment

Here's the full deployment combining all elements:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pcf-api
  labels:
    app: pcf-api
    version: v1
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 1
  selector:
    matchLabels:
      app: pcf-api
  template:
    metadata:
      labels:
        app: pcf-api
        version: v1
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: pcf-api
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
      - name: pcf-api
        image: pcf-api:v1.0.0
        imagePullPolicy: IfNotPresent
        ports:
        - name: http
          containerPort: 8080
          protocol: TCP
        env:
        - name: RUST_LOG
          value: "info,pcf_api=debug"
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: POD_NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
        envFrom:
        - configMapRef:
            name: pcf-api-config
        - secretRef:
            name: pcf-api-secrets
        resources:
          requests:
            memory: "64Mi"
            cpu: "100m"
          limits:
            memory: "256Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: http
          initialDelaySeconds: 10
          periodSeconds: 30
          timeoutSeconds: 3
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: http
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        securityContext:
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
        volumeMounts:
        - name: tmp
          mountPath: /tmp
        - name: cache
          mountPath: /app/cache
      volumes:
      - name: tmp
        emptyDir: {}
      - name: cache
        emptyDir:
          sizeLimit: 1Gi
```

### 8. Horizontal Pod Autoscaler (HPA)

Enable automatic scaling based on metrics:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: pcf-api
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: pcf-api
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300  # Wait 5 min before scaling down
      policies:
      - type: Percent
        value: 50  # Scale down by max 50%
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60   # Scale up faster
      policies:
      - type: Percent
        value: 100  # Double pods if needed
        periodSeconds: 30
```

## Deployment Commands

### Basic Deployment
```bash
# Apply all manifests
kubectl apply -f k8s/

# Check deployment status
kubectl rollout status deployment/pcf-api

# Get pods
kubectl get pods -l app=pcf-api

# Check logs
kubectl logs -l app=pcf-api --tail=100

# Describe pod for debugging
kubectl describe pod <pod-name>
```

### Rolling Update
```bash
# Update image
kubectl set image deployment/pcf-api pcf-api=pcf-api:v2.0.0

# Watch rollout
kubectl rollout status deployment/pcf-api

# Rollback if needed
kubectl rollout undo deployment/pcf-api
```

### Scaling
```bash
# Manual scale
kubectl scale deployment/pcf-api --replicas=5

# Check HPA status
kubectl get hpa pcf-api --watch
```

## Common Issues and Solutions

### 1. Pod Stuck in Pending
**Check resources:**
```bash
kubectl describe pod <pod-name>
kubectl top nodes  # Check node capacity
```

### 2. CrashLoopBackOff
**Check logs and events:**
```bash
kubectl logs <pod-name> --previous
kubectl describe pod <pod-name>
```

### 3. Readiness Probe Failing
**Test endpoint manually:**
```bash
kubectl exec <pod-name> -- curl localhost:8080/health/ready
```

### 4. OOMKilled (Out of Memory)
**Increase memory limits or optimize app:**
```yaml
resources:
  limits:
    memory: "512Mi"  # Increase from 256Mi
```

### 5. Image Pull Errors
**Check image name and pull policy:**
```bash
kubectl describe pod <pod-name> | grep -A5 "Events"
```

## Best Practices

### 1. Use Namespaces
Organize resources by environment:
```bash
kubectl create namespace production
kubectl apply -f k8s/ -n production
```

### 2. Label Everything
Use consistent labels:
```yaml
labels:
  app: pcf-api
  version: v1
  component: backend
  environment: production
```

### 3. Pod Disruption Budgets
Ensure availability during updates:
```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: pcf-api
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: pcf-api
```

### 4. Network Policies
Control traffic flow:
```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: pcf-api
spec:
  podSelector:
    matchLabels:
      app: pcf-api
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: frontend
    ports:
    - protocol: TCP
      port: 8080
```

### 5. Service Accounts
Use specific permissions:
```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: pcf-api
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: pcf-api
rules:
- apiGroups: [""]
  resources: ["configmaps"]
  verbs: ["get", "list", "watch"]
```

## Monitoring and Observability

### Prometheus Annotations
```yaml
annotations:
  prometheus.io/scrape: "true"
  prometheus.io/port: "8080"
  prometheus.io/path: "/metrics"
```

### Logging Best Practices
1. Use structured JSON logging
2. Include trace IDs
3. Don't log sensitive data
4. Use appropriate log levels

### Health Check Endpoints
- `/health` - Simple liveness check
- `/health/ready` - Comprehensive readiness
- `/metrics` - Prometheus metrics

## Deployment Checklist

Before deploying to production:

- [ ] Resource requests and limits defined
- [ ] Health probes configured
- [ ] Security context applied
- [ ] ConfigMaps for configuration
- [ ] Secrets properly managed
- [ ] HPA configured for scaling
- [ ] Service exposed correctly
- [ ] Labels consistent
- [ ] Monitoring enabled
- [ ] Logs structured
- [ ] Documentation updated

## Useful Commands Reference

```bash
# Get all resources
kubectl get all -l app=pcf-api

# Port forward for testing
kubectl port-forward svc/pcf-api 8080:80

# Execute commands in pod
kubectl exec -it <pod-name> -- /bin/sh

# Copy files from pod
kubectl cp <pod-name>:/path/to/file ./local-file

# Check resource usage
kubectl top pods -l app=pcf-api

# Get pod YAML
kubectl get pod <pod-name> -o yaml

# Dry run to validate
kubectl apply -f k8s/ --dry-run=client

# Delete all resources
kubectl delete -f k8s/
```

## Next Steps

1. Deploy your application to a test namespace
2. Verify all health checks pass
3. Test scaling with load
4. Set up monitoring dashboards
5. Document your deployment process

## Additional Resources

- [Kubernetes API documentation](https://kubernetes.io/docs/reference/kubernetes-api/)
- [kubectl cheat sheet](https://kubernetes.io/docs/reference/kubectl/cheatsheet/)
- [Kubernetes patterns](https://kubernetes.io/docs/concepts/cluster-administration/manage-deployment/)
- [Security best practices](https://kubernetes.io/docs/concepts/security/)