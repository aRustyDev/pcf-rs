#!/usr/bin/env python3
"""
Additional migration for remaining files
"""

import shutil
from pathlib import Path

SRC_DIR = Path("/Users/analyst/repos/code/public/pcf-rs/api/docs/src")

ADDITIONAL_MIGRATIONS = {
    # Shared content - distribute to appropriate sections
    "shared/patterns/overview.md": "developer/architecture/patterns.old.md",
    "shared/patterns/configuration.md": "developer/modules/configuration/patterns.old.md",
    "shared/patterns/error-handling.md": "developer/modules/errors/patterns.old.md",
    "shared/design-patterns.md": "developer/architecture/design-patterns.old.md",
    "shared/glossary.md": "reference/glossary.md",
    "shared/standards/api.md": "developer/contributing/api-standards.old.md",
    "shared/standards/documentation.md": "developer/contributing/documentation-standards.old.md",
    "shared/standards/code.md": "developer/contributing/code-standards.old.md",
    "shared/security/principles.md": "developer/security/principles.old.md",
    "shared/security/threat-model.md": "developer/security/threat-model.old.md",
    "shared/security/best-practices.md": "developer/security/best-practices.old.md",
    "shared/security-standards.md": "developer/security/standards.old.md",
    "shared/lessons-learned.md": "developer/architecture/lessons-learned.old.md",
    
    # Appendices - move to reference
    "appendices/migrations/api.md": "reference/migrations/api.old.md",
    "appendices/migrations/versions.md": "reference/migrations/versions.old.md",
    "appendices/migrations/database.md": "reference/migrations/database.old.md",
    "appendices/licenses.md": "reference/licenses.md",
    "appendices/third-party.md": "reference/third-party.md",
    "appendices/deprecated/migration.md": "reference/deprecated/migration.old.md",
    "appendices/deprecated/index.md": "reference/deprecated/README.md",
    
    # User files that weren't migrated
    "users/troubleshooting/common-issues.md": "user/troubleshooting/README.md",
    "users/troubleshooting/auth.md": "user/troubleshooting/auth.old.md",
    "users/troubleshooting/connections.md": "user/troubleshooting/connections.old.md",
    "users/examples/go.md": "user/cookbook/examples-go.old.md",
    "users/examples/python.md": "user/cookbook/examples-python.old.md",
    "users/examples/rust.md": "user/cookbook/examples-rust.old.md",
    "users/examples/javascript.md": "user/cookbook/examples-javascript.old.md",
    "users/errors/retry.md": "user/api/errors/retry.old.md",
    "users/errors/format.md": "user/api/errors/format.old.md",
    "users/errors/codes.md": "user/api/errors/codes.old.md",
    "users/rate-limiting/best-practices.md": "user/api/rate-limiting/best-practices.old.md",
    "users/rate-limiting/limits.md": "user/api/rate-limiting/limits.old.md",
    "users/rate-limiting/index.md": "user/api/rate-limiting/README.md",
    
    # Developer cookbook files
    "developers/cookbook/performance.md": "developer/cookbook/performance.md",
    "developers/cookbook/patterns.md": "developer/cookbook/patterns.md",
    "developers/cookbook/debugging.md": "developer/cookbook/debugging.md",
    
    # Module files that need to be moved
    "developers/modules/health/index.md": "developer/modules/health/README.md",
    "developers/modules/schema/index.md": "developer/modules/schema/README.md", 
    "developers/modules/logging/index.md": "developer/modules/logging/README.md",
}

def migrate_remaining():
    migrated = 0
    for old_path, new_path in ADDITIONAL_MIGRATIONS.items():
        old_file = SRC_DIR / old_path
        new_file = SRC_DIR / new_path
        
        if old_file.exists():
            new_file.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(str(old_file), str(new_file))
            migrated += 1
            print(f"✓ {old_path} -> {new_path}")
    
    return migrated

