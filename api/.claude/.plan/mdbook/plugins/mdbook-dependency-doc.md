# MDBook Dependency Documentation Plugin Specification

## Overview

This plugin automatically generates comprehensive dependency documentation by merging information from `Cargo.toml` with rationales from `dependencies.toml`, flagging missing documentation and deprecated dependencies.

## Plugin Name
`mdbook-dependency-doc`

## Features

### 1. Dependency Merging
- Parse `Cargo.toml` for actual dependencies
- Merge with `dependencies.toml` for rationales
- Support workspace dependencies
- Handle different dependency types (normal, dev, build)

### 2. Documentation Generation
- Categorized dependency listing
- Version tracking and updates
- License compatibility matrix
- Security audit integration
- Dependency graph visualization

### 3. Quality Checks
- Flag dependencies without rationales
- Identify deprecated dependencies
- Check for security advisories
- License compatibility warnings
- Version currency analysis

### 4. Enhanced Features
- Alternative comparison tables
- Migration guides for replacements
- Performance impact notes
- Bundle size analysis

## Technical Implementation

### Architecture
```
mdbook-dependency-doc/
├── src/
│   ├── main.rs              # MDBook preprocessor entry
│   ├── parser/
│   │   ├── cargo.rs         # Cargo.toml parser
│   │   ├── rationale.rs     # dependencies.toml parser
│   │   └── merger.rs        # Merge logic
│   ├── analyzer/
│   │   ├── security.rs      # Security advisory check
│   │   ├── license.rs       # License compatibility
│   │   ├── deprecated.rs    # Deprecation detection
│   │   └── metrics.rs       # Size/performance metrics
│   ├── generator/
│   │   ├── markdown.rs      # Markdown generation
│   │   ├── graphs.rs        # Dependency graphs
│   │   └── badges.rs        # Status badges
│   └── templates/
│       ├── dependency.hbs   # Individual dependency
│       ├── category.hbs     # Category page
│       └── overview.hbs     # Dependencies overview
├── Cargo.toml
└── README.md
```

### Configuration

In `book.toml`:
```toml
[preprocessor.dependency-doc]
command = "mdbook-dependency-doc"
renderer = ["html"]

[preprocessor.dependency-doc.config]
# Paths to dependency files
cargo_toml = "../Cargo.toml"
dependencies_toml = "../dependencies.toml"
workspace_root = "../"  # For workspace projects

# Output configuration
output_dir = "developers/dependencies"

# Categories for grouping
categories = [
    { name = "async", pattern = "tokio|async-*|futures" },
    { name = "web", pattern = "axum|tower|hyper" },
    { name = "database", pattern = "sqlx|diesel|sea-orm" },
    { name = "serialization", pattern = "serde|json|toml" }
]

# Quality checks
require_rationale = true
check_security = true
check_licenses = true
check_deprecated = true

# License compatibility
allowed_licenses = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "ISC"
]

# Security advisory database
advisory_db = "https://github.com/RustSec/advisory-db"

# Performance metrics
analyze_size = true
analyze_build_time = true

# Graph generation
generate_graph = true
graph_format = "svg"  # svg, png, interactive
```

### Dependencies.toml Format

```toml
# dependencies.toml - Rationales and metadata for dependencies

[dependencies.tokio]
rationale = "Industry-standard async runtime with excellent performance"
alternatives_considered = ["async-std", "smol"]
why_chosen = "Largest ecosystem, best documentation, proven in production"
category = "async"
performance_impact = "minimal - only active during async operations"
migration_cost = "high"

[dependencies.axum]
rationale = "Modern web framework built on Tower"
alternatives_considered = ["actix-web", "rocket", "warp"]
why_chosen = "Type-safe, excellent async support, Tower ecosystem"
category = "web"
notes = "Pairs well with our Tower middleware stack"

[dependencies.serde]
rationale = "De facto standard for serialization in Rust"
alternatives_considered = ["bincode", "postcard"]
why_chosen = "Ubiquitous support, compile-time safety, performance"
category = "serialization"
security_notes = "Always validate untrusted input"

# Deprecated dependencies
[deprecated.actix-web]
replaced_by = "axum"
migration_guide = "docs/migrations/actix-to-axum.md"
removal_version = "2.0.0"
reason = "Simplifying to single web framework"
```

### Implementation Details

