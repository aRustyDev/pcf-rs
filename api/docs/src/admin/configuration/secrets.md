# Secrets Management

Comprehensive guide to managing secrets and sensitive configuration data for the PCF API in production environments.

<!-- toc -->

## Overview

Secrets management is critical for securing the PCF API. This guide covers best practices for handling sensitive data like database passwords, API keys, JWT secrets, and other confidential configuration values.

## Secret Types

### Application Secrets

| Secret Type | Description | Example Variable |
|-------------|-------------|------------------|
| JWT Secret | Token signing key | `PCF_API__AUTH__JWT__SECRET` |
| Database Password | Database credentials | `PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD` |
| SpiceDB Token | Authorization service key | `PCF_API__SERVICES__SPICEDB__TOKEN` |
| OAuth Client Secret | OAuth2 authentication | `PCF_API__AUTH__OAUTH2__CLIENT_SECRET` |
| API Keys | Third-party service keys | Custom variables |
| Encryption Keys | Data encryption keys | Custom variables |

### Infrastructure Secrets

| Secret Type | Description | Usage |
|-------------|-------------|-------|
| TLS Certificates | HTTPS certificates | Server configuration |
| SSH Keys | Deployment keys | CI/CD pipelines |
| Service Tokens | Inter-service auth | Microservices |
| Monitoring Keys | APM/logging tokens | Observability |

## Secret Management Strategies

### 1. Environment Variables

Most common approach for container deployments:

```bash
#!/bin/bash
# start-api.sh

# Load secrets from secure source
export PCF_API__AUTH__JWT__SECRET=$(vault kv get -field=secret secret/pcf-api/jwt)
export PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=$(vault kv get -field=password secret/pcf-api/db)

# Start application
exec ./pcf-api
```

**Pros:**
- Simple implementation
- Wide tool support
- Container-friendly

**Cons:**
- Visible in process list
- Can leak in logs
- Limited rotation

### 2. File-Based Secrets

Mount secrets as files:

```bash
# Docker compose example
volumes:
  - /run/secrets/jwt_secret:/secrets/jwt_secret:ro
  - /run/secrets/db_password:/secrets/db_password:ro

# Application reads from files
export PCF_API__AUTH__JWT__SECRET=$(cat /secrets/jwt_secret)
export PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=$(cat /secrets/db_password)
```

**Pros:**
- More secure than env vars
- Better access control
- Easier rotation

**Cons:**
- Requires file system access
- More complex setup

### 3. Secret Management Services

#### HashiCorp Vault

```bash
# Initialize Vault client
export VAULT_ADDR='https://vault.example.com:8200'
export VAULT_TOKEN='your-vault-token'

# Retrieve secrets
vault kv get -format=json secret/pcf-api | jq -r '.data.data' > secrets.json

# Export as environment variables
export PCF_API__AUTH__JWT__SECRET=$(jq -r '.jwt_secret' secrets.json)
export PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=$(jq -r '.db_password' secrets.json)

# Clean up
rm secrets.json
```

#### AWS Secrets Manager

```bash
#!/bin/bash
# aws-secrets.sh

# Retrieve secret
SECRET_JSON=$(aws secretsmanager get-secret-value \
  --secret-id pcf-api/production \
  --query SecretString \
  --output text)

# Parse and export
export PCF_API__AUTH__JWT__SECRET=$(echo $SECRET_JSON | jq -r '.jwt_secret')
export PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=$(echo $SECRET_JSON | jq -r '.db_password')
```

#### Azure Key Vault

```bash
# Login to Azure
az login --service-principal -u $AZURE_CLIENT_ID -p $AZURE_CLIENT_SECRET --tenant $AZURE_TENANT_ID

# Retrieve secrets
export PCF_API__AUTH__JWT__SECRET=$(az keyvault secret show \
  --vault-name pcf-api-vault \
  --name jwt-secret \
  --query value -o tsv)

export PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=$(az keyvault secret show \
  --vault-name pcf-api-vault \
  --name db-password \
  --query value -o tsv)
```

#### Google Secret Manager

```bash
# Authenticate
gcloud auth activate-service-account --key-file=$GOOGLE_APPLICATION_CREDENTIALS

# Retrieve secrets
export PCF_API__AUTH__JWT__SECRET=$(gcloud secrets versions access latest \
  --secret="pcf-api-jwt-secret")

export PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=$(gcloud secrets versions access latest \
  --secret="pcf-api-db-password")
```

## Kubernetes Secrets

### Creating Secrets

```yaml
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: pcf-api-secrets
  namespace: production
type: Opaque
stringData:
  PCF_API__AUTH__JWT__SECRET: "your-jwt-secret"
  PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD: "your-db-password"
  PCF_API__SERVICES__SPICEDB__TOKEN: "your-spicedb-token"
```

```bash
# Create from file
kubectl create secret generic pcf-api-secrets \
  --from-env-file=.env.production \
  --namespace=production

# Create from literal values
kubectl create secret generic pcf-api-secrets \
  --from-literal=PCF_API__AUTH__JWT__SECRET='your-jwt-secret' \
  --from-literal=PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD='your-db-password' \
  --namespace=production
```

