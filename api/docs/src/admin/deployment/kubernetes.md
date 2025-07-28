# Kubernetes Deployment

Comprehensive guide to deploying the PCF API on Kubernetes, including manifests, Helm charts, scaling strategies, and production best practices.

<!-- toc -->

## Overview

Kubernetes provides a robust platform for deploying the PCF API with features like auto-scaling, self-healing, rolling updates, and service discovery. This guide covers everything from basic deployments to advanced production configurations.

## Prerequisites

- Kubernetes cluster (1.26+)
- kubectl CLI tool
- Helm 3 (optional)
- Container registry access
- Ingress controller (nginx, traefik, etc.)

## Basic Deployment

### Namespace

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: pcf-system
  labels:
    app.kubernetes.io/name: pcf
    app.kubernetes.io/part-of: pcf-platform
```

### ConfigMap

```yaml
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: pcf-api-config
  namespace: pcf-system
data:
  production.toml: |
    [server]
    bind = "0.0.0.0"
    port = 8080
    workers = 0  # Use all available cores
    
    [graphql]
    playground_enabled = false
    introspection_enabled = false
    max_depth = 10
    max_complexity = 500
    
    [monitoring]
    metrics_enabled = true
    metrics_path = "/metrics"
    metrics_port = 9090
    
    [database]
    max_connections = 100
    min_connections = 10
    connect_timeout = 30
    idle_timeout = 600
    max_lifetime = 1800
  
  logging.json: |
    {
      "level": "info",
      "format": "json",
      "include_timestamp": true,
      "include_trace_id": true
    }
```

### Secret

```yaml
# secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: pcf-api-secrets
  namespace: pcf-system
type: Opaque
stringData:
  DATABASE_URL: "postgresql://pcf:password@postgres:5432/pcf_prod?sslmode=require"
  REDIS_URL: "redis://:password@redis:6379/0"
  JWT_SECRET: "your-super-secret-jwt-key"
  ENCRYPTION_KEY: "32-byte-encryption-key-here"
  OAUTH_CLIENT_SECRET: "oauth-client-secret"
```

### Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pcf-api
  namespace: pcf-system
  labels:
    app.kubernetes.io/name: pcf-api
    app.kubernetes.io/version: "1.0.0"
    app.kubernetes.io/component: api
spec:
  replicas: 3
  selector:
    matchLabels:
      app.kubernetes.io/name: pcf-api
      app.kubernetes.io/component: api
  template:
    metadata:
      labels:
        app.kubernetes.io/name: pcf-api
        app.kubernetes.io/component: api
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: pcf-api
      securityContext:
        runAsNonRoot: true
        runAsUser: 1001
        fsGroup: 1001
      containers:
      - name: api
        image: pcf-api:1.0.0
        imagePullPolicy: IfNotPresent
        ports:
        - name: http
          containerPort: 8080
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP
        env:
        - name: PCF_API__ENVIRONMENT
          value: "production"
        - name: PCF_API__CONFIG_DIR
          value: "/etc/pcf/config"
        - name: RUST_LOG
          value: "info,pcf_api=debug"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: pcf-api-secrets
              key: DATABASE_URL
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: pcf-api-secrets
              key: REDIS_URL
        - name: PCF_API__AUTH__JWT__SECRET
          valueFrom:
            secretKeyRef:
              name: pcf-api-secrets
              key: JWT_SECRET
        volumeMounts:
        - name: config
          mountPath: /etc/pcf/config
          readOnly: true
        - name: temp
          mountPath: /tmp
        resources:
          requests:
            cpu: 500m
            memory: 512Mi
          limits:
            cpu: 2000m
            memory: 2Gi
        livenessProbe:
          httpGet:
            path: /health
            port: http
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: http
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        startupProbe:
          httpGet:
            path: /health/startup
            port: http
          initialDelaySeconds: 0
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 30
        securityContext:
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
      volumes:
      - name: config
        configMap:
          name: pcf-api-config
      - name: temp
        emptyDir: {}
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app.kubernetes.io/name
                  operator: In
                  values:
                  - pcf-api
              topologyKey: kubernetes.io/hostname
```

