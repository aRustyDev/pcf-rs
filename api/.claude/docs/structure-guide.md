# PCF API Documentation Structure Guide

## Overview
This guide describes the current documentation structure after the reorganization completed on 2025-07-28.

## Design Principles

1. **Audience-Based Organization**: Content is organized by primary audience (admin, developer, user)
2. **Nested Navigation**: Hierarchical structure with collapsible sections
3. **Consistent Naming**: All directories use README.md as their default page
4. **No Empty Modules**: Only sections with actual content appear in navigation
5. **Clear Separation**: Each audience has their own perspective on similar topics

## Directory Structure

```
docs/src/
├── README.md                    # Main documentation overview
├── SUMMARY.md                   # Navigation structure
├── admin/                       # Administrator documentation
│   ├── README.md               # Admin overview
│   ├── quickstart/             # Getting started guides
│   │   └── README.md
│   ├── architecture/           # System architecture
│   │   ├── README.md
│   │   └── dependencies.md
│   ├── configuration/          # Configuration reference
│   │   ├── README.md
│   │   ├── environment.md
│   │   ├── application.md
│   │   ├── secrets.md
│   │   ├── feature-flags.md
│   │   ├── infrastructure.md
│   │   └── database.md
│   ├── deployment/             # Deployment guides
│   │   ├── README.md
│   │   ├── docker.md
│   │   ├── kubernetes.md
│   │   └── cloud.old.md
│   ├── observability/          # Monitoring and logging
│   │   ├── README.md
│   │   ├── metrics.md
│   │   ├── logging.md
│   │   ├── tracing.md
│   │   ├── healthchecks.md
│   │   ├── alerting.md
│   │   └── readiness.md
│   ├── security/               # Security hardening
│   │   ├── README.md
│   │   ├── hardening.md
│   │   ├── certifications.md
│   │   ├── tls.old.md
│   │   ├── network.old.md
│   │   └── audit.old.md
│   ├── performance/            # Performance tuning
│   │   ├── README.md
│   │   └── tips.md
│   ├── troubleshooting/        # Problem resolution
│   │   ├── README.md
│   │   ├── connections.old.md
│   │   ├── memory.old.md
│   │   └── debugging.old.md
│   └── cookbook/               # Practical recipes
│       ├── README.md
│       ├── backup.md
│       ├── scaling.md
│       ├── updates.md
│       └── disaster-recovery.md
│
├── developer/                   # Developer documentation
│   ├── README.md               # Developer overview
│   ├── quickstart/             # Development setup
│   │   ├── README.md
│   │   ├── development.md
│   │   ├── docker.md
│   │   └── documentation.md
│   ├── architecture/           # Technical architecture
│   │   ├── README.md
│   │   ├── dependencies.md
│   │   └── *.old.md files
│   ├── api/                    # API implementation
│   │   ├── README.md
│   │   ├── graphql/
│   │   │   ├── README.md
│   │   │   ├── queries.md
│   │   │   ├── mutations.md
│   │   │   ├── subscriptions.md
│   │   │   └── resolvers.md
│   │   ├── rest.md
│   │   └── *.old.md files
│   ├── schema/                 # Schema definitions
│   │   ├── README.md
│   │   └── graphql/
│   │       ├── README.md
│   │       ├── schema.md
│   │       ├── types.md
│   │       └── resolvers.md
│   ├── modules/                # Module documentation
│   │   ├── README.md
│   │   ├── configuration/
│   │   ├── graphql/
│   │   ├── errors/
│   │   ├── middleware/
│   │   ├── services/
│   │   ├── health/
│   │   ├── schema/
│   │   └── logging/
│   ├── security/               # Security implementation
│   │   ├── README.md
│   │   ├── authentication.md
│   │   ├── authorization.md
│   │   ├── encryption.md
│   │   └── *.old.md files
│   ├── observability/          # Instrumentation guides
│   │   ├── README.md
│   │   ├── metrics.md
│   │   ├── logging.md
│   │   ├── tracing.md
│   │   ├── healthchecks.md
│   │   ├── alerting.md
│   │   └── readiness.md
│   ├── performance/            # Optimization guides
│   │   ├── README.md
│   │   └── tips.md
│   ├── troubleshooting/        # Debug guides
│   │   ├── README.md
│   │   ├── configuration.md
│   │   ├── graphql.md
│   │   ├── errors.md
│   │   ├── middleware.md
│   │   └── services.md
│   ├── contributing/           # Contribution guides
│   │   ├── README.md
│   │   ├── code-style.md
│   │   ├── documentation.md
│   │   ├── git.md
│   │   ├── conventional-commit.md
│   │   ├── testing/
│   │   │   ├── README.md
│   │   │   ├── tdd.md
│   │   │   ├── strategy.md
│   │   │   ├── unit.md
│   │   │   ├── integration.md
│   │   │   ├── end-to-end.md
│   │   │   └── benchmarks.md
│   │   └── *.old.md files
│   └── cookbook/               # Code examples
│       ├── README.md
│       ├── performance.md
│       ├── patterns.md
│       └── debugging.md
│
├── user/                        # User documentation
│   ├── README.md               # User overview
│   ├── quickstart/             # Getting started
│   │   ├── README.md
│   │   └── *.old.md files
│   ├── api/                    # API reference
│   │   ├── README.md
│   │   ├── rest/
│   │   │   └── README.md
│   │   ├── graphql/
│   │   │   ├── README.md
│   │   │   ├── resolvers.md
│   │   │   └── *.old.md files
│   │   ├── authentication.md
│   │   ├── errors/
│   │   │   └── *.old.md files
│   │   ├── rate-limiting/
│   │   │   ├── README.md
│   │   │   └── *.old.md files
│   │   └── websockets.old.md
│   ├── cookbook/               # Usage examples
│   │   ├── README.md
│   │   ├── batch.md
│   │   ├── webhooks.md
│   │   ├── realtime.md
│   │   └── examples-*.old.md
│   ├── architecture/           # High-level overview
│   │   └── README.md
│   └── troubleshooting/        # Common issues
│       ├── README.md
│       └── *.old.md files
│
└── reference/                   # Cross-cutting references
    ├── README.md               # Reference overview
    ├── glossary.md            # Term definitions
    ├── licenses.md            # License information
    ├── third-party.md         # Third-party credits
    ├── migrations/            # Migration guides
    │   └── *.old.md files
    └── deprecated/            # Deprecated features
        ├── README.md
        └── migration.old.md
```