### Using Secrets in Deployments

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pcf-api
spec:
  template:
    spec:
      containers:
      - name: api
        image: pcf-api:latest
        envFrom:
        - secretRef:
            name: pcf-api-secrets
        env:
        - name: PCF_API__ENVIRONMENT
          value: "production"
```

### Sealed Secrets

For GitOps workflows:

```bash
# Install sealed-secrets controller
kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.18.0/controller.yaml

# Create sealed secret
echo -n 'your-jwt-secret' | kubectl create secret generic pcf-api-secrets \
  --dry-run=client \
  --from-file=PCF_API__AUTH__JWT__SECRET=/dev/stdin \
  -o yaml | kubeseal -o yaml > sealed-secrets.yaml

# Commit sealed-secrets.yaml to Git (safe)
```

## Docker Secrets

### Docker Swarm

```bash
# Create secrets
echo "your-jwt-secret" | docker secret create jwt_secret -
echo "your-db-password" | docker secret create db_password -

# Use in stack
version: '3.8'
services:
  api:
    image: pcf-api:latest
    secrets:
      - jwt_secret
      - db_password
    environment:
      PCF_API__AUTH__JWT__SECRET_FILE: /run/secrets/jwt_secret
      PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD_FILE: /run/secrets/db_password

secrets:
  jwt_secret:
    external: true
  db_password:
    external: true
```

### Docker Compose (Development)

```yaml
# docker-compose.yml
version: '3.8'
services:
  api:
    image: pcf-api:latest
    environment:
      PCF_API__AUTH__JWT__SECRET: ${JWT_SECRET}
      PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD: ${DB_PASSWORD}
    env_file:
      - .env.local  # Not committed to Git
```

## Secret Rotation

### Automated Rotation Script

```bash
#!/bin/bash
# rotate-secrets.sh

set -euo pipefail

# Configuration
VAULT_PATH="secret/pcf-api"
K8S_NAMESPACE="production"
K8S_SECRET="pcf-api-secrets"

# Generate new JWT secret
NEW_JWT_SECRET=$(openssl rand -base64 32)

# Update Vault
vault kv put $VAULT_PATH jwt_secret=$NEW_JWT_SECRET

# Update Kubernetes
kubectl create secret generic $K8S_SECRET-new \
  --from-literal=PCF_API__AUTH__JWT__SECRET=$NEW_JWT_SECRET \
  --namespace=$K8S_NAMESPACE \
  --dry-run=client -o yaml | kubectl apply -f -

# Trigger rolling update
kubectl rollout restart deployment/pcf-api -n $K8S_NAMESPACE

# Wait for rollout
kubectl rollout status deployment/pcf-api -n $K8S_NAMESPACE

# Clean up old secret
kubectl delete secret $K8S_SECRET -n $K8S_NAMESPACE || true
kubectl patch secret $K8S_SECRET-new -n $K8S_NAMESPACE \
  -p '{"metadata":{"name":"'$K8S_SECRET'"}}'

echo "Secret rotation completed successfully"
```

### Zero-Downtime Rotation

```bash
#!/bin/bash
# zero-downtime-rotation.sh

# Step 1: Add new secret alongside old
export PCF_API__AUTH__JWT__SECRET_NEW=$NEW_SECRET
export PCF_API__AUTH__JWT__SECRET=$OLD_SECRET

# Step 2: Deploy with dual secret support
kubectl set env deployment/pcf-api \
  PCF_API__AUTH__JWT__SECRET_NEW=$NEW_SECRET \
  -n production

# Step 3: Wait for all pods to support both secrets
kubectl rollout status deployment/pcf-api -n production

# Step 4: Switch primary secret
kubectl set env deployment/pcf-api \
  PCF_API__AUTH__JWT__SECRET=$NEW_SECRET \
  -n production

# Step 5: Remove old secret support
kubectl set env deployment/pcf-api \
  PCF_API__AUTH__JWT__SECRET_NEW- \
  -n production
```

## Security Best Practices

### 1. Never Commit Secrets

```gitignore
# .gitignore
.env*
!.env.example
secrets/
*.key
*.pem
*.p12
*.pfx
```

### 2. Use Strong Secrets

```bash
# Generate strong secrets
openssl rand -base64 32  # For JWT secret
openssl rand -hex 32     # For API keys
pwgen -s 32 1           # Using pwgen tool

# Validate secret strength
echo -n "$SECRET" | wc -c  # Should be 32+ characters
```

### 3. Limit Secret Access

```bash
# Kubernetes RBAC
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: secret-reader
  namespace: production
rules:
- apiGroups: [""]
  resources: ["secrets"]
  resourceNames: ["pcf-api-secrets"]
  verbs: ["get"]
```

### 4. Audit Secret Access

```bash
# Enable audit logging
kubectl audit enable

# Monitor secret access
kubectl get events --field-selector reason=SecretAccessed
```

### 5. Encrypt Secrets at Rest

```yaml
# Kubernetes encryption config
apiVersion: apiserver.config.k8s.io/v1
kind: EncryptionConfiguration
resources:
  - resources:
    - secrets
    providers:
    - aescbc:
        keys:
        - name: key1
          secret: <base64-encoded-secret>
```

## Development Workflow

### Local Development

```bash
# .env.example (committed)
PCF_API__AUTH__JWT__SECRET=change-me-in-production
PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=change-me-in-production

# .env.local (not committed)
PCF_API__AUTH__JWT__SECRET=dev-secret-key
PCF_API__SERVICES__DATABASE__SURREALDB__PASSWORD=dev-password

# Load for development
source .env.local
cargo run
```

### CI/CD Pipeline

```yaml
# GitHub Actions example
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Configure AWS credentials
      uses: aws-actions/configure-aws-credentials@v1
      with:
        aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
        aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
        aws-region: us-east-1
    
    - name: Retrieve secrets
      run: |
        SECRET_JSON=$(aws secretsmanager get-secret-value \
          --secret-id pcf-api/production \
          --query SecretString \
          --output text)
        
        echo "PCF_API__AUTH__JWT__SECRET=$(echo $SECRET_JSON | jq -r '.jwt_secret')" >> $GITHUB_ENV
```

## Monitoring and Alerting

### Secret Expiration Monitoring

```bash
#!/bin/bash
# check-secret-age.sh

# Check secret age
SECRET_AGE=$(vault kv metadata get -field=created_time secret/pcf-api)
CURRENT_TIME=$(date +%s)
SECRET_TIME=$(date -d "$SECRET_AGE" +%s)
AGE_DAYS=$(( ($CURRENT_TIME - $SECRET_TIME) / 86400 ))

if [ $AGE_DAYS -gt 90 ]; then
  echo "WARNING: Secret is $AGE_DAYS days old and should be rotated"
  # Send alert
  curl -X POST $SLACK_WEBHOOK -d '{"text":"PCF API secrets need rotation"}'
fi
```

### Access Monitoring

```yaml
# Prometheus rule
groups:
- name: secret_access
  rules:
  - alert: UnauthorizedSecretAccess
    expr: |
      kube_pod_info{namespace="production",pod=~".*pcf-api.*"}
      unless
      kube_pod_labels{namespace="production",label_app="pcf-api"}
    for: 1m
    annotations:
      summary: "Unauthorized secret access detected"
```

## Emergency Procedures

### Secret Compromise Response

1. **Immediate Actions**
   ```bash
   # Revoke compromised secret
   kubectl delete secret pcf-api-secrets -n production
   
   # Generate new secrets
   ./generate-emergency-secrets.sh
   
   # Deploy with new secrets
   kubectl apply -f emergency-secrets.yaml
   
   # Force restart all pods
   kubectl delete pods -l app=pcf-api -n production
   ```

2. **Audit Trail**
   ```bash
   # Check access logs
   kubectl logs -n production -l app=pcf-api --since=24h | grep -i auth
   
   # Export audit logs
   kubectl get events -n production --sort-by='.lastTimestamp' > audit.log
   ```

3. **Communication**
   - Notify security team
   - Document incident
   - Update runbooks

## Testing Secret Management

### Integration Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_secret_loading() {
        // Test missing secret handling
        std::env::remove_var("PCF_API__AUTH__JWT__SECRET");
        let result = load_config();
        assert!(result.is_err());
        
        // Test secret validation
        std::env::set_var("PCF_API__AUTH__JWT__SECRET", "short");
        let result = load_config();
        assert!(result.is_err(), "Secret too short");
    }
}
```

### Security Scanning

```bash
# Scan for hardcoded secrets
trufflehog filesystem --directory . --json

# Check for exposed secrets in Git history
gitleaks detect --source . -v

# Validate Kubernetes secrets
kubectl auth can-i get secrets --namespace=production
```

## Troubleshooting

### Common Issues

1. **Secret Not Found**
   ```bash
   Error: PCF_API__AUTH__JWT__SECRET not set
   
   # Debug: List available secrets
   env | grep PCF_API__ | grep -v PASSWORD
   ```

2. **Permission Denied**
   ```bash
   Error: Failed to read secret: permission denied
   
   # Fix: Check service account permissions
   kubectl auth can-i get secrets -n production --as=system:serviceaccount:production:pcf-api
   ```

3. **Secret Rotation Failure**
   ```bash
   # Manual rollback procedure
   kubectl rollout undo deployment/pcf-api -n production
   
   # Restore previous secret
   kubectl apply -f backup/secrets-backup.yaml
   ```

## Summary

Effective secrets management requires:
1. **Never store secrets in code** - Use external secret management
2. **Rotate regularly** - Implement automated rotation
3. **Limit access** - Use principle of least privilege
4. **Monitor usage** - Audit and alert on anomalies
5. **Plan for compromise** - Have emergency procedures ready