def create_missing_files():
    """Create important missing files for complete structure"""
    files_to_create = [
        # Admin files
        ("admin/configuration/infrastructure.md", "# Infrastructure Configuration\n\nGuide to infrastructure configuration for PCF API."),
        ("admin/configuration/database.md", "# Database Configuration\n\nDatabase configuration and optimization guide."),
        ("admin/observability/readiness.md", "# Readiness Probes\n\nImplementing and configuring readiness probes for PCF API."),
        ("admin/security/certifications.md", "# Security Certifications\n\nSecurity certifications and compliance information."),
        
        # Developer files
        ("developer/api/rest.md", "# REST API Implementation\n\nREST API implementation details for developers."),
        ("developer/schema/graphql/types.md", "# GraphQL Types\n\nGraphQL type definitions and implementation."),
        ("developer/schema/graphql/resolvers.md", "# GraphQL Resolvers\n\nGraphQL resolver implementation guide."),
        ("developer/contributing/git.md", "# Git Workflow\n\nGit workflow and best practices for contributors."),
        ("developer/contributing/conventional-commit.md", "# Conventional Commits\n\nConventional commit message guidelines."),
        ("developer/contributing/testing/tdd.md", "# Test-Driven Development\n\nTDD practices and guidelines."),
        ("developer/contributing/testing/end-to-end.md", "# End-to-End Testing\n\nEnd-to-end testing strategies and implementation."),
        
        # User files
        ("user/api/graphql/resolvers.md", "# GraphQL Resolvers Reference\n\nGraphQL resolver reference for API users."),
        
        # Module structure files (for each module)
        ("developer/modules/configuration/dependencies.md", "# Configuration Module Dependencies\n\nDependencies used by the configuration module."),
        ("developer/modules/configuration/extending.md", "# Extending Configuration\n\nHow to extend the configuration module."),
        ("developer/modules/configuration/implementations.md", "# Configuration Implementations\n\nConfiguration module implementation details."),
        
        ("developer/modules/graphql/dependencies.md", "# GraphQL Module Dependencies\n\nDependencies used by the GraphQL module."),
        ("developer/modules/graphql/extending.md", "# Extending GraphQL\n\nHow to extend the GraphQL module."),
        ("developer/modules/graphql/implementations.md", "# GraphQL Implementations\n\nGraphQL module implementation details."),
        
        ("developer/modules/errors/dependencies.md", "# Error Module Dependencies\n\nDependencies used by the error handling module."),
        ("developer/modules/errors/extending.md", "# Extending Error Handling\n\nHow to extend the error handling module."),
        ("developer/modules/errors/implementations.md", "# Error Implementations\n\nError handling module implementation details."),
        
        ("developer/modules/middleware/dependencies.md", "# Middleware Dependencies\n\nDependencies used by the middleware module."),
        ("developer/modules/middleware/extending.md", "# Extending Middleware\n\nHow to extend the middleware module."),
        ("developer/modules/middleware/implementations.md", "# Middleware Implementations\n\nMiddleware module implementation details."),
        
        ("developer/modules/services/dependencies.md", "# Services Dependencies\n\nDependencies used by the services module."),
        ("developer/modules/services/extending.md", "# Extending Services\n\nHow to extend the services module."),
        ("developer/modules/services/implementations.md", "# Services Implementations\n\nServices module implementation details."),
    ]
    
    created = 0
    for file_path, content in files_to_create:
        full_path = SRC_DIR / file_path
        if not full_path.exists():
            full_path.parent.mkdir(parents=True, exist_ok=True)
            full_path.write_text(content)
            created += 1
    
    return created

def cleanup_old_dirs():
    """Remove old empty directories"""
    old_dirs = ["administrators", "developers", "users", "shared", "appendices", "quick-start"]
    for dir_name in old_dirs:
        dir_path = SRC_DIR / dir_name
        if dir_path.exists() and dir_path.is_dir():
            try:
                shutil.rmtree(dir_path)
                print(f"✓ Removed old directory: {dir_name}")
            except Exception as e:
                print(f"✗ Could not remove {dir_name}: {e}")

if __name__ == "__main__":
    print("\nMigrating remaining files...")
    
    # Create necessary directories
    for dir_path in ["reference/migrations", "reference/deprecated", "user/troubleshooting", 
                     "user/api/errors", "user/api/rate-limiting", "developer/modules/health",
                     "developer/modules/schema", "developer/modules/logging"]:
        (SRC_DIR / dir_path).mkdir(parents=True, exist_ok=True)
    
    migrated = migrate_remaining()
    print(f"\n✓ Migrated {migrated} additional files")
    
    created = create_missing_files()
    print(f"✓ Created {created} missing files")
    
    cleanup_old_dirs()
    print("\n✓ Additional migration complete!")