## Navigation Patterns

### Sidebar Organization
1. **Top Level**: Major sections (Admin, Developer, User, Reference)
2. **Second Level**: Category groupings within each section
3. **Third Level**: Individual topics or sub-categories
4. **Fourth Level**: Specific guides or detailed topics

### Naming Conventions
- **Directories**: lowercase with hyphens (e.g., `quick-start/`)
- **Files**: lowercase with hyphens (e.g., `getting-started.md`)
- **Default Pages**: Always `README.md` in each directory
- **Review Files**: Marked with `.old.md` extension

## Content Guidelines

### Admin Section
- Focus on operational concerns
- Include deployment, monitoring, security
- Practical guides for running in production
- Troubleshooting from an ops perspective

### Developer Section
- Technical implementation details
- API design and architecture
- Module documentation
- Contributing guidelines
- Code examples and patterns

### User Section
- API usage documentation
- Client integration guides
- Common use cases
- Troubleshooting from a user perspective

### Reference Section
- Cross-cutting concerns
- Glossary and terminology
- License and legal information
- Migration guides between versions

## File Status

### Active Files
- Standard `.md` files are active documentation
- All have been migrated to new structure
- Links updated to reflect new paths

### Review Files (.old.md)
- 50 files marked for content review
- May contain outdated information
- Need integration or removal
- Tracked separately for cleanup

## Search and Discovery

### Search Index
- Built automatically by mdBook
- Includes all active `.md` files
- Excludes `.old.md` files
- Updated on each build

### Cross-References
- Use relative paths for internal links
- Always link to `.md` files, not `.html`
- Prefer linking to README.md for sections

## Maintenance

### Adding New Content
1. Choose appropriate section based on audience
2. Create directory if needed with README.md
3. Update SUMMARY.md to include in navigation
4. Follow existing patterns for consistency

### Updating Existing Content
1. Edit files in place
2. Update any changed links
3. Test build locally before committing
4. Check for broken links

### Removing Content
1. Delete the file
2. Remove from SUMMARY.md
3. Add redirects in book.toml if needed
4. Update any incoming links