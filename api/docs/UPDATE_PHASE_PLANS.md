# Plan to Update MDBook Phase Plans for New Documentation Structure

## Overview
This plan outlines the necessary updates to align all documentation references with the new structure that was implemented during the reorganization.

## Structure Changes Summary

### Directory Mapping
- `administrators/` → `admin/`
- `developers/` → `developer/`
- `users/` → `user/`
- `shared/` → distributed to appropriate sections
- `appendices/` → `reference/`
- `quick-start/` → distributed to respective `*/quickstart/` directories

### File Naming Convention
- All `index.md` files → `README.md`
- Empty files removed from navigation
- Content marked for review with `.old.md` extension

## Required Updates

### 1. Update book.toml Configuration
**File**: `/Users/analyst/repos/code/public/pcf-rs/api/docs/book.toml`

#### Actions:
- [ ] Update redirect on line 132:
  ```toml
  # From:
  "/api/graphql.html" = "developers/graphql/index.html"
  # To:
  "/api/graphql.html" = "developer/api/graphql/README.html"
  ```
- [ ] Update phase-related comment (line 30-32) to reflect current status
- [ ] Add new redirects for old URLs to maintain backwards compatibility

### 2. Create Path Migration Reference
**New File**: `/Users/analyst/repos/code/public/pcf-rs/api/.claude/docs/path-migration.md`

Document all path changes for easy reference:
```markdown
# Documentation Path Migration Reference

## Admin Section
- `/administrators/overview.md` → `/admin/README.md`
- `/administrators/configuration/index.md` → `/admin/configuration/README.md`
- `/administrators/monitoring/` → `/admin/observability/`

## Developer Section
- `/developers/overview.md` → `/developer/README.md`
- `/developers/api-reference/` → `/developer/api/`
- `/developers/graphql/` → `/developer/api/graphql/` and `/developer/schema/graphql/`

## User Section
- `/users/overview.md` → `/user/README.md`
- `/users/api-endpoints/` → `/user/api/`
```

### 3. Update Documentation References in Code
Search and replace old documentation paths in:
- Source code comments
- README files
- Configuration examples
- Test fixtures

### 4. Update Claude Helper Files
Files that may need updating:
- `/Users/analyst/repos/code/public/pcf-rs/api/.claude/junior-dev-helper/*.md`
- Any files referencing documentation structure

### 5. Create Documentation Structure Guide
**New File**: `/Users/analyst/repos/code/public/pcf-rs/api/.claude/docs/structure-guide.md`

```markdown
# PCF API Documentation Structure Guide

## Current Structure (Post-Migration)

### Top-Level Sections
1. **admin/** - Administrator documentation
   - quickstart/ - Getting started guides
   - architecture/ - System architecture
   - configuration/ - Configuration reference
   - deployment/ - Deployment guides
   - observability/ - Monitoring and logging
   - security/ - Security hardening
   - performance/ - Performance tuning
   - troubleshooting/ - Problem resolution
   - cookbook/ - Practical recipes

2. **developer/** - Developer documentation
   - quickstart/ - Development setup
   - architecture/ - Technical architecture
   - api/ - API implementation
   - schema/ - Schema definitions
   - modules/ - Module documentation
   - security/ - Security implementation
   - observability/ - Instrumentation
   - performance/ - Optimization
   - troubleshooting/ - Debug guides
   - contributing/ - Contribution guides
   - cookbook/ - Code examples

3. **user/** - User documentation
   - quickstart/ - Getting started
   - api/ - API reference
   - cookbook/ - Usage examples
   - architecture/ - High-level overview

4. **reference/** - Cross-cutting references
   - glossary.md
   - licenses.md
   - third-party.md
   - migrations/
   - deprecated/

## Navigation Rules
- Each directory has a README.md as its default page
- Nested structure with collapsible sections
- No numbering in sidebar
- Empty modules removed from navigation
```

### 6. Update Migration Scripts Archive
Move executed migration scripts to an archive:
```bash
mkdir -p /Users/analyst/repos/code/public/pcf-rs/api/docs/.archive/migrations
mv migrate_docs.py migrate_remaining.py .archive/migrations/
```

### 7. Clean Up Backup Directory
After verifying the migration is stable:
```bash
# Archive the backup for reference
tar -czf src.backup.tar.gz src.backup/
# Remove the backup directory
rm -rf src.backup/
```

### 8. Update .old.md Files Integration Plan
Create a tracker for the 50 .old.md files:
```markdown
# .old.md Files Integration Tracker

## Priority 1 - Core Documentation
- [ ] admin/deployment/cloud.old.md
- [ ] admin/security/tls.old.md
- [ ] developer/architecture/request-flow.old.md
...

## Priority 2 - Extended Features
- [ ] user/api/websockets.old.md
- [ ] developer/api/types.old.md
...

## Priority 3 - Examples and Patterns
- [ ] user/cookbook/examples-*.old.md
- [ ] developer/architecture/patterns.old.md
...
```

## Implementation Steps

1. **Immediate Actions**
   - Update book.toml redirects
   - Create path migration reference
   - Archive migration scripts

2. **Short-term Actions** (1-2 days)
   - Update code references to documentation
   - Create structure guide
   - Begin .old.md file integration

3. **Medium-term Actions** (1 week)
   - Complete .old.md file integration
   - Update all helper documentation
   - Clean up backup directories

4. **Long-term Actions** (2 weeks)
   - Full documentation review
   - Update external references
   - Create comprehensive redirect map

## Validation Checklist

- [ ] All builds succeed without errors
- [ ] No broken internal links
- [ ] Redirects work for old URLs
- [ ] Search index updated
- [ ] Navigation tree properly nested
- [ ] No references to old paths remain
- [ ] All .old.md files processed

## Notes

- The migration preserved all content while reorganizing structure
- 121 files were successfully migrated
- 44 new README.md files were created
- 50 .old.md files need content review and integration
- No data was lost during migration