#### Dependency Parser and Merger
```rust
use cargo_toml::Manifest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct DependencyRationale {
    rationale: String,
    alternatives_considered: Option<Vec<String>>,
    why_chosen: Option<String>,
    category: Option<String>,
    performance_impact: Option<String>,
    migration_cost: Option<String>,
    security_notes: Option<String>,
    notes: Option<String>,
}

pub struct DependencyMerger {
    manifest: Manifest,
    rationales: HashMap<String, DependencyRationale>,
    deprecated: HashMap<String, DeprecatedInfo>,
}

impl DependencyMerger {
    pub fn merge(&self) -> Result<Vec<DependencyDoc>> {
        let mut dependencies = Vec::new();
        
        // Process normal dependencies
        for (name, dep) in &self.manifest.dependencies {
            let mut doc = DependencyDoc {
                name: name.clone(),
                version: self.extract_version(dep),
                source: DependencySource::Normal,
                rationale: self.rationales.get(name).cloned(),
                is_missing_rationale: self.rationales.get(name).is_none(),
                security_advisories: Vec::new(),
                license: None,
                size_impact: None,
            };
            
            // Check if deprecated
            if let Some(deprecated) = self.deprecated.get(name) {
                doc.deprecated = Some(deprecated.clone());
            }
            
            dependencies.push(doc);
        }
        
        // Flag rationales without dependencies
        for (name, rationale) in &self.rationales {
            if !self.manifest.dependencies.contains_key(name) {
                // Log warning about orphaned rationale
            }
        }
        
        Ok(dependencies)
    }
}
```

#### Security and License Analyzer
```rust
use rustsec::database::Database;
use cargo_license::DependencyDetails;

pub struct SecurityAnalyzer {
    advisory_db: Database,
}

impl SecurityAnalyzer {
    pub async fn check_advisories(&self, dependencies: &[DependencyDoc]) 
        -> HashMap<String, Vec<Advisory>> {
        let mut results = HashMap::new();
        
        for dep in dependencies {
            let advisories = self.advisory_db
                .query_package(&dep.name, &dep.version);
                
            if !advisories.is_empty() {
                results.insert(dep.name.clone(), advisories);
            }
        }
        
        results
    }
}

pub struct LicenseAnalyzer {
    allowed_licenses: Vec<String>,
}

impl LicenseAnalyzer {
    pub fn analyze(&self, dependencies: &[DependencyDoc]) -> LicenseReport {
        let mut report = LicenseReport::default();
        
        for dep in dependencies {
            let license = self.get_license(&dep.name);
            
            if let Some(license) = license {
                if !self.allowed_licenses.contains(&license) {
                    report.incompatible.push((dep.name.clone(), license));
                }
            } else {
                report.unknown.push(dep.name.clone());
            }
        }
        
        report
    }
}
```

#### Documentation Generator
```rust
pub struct DependencyDocGenerator {
    templates: Handlebars,
    config: Config,
}

impl DependencyDocGenerator {
    pub fn generate(&self, dependencies: Vec<DependencyDoc>) -> Result<Documentation> {
        let mut docs = Documentation::new();
        
        // Generate overview page
        docs.add_page("index.md", self.generate_overview(&dependencies)?);
        
        // Generate category pages
        let categorized = self.categorize_dependencies(&dependencies);
        for (category, deps) in categorized {
            docs.add_page(
                &format!("{}.md", category),
                self.generate_category_page(&category, &deps)?
            );
        }
        
        // Generate individual dependency pages
        for dep in &dependencies {
            if dep.rationale.is_some() || dep.deprecated.is_some() {
                docs.add_page(
                    &format!("deps/{}.md", dep.name),
                    self.generate_dependency_page(dep)?
                );
            }
        }
        
        // Generate quality report
        docs.add_page("quality-report.md", self.generate_quality_report(&dependencies)?);
        
        Ok(docs)
    }
}
```

### Generated Documentation Structure

```
developers/dependencies/
├── index.md                # Overview with stats
├── async.md               # Async runtime dependencies
├── web.md                 # Web framework dependencies
├── database.md            # Database dependencies
├── serialization.md       # Serialization dependencies
├── deps/
│   ├── tokio.md          # Detailed tokio documentation
│   ├── axum.md           # Detailed axum documentation
│   └── ...
├── quality-report.md      # Missing rationales, issues
├── license-report.md      # License compatibility
├── security-report.md     # Security advisories
└── dependency-graph.svg   # Visual dependency tree
```

### Overview Page Template

