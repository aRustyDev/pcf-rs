#!/usr/bin/env python3
"""
Documentation migration script for PCF API docs reorganization
"""

import os
import shutil
import re
from pathlib import Path
import json

# Base paths
SRC_DIR = Path("/Users/analyst/repos/code/public/pcf-rs/api/docs/src")
BACKUP_DIR = Path("/Users/analyst/repos/code/public/pcf-rs/api/docs/src.backup")

# Mapping of old paths to new paths
MIGRATION_MAP = {
    # Admin section (from administrators)
    "administrators/overview.md": "admin/README.md",
    "administrators/configuration/index.md": "admin/configuration/README.md",
    "administrators/configuration/environment.md": "admin/configuration/environment.md",
    "administrators/configuration/files.md": "admin/configuration/application.md",
    "administrators/configuration/secrets.md": "admin/configuration/secrets.md",
    "administrators/configuration/features.md": "admin/configuration/feature-flags.md",
    
    # Deployment
    "administrators/deployment/index.md": "admin/deployment/README.md",
    "administrators/deployment/docker.md": "admin/deployment/docker.md",
    "administrators/deployment/kubernetes.md": "admin/deployment/kubernetes.md",
    "administrators/deployment/cloud.md": "admin/deployment/cloud.old.md",
    "administrators/deployment/architecture.md": "admin/architecture/README.md",
    
    # Monitoring -> Observability
    "administrators/monitoring/index.md": "admin/observability/README.md",
    "administrators/monitoring/metrics.md": "admin/observability/metrics.md",
    "administrators/monitoring/logging.md": "admin/observability/logging.md",
    "administrators/monitoring/tracing.md": "admin/observability/tracing.md",
    "administrators/monitoring/health-checks.md": "admin/observability/healthchecks.md",
    "administrators/monitoring/alerting.md": "admin/observability/alerting.md",
    
    # Security
    "administrators/security/index.md": "admin/security/README.md",
    "administrators/security/hardening.md": "admin/security/hardening.md",
    "administrators/security/tls.md": "admin/security/tls.old.md",
    "administrators/security/network.md": "admin/security/network.old.md",
    "administrators/security/audit.md": "admin/security/audit.old.md",
    
    # Troubleshooting
    "administrators/troubleshooting/common-issues.md": "admin/troubleshooting/README.md",
    "administrators/troubleshooting/performance.md": "admin/performance/tips.md",
    "administrators/troubleshooting/connections.md": "admin/troubleshooting/connections.old.md",
    "administrators/troubleshooting/memory.md": "admin/troubleshooting/memory.old.md",
    "administrators/troubleshooting/debugging.md": "admin/troubleshooting/debugging.old.md",
    
    # Cookbook
    "administrators/cookbook/backup.md": "admin/cookbook/backup.md",
    "administrators/cookbook/scaling.md": "admin/cookbook/scaling.md",
    "administrators/cookbook/updates.md": "admin/cookbook/updates.md",
    "administrators/cookbook/disaster-recovery.md": "admin/cookbook/disaster-recovery.md",
    
    # Developer section
    "developers/overview.md": "developer/README.md",
    "developers/architecture/system-overview.md": "developer/architecture/README.md",
    "developers/architecture/request-flow.md": "developer/architecture/request-flow.old.md",
    "developers/architecture/design-patterns.md": "developer/architecture/design-patterns.old.md",
    "developers/architecture/diagrams.md": "developer/architecture/diagrams.old.md",
    
    # Dependencies
    "developers/dependencies/index.md": "developer/architecture/dependencies.md",
    "developers/dependencies/core.md": "developer/architecture/core-dependencies.old.md",
    "developers/dependencies/dev.md": "developer/architecture/dev-dependencies.old.md",
    "developers/dependencies/analysis.md": "developer/architecture/dependency-analysis.old.md",
    
    # API Reference -> API
    "developers/api-reference/index.md": "developer/api/README.md",
    "developers/api-reference/types.md": "developer/api/types.old.md",
    "developers/api-reference/traits.md": "developer/api/traits.old.md",
    "developers/api-reference/functions.md": "developer/api/functions.old.md",
    
    # GraphQL
    "developers/graphql/schema.md": "developer/schema/graphql/schema.md",
    "developers/graphql/queries.md": "developer/api/graphql/queries.md",
    "developers/graphql/mutations.md": "developer/api/graphql/mutations.md",
    "developers/graphql/subscriptions.md": "developer/api/graphql/subscriptions.md",
    "developers/graphql/best-practices.md": "developer/api/graphql/README.md",
    
    # Contributing
    "developers/contributing/getting-started.md": "developer/contributing/README.md",
    "developers/contributing/setup.md": "developer/quickstart/development.md",
    "developers/contributing/code-style.md": "developer/contributing/code-style.md",
    "developers/contributing/testing.md": "developer/contributing/testing/README.md",
    "developers/contributing/documentation.md": "developer/contributing/documentation.md",
    
    # Testing
    "developers/testing/strategy.md": "developer/contributing/testing/strategy.md",
    "developers/testing/unit-tests.md": "developer/contributing/testing/unit.md",
    "developers/testing/integration-tests.md": "developer/contributing/testing/integration.md",
    "developers/testing/performance-tests.md": "developer/contributing/testing/benchmarks.md",
    
    # Modules
    "developers/modules/config/index.md": "developer/modules/configuration/README.md",
    "developers/modules/graphql/index.md": "developer/modules/graphql/README.md",
    "developers/modules/error/index.md": "developer/modules/errors/README.md",
    "developers/modules/server/index.md": "developer/modules/middleware/README.md",
    "developers/modules/services/index.md": "developer/modules/services/README.md",
    
    # User section
    "users/overview.md": "user/README.md",
    "users/quick-start.md": "user/quickstart/README.md",
    "users/first-request.md": "user/quickstart/first-request.old.md",
    
    # API endpoints
    "users/api-endpoints/rest.md": "user/api/rest/README.md",
    "users/api-endpoints/graphql.md": "user/api/graphql/README.md",
    "users/api-endpoints/websockets.md": "user/api/websockets.old.md",
    
    # Authentication
    "users/authentication/index.md": "user/api/authentication.md",
    
    # GraphQL specifics
    "users/graphql/queries.md": "user/api/graphql/queries.old.md",
    "users/graphql/mutations.md": "user/api/graphql/mutations.old.md",
    "users/graphql/subscriptions.md": "user/api/graphql/subscriptions.old.md",
    "users/graphql/pagination.md": "user/api/graphql/pagination.old.md",
    "users/graphql/errors.md": "user/api/graphql/errors.old.md",
    
    # Cookbook
    "users/cookbook/batch.md": "user/cookbook/batch.md",
    "users/cookbook/webhooks.md": "user/cookbook/webhooks.md",
    "users/cookbook/realtime.md": "user/cookbook/realtime.md",
    
    # Root level files
    "introduction.md": "README.md",
    "quick-start/developers.md": "developer/quickstart/docker.md",
    "quick-start/administrators.md": "admin/quickstart/README.md",
    "quick-start/users.md": "user/quickstart/getting-started.old.md",
}