### Service

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: pcf-api
  namespace: pcf-system
  labels:
    app.kubernetes.io/name: pcf-api
    app.kubernetes.io/component: api
spec:
  type: ClusterIP
  ports:
  - name: http
    port: 80
    targetPort: http
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: metrics
    protocol: TCP
  selector:
    app.kubernetes.io/name: pcf-api
    app.kubernetes.io/component: api
```

### Ingress

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: pcf-api
  namespace: pcf-system
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/force-ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-body-size: "10m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "300"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "300"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - api.example.com
    secretName: pcf-api-tls
  rules:
  - host: api.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: pcf-api
            port:
              number: 80
```

## Auto-Scaling

### Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: pcf-api-hpa
  namespace: pcf-system
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: pcf-api
  minReplicas: 3
  maxReplicas: 20
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
  - type: Pods
    pods:
      metric:
        name: http_requests_per_second
      target:
        type: AverageValue
        averageValue: "1000"
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Pods
        value: 2
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Pods
        value: 4
        periodSeconds: 60
      - type: Percent
        value: 100
        periodSeconds: 60
```

### Vertical Pod Autoscaler

```yaml
# vpa.yaml
apiVersion: autoscaling.k8s.io/v1
kind: VerticalPodAutoscaler
metadata:
  name: pcf-api-vpa
  namespace: pcf-system
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: pcf-api
  updatePolicy:
    updateMode: "Auto"  # or "Recreate" or "Off"
  resourcePolicy:
    containerPolicies:
    - containerName: api
      minAllowed:
        cpu: 200m
        memory: 256Mi
      maxAllowed:
        cpu: 4
        memory: 8Gi
      controlledResources: ["cpu", "memory"]
```

## Service Mesh Integration

### Istio Configuration

```yaml
# virtual-service.yaml
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: pcf-api
  namespace: pcf-system
spec:
  hosts:
  - api.example.com
  gateways:
  - pcf-gateway
  http:
  - match:
    - uri:
        prefix: "/"
    route:
    - destination:
        host: pcf-api
        port:
          number: 80
      weight: 100
    timeout: 30s
    retries:
      attempts: 3
      perTryTimeout: 10s
      retryOn: gateway-error,connect-failure,refused-stream
```

```yaml
# destination-rule.yaml
apiVersion: networking.istio.io/v1beta1
kind: DestinationRule
metadata:
  name: pcf-api
  namespace: pcf-system
spec:
  host: pcf-api
  trafficPolicy:
    connectionPool:
      tcp:
        maxConnections: 100
      http:
        http1MaxPendingRequests: 100
        http2MaxRequests: 100
        maxRequestsPerConnection: 2
    loadBalancer:
      simple: LEAST_REQUEST
    outlierDetection:
      consecutiveErrors: 5
      interval: 30s
      baseEjectionTime: 30s
      maxEjectionPercent: 50
      minHealthPercent: 50
```

## Database Connection

### Cloud SQL Proxy (GCP)

```yaml
# deployment-with-cloudsql.yaml
spec:
  template:
    spec:
      containers:
      - name: api
        # ... main container config ...
      - name: cloud-sql-proxy
        image: gcr.io/cloudsql-docker/gce-proxy:latest
        command:
          - "/cloud_sql_proxy"
          - "-instances=project:region:instance=tcp:5432"
          - "-credential_file=/secrets/service_account.json"
        securityContext:
          runAsNonRoot: true
        volumeMounts:
        - name: sa-key
          mountPath: /secrets/
          readOnly: true
      volumes:
      - name: sa-key
        secret:
          secretName: cloudsql-sa-key
```

### External Database with SSL

```yaml
# secret-with-certs.yaml
apiVersion: v1
kind: Secret
metadata:
  name: pcf-db-certs
  namespace: pcf-system