```handlebars
# Dependencies Overview

Total dependencies: {{total_count}}

## Summary

| Category | Count | Size Impact | Security Issues |
|----------|-------|-------------|-----------------|
{{#each categories}}
| {{name}} | {{count}} | {{size}} | {{#if security_issues}}⚠️ {{security_count}}{{else}}✅ None{{/if}} |
{{/each}}

## Quality Metrics

- Dependencies with rationales: {{with_rationale_count}}/{{total_count}} ({{rationale_percentage}}%)
- Deprecated dependencies: {{deprecated_count}}
- Security advisories: {{security_advisory_count}}
- License issues: {{license_issue_count}}

{{#if missing_rationales}}
## ⚠️ Missing Documentation

The following dependencies lack rationales:
{{#each missing_rationales}}
- `{{name}}` ({{version}})
{{/each}}

Please add rationales to `dependencies.toml`.
{{/if}}

{{#if deprecated}}
## 📦 Deprecated Dependencies

{{#each deprecated}}
- `{{name}}` → migrate to `{{replaced_by}}` ([migration guide]({{migration_guide}}))
{{/each}}
{{/if}}

## Dependency Graph

![Dependency Graph](./dependency-graph.svg)
```

### Individual Dependency Template

```handlebars
# {{name}}

**Version**: {{version}}  
**Category**: {{category}}  
**License**: {{license}}  
**Source**: {{source}}

{{#if rationale}}
## Rationale

{{rationale.rationale}}

{{#if rationale.why_chosen}}
### Why Chosen

{{rationale.why_chosen}}
{{/if}}

{{#if rationale.alternatives_considered}}
### Alternatives Considered

{{#each rationale.alternatives_considered}}
- {{this}}
{{/each}}
{{/if}}

{{#if rationale.performance_impact}}
### Performance Impact

{{rationale.performance_impact}}
{{/if}}

{{#if rationale.security_notes}}
### Security Notes

{{rationale.security_notes}}
{{/if}}
{{/if}}

{{#if deprecated}}
## ⚠️ Deprecation Notice

This dependency is scheduled for removal in version {{deprecated.removal_version}}.

**Replacement**: [`{{deprecated.replaced_by}}`](./{{deprecated.replaced_by}}.md)  
**Migration Guide**: [{{deprecated.migration_guide}}]({{deprecated.migration_guide}})  
**Reason**: {{deprecated.reason}}
{{/if}}

{{#if security_advisories}}
## 🔒 Security Advisories

{{#each security_advisories}}
### {{id}}: {{title}}

**Severity**: {{severity}}  
**Affected versions**: {{affected_versions}}  
**Patched versions**: {{patched_versions}}  

{{description}}
{{/each}}
{{/if}}

## Usage in Project

```toml
[dependencies]
{{name}} = "{{version}}"
```

{{#if features_used}}
### Features Used

{{#each features_used}}
- `{{this}}`
{{/each}}
{{/if}}

## Links

- [crates.io](https://crates.io/crates/{{name}})
- [Documentation](https://docs.rs/{{name}})
{{#if repository}}- [Repository]({{repository}}){{/if}}
```

### Interactive Dependency Graph

```javascript
// D3.js based interactive dependency graph
class DependencyGraph {
    constructor(containerId, data) {
        this.container = d3.select(containerId);
        this.data = data;
        this.width = 800;
        this.height = 600;
        
        this.initializeGraph();
    }
    
    initializeGraph() {
        const svg = this.container.append('svg')
            .attr('width', this.width)
            .attr('height', this.height);
            
        const simulation = d3.forceSimulation(this.data.nodes)
            .force('link', d3.forceLink(this.data.links).id(d => d.id))
            .force('charge', d3.forceManyBody())
            .force('center', d3.forceCenter(this.width / 2, this.height / 2));
            
        // Add zoom behavior
        const zoom = d3.zoom()
            .scaleExtent([0.1, 10])
            .on('zoom', (event) => {
                g.attr('transform', event.transform);
            });
            
        svg.call(zoom);
        
        // Create links and nodes
        // Add tooltips showing dependency details
        // Color code by category or security status
    }
}
```

## Installation
```bash
cargo install mdbook-dependency-doc
```

## CI Integration

```yaml
# Check for missing rationales in CI
- name: Check dependency documentation
  run: |
    mdbook-dependency-doc check
    if [ $? -ne 0 ]; then
      echo "::error::Missing dependency rationales"
      exit 1
    fi
```

## Testing Strategy
1. Unit tests for TOML parsing
2. Integration tests with real Cargo projects
3. Security advisory checking tests
4. License detection tests
5. Graph generation tests

## Future Enhancements
1. SBOM (Software Bill of Materials) generation
2. Dependency update automation with rationale preservation
3. Cost analysis (build time, binary size impact)
4. Supply chain security scoring
5. Integration with deps.rs badges
6. Automated PR generation for updates