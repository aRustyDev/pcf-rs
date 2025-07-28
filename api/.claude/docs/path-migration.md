# Documentation Path Migration Reference

This document maps all old documentation paths to their new locations after the reorganization.

## Admin Section (formerly administrators/)

### Overview & Quick Start
- `/administrators/overview.md` → `/admin/README.md`
- `/administrators/quick-start.md` → `/admin/quickstart/README.md`
- `/quick-start/administrators.md` → `/admin/quickstart/README.md`

### Configuration
- `/administrators/configuration/index.md` → `/admin/configuration/README.md`
- `/administrators/configuration/environment.md` → `/admin/configuration/environment.md`
- `/administrators/configuration/files.md` → `/admin/configuration/application.md`
- `/administrators/configuration/secrets.md` → `/admin/configuration/secrets.md`
- `/administrators/configuration/features.md` → `/admin/configuration/feature-flags.md`

### Deployment
- `/administrators/deployment/index.md` → `/admin/deployment/README.md`
- `/administrators/deployment/docker.md` → `/admin/deployment/docker.md`
- `/administrators/deployment/kubernetes.md` → `/admin/deployment/kubernetes.md`
- `/administrators/deployment/cloud.md` → `/admin/deployment/cloud.old.md`
- `/administrators/deployment/architecture.md` → `/admin/architecture/README.md`

### Monitoring → Observability
- `/administrators/monitoring/index.md` → `/admin/observability/README.md`
- `/administrators/monitoring/metrics.md` → `/admin/observability/metrics.md`
- `/administrators/monitoring/logging.md` → `/admin/observability/logging.md`
- `/administrators/monitoring/tracing.md` → `/admin/observability/tracing.md`
- `/administrators/monitoring/health-checks.md` → `/admin/observability/healthchecks.md`
- `/administrators/monitoring/alerting.md` → `/admin/observability/alerting.md`

### Security
- `/administrators/security/index.md` → `/admin/security/README.md`
- `/administrators/security/hardening.md` → `/admin/security/hardening.md`
- `/administrators/security/tls.md` → `/admin/security/tls.old.md`
- `/administrators/security/network.md` → `/admin/security/network.old.md`
- `/administrators/security/audit.md` → `/admin/security/audit.old.md`

### Troubleshooting
- `/administrators/troubleshooting/common-issues.md` → `/admin/troubleshooting/README.md`
- `/administrators/troubleshooting/performance.md` → `/admin/performance/tips.md`
- `/administrators/troubleshooting/connections.md` → `/admin/troubleshooting/connections.old.md`
- `/administrators/troubleshooting/memory.md` → `/admin/troubleshooting/memory.old.md`
- `/administrators/troubleshooting/debugging.md` → `/admin/troubleshooting/debugging.old.md`

### Cookbook
- `/administrators/cookbook/backup.md` → `/admin/cookbook/backup.md`
- `/administrators/cookbook/scaling.md` → `/admin/cookbook/scaling.md`
- `/administrators/cookbook/updates.md` → `/admin/cookbook/updates.md`
- `/administrators/cookbook/disaster-recovery.md` → `/admin/cookbook/disaster-recovery.md`

## Developer Section (formerly developers/)

### Overview & Quick Start
- `/developers/overview.md` → `/developer/README.md`
- `/quick-start/developers.md` → `/developer/quickstart/docker.md`

### Architecture
- `/developers/architecture/system-overview.md` → `/developer/architecture/README.md`
- `/developers/architecture/request-flow.md` → `/developer/architecture/request-flow.old.md`
- `/developers/architecture/design-patterns.md` → `/developer/architecture/design-patterns.old.md`
- `/developers/architecture/diagrams.md` → `/developer/architecture/diagrams.old.md`