type: Opaque
data:
  ca.crt: # base64 encoded CA certificate
  client.crt: # base64 encoded client certificate
  client.key: # base64 encoded client key
---
# Mount in deployment
volumeMounts:
- name: db-certs
  mountPath: /etc/ssl/certs/db
  readOnly: true
volumes:
- name: db-certs
  secret:
    secretName: pcf-db-certs
```

## Monitoring and Observability

### ServiceMonitor (Prometheus Operator)

```yaml
# servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: pcf-api
  namespace: pcf-system
  labels:
    app.kubernetes.io/name: pcf-api
spec:
  selector:
    matchLabels:
      app.kubernetes.io/name: pcf-api
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
    scheme: http
    scrapeTimeout: 10s
```

### PodMonitor

```yaml
# podmonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: PodMonitor
metadata:
  name: pcf-api
  namespace: pcf-system
spec:
  selector:
    matchLabels:
      app.kubernetes.io/name: pcf-api
  podMetricsEndpoints:
  - port: metrics
    interval: 30s
    path: /metrics
```

## Security

### ServiceAccount and RBAC

```yaml
# rbac.yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: pcf-api
  namespace: pcf-system
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: pcf-api
  namespace: pcf-system
rules:
- apiGroups: [""]
  resources: ["configmaps"]
  verbs: ["get", "list", "watch"]
