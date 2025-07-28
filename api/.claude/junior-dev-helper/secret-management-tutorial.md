# Secret Management Tutorial - Junior Developer Guide

## What You'll Learn

This tutorial teaches you how to handle sensitive data (passwords, API keys, tokens) safely in containerized applications, with a focus on Kubernetes secrets and environment variables.

## Why Secret Management is Critical

- **Security Breaches**: Exposed secrets are the #1 cause of data breaches
- **Compliance**: Regulations require proper secret handling
- **Rotation**: Secrets need to be changed without downtime
- **Audit Trail**: Need to track who accessed what
- **Developer Experience**: Easy to use correctly, hard to misuse

## Types of Secrets

Understanding different secret types helps you handle them appropriately:

1. **Database Credentials**: Connection strings, passwords
2. **API Keys**: Third-party service authentication
3. **Encryption Keys**: For data at rest/in transit
4. **Certificates**: TLS/SSL certificates
5. **Tokens**: JWT secrets, OAuth tokens

## Secret Management Anti-Patterns

### ❌ What NOT to Do

**1. Hardcoding in Source Code:**
```rust
// NEVER DO THIS!
const DATABASE_URL: &str = "postgresql://user:password@host/db";
```

**2. Committing .env Files:**
```bash
# .env file with real secrets
DATABASE_PASSWORD=super_secret_password
API_KEY=sk_live_abcd1234efgh5678
```

**3. Building Secrets into Images:**
```dockerfile
# NEVER DO THIS!
ENV DATABASE_PASSWORD=my_password
COPY .env /app/.env
```

**4. Logging Secrets:**
```rust
// NEVER DO THIS!
println!("Connecting with password: {}", password);
```

## Kubernetes Secret Management

### Creating Secrets

**Method 1: From Literals**
```bash
kubectl create secret generic pcf-api-secrets \
    --from-literal=database-password='my$ecretPa$$' \
    --from-literal=jwt-secret='super-long-random-string'
```

**Method 2: From Files**
```bash
# Create secret files
echo -n 'my$ecretPa$$' > ./database-password.txt
echo -n 'super-long-random-string' > ./jwt-secret.txt

# Create secret from files
kubectl create secret generic pcf-api-secrets \
    --from-file=database-password=./database-password.txt \
    --from-file=jwt-secret=./jwt-secret.txt

# Clean up files immediately!
rm ./database-password.txt ./jwt-secret.txt
```

**Method 3: From YAML (Declarative)**
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: pcf-api-secrets
type: Opaque
stringData:  # Use stringData for plain text (auto-encoded)
  database-password: "my$ecretPa$$"
  jwt-secret: "super-long-random-string"
```

### Using Secrets in Pods

**As Environment Variables:**
```yaml
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: pcf-api
    image: pcf-api:latest
    env:
    # Single secret value
    - name: DATABASE_PASSWORD
      valueFrom:
        secretKeyRef:
          name: pcf-api-secrets
          key: database-password
    
    # All secrets as env vars
    envFrom:
    - secretRef:
        name: pcf-api-secrets
```

**As Volume Mounts:**
```yaml
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: pcf-api
    volumeMounts:
    - name: secrets
      mountPath: /etc/secrets
      readOnly: true
  volumes:
  - name: secrets
    secret:
      secretName: pcf-api-secrets
      # Each key becomes a file
      # /etc/secrets/database-password
      # /etc/secrets/jwt-secret
```

### Secret Templates

Create a template for easy secret generation:

**secret-template.yaml:**
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: pcf-api-secrets
type: Opaque
stringData:
  # Database
  DATABASE__URL: "postgresql://REPLACE_USER:REPLACE_PASS@REPLACE_HOST:5432/REPLACE_DB"
  
  # Authentication
  AUTH__JWT_SECRET: "REPLACE_JWT_SECRET"
  AUTH__REFRESH_SECRET: "REPLACE_REFRESH_SECRET"
  
  # External Services
  SPICEDB__PRESHARED_KEY: "REPLACE_SPICEDB_KEY"
  
  # Monitoring
  METRICS__BASIC_AUTH_PASSWORD: "REPLACE_METRICS_PASS"
```

