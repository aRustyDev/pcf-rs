# External Secrets Requirements

This document lists all the secrets that must be configured in Vault for the PCF Helm chart to function properly.

## Vault Setup

The PCF Helm chart expects secrets to be stored in Vault under the following path structure:
- Base path: `secret/data/pcf/` (for KV v2 engine)

## Required Secrets

### 1. Grafana Admin Credentials
**Path:** `pcf/grafana`
**Keys:**
- `admin-password`: Password for the Grafana admin user

**Example Vault command:**
```bash
vault kv put secret/pcf/grafana \
  admin-password="your-secure-password"
```

### 2. SpiceDB Pre-shared Key
**Path:** `pcf/spicedb`
**Keys:**
- `preshared-key`: Pre-shared key for SpiceDB gRPC authentication (minimum 32 characters)

**Example Vault command:**
```bash
vault kv put secret/pcf/spicedb \
  preshared-key="$(openssl rand -hex 32)"
```

### 3. SurrealDB Credentials
**Path:** `pcf/surrealdb`
**Keys:**
- `username`: Database admin username
- `password`: Database admin password

**Example Vault command:**
```bash
vault kv put secret/pcf/surrealdb \
  username="admin" \
  password="$(openssl rand -base64 32)"
```

### 4. ORY Hydra Secrets
**Path:** `pcf/hydra`
**Keys:**
- `system-secret`: System secret for Hydra (minimum 16 characters)
- `cookie-secret`: Cookie secret for Hydra (exactly 32 characters)

**Example Vault command:**
```bash
vault kv put secret/pcf/hydra \
  system-secret="$(openssl rand -hex 16)" \
  cookie-secret="$(openssl rand -hex 16)"
```

### 5. ORY Kratos Secrets
**Path:** `pcf/kratos`
**Keys:**
- `secrets-default`: Default secrets for Kratos (32 characters)
- `secrets-cookie`: Cookie secrets for Kratos (32 characters)
- `secrets-cipher`: Cipher secrets for Kratos (32 characters)

**Example Vault command:**
```bash
vault kv put secret/pcf/kratos \
  secrets-default="$(openssl rand -hex 16)" \
  secrets-cookie="$(openssl rand -hex 16)" \
  secrets-cipher="$(openssl rand -hex 16)"
```

### 6. API Service Secrets
**Path:** `pcf/api`
**Keys:**
- `jwt-secret`: JWT signing secret for API authentication
- `api-key`: Internal API key for service-to-service communication

**Example Vault command:**
```bash
vault kv put secret/pcf/api \
  jwt-secret="$(openssl rand -base64 64)" \
  api-key="$(openssl rand -hex 32)"
```

## Kubernetes Authentication Setup

The PCF Helm chart uses Kubernetes authentication to access Vault. You need to:

1. Enable Kubernetes auth in Vault:
```bash
vault auth enable kubernetes
```

2. Configure the Kubernetes auth method:
```bash
vault write auth/kubernetes/config \
    kubernetes_host="https://$KUBERNETES_HOST:443" \
    kubernetes_ca_cert=@ca.crt
```

3. Create a policy for PCF:
```bash
vault policy write pcf-policy - <<EOF
path "secret/data/pcf/*" {
  capabilities = ["read", "list"]
}
EOF
```

4. Create a role for PCF:
```bash
vault write auth/kubernetes/role/pcf-role \
    bound_service_account_names=pcf-vault-auth \
    bound_service_account_namespaces=pcf \
    policies=pcf-policy \
    ttl=24h
```

## Service Account Setup

The chart will create the necessary service account (`pcf-vault-auth`) automatically. This service account will be used by External Secrets Operator to authenticate with Vault.

## Verification

After setting up all secrets, you can verify they're accessible:

```bash
# List all secrets under pcf path
vault kv list secret/pcf

# Read a specific secret (example)
vault kv get secret/pcf/grafana
```

## Security Best Practices

1. Use strong, randomly generated passwords and keys
2. Rotate secrets regularly
3. Limit access to the Vault path using appropriate policies
4. Enable audit logging in Vault
5. Use separate Vault namespaces for different environments (dev, staging, prod)
6. Never commit secrets to version control
7. Use Vault's transit engine for encryption operations when possible