def create_backup():
    """Create a backup of the current documentation"""
    if BACKUP_DIR.exists():
        shutil.rmtree(BACKUP_DIR)
    shutil.copytree(SRC_DIR, BACKUP_DIR)
    print(f"✓ Created backup at {BACKUP_DIR}")

def clean_empty_files():
    """Remove files that are essentially empty (only title or very short)"""
    empty_files = []
    for md_file in SRC_DIR.rglob("*.md"):
        if md_file.is_file():
            content = md_file.read_text().strip()
            # Check if file is essentially empty
            lines = content.split('\n')
            non_empty_lines = [l.strip() for l in lines if l.strip() and not l.strip().startswith('#')]
            
            # If file has less than 2 non-header lines or less than 100 chars total
            if len(non_empty_lines) < 2 or len(content) < 100:
                empty_files.append(md_file)
    
    return empty_files

def migrate_files():
    """Migrate files according to the mapping"""
    migrated = []
    failed = []
    
    for old_path, new_path in MIGRATION_MAP.items():
        old_file = SRC_DIR / old_path
        new_file = SRC_DIR / new_path
        
        if old_file.exists():
            # Create parent directory if needed
            new_file.parent.mkdir(parents=True, exist_ok=True)
            
            try:
                # Read content and update links
                content = old_file.read_text()
                content = update_links(content)
                
                # Write to new location
                new_file.write_text(content)
                migrated.append((old_path, new_path))
                
                # Remove old file
                old_file.unlink()
                
            except Exception as e:
                failed.append((old_path, new_path, str(e)))
        else:
            # Check if it's already been moved or doesn't exist
            if not new_file.exists():
                failed.append((old_path, new_path, "Source file not found"))
    
    return migrated, failed

