# Cert-Manager Setup for PCF

This document outlines the configuration required for cert-manager to work with PCF, particularly for Linkerd certificate management.

## Prerequisites

1. Cert-manager must be installed with the experimental certificate signing request controllers enabled (configured in values.yaml)
2. IPv6 support in your cluster (handled by Cilium CNI)

## Linkerd Certificate Setup

Linkerd requires a trust anchor certificate and an issuer certificate. Since we're using cert-manager as the external CA, follow these steps:

### 1. Create Trust Anchor Certificate

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: linkerd-identity-trust-anchor
  namespace: cert-manager
spec:
  secretName: linkerd-identity-trust-anchor
  duration: 8760h # 365 days
  renewBefore: 720h # 30 days
  subject:
    organizations:
      - "linkerd.io"
  commonName: "root.linkerd.cluster.local"
  isCA: true
  issuerRef:
    name: selfsigned-issuer
    kind: ClusterIssuer
```

### 2. Create Self-Signed Issuer

```yaml
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: selfsigned-issuer
spec:
  selfSigned: {}
```

### 3. Create Linkerd Issuer Certificate

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: linkerd-identity-issuer
  namespace: linkerd
spec:
  secretName: linkerd-identity-issuer
  duration: 2160h # 90 days
  renewBefore: 240h # 10 days
  subject:
    organizations:
      - "linkerd.io"
  commonName: "identity.linkerd.cluster.local"
  isCA: true
  privateKey:
    algorithm: ECDSA
  issuerRef:
    name: linkerd-trust-anchor-issuer
    kind: ClusterIssuer
```

### 4. Create Trust Anchor Issuer

```yaml
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: linkerd-trust-anchor-issuer
spec:
  ca:
    secretName: linkerd-identity-trust-anchor
```

## Application Certificate Management

For PCF services that need TLS certificates:

### 1. Create a Certificate for Each Service

Example for the API service:

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: api-tls
  namespace: pcf
spec:
  secretName: api-tls
  duration: 720h # 30 days
  renewBefore: 240h # 10 days
  subject:
    organizations:
      - "pcf"
  dnsNames:
    - api.pcf.local
    - api.pcf.svc.cluster.local
  ipAddresses:
    - "::1" # IPv6 loopback
  issuerRef:
    name: pcf-issuer
    kind: Issuer
```

### 2. Create PCF Issuer

```yaml
apiVersion: cert-manager.io/v1
kind: Issuer
metadata:
  name: pcf-issuer
  namespace: pcf
spec:
  ca:
    secretName: pcf-ca-secret
```

## IPv6 Considerations

Since Cilium is configured for IPv6-only:

1. All certificates should include IPv6 addresses in the SAN fields
2. Use IPv6 addresses for internal service communication
3. Example IPv6 addresses for certificates:
   - `::1` - IPv6 loopback
   - `fd00::/8` - Unique local addresses (ULA)
   - Service ClusterIPs will be assigned from the IPv6 CIDR range

## Deployment Order

1. Deploy cert-manager with the PCF Helm chart dependencies
2. Apply the ClusterIssuers and root certificates
3. Deploy Linkerd with the external CA configuration
4. Deploy the PCF services

## Verification

Check certificate status:

```bash
# List all certificates
kubectl get certificates -A

# Check certificate details
kubectl describe certificate linkerd-identity-issuer -n linkerd

# Verify cert-manager is working
kubectl get clusterissuers
kubectl get issuers -A
```

## Troubleshooting

1. **Certificate not issuing**: Check cert-manager logs
   ```bash
   kubectl logs -n cert-manager deploy/cert-manager
   ```

2. **Linkerd identity issues**: Verify the trust anchor
   ```bash
   kubectl get secret linkerd-identity-trust-anchor -n cert-manager -o yaml
   ```

3. **IPv6 connectivity**: Ensure Cilium is properly configured
   ```bash
   kubectl get ciliumnode -o wide
   ```

## Production Recommendations

1. Use a proper CA issuer (not self-signed) for production
2. Consider using Let's Encrypt for public-facing services
3. Implement certificate rotation policies
4. Monitor certificate expiration with Prometheus alerts
5. Use separate issuers for different environments
6. Enable ACME DNS01 challenge for wildcard certificates if needed