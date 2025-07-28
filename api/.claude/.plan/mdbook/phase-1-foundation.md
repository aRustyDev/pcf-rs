# Phase 1: Foundation & Structure (Days 1-3)

## Overview
This phase establishes the MDBook infrastructure, installs necessary plugins, and creates the foundational structure for the documentation system.

## Day 1: MDBook Initialization and Basic Setup

### Morning Tasks (4 hours)

#### 1.1 Initialize MDBook Project
```bash
cd api/
mdbook init docs
```

**Expected Structure:**
```
api/docs/
├── book.toml
└── src/
    ├── chapter_1.md
    └── SUMMARY.md
```

#### 1.2 Remove Default Content
- [ ] Delete `chapter_1.md`
- [ ] Clear default content from `SUMMARY.md`

#### 1.3 Create Directory Structure
```bash
cd api/docs/src/
mkdir -p quick-start shared developers administrators users reference appendices
mkdir -p shared/{patterns,standards,security}
mkdir -p developers/{overview,architecture,modules,api-reference,graphql,contributing,testing,dependencies,cookbook}
mkdir -p administrators/{overview,deployment,configuration,monitoring,security,troubleshooting,cookbook}
mkdir -p users/{overview,authentication,api-endpoints,rate-limiting,errors,troubleshooting,cookbook}
mkdir -p reference/{changelog,roadmap,benchmarks,compliance}
mkdir -p appendices/{deprecated,migrations}
```

#### 1.4 Create Module Subdirectories
```bash
cd api/docs/src/developers/modules/
mkdir -p {config,error,graphql,health,logging,schema,server,services}
```

### Afternoon Tasks (4 hours)

#### 1.5 Configure book.toml
Create comprehensive `book.toml` with all plugin configurations:

```toml
[book]
title = "PCF API Documentation"
authors = ["PCF Contributors"]
description = "Comprehensive documentation for the PCF API"
language = "en"
multilingual = false
src = "src"

[build]
build-dir = "book"
create-missing = true

[preprocessor.index]
enable = true

[preprocessor.links]
enable = true

# Additional configurations...
```

#### 1.6 Create Custom Theme Directory
```bash
cd api/docs/
mkdir -p theme
touch theme/custom.css theme/custom.js theme/favicon.svg
```

#### 1.7 Initial CSS Styling
Create `theme/custom.css` with:
- PCF branding colors
- Custom typography
- Interactive element styles
- Responsive design tweaks

## Day 2: Plugin Installation and Configuration

### Morning Tasks (4 hours)

#### 2.1 Install Essential Plugins

**Priority-based plugin installation:**

