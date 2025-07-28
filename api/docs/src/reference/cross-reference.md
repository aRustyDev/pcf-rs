# Cross-Reference Matrix

This matrix helps you find relevant documentation based on your role and the topic you're interested in.

## Documentation by Audience and Topic

| Topic | Developers | Administrators | API Users |
|-------|------------|----------------|-----------|
| **Getting Started** | [Development Setup](/developer/quickstart/development.md) | [Quick Start Guide](/admin/quickstart/README.md) | [First Request](/user/quickstart/README.md) |
| **Configuration** | [Config Module](/developer/modules/configuration/README.md) | [Configuration Guide](/admin/configuration/README.md) | - |
| **GraphQL** | [GraphQL Module](/developer/modules/graphql/README.md) | [GraphQL Deployment](/admin/deployment/README.md) | [GraphQL API](/user/api/graphql/README.md) |
| **REST API** | [REST Implementation](/developer/api/rest.md) | [REST Configuration](/admin/configuration/README.md) | [REST Reference](/user/api/rest/README.md) |
| **Security** | [Security Implementation](/developer/security/README.md) | [Security Hardening](/admin/security/README.md) | [Authentication](/user/api/authentication.md) |
| **Performance** | [Performance Guide](/developer/performance/README.md) | [Performance Tuning](/admin/performance/README.md) | [Rate Limiting](/user/api/rate-limiting/README.md) |
| **Monitoring** | [Observability](/developer/observability/README.md) | [Monitoring Setup](/admin/observability/README.md) | - |
| **Troubleshooting** | [Debug Guide](/developer/troubleshooting/README.md) | [Admin Troubleshooting](/admin/troubleshooting/README.md) | [User Troubleshooting](/user/troubleshooting/README.md) |
| **Deployment** | [Docker Development](/developer/quickstart/docker.md) | [Production Deployment](/admin/deployment/README.md) | - |
| **Testing** | [Testing Guide](/developer/contributing/testing/README.md) | [Health Checks](/admin/observability/healthchecks.md) | - |

## Common Tasks by Role

### For Developers

| Task | Primary Documentation | Related Topics |
|------|----------------------|----------------|
| Set up development environment | [Development Setup](/developer/quickstart/development.md) | [Docker Setup](/developer/quickstart/docker.md), [Configuration](/developer/modules/configuration/README.md) |
| Add new GraphQL query | [GraphQL Module](/developer/modules/graphql/README.md) | [Schema Design](/developer/schema/graphql/README.md), [Testing](/developer/contributing/testing/README.md) |
| Implement authentication | [Security Module](/developer/security/authentication.md) | [Authorization](/developer/security/authorization.md), [JWT Handling](/developer/security/README.md) |
| Debug performance issue | [Performance Guide](/developer/performance/README.md) | [Observability](/developer/observability/README.md), [Benchmarks](/reference/benchmarks/README.md) |
| Add new module | [Module Structure](/developer/modules/README.md) | [Architecture](/developer/architecture/README.md), [Contributing](/developer/contributing/README.md) |

### For Administrators

| Task | Primary Documentation | Related Topics |
|------|----------------------|----------------|
| Deploy to production | [Deployment Guide](/admin/deployment/README.md) | [Kubernetes](/admin/deployment/kubernetes.md), [Docker](/admin/deployment/docker.md) |
| Configure monitoring | [Observability Setup](/admin/observability/README.md) | [Metrics](/admin/observability/metrics.md), [Alerting](/admin/observability/alerting.md) |
| Secure the API | [Security Hardening](/admin/security/hardening.md) | [TLS Configuration](/admin/security/tls.md), [Network Security](/admin/security/network.md) |
| Scale the system | [Scaling Guide](/admin/cookbook/scaling.md) | [Performance Tips](/admin/performance/tips.md), [Load Testing](/reference/benchmarks/README.md) |
| Backup and restore | [Backup Guide](/admin/cookbook/backup.md) | [Disaster Recovery](/admin/cookbook/disaster-recovery.md), [Database Config](/admin/configuration/database.md) |

### For API Users