def update_links(content):
    """Update internal links to match new structure"""
    # Update directory references
    replacements = [
        (r'/administrators/', '/admin/'),
        (r'/developers/', '/developer/'),
        (r'/users/', '/user/'),
        (r'administrators/', 'admin/'),
        (r'developers/', 'developer/'),
        (r'users/', 'user/'),
        (r'index\.md', 'README.md'),
        (r'Index\.md', 'README.md'),
    ]
    
    for old, new in replacements:
        content = re.sub(old, new, content, flags=re.IGNORECASE)
    
    return content

def update_summary_md():
    """Update SUMMARY.md with new structure"""
    summary_content = """# Summary

[Overview](README.md)

# Reference

- [Overview](reference/README.md)

# Admin

- [Overview](admin/README.md)
  - [Quick Start](admin/quickstart/README.md)
  - [Cookbook](admin/cookbook/README.md)
    - [Backup](admin/cookbook/backup.md)
    - [Scaling](admin/cookbook/scaling.md)
    - [Updates](admin/cookbook/updates.md)
    - [Disaster Recovery](admin/cookbook/disaster-recovery.md)
  - [Architecture](admin/architecture/README.md)
    - [Dependencies](admin/architecture/dependencies.md)
  - [Configuration](admin/configuration/README.md)
    - [Environment](admin/configuration/environment.md)
    - [Application](admin/configuration/application.md)
    - [Secrets](admin/configuration/secrets.md)
    - [Feature Flags](admin/configuration/feature-flags.md)
    - [Infrastructure](admin/configuration/infrastructure.md)
    - [Database](admin/configuration/database.md)
  - [Deployment](admin/deployment/README.md)
    - [Docker](admin/deployment/docker.md)
    - [Kubernetes](admin/deployment/kubernetes.md)
  - [Observability](admin/observability/README.md)
    - [Metrics](admin/observability/metrics.md)
    - [Logging](admin/observability/logging.md)
    - [Tracing](admin/observability/tracing.md)
    - [Health Checks](admin/observability/healthchecks.md)
    - [Alerting](admin/observability/alerting.md)
    - [Readiness](admin/observability/readiness.md)
  - [Security](admin/security/README.md)
    - [Hardening](admin/security/hardening.md)
    - [Certifications](admin/security/certifications.md)
  - [Performance](admin/performance/README.md)
    - [Tips](admin/performance/tips.md)
  - [Troubleshooting](admin/troubleshooting/README.md)

# Developer

- [Overview](developer/README.md)
  - [Quick Start](developer/quickstart/README.md)
    - [Development](developer/quickstart/development.md)
    - [Docker](developer/quickstart/docker.md)
    - [Documentation](developer/quickstart/documentation.md)
  - [Architecture](developer/architecture/README.md)
    - [Dependencies](developer/architecture/dependencies.md)
  - [API](developer/api/README.md)
    - [GraphQL](developer/api/graphql/README.md)
      - [Queries](developer/api/graphql/queries.md)
      - [Mutations](developer/api/graphql/mutations.md)
      - [Subscriptions](developer/api/graphql/subscriptions.md)
      - [Resolvers](developer/api/graphql/resolvers.md)
    - [REST](developer/api/rest.md)
  - [Schema](developer/schema/README.md)
    - [GraphQL](developer/schema/graphql/README.md)
      - [Schema](developer/schema/graphql/schema.md)
      - [Types](developer/schema/graphql/types.md)
      - [Resolvers](developer/schema/graphql/resolvers.md)
  - [Modules](developer/modules/README.md)
    - [Configuration](developer/modules/configuration/README.md)
      - [Dependencies](developer/modules/configuration/dependencies.md)
      - [Extending](developer/modules/configuration/extending.md)
      - [Implementations](developer/modules/configuration/implementations.md)
    - [GraphQL](developer/modules/graphql/README.md)
      - [Dependencies](developer/modules/graphql/dependencies.md)
      - [Extending](developer/modules/graphql/extending.md)
      - [Implementations](developer/modules/graphql/implementations.md)
    - [Errors](developer/modules/errors/README.md)
      - [Dependencies](developer/modules/errors/dependencies.md)
      - [Extending](developer/modules/errors/extending.md)
      - [Implementations](developer/modules/errors/implementations.md)
    - [Middleware](developer/modules/middleware/README.md)
      - [Dependencies](developer/modules/middleware/dependencies.md)
      - [Extending](developer/modules/middleware/extending.md)
      - [Implementations](developer/modules/middleware/implementations.md)
    - [Services](developer/modules/services/README.md)
      - [Dependencies](developer/modules/services/dependencies.md)
      - [Extending](developer/modules/services/extending.md)
      - [Implementations](developer/modules/services/implementations.md)
  - [Security](developer/security/README.md)
    - [Authentication](developer/security/authentication.md)
    - [Authorization](developer/security/authorization.md)
    - [Encryption](developer/security/encryption.md)
  - [Observability](developer/observability/README.md)
    - [Metrics](developer/observability/metrics.md)
    - [Logging](developer/observability/logging.md)
    - [Tracing](developer/observability/tracing.md)
    - [Health Checks](developer/observability/healthchecks.md)
    - [Alerting](developer/observability/alerting.md)
    - [Readiness](developer/observability/readiness.md)
  - [Performance](developer/performance/README.md)
    - [Tips](developer/performance/tips.md)
  - [Troubleshooting](developer/troubleshooting/README.md)
    - [Configuration](developer/troubleshooting/configuration.md)
    - [GraphQL](developer/troubleshooting/graphql.md)
    - [Errors](developer/troubleshooting/errors.md)
    - [Middleware](developer/troubleshooting/middleware.md)
    - [Services](developer/troubleshooting/services.md)
  - [Contributing](developer/contributing/README.md)
    - [Code Style](developer/contributing/code-style.md)
    - [Documentation](developer/contributing/documentation.md)
    - [Git](developer/contributing/git.md)
    - [Conventional Commits](developer/contributing/conventional-commit.md)
    - [Testing](developer/contributing/testing/README.md)
      - [TDD](developer/contributing/testing/tdd.md)
      - [Strategy](developer/contributing/testing/strategy.md)
      - [Unit Tests](developer/contributing/testing/unit.md)
      - [Integration Tests](developer/contributing/testing/integration.md)
      - [End-to-End Tests](developer/contributing/testing/end-to-end.md)
      - [Benchmarks](developer/contributing/testing/benchmarks.md)
  - [Cookbook](developer/cookbook/README.md)

# User

- [Overview](user/README.md)
  - [Quick Start](user/quickstart/README.md)
  - [API](user/api/README.md)
    - [REST](user/api/rest/README.md)
    - [GraphQL](user/api/graphql/README.md)
      - [Resolvers](user/api/graphql/resolvers.md)
    - [Authentication](user/api/authentication.md)
  - [Cookbook](user/cookbook/README.md)
    - [Batch Operations](user/cookbook/batch.md)
    - [Webhooks](user/cookbook/webhooks.md)
    - [Real-time](user/cookbook/realtime.md)
  - [Architecture](user/architecture/README.md)
"""
    
    summary_file = SRC_DIR / "SUMMARY.md"
    summary_file.write_text(summary_content)
    print("✓ Updated SUMMARY.md with new structure")