- apiGroups: [""]
  resources: ["secrets"]
  verbs: ["get"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: pcf-api
  namespace: pcf-system
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: pcf-api
subjects:
- kind: ServiceAccount
  name: pcf-api
  namespace: pcf-system
```

### NetworkPolicy

```yaml
# networkpolicy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: pcf-api
  namespace: pcf-system
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/name: pcf-api
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    - podSelector:
        matchLabels:
          app.kubernetes.io/name: prometheus
    ports:
    - protocol: TCP
      port: 8080
    - protocol: TCP
      port: 9090
  egress:
  - to:
    - podSelector:
        matchLabels:
          app.kubernetes.io/name: postgres
    ports:
    - protocol: TCP
      port: 5432
  - to:
    - podSelector:
        matchLabels:
          app.kubernetes.io/name: redis
    ports:
    - protocol: TCP
      port: 6379
  - to:
    - namespaceSelector: {}
      podSelector:
        matchLabels:
          k8s-app: kube-dns
    ports:
    - protocol: UDP
      port: 53
```

### PodSecurityPolicy

```yaml
# podsecuritypolicy.yaml
apiVersion: policy/v1beta1
kind: PodSecurityPolicy
metadata:
  name: pcf-api-psp
spec:
  privileged: false
  allowPrivilegeEscalation: false
  requiredDropCapabilities:
  - ALL
  volumes:
  - 'configMap'
  - 'emptyDir'
  - 'projected'
  - 'secret'
  - 'downwardAPI'
  - 'persistentVolumeClaim'
  hostNetwork: false
  hostIPC: false
  hostPID: false
  runAsUser:
    rule: 'MustRunAsNonRoot'
  seLinux:
    rule: 'RunAsAny'
  supplementalGroups:
    rule: 'RunAsAny'
  fsGroup:
    rule: 'RunAsAny'
  readOnlyRootFilesystem: true
```

## Helm Chart

### Chart Structure

```
helm/pcf-api/
├── Chart.yaml
├── values.yaml
├── values-prod.yaml
├── templates/
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── ingress.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   ├── hpa.yaml
│   ├── servicemonitor.yaml
│   └── _helpers.tpl
└── README.md
```

### values.yaml

```yaml
# Default values for pcf-api
replicaCount: 3

image:
  repository: pcf-api
  pullPolicy: IfNotPresent
  tag: "1.0.0"

serviceAccount:
  create: true
  annotations: {}
  name: ""

service:
  type: ClusterIP
  port: 80
  metricsPort: 9090

ingress:
  enabled: true
  className: "nginx"
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
  hosts:
    - host: api.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: pcf-api-tls
      hosts:
        - api.example.com

resources:
  limits:
    cpu: 2000m
    memory: 2Gi
  requests:
    cpu: 500m
    memory: 512Mi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 20
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

config:
  environment: production
  server:
    workers: 0
  graphql:
    playgroundEnabled: false
    introspectionEnabled: false
  database:
    maxConnections: 100

secrets:
  databaseUrl: ""
  redisUrl: ""
  jwtSecret: ""
```

### Helm Deployment

```bash
# Install
helm install pcf-api ./helm/pcf-api \
  --namespace pcf-system \
  --create-namespace \
  --values values-prod.yaml

# Upgrade
helm upgrade pcf-api ./helm/pcf-api \
  --namespace pcf-system \
  --values values-prod.yaml

# Rollback
helm rollback pcf-api 1 --namespace pcf-system

# Uninstall
helm uninstall pcf-api --namespace pcf-system
```

## Production Best Practices

### 1. Resource Quotas

```yaml
# resourcequota.yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: pcf-api-quota
  namespace: pcf-system
spec:
  hard:
    requests.cpu: "20"
    requests.memory: "40Gi"
    limits.cpu: "40"
    limits.memory: "80Gi"
    persistentvolumeclaims: "10"
    services.loadbalancers: "2"
```

### 2. Pod Disruption Budget

```yaml
# pdb.yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: pcf-api-pdb
  namespace: pcf-system
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app.kubernetes.io/name: pcf-api
```

### 3. Priority Classes

```yaml
# priorityclass.yaml
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: pcf-critical
value: 1000
globalDefault: false
description: "Critical PCF services"
---
# Use in deployment
spec:
  priorityClassName: pcf-critical
```

### 4. Topology Spread Constraints

```yaml
spec:
  topologySpreadConstraints:
  - maxSkew: 1
    topologyKey: topology.kubernetes.io/zone
    whenUnsatisfiable: DoNotSchedule
    labelSelector:
      matchLabels:
        app.kubernetes.io/name: pcf-api
```

## Troubleshooting

### Common Issues

```bash
# Pod not starting
kubectl describe pod pcf-api-xxx -n pcf-system
kubectl logs pcf-api-xxx -n pcf-system

# Check events
kubectl get events -n pcf-system --sort-by='.lastTimestamp'

# Resource issues
kubectl top nodes
kubectl top pods -n pcf-system

# Network issues
kubectl exec -it pcf-api-xxx -n pcf-system -- nc -zv postgres 5432

# Debug container
kubectl debug pcf-api-xxx -n pcf-system -it --image=busybox
```

### Useful Commands

```bash
# Port forwarding
kubectl port-forward svc/pcf-api 8080:80 -n pcf-system

# Get logs
kubectl logs -f deployment/pcf-api -n pcf-system

# Scale deployment
kubectl scale deployment pcf-api --replicas=5 -n pcf-system

# Update image
kubectl set image deployment/pcf-api api=pcf-api:1.1.0 -n pcf-system

# Check rollout status
kubectl rollout status deployment/pcf-api -n pcf-system

# Rollback
kubectl rollout undo deployment/pcf-api -n pcf-system
```

## Summary

Successful Kubernetes deployment requires:
1. **Proper resource management** - Set appropriate requests and limits
2. **Health checks** - Configure all three probe types
3. **Security policies** - Implement RBAC and network policies
4. **Observability** - Enable metrics and logging
5. **High availability** - Use multiple replicas with anti-affinity
6. **Auto-scaling** - Configure HPA and optionally VPA
7. **Graceful updates** - Use rolling deployments with PDB
8. **Monitoring** - Integrate with Prometheus and Grafana
9. **Backup strategy** - Regular backups of persistent data
10. **Documentation** - Keep deployment docs up to date
