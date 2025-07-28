# PCF API Specifications

This directory contains detailed specifications for various aspects of the PCF API system.

## Specification Documents

### Core Functionality
- **[Error Handling](./error-handling.md)** - Error categories, response formats, and handling patterns
- **[Configuration](./configuration.md)** - Configuration management with Figment, precedence rules
- **[Health Checks](./health-checks.md)** - Liveness and readiness endpoints, degraded mode operations
- **[GraphQL Schema](./graphql-schema.md)** - Schema design, type system, and best practices

### Operations & Monitoring
- **[Authorization](./authorization.md)** - SpiceDB integration, caching strategy, permission patterns
- **[Metrics](./metrics.md)** - Prometheus metrics, cardinality control, monitoring strategy
- **[Logging](./logging.md)** - Structured logging, trace correlation, log levels

### Development & Quality
- **[Testing Strategy](./testing-strategy.md)** - Test organization, coverage requirements, patterns
- **[MDBook Documentation](./mdbook-documentation.md)** - Comprehensive documentation system specification

## Quick Reference

### Error Response Format
```json
{
  "error": {
    "code": "INVALID_INPUT",
    "message": "Email address is not valid",
    "trace_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

### Configuration Precedence
1. Default values (in code)
2. Configuration files (TOML/JSON/YAML)
3. Environment variables
4. CLI arguments

### Health Check Endpoints
- `/health` - Simple liveness check
- `/health/ready` - Comprehensive readiness check with service statuses

### Metrics Endpoint
- `/metrics` - Prometheus-compatible metrics

## Implementation Status

| Specification | Status | Notes |
|--------------|--------|-------|
| Error Handling | ✅ Implemented | Full error taxonomy |
| Configuration | ✅ Implemented | Figment integration |
| Health Checks | ✅ Implemented | Both endpoints active |
| GraphQL Schema | 🚧 In Progress | Core types defined |
| Authorization | 🚧 In Progress | SpiceDB integration pending |
| Metrics | ✅ Implemented | Prometheus metrics |
| Logging | ✅ Implemented | Tracing integration |
| Testing Strategy | ✅ Implemented | 80% coverage achieved |
| MDBook Documentation | 📋 Planned | Specification complete |

## Adding New Specifications

When adding a new specification:
1. Create a new `.md` file in this directory
2. Follow the existing format with clear sections
3. Include code examples where applicable
4. Update this README with a link and description
5. Update the main [SPEC.md](../SPEC.md) if it's a major component

## Specification Template

```markdown
# [Component Name] Specification

## Overview
Brief description of the component and its purpose.

## Requirements
- Requirement 1
- Requirement 2

## Design
Detailed design decisions and rationale.

## Implementation
Code examples and patterns.

## Testing
How to test this component.

## Future Considerations
Potential improvements or extensions.
```