def cleanup_empty_directories():
    """Remove empty directories after migration"""
    for dirpath, dirnames, filenames in os.walk(SRC_DIR, topdown=False):
        if not dirnames and not filenames and dirpath != str(SRC_DIR):
            try:
                os.rmdir(dirpath)
            except:
                pass

def main():
    print("Starting PCF API documentation migration...")
    
    # Step 1: Create backup
    create_backup()
    
    # Step 2: Identify empty files
    empty_files = clean_empty_files()
    print(f"\n✓ Identified {len(empty_files)} empty files to skip")
    
    # Step 3: Migrate files
    migrated, failed = migrate_files()
    print(f"\n✓ Migrated {len(migrated)} files successfully")
    if failed:
        print(f"✗ Failed to migrate {len(failed)} files:")
        for old, new, reason in failed:
            print(f"  - {old} -> {new}: {reason}")
    
    # Step 4: Update SUMMARY.md
    update_summary_md()
    
    # Step 5: Cleanup empty directories
    cleanup_empty_directories()
    print("\n✓ Cleaned up empty directories")
    
    # Step 6: Create missing README files
    create_missing_readmes()
    
    print("\n✓ Migration complete!")
    
    # Report on .old.md files created
    old_files = list(SRC_DIR.rglob("*.old.md"))
    if old_files:
        print(f"\n📝 Created {len(old_files)} .old.md files for content that needs review")