### Dependencies
- `/developers/dependencies/index.md` → `/developer/architecture/dependencies.md`
- `/developers/dependencies/core.md` → `/developer/architecture/core-dependencies.old.md`
- `/developers/dependencies/dev.md` → `/developer/architecture/dev-dependencies.old.md`
- `/developers/dependencies/analysis.md` → `/developer/architecture/dependency-analysis.old.md`

### API Reference → API
- `/developers/api-reference/index.md` → `/developer/api/README.md`
- `/developers/api-reference/types.md` → `/developer/api/types.old.md`
- `/developers/api-reference/traits.md` → `/developer/api/traits.old.md`
- `/developers/api-reference/functions.md` → `/developer/api/functions.old.md`

### GraphQL
- `/developers/graphql/schema.md` → `/developer/schema/graphql/schema.md`
- `/developers/graphql/queries.md` → `/developer/api/graphql/queries.md`
- `/developers/graphql/mutations.md` → `/developer/api/graphql/mutations.md`
- `/developers/graphql/subscriptions.md` → `/developer/api/graphql/subscriptions.md`
- `/developers/graphql/best-practices.md` → `/developer/api/graphql/README.md`

### Contributing
- `/developers/contributing/getting-started.md` → `/developer/contributing/README.md`
- `/developers/contributing/setup.md` → `/developer/quickstart/development.md`
- `/developers/contributing/code-style.md` → `/developer/contributing/code-style.md`
- `/developers/contributing/testing.md` → `/developer/contributing/testing/README.md`
- `/developers/contributing/documentation.md` → `/developer/contributing/documentation.md`

### Testing
- `/developers/testing/strategy.md` → `/developer/contributing/testing/strategy.md`
- `/developers/testing/unit-tests.md` → `/developer/contributing/testing/unit.md`
- `/developers/testing/integration-tests.md` → `/developer/contributing/testing/integration.md`
- `/developers/testing/performance-tests.md` → `/developer/contributing/testing/benchmarks.md`

### Modules
- `/developers/modules/config/index.md` → `/developer/modules/configuration/README.md`
- `/developers/modules/graphql/index.md` → `/developer/modules/graphql/README.md`
- `/developers/modules/error/index.md` → `/developer/modules/errors/README.md`
- `/developers/modules/server/index.md` → `/developer/modules/middleware/README.md`
- `/developers/modules/services/index.md` → `/developer/modules/services/README.md`
- `/developers/modules/health/index.md` → `/developer/modules/health/README.md`
- `/developers/modules/schema/index.md` → `/developer/modules/schema/README.md`
- `/developers/modules/logging/index.md` → `/developer/modules/logging/README.md`

### Cookbook
- `/developers/cookbook/performance.md` → `/developer/cookbook/performance.md`
- `/developers/cookbook/patterns.md` → `/developer/cookbook/patterns.md`
- `/developers/cookbook/debugging.md` → `/developer/cookbook/debugging.md`

## User Section (formerly users/)

### Overview & Quick Start
- `/users/overview.md` → `/user/README.md`
- `/users/quick-start.md` → `/user/quickstart/README.md`
- `/quick-start/users.md` → `/user/quickstart/getting-started.old.md`
- `/users/first-request.md` → `/user/quickstart/first-request.old.md`

### API Endpoints
- `/users/api-endpoints/rest.md` → `/user/api/rest/README.md`
- `/users/api-endpoints/graphql.md` → `/user/api/graphql/README.md`
- `/users/api-endpoints/websockets.md` → `/user/api/websockets.old.md`

### Authentication
- `/users/authentication/index.md` → `/user/api/authentication.md`

### GraphQL
- `/users/graphql/queries.md` → `/user/api/graphql/queries.old.md`
- `/users/graphql/mutations.md` → `/user/api/graphql/mutations.old.md`
- `/users/graphql/subscriptions.md` → `/user/api/graphql/subscriptions.old.md`
- `/users/graphql/pagination.md` → `/user/api/graphql/pagination.old.md`
- `/users/graphql/errors.md` → `/user/api/graphql/errors.old.md`