| Task | Primary Documentation | Related Topics |
|------|----------------------|----------------|
| Make first API call | [Quick Start](/user/quickstart/README.md) | [Authentication](/user/api/authentication.md), [Examples](/user/cookbook/README.md) |
| Use GraphQL API | [GraphQL Guide](/user/api/graphql/README.md) | [Playground](/user/api/graphql/playground.md), [Schema Reference](/developer/schema/graphql/reference.md) |
| Handle errors | [Error Reference](/user/api/errors/README.md) | [Retry Strategies](/user/api/errors/retry.md), [Error Codes](/user/api/errors/codes.md) |
| Implement webhooks | [Webhooks Guide](/user/cookbook/webhooks.md) | [Real-time Updates](/user/cookbook/realtime.md), [Authentication](/user/api/authentication.md) |
| Optimize requests | [Batch Operations](/user/cookbook/batch.md) | [Rate Limiting](/user/api/rate-limiting/README.md), [Performance Tips](/reference/benchmarks/graphql-performance.md) |

## Module Documentation Map

| Module | Developer Docs | Admin Docs | User Docs |
|--------|---------------|------------|-----------|
| Configuration | [Module Docs](/developer/modules/configuration/README.md) | [Config Guide](/admin/configuration/README.md) | - |
| GraphQL | [Module Docs](/developer/modules/graphql/README.md) | [GraphQL Setup](/admin/deployment/README.md) | [API Reference](/user/api/graphql/README.md) |
| Errors | [Module Docs](/developer/modules/errors/README.md) | [Error Monitoring](/admin/observability/alerting.md) | [Error Handling](/user/api/errors/README.md) |
| Middleware | [Module Docs](/developer/modules/middleware/README.md) | [Middleware Config](/admin/configuration/README.md) | - |
| Services | [Module Docs](/developer/modules/services/README.md) | [Service Management](/admin/deployment/README.md) | - |

## Technology-Specific Guides

| Technology | Overview | Implementation | Operations | Usage |
|------------|----------|----------------|------------|-------|
| GraphQL | [Schema Design](/developer/schema/graphql/README.md) | [GraphQL Module](/developer/modules/graphql/README.md) | [GraphQL Monitoring](/admin/observability/metrics.md) | [GraphQL API](/user/api/graphql/README.md) |
| REST | [API Design](/developer/api/README.md) | [REST Implementation](/developer/api/rest.md) | [REST Configuration](/admin/configuration/README.md) | [REST Reference](/user/api/rest/README.md) |
| WebSockets | [Real-time Design](/developer/architecture/README.md) | [Subscription Implementation](/developer/api/graphql/subscriptions.md) | [WebSocket Monitoring](/admin/observability/README.md) | [Real-time Guide](/user/cookbook/realtime.md) |
| Docker | [Docker Development](/developer/quickstart/docker.md) | [Dockerfile](/developer/contributing/README.md) | [Docker Deployment](/admin/deployment/docker.md) | - |
| Kubernetes | [K8s Architecture](/admin/deployment/kubernetes-architecture.md) | [Helm Charts](/admin/deployment/kubernetes.md) | [K8s Operations](/admin/cookbook/scaling.md) | - |

## Quick Links by Use Case

### "I want to..."

#### Develop
- [Set up my development environment](/developer/quickstart/development.md)
- [Understand the architecture](/developer/architecture/README.md)
- [Add a new feature](/developer/contributing/README.md)
- [Write tests](/developer/contributing/testing/README.md)
- [Debug an issue](/developer/troubleshooting/README.md)

#### Deploy
- [Deploy with Docker](/admin/deployment/docker.md)
- [Deploy to Kubernetes](/admin/deployment/kubernetes.md)
- [Configure for production](/admin/configuration/README.md)
- [Set up monitoring](/admin/observability/README.md)
- [Secure the deployment](/admin/security/README.md)

#### Use the API
- [Get started quickly](/user/quickstart/README.md)
- [Authenticate requests](/user/api/authentication.md)
- [Query with GraphQL](/user/api/graphql/README.md)
- [Handle errors gracefully](/user/api/errors/README.md)
- [Implement real-time features](/user/cookbook/realtime.md)