**CRITICAL** (documentation won't build without these):
```bash
cargo install mdbook --version 0.4.36
cargo install mdbook-mermaid  # For diagrams
```

**IMPORTANT** (significantly enhance user experience):
```bash
cargo install mdbook-admonish     # Callout boxes
cargo install mdbook-toc          # Table of contents
cargo install mdbook-linkcheck    # Link validation
cargo install mdbook-pagetoc      # Page navigation
```

**OPTIONAL** (nice to have):
```bash
cargo install mdbook-glossary     # Term tooltips
cargo install mdbook-katex        # Math formulas
cargo install mdbook-open-on-gh   # GitHub edit links
cargo install mdbook-variables    # Content reuse
```

#### 2.2 Plugin Installation Verification and Fallbacks

**For each plugin category:**

**CRITICAL Plugin Failures**:
- If `mdbook` fails: Stop and troubleshoot (no fallback possible)
- If `mdbook-mermaid` fails: 
  - Use code blocks with mermaid syntax (renders on GitHub)
  - Consider static diagram images as last resort
  - Document in README for future resolution

**IMPORTANT Plugin Failures**:
- If `mdbook-admonish` fails: Use blockquotes with emoji indicators
- If `mdbook-toc` fails: Create manual table of contents
- If `mdbook-linkcheck` fails: Run manual link verification post-build
- If `mdbook-pagetoc` fails: Rely on main TOC only

**OPTIONAL Plugin Failures**:
- Document in issues for post-launch enhancement
- Continue without these features
- Note impact in deployment documentation

#### 2.3 Configure Plugin Settings in book.toml
Add detailed configuration for each plugin:
- Mermaid theme settings
- Admonish custom types
- TOC depth levels
- Link checker rules
- Variable definitions

### Afternoon Tasks (4 hours)

#### 2.4 Create Plugin Test Pages
Create test pages to verify each plugin works:
- `src/test/plugin-mermaid.md` - Mermaid diagram
- `src/test/plugin-admonish.md` - Admonish blocks
- `src/test/plugin-glossary.md` - Glossary terms
- `src/test/plugin-katex.md` - Math formulas

#### 2.5 Create Custom JavaScript
`theme/custom.js` for:
- Interactive diagram placeholders
- API playground placeholders
- Copy button for code blocks
- Search enhancements

#### 2.6 Create Placeholder Pages for Custom Plugins
Document where custom plugins will integrate:
- `<!-- mdbook-auto-doc: module=config -->`
- `<!-- mdbook-graphql-introspection: schema.graphql -->`
- `<!-- mdbook-dependency-doc: Cargo.toml -->`
- `<!-- mdbook-interactive-diagrams: enabled -->`

## Day 3: Dependencies Documentation and CI/CD Setup

### Morning Tasks (4 hours)

#### 3.1 Create dependencies.toml

Create `api/dependencies.toml` with rationales for key dependencies:

**Minimum Required Content**:
```toml
# Document at least CRITICAL dependencies first
[dependencies.tokio]
version = "1.35"
category = "async-runtime"
rationale = "Industry-standard async runtime, required for async-graphql"

[dependencies.axum]
version = "0.7"
category = "web-framework"
rationale = "Modern web framework with excellent Tower integration"

[dependencies.async-graphql]
version = "6.0"
category = "graphql"
rationale = "Most mature Rust GraphQL server implementation"

# Add more as time permits...
```

**Target Content Structure** (if time allows):
```toml
[dependencies.tokio]
version = "1.35"
license = "MIT"
category = "async-runtime"
rationale = """
Industry-standard async runtime for Rust. Chosen for:
- Excellent performance and low overhead
- Comprehensive ecosystem support
- Active maintenance and community
- Production-proven in high-scale systems
"""
alternatives_considered = [
    { name = "async-std", reason_not_chosen = "Smaller ecosystem, less adoption" },
    { name = "smol", reason_not_chosen = "Less mature, limited features" }
]
migration_cost = "high"
migration_notes = "Would require rewriting all async code"
```

**Acceptable Shortcuts**:
- Start with critical dependencies only
- Add comprehensive details incrementally
- Use "TODO: Add alternatives" for future enhancement
- Link to decision documents if they exist elsewhere

#### 3.2 Document Dependency Categories
- **Core Infrastructure**: tokio, axum, tower
- **GraphQL**: async-graphql, dataloader
- **Configuration**: figment, garde
- **Database**: surrealdb
- **Observability**: tracing, metrics
- **Utilities**: uuid, chrono, serde

### Afternoon Tasks (4 hours)

#### 3.3 Create GitHub Actions Workflow
Create `.github/workflows/mdbook.yml`:

```yaml
name: MDBook Documentation

on:
  push:
    branches: [main]
    paths:
      - 'api/docs/**'
      - 'api/src/**'
      - '.github/workflows/mdbook.yml'
  pull_request:
    paths:
      - 'api/docs/**'
      - 'api/src/**'
      - '.github/workflows/mdbook.yml'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rust/toolchain@v1
        with:
          toolchain: stable
          
      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
          
      - name: Install MDBook and plugins
        run: |
          cargo install mdbook --version 0.4.36
          cargo install mdbook-mermaid
          cargo install mdbook-admonish
          cargo install mdbook-toc
          cargo install mdbook-linkcheck
          cargo install mdbook-glossary
          cargo install mdbook-katex
          cargo install mdbook-pagetoc
          cargo install mdbook-open-on-gh
          cargo install mdbook-variables
          
      - name: Build documentation
        run: |
          cd api/docs
          mdbook build
          
      - name: Check links
        run: |
          cd api/docs
          mdbook test
          
      - name: Upload artifact (PR preview)
        if: github.event_name == 'pull_request'
        uses: actions/upload-artifact@v3
        with:
          name: mdbook-preview
          path: api/docs/book
          
      - name: Deploy to GitHub Pages
        if: github.ref == 'refs/heads/main'
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./api/docs/book
          cname: docs.pcf-api.org
```

#### 3.4 Create Initial Content Files
Create placeholder files for immediate structure:

- `src/introduction.md` - Project overview
- `src/SUMMARY.md` - Complete table of contents
- `src/quick-start/developers.md` - Developer quick start
- `src/quick-start/administrators.md` - Admin quick start
- `src/quick-start/users.md` - User quick start
- `src/shared/glossary.md` - Initial glossary terms

#### 3.5 Test Build and Deployment
- [ ] Run `mdbook build` locally
- [ ] Test all plugins are working
- [ ] Verify GitHub Actions workflow
- [ ] Test PR preview functionality
- [ ] Confirm GitHub Pages deployment

## Deliverables Checklist

### Directory Structure
- [ ] Complete `api/docs/` directory structure created
- [ ] All module subdirectories in place
- [ ] Theme directory with custom assets

### Configuration Files
- [ ] `book.toml` with all plugin configurations
- [ ] `dependencies.toml` with complete rationales
- [ ] `.github/workflows/mdbook.yml` CI/CD pipeline

### Plugin Setup
- [ ] All 9 essential plugins installed
- [ ] Plugin configurations tested
- [ ] Placeholder markers for custom plugins

### Initial Content
- [ ] Introduction page
- [ ] Complete SUMMARY.md structure
- [ ] Quick start guides (placeholders)
- [ ] Test pages for plugin verification

### CI/CD Pipeline
- [ ] GitHub Actions workflow created
- [ ] PR preview deployment configured
- [ ] GitHub Pages deployment ready
- [ ] Link checking automated

## Success Criteria

### Minimum Viable Success (Launch Ready)
1. **Local Build**: `mdbook build` completes (warnings acceptable if documented)
2. **Core Plugins**: CRITICAL plugins functional, IMPORTANT plugins mostly working
3. **CI/CD Pipeline**: Builds and deploys to staging/preview
4. **Documentation Structure**: Core directories created, expandable later
5. **Dependencies Documented**: High-priority dependencies have rationales

### Target Success (Good Experience)
1. **Clean Build**: Minimal warnings, no errors
2. **Most Plugins**: 80%+ of plugins working as intended
3. **Full CI/CD**: Production deployment functional
4. **Complete Structure**: All planned directories in place
5. **Full Documentation**: All dependencies documented with rationales

### Stretch Goals (Excellent)
1. **Perfect Build**: No warnings or errors
2. **All Plugins**: 100% plugin functionality
3. **Advanced CI/CD**: Performance tests, coverage reports
4. **Enhanced Structure**: Additional helpful organization
5. **Rich Documentation**: Dependencies include migration guides

## Risk Mitigation

### Plugin Compatibility Issues
- Test each plugin individually before combining
- Have fallback options for each plugin
- Document any version constraints

### Build Performance
- Use cargo caching in CI/CD
- Optimize image sizes early
- Consider build parallelization

### Deployment Issues
- Test GitHub Pages configuration locally
- Ensure CNAME is properly configured
- Have backup deployment options

## Next Phase Preview

Phase 2 will focus on:
- Creating comprehensive module documentation
- Writing API reference documentation
- Developing shared documentation resources
- Extracting lessons learned from reviews
- Building the core content structure

---

*This phase establishes the critical foundation for the entire documentation system. Ensure all deliverables are complete before proceeding to Phase 2.*