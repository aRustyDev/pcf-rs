# PCF Helm Chart

This Helm chart deploys the Personal Cloud Framework (PCF) with a complete observability stack, identity management, and authorization services.

## Architecture Overview

The PCF helm chart includes:
- **Traefik**: Ingress controller for routing
- **Linkerd**: Service mesh for secure service-to-service communication
- **Cilium**: CNI with IPv6-only support
- **Cert-Manager**: Certificate management for TLS
- **External Secrets**: Secret management with Vault integration
- **ORY Kratos & Hydra**: Identity and OAuth2/OIDC services
- **SpiceDB**: Authorization engine
- **SurrealDB**: Multi-model database
- **Grafana, Loki, Tempo**: Observability stack
- **OpenTelemetry Collector**: Telemetry collection

## Prerequisites

1. Kubernetes cluster 1.25+
2. Helm 3.10+
3. IPv6 support in your cluster
4. Vault instance for secret management
5. kubectl configured to access your cluster

## Pre-Installation Setup

### 1. Setup Vault

Follow the instructions in [templates/external-secrets/SECRETS_REQUIREMENTS.md](templates/external-secrets/SECRETS_REQUIREMENTS.md) to:
- Configure Vault with required secrets
- Setup Kubernetes authentication
- Create necessary policies and roles

### 2. Install CRDs

Some dependencies require CRDs to be installed first:

```bash
# Install cert-manager CRDs
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.3/cert-manager.crds.yaml

# Install Linkerd CRDs
linkerd install --crds | kubectl apply -f -

# External Secrets CRDs are installed by the chart
```

## Installation

### 1. Add Helm Repositories

```bash
# Add all required repositories
helm repo add traefik https://traefik.github.io/charts
helm repo add grafana https://grafana.github.io/helm-charts
helm repo add linkerd https://helm.linkerd.io/stable
helm repo add jetstack https://charts.jetstack.io
helm repo add external-secrets https://charts.external-secrets.io
helm repo add cilium https://helm.cilium.io/
helm repo update
```

### 2. Configure Values

Create a custom values file `my-values.yaml`:

```yaml
global:
  domain: "your-domain.com"  # Change this to your domain
  storageClass: "your-storage-class"  # Change to your storage class

vaultBackendStore:
  server: "https://your-vault-server:8200"  # Your Vault server
  auth:
    kubernetes:
      role: "pcf-role"  # Must match your Vault configuration

# Disable components you don't need
# cilium:
#   enabled: false  # If you already have a CNI
# linkerd2:
#   enabled: false  # If you already have Linkerd installed
# cert-manager:
#   enabled: false  # If you already have cert-manager
```

### 3. Install the Chart

```bash
# Update dependencies
helm dependency update ./pcf-chart

# Install with custom values
helm install pcf ./pcf-chart \
  --namespace pcf \
  --create-namespace \
  -f my-values.yaml
```

## Post-Installation Setup

### 1. Setup Cert-Manager for Linkerd

Follow the instructions in [CERT_MANAGER_SETUP.md](CERT_MANAGER_SETUP.md) to configure certificates for Linkerd.

### 2. Verify Installation

```bash
# Check all pods are running
kubectl get pods -n pcf

# Check ingress routes
kubectl get ingress -n pcf

# Check external secrets are synced
kubectl get externalsecrets -n pcf
```

### 3. Access Services

With default configuration, services are available at:
- Traefik Dashboard: `http://traefik.pcf.local`
- Grafana: `http://grafana.pcf.local` (admin/[from-vault])
- API: `http://api.pcf.local`
- Kratos: `http://kratos.pcf.local`
- Hydra: `http://hydra.pcf.local`
- SpiceDB: `http://spicedb.pcf.local`
- SurrealDB: `http://surrealdb.pcf.local`

## Configuration

### Resource Limits

All services have configurable resource limits in `values.yaml`. Default limits match the docker-compose configuration.

### Persistence

Persistent volumes are created for:
- SurrealDB data
- Grafana data and configuration
- Loki data
- Tempo traces

### IPv6 Configuration

The chart is configured for IPv6-only operation:
- Cilium CNI with IPv6 enabled, IPv4 disabled
- No iptables rules
- Service addresses use IPv6

### Service Mesh

All services are automatically injected with Linkerd sidecars for:
- mTLS between services
- Traffic observability
- Circuit breaking and retries

## Upgrading

```bash
# Update dependencies
helm dependency update ./pcf-chart

# Upgrade release
helm upgrade pcf ./pcf-chart \
  --namespace pcf \
  -f my-values.yaml
```

## Uninstallation

```bash
# Delete the release
helm uninstall pcf -n pcf

# Delete the namespace (optional)
kubectl delete namespace pcf

# Clean up CRDs (careful - this affects other installations)
# kubectl delete -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.3/cert-manager.crds.yaml
```

## Troubleshooting

### Pods Not Starting

1. Check external secrets are synced:
   ```bash
   kubectl get externalsecrets -n pcf
   kubectl describe externalsecret <name> -n pcf
   ```

2. Check Vault connectivity:
   ```bash
   kubectl logs -n pcf deployment/external-secrets
   ```

### IPv6 Connectivity Issues

1. Verify Cilium status:
   ```bash
   kubectl get ciliumnode -o wide
   cilium status
   ```

2. Check node IPv6 configuration:
   ```bash
   kubectl get nodes -o wide
   ```

### Certificate Issues

1. Check cert-manager logs:
   ```bash
   kubectl logs -n cert-manager deployment/cert-manager
   ```

2. Verify certificates:
   ```bash
   kubectl get certificates -A
   kubectl describe certificate <name> -n pcf
   ```

### Service Mesh Issues

1. Check Linkerd status:
   ```bash
   linkerd check
   ```

2. Verify pod injection:
   ```bash
   kubectl get pods -n pcf -o yaml | grep "linkerd.io/inject"
   ```

## Development

### Adding New Services

1. Create templates in `templates/<service-name>/`
2. Add configuration to `values.yaml`
3. Update dependencies in `Chart.yaml` if needed
4. Add external secrets configuration if required

### Testing

```bash
# Lint the chart
helm lint ./pcf-chart

# Dry run installation
helm install pcf ./pcf-chart --dry-run --debug

# Template rendering
helm template pcf ./pcf-chart
```

## Security Considerations

1. **Secrets**: All sensitive data is managed through External Secrets and Vault
2. **Network Policies**: Consider implementing network policies for pod-to-pod communication
3. **RBAC**: Service accounts are created with minimal permissions
4. **TLS**: All internal communication uses mTLS via Linkerd
5. **IPv6**: IPv6-only configuration reduces attack surface

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

[Your License Here]

## Support

For issues and questions:
- GitHub Issues: [your-repo-url]
- Documentation: [your-docs-url]