### Troubleshooting
- `/users/troubleshooting/common-issues.md` → `/user/troubleshooting/README.md`
- `/users/troubleshooting/auth.md` → `/user/troubleshooting/auth.old.md`
- `/users/troubleshooting/connections.md` → `/user/troubleshooting/connections.old.md`

### Examples
- `/users/examples/go.md` → `/user/cookbook/examples-go.old.md`
- `/users/examples/python.md` → `/user/cookbook/examples-python.old.md`
- `/users/examples/rust.md` → `/user/cookbook/examples-rust.old.md`
- `/users/examples/javascript.md` → `/user/cookbook/examples-javascript.old.md`

### Errors
- `/users/errors/retry.md` → `/user/api/errors/retry.old.md`
- `/users/errors/format.md` → `/user/api/errors/format.old.md`
- `/users/errors/codes.md` → `/user/api/errors/codes.old.md`

### Rate Limiting
- `/users/rate-limiting/index.md` → `/user/api/rate-limiting/README.md`
- `/users/rate-limiting/best-practices.md` → `/user/api/rate-limiting/best-practices.old.md`
- `/users/rate-limiting/limits.md` → `/user/api/rate-limiting/limits.old.md`

### Cookbook
- `/users/cookbook/batch.md` → `/user/cookbook/batch.md`
- `/users/cookbook/webhooks.md` → `/user/cookbook/webhooks.md`
- `/users/cookbook/realtime.md` → `/user/cookbook/realtime.md`

## Shared Content (distributed)

### Patterns
- `/shared/patterns/overview.md` → `/developer/architecture/patterns.old.md`
- `/shared/patterns/configuration.md` → `/developer/modules/configuration/patterns.old.md`
- `/shared/patterns/error-handling.md` → `/developer/modules/errors/patterns.old.md`

### Standards
- `/shared/standards/api.md` → `/developer/contributing/api-standards.old.md`
- `/shared/standards/documentation.md` → `/developer/contributing/documentation-standards.old.md`
- `/shared/standards/code.md` → `/developer/contributing/code-standards.old.md`

### Security
- `/shared/security/principles.md` → `/developer/security/principles.old.md`
- `/shared/security/threat-model.md` → `/developer/security/threat-model.old.md`
- `/shared/security/best-practices.md` → `/developer/security/best-practices.old.md`
- `/shared/security-standards.md` → `/developer/security/standards.old.md`

### Other
- `/shared/design-patterns.md` → `/developer/architecture/design-patterns.old.md`
- `/shared/glossary.md` → `/reference/glossary.md`
- `/shared/lessons-learned.md` → `/developer/architecture/lessons-learned.old.md`

## Appendices → Reference

### Migrations
- `/appendices/migrations/api.md` → `/reference/migrations/api.old.md`
- `/appendices/migrations/versions.md` → `/reference/migrations/versions.old.md`
- `/appendices/migrations/database.md` → `/reference/migrations/database.old.md`

### Deprecated
- `/appendices/deprecated/index.md` → `/reference/deprecated/README.md`
- `/appendices/deprecated/migration.md` → `/reference/deprecated/migration.old.md`

### Other
- `/appendices/licenses.md` → `/reference/licenses.md`
- `/appendices/third-party.md` → `/reference/third-party.md`

## Root Level Files
- `/introduction.md` → `/README.md`

## Key Changes Summary

1. **Directory Renaming**:
   - `administrators` → `admin`
   - `developers` → `developer`
   - `users` → `user`

2. **File Naming**:
   - All `index.md` → `README.md`

3. **Content Reorganization**:
   - `monitoring` → `observability`
   - `api-reference` → `api`
   - `shared` content distributed to relevant sections
   - `appendices` → `reference`

4. **Files Marked for Review**:
   - 50 files marked with `.old.md` extension need content review and integration