def create_missing_readmes():
    """Create README.md files for directories that need them"""
    readme_dirs = [
        ("admin", "# Admin Documentation\n\nComprehensive documentation for PCF API administrators."),
        ("admin/cookbook", "# Admin Cookbook\n\nPractical recipes and solutions for common administrative tasks."),
        ("admin/quickstart", "# Admin Quick Start\n\nGet up and running quickly with PCF API administration."),
        ("admin/performance", "# Performance\n\nPerformance optimization and monitoring guides."),
        ("developer", "# Developer Documentation\n\nComplete guide for developers working with the PCF API."),
        ("developer/cookbook", "# Developer Cookbook\n\nCode examples and solutions for common development tasks."),
        ("developer/quickstart", "# Developer Quick Start\n\nGet started quickly with PCF API development."),
        ("developer/modules", "# PCF API Modules\n\nDetailed documentation for all PCF API modules."),
        ("developer/schema", "# Schema Documentation\n\nComplete schema reference for PCF API."),
        ("developer/security", "# Security Implementation\n\nSecurity architecture and implementation details."),
        ("developer/observability", "# Observability Implementation\n\nObservability and monitoring implementation guide."),
        ("developer/performance", "# Performance Guide\n\nPerformance optimization techniques for developers."),
        ("developer/troubleshooting", "# Troubleshooting Guide\n\nCommon issues and solutions for developers."),
        ("user", "# User Documentation\n\nComplete guide for PCF API users."),
        ("user/api", "# API Reference\n\nComplete API reference for PCF API users."),
        ("user/architecture", "# Architecture Overview\n\nHigh-level architecture overview for API users."),
        ("reference", "# Reference Documentation\n\nQuick reference guides and API documentation."),
    ]
    
    for dir_path, content in readme_dirs:
        readme_file = SRC_DIR / dir_path / "README.md"
        if not readme_file.exists():
            readme_file.parent.mkdir(parents=True, exist_ok=True)
            readme_file.write_text(content)
    
    print(f"✓ Created {len(readme_dirs)} README files")

if __name__ == "__main__":
    main()