# .old.md Files Integration Tracker

This document tracks the 50 files marked with `.old.md` extension that need review and integration into the new documentation structure.

## Summary
- **Total Files**: 50
- **Priority 1 (Core)**: 15 files
- **Priority 2 (Extended)**: 20 files
- **Priority 3 (Examples)**: 15 files

## Priority 1 - Core Documentation (15 files)
These files contain essential information that should be integrated first.

### Admin (7 files)
- [ ] `admin/deployment/cloud.old.md` - Cloud deployment guide
- [ ] `admin/security/tls.old.md` - TLS configuration
- [ ] `admin/security/network.old.md` - Network security
- [ ] `admin/security/audit.old.md` - Audit logging
- [ ] `admin/troubleshooting/connections.old.md` - Connection issues
- [ ] `admin/troubleshooting/debugging.old.md` - Debugging guide
- [ ] `admin/troubleshooting/memory.old.md` - Memory management

### Developer (8 files)
- [ ] `developer/architecture/request-flow.old.md` - Request processing flow
- [ ] `developer/architecture/design-patterns.old.md` - Design patterns (merge with existing)
- [ ] `developer/api/types.old.md` - API type definitions
- [ ] `developer/api/traits.old.md` - API traits
- [ ] `developer/api/functions.old.md` - API functions
- [ ] `developer/security/principles.old.md` - Security principles
- [ ] `developer/security/threat-model.old.md` - Threat modeling
- [ ] `developer/security/best-practices.old.md` - Security best practices

## Priority 2 - Extended Features (20 files)
These files contain additional features and detailed documentation.

### Developer (13 files)
- [ ] `developer/architecture/core-dependencies.old.md` - Core dependencies
- [ ] `developer/architecture/dependency-analysis.old.md` - Dependency analysis
- [ ] `developer/architecture/dev-dependencies.old.md` - Development dependencies
- [ ] `developer/architecture/diagrams.old.md` - Architecture diagrams
- [ ] `developer/architecture/lessons-learned.old.md` - Lessons learned
- [ ] `developer/architecture/patterns.old.md` - Architecture patterns
- [ ] `developer/contributing/api-standards.old.md` - API standards
- [ ] `developer/contributing/code-standards.old.md` - Code standards
- [ ] `developer/contributing/documentation-standards.old.md` - Documentation standards
- [ ] `developer/modules/configuration/patterns.old.md` - Configuration patterns
- [ ] `developer/modules/errors/patterns.old.md` - Error handling patterns
- [ ] `developer/security/standards.old.md` - Security standards
- [ ] `reference/deprecated/migration.old.md` - Migration from deprecated features

### Reference (3 files)
- [ ] `reference/migrations/api.old.md` - API migration guide
- [ ] `reference/migrations/database.old.md` - Database migration guide
- [ ] `reference/migrations/versions.old.md` - Version migration guide

### User (4 files)
- [ ] `user/api/websockets.old.md` - WebSocket API
- [ ] `user/api/errors/codes.old.md` - Error codes
- [ ] `user/api/errors/format.old.md` - Error format
- [ ] `user/api/errors/retry.old.md` - Retry strategies

## Priority 3 - Examples and Patterns (15 files)
These files contain examples and specific use cases.

### User (15 files)
- [ ] `user/api/graphql/queries.old.md` - GraphQL query examples
- [ ] `user/api/graphql/mutations.old.md` - GraphQL mutation examples
- [ ] `user/api/graphql/subscriptions.old.md` - GraphQL subscription examples
- [ ] `user/api/graphql/pagination.old.md` - Pagination guide
- [ ] `user/api/graphql/errors.old.md` - GraphQL error handling
- [ ] `user/api/rate-limiting/best-practices.old.md` - Rate limiting best practices
- [ ] `user/api/rate-limiting/limits.old.md` - Rate limit details
- [ ] `user/cookbook/examples-go.old.md` - Go examples
- [ ] `user/cookbook/examples-javascript.old.md` - JavaScript examples
- [ ] `user/cookbook/examples-python.old.md` - Python examples
- [ ] `user/cookbook/examples-rust.old.md` - Rust examples
- [ ] `user/quickstart/first-request.old.md` - First request guide
- [ ] `user/quickstart/getting-started.old.md` - Getting started guide
- [ ] `user/troubleshooting/auth.old.md` - Authentication troubleshooting
- [ ] `user/troubleshooting/connections.old.md` - Connection troubleshooting

## Integration Strategy

### For each file:
1. **Review content**: Check if still relevant and accurate
2. **Merge or replace**: 
   - If target file exists, merge content
   - If no target exists, rename and update
3. **Update links**: Fix any internal references
4. **Remove .old extension**: Once integrated
5. **Update SUMMARY.md**: If adding new pages

### Content Guidelines:
- Preserve valuable examples and explanations
- Update outdated API references
- Consolidate duplicate information
- Maintain consistent formatting
- Add missing cross-references

### Tracking Progress:
- Check off items as completed
- Note any files that can be deleted
- Document major changes made
- Update this tracker regularly

## Notes
- Some .old.md files may contain outdated information that should be archived rather than integrated
- Priority can be adjusted based on user needs and feedback
- Consider creating new sections if patterns emerge (e.g., all language examples could become a "Client Libraries" section)