**Secret creation script:**
```bash
#!/bin/bash
# scripts/create-secrets.sh
set -euo pipefail

ENV_FILE="${1:-.env.production}"
NAMESPACE="${NAMESPACE:-default}"

# Load environment variables
if [ ! -f "$ENV_FILE" ]; then
    echo "❌ Environment file $ENV_FILE not found!"
    echo "Create it with: cp .env.example $ENV_FILE"
    exit 1
fi

# Source the environment file
set -a
source "$ENV_FILE"
set +a

# Create temporary secret file
TEMP_SECRET=$(mktemp)
trap "rm -f $TEMP_SECRET" EXIT

# Replace placeholders
cp k8s/secret-template.yaml "$TEMP_SECRET"
sed -i "s/REPLACE_USER/$DATABASE_USER/g" "$TEMP_SECRET"
sed -i "s/REPLACE_PASS/$DATABASE_PASSWORD/g" "$TEMP_SECRET"
sed -i "s/REPLACE_HOST/$DATABASE_HOST/g" "$TEMP_SECRET"
sed -i "s/REPLACE_DB/$DATABASE_NAME/g" "$TEMP_SECRET"
sed -i "s/REPLACE_JWT_SECRET/$JWT_SECRET/g" "$TEMP_SECRET"
sed -i "s/REPLACE_REFRESH_SECRET/$REFRESH_SECRET/g" "$TEMP_SECRET"
sed -i "s/REPLACE_SPICEDB_KEY/$SPICEDB_PRESHARED_KEY/g" "$TEMP_SECRET"
sed -i "s/REPLACE_METRICS_PASS/$METRICS_PASSWORD/g" "$TEMP_SECRET"

# Apply to Kubernetes
kubectl apply -f "$TEMP_SECRET" -n "$NAMESPACE"

echo "✅ Secrets created successfully in namespace: $NAMESPACE"
```

## Application-Level Secret Handling

### 1. Loading Secrets Safely

```rust
use std::env;
use secrecy::{ExposeSecret, Secret};

#[derive(Debug)]
pub struct DatabaseConfig {
    pub url: Secret<String>,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let url = env::var("DATABASE__URL")
            .map(Secret::new)
            .map_err(|_| ConfigError::MissingSecret("DATABASE__URL"))?;
            
        Ok(Self { url })
    }
    
    pub fn connect(&self) -> Result<DbConnection, DbError> {
        // Only expose secret when actually needed
        let conn = Database::connect(self.url.expose_secret())?;
        Ok(conn)
    }
}
```

### 2. Preventing Secret Logging

**Custom Debug Implementation:**
```rust
use std::fmt;

pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("server", &self.server)
            .field("database", &"<REDACTED>")
            .field("auth", &"<REDACTED>")
            .finish()
    }
}
```

**Structured Logging with Sanitization:**
```rust
use tracing::{info, instrument};

#[instrument(skip(password))]  // Skip sensitive parameters
pub async fn login(username: &str, password: &str) -> Result<Token, Error> {
    info!(username = %username, "User login attempt");
    // Never log the password!
    
    authenticate(username, password).await
}
```

### 3. Secret Validation

Always validate secrets on startup:

```rust
pub async fn validate_secrets() -> Result<(), StartupError> {
    // Check required environment variables
    let required_vars = [
        "DATABASE__URL",
        "AUTH__JWT_SECRET",
        "AUTH__REFRESH_SECRET",
    ];
    
    let mut missing = Vec::new();
    for var in &required_vars {
        if env::var(var).is_err() {
            missing.push(*var);
        }
    }
    
    if !missing.is_empty() {
        return Err(StartupError::MissingSecrets(missing));
    }
    
    // Validate format (without logging values!)
    let db_url = env::var("DATABASE__URL")?;
    if !db_url.starts_with("postgresql://") {
        return Err(StartupError::InvalidSecret("DATABASE__URL format"));
    }
    
    Ok(())
}
```

## Secret Rotation

### 1. Zero-Downtime Rotation Strategy

Support multiple valid secrets during rotation:

```rust
pub struct JwtValidator {
    current_secret: Secret<String>,
    previous_secret: Option<Secret<String>>,
}

impl JwtValidator {
    pub fn validate(&self, token: &str) -> Result<Claims, Error> {
        // Try current secret first
        if let Ok(claims) = validate_with_secret(token, &self.current_secret) {
            return Ok(claims);
        }
        
        // Fall back to previous secret during rotation
        if let Some(ref previous) = self.previous_secret {
            if let Ok(claims) = validate_with_secret(token, previous) {
                info!("Token validated with previous secret");
                return Ok(claims);
            }
        }
        
        Err(Error::InvalidToken)
    }
}
```

### 2. Kubernetes Secret Rotation

**Step-by-step rotation process:**

```bash
# 1. Create new secret version
kubectl create secret generic pcf-api-secrets-v2 \
    --from-literal=database-password='new$ecretPa$$' \
    --from-literal=jwt-secret='new-super-long-random-string'

# 2. Update deployment to use new secret
kubectl patch deployment pcf-api -p '
spec:
  template:
    spec:
      containers:
      - name: pcf-api
        envFrom:
        - secretRef:
            name: pcf-api-secrets-v2
'

# 3. Wait for rollout
kubectl rollout status deployment/pcf-api

# 4. Delete old secret (after verification)
kubectl delete secret pcf-api-secrets
```

### 3. Automated Rotation

Use CronJob for periodic rotation:

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: secret-rotation
spec:
  schedule: "0 2 1 * *"  # Monthly at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          serviceAccountName: secret-rotator
          containers:
          - name: rotator
            image: secret-rotator:latest
            command:
            - /bin/sh
            - -c
            - |
              # Generate new secrets
              NEW_JWT_SECRET=$(openssl rand -base64 32)
              
              # Create new secret
              kubectl create secret generic pcf-api-secrets-new \
                --from-literal=jwt-secret="$NEW_JWT_SECRET"
              
              # Trigger rollout
              kubectl set env deployment/pcf-api \
                ROTATION_TIMESTAMP="$(date +%s)"
```

## Development vs Production

### Development (.env files)

For local development, use .env files but NEVER commit them:

**.env.example** (commit this):
```bash
# Database
DATABASE_USER=pcf_user
DATABASE_PASSWORD=change_me_in_local_env
DATABASE_HOST=localhost
DATABASE_NAME=pcf_dev

# Auth
JWT_SECRET=change_me_use_long_random_string_in_production
REFRESH_SECRET=another_long_random_string

# External Services
SPICEDB_PRESHARED_KEY=local_dev_key
```

**.gitignore** (ensure these are ignored):
```
.env
.env.*
!.env.example
```

### Production Best Practices

1. **Use a Secret Management Service:**
   - Kubernetes Secrets (basic)
   - HashiCorp Vault (advanced)
   - AWS Secrets Manager
   - Azure Key Vault
   - Google Secret Manager

2. **Encrypt Secrets at Rest:**
   ```yaml
   # Enable encryption in Kubernetes
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

3. **Use RBAC to Limit Access:**
   ```yaml
   apiVersion: rbac.authorization.k8s.io/v1
   kind: Role
   metadata:
     name: secret-reader
   rules:
   - apiGroups: [""]
     resources: ["secrets"]
     verbs: ["get", "list"]
     resourceNames: ["pcf-api-secrets"]
   ```

## Secret Scanning and Prevention

### Pre-commit Hooks

Install git-secrets to prevent accidental commits:

```bash
# Install git-secrets
brew install git-secrets  # macOS
# or
git clone https://github.com/awslabs/git-secrets.git
cd git-secrets && make install

# Set up in your repo
git secrets --install
git secrets --register-aws  # AWS patterns
git secrets --register-gcp  # GCP patterns

# Add custom patterns
git secrets --add 'sk_live_[0-9a-zA-Z]{24}'  # Stripe keys
git secrets --add 'xox[baprs]-[0-9a-zA-Z]{10,48}'  # Slack tokens
```

### CI/CD Secret Scanning

Add to your pipeline:

```yaml
- name: Secret Scanning
  run: |
    # Using TruffleHog
    docker run --rm -v "$PWD:/repo" \
      trufflesecurity/trufflehog:latest \
      filesystem /repo --only-verified
    
    # Using detect-secrets
    pip install detect-secrets
    detect-secrets scan --all-files
```

## Emergency Response

### If Secrets Are Exposed:

1. **Immediate Actions:**
   ```bash
   # Rotate affected secrets immediately
   kubectl create secret generic pcf-api-secrets-emergency \
     --from-literal=compromised-key='new-value'
   
   # Force pod restart
   kubectl rollout restart deployment/pcf-api
   ```

2. **Audit Access:**
   ```bash
   # Check who accessed the secret
   kubectl get events --field-selector reason=SecretAccessed
   
   # Review audit logs
   kubectl logs -n kube-system -l component=kube-apiserver | grep secret
   ```

3. **Update Dependencies:**
   - Revoke old API keys
   - Update webhook URLs
   - Notify affected parties

## Testing Secret Management

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    
    #[test]
    #[serial]  // Prevent parallel env var conflicts
    fn test_missing_secret() {
        env::remove_var("DATABASE__URL");
        
        let result = DatabaseConfig::from_env();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::MissingSecret("DATABASE__URL")
        ));
    }
    
    #[test]
    #[serial]
    fn test_secret_not_logged() {
        env::set_var("DATABASE__URL", "postgresql://user:secret@host/db");
        
        let config = DatabaseConfig::from_env().unwrap();
        let debug_output = format!("{:?}", config);
        
        assert!(!debug_output.contains("secret"));
        assert!(debug_output.contains("REDACTED"));
    }
}
```

### Integration Tests

```bash
#!/bin/bash
# tests/secret-integration-test.sh

# Test secret creation
kubectl create secret generic test-secret \
    --from-literal=test-key=test-value \
    -n test

# Verify secret exists
kubectl get secret test-secret -n test

# Test pod can access secret
kubectl run test-pod --image=busybox \
    --env="TEST_KEY=\$(TEST_KEY)" \
    --overrides='
{
  "spec": {
    "containers": [{
      "name": "test-pod",
      "image": "busybox",
      "env": [{
        "name": "TEST_KEY",
        "valueFrom": {
          "secretKeyRef": {
            "name": "test-secret",
            "key": "test-key"
          }
        }
      }]
    }]
  }
}' \
    -- sh -c 'echo "Secret value: $TEST_KEY"'

# Cleanup
kubectl delete pod test-pod -n test
kubectl delete secret test-secret -n test
```

## Secret Management Checklist

Before deploying:

- [ ] No secrets in source code
- [ ] No secrets in container images  
- [ ] .env files in .gitignore
- [ ] Secrets loaded from environment
- [ ] Custom Debug implementations
- [ ] Log sanitization implemented
- [ ] Secret validation on startup
- [ ] Rotation strategy defined
- [ ] RBAC policies configured
- [ ] Pre-commit hooks installed
- [ ] CI/CD scanning enabled
- [ ] Emergency procedures documented

## Common Mistakes and Solutions

### 1. Secret in Error Messages
```rust
// ❌ BAD
Err(format!("Failed to connect to {}", database_url))

// ✅ GOOD
Err("Failed to connect to database".to_string())
```

### 2. Secrets in URLs
```rust
// ❌ BAD
let url = format!("https://api.service.com?key={}", api_key);

// ✅ GOOD - Use headers
let client = reqwest::Client::new();
let response = client.get("https://api.service.com")
    .header("Authorization", format!("Bearer {}", api_key))
    .send()
    .await?;
```

### 3. Weak Secret Generation
```bash
# ❌ BAD - Predictable
JWT_SECRET=my-app-secret-2024

# ✅ GOOD - Cryptographically random
JWT_SECRET=$(openssl rand -base64 64)
```

## Next Steps

1. Implement secret validation in your app
2. Set up Kubernetes secrets
3. Configure secret rotation
4. Add pre-commit hooks
5. Document secret management procedures

## Additional Resources

- [Kubernetes Secrets Documentation](https://kubernetes.io/docs/concepts/configuration/secret/)
- [OWASP Secrets Management](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
- [12 Factor App - Config](https://12factor.net/config)
- [HashiCorp Vault Tutorials](https://learn.hashicorp.com/vault)