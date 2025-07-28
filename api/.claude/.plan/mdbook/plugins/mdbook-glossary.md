# MDBook Glossary Plugin Specification

## Overview

This plugin provides automatic glossary term detection, tooltip generation, and centralized glossary management for MDBook documentation. It enhances reader comprehension by providing instant definitions for technical terms throughout the documentation.

## Plugin Name
`mdbook-glossary`

## Features

### 1. Glossary Term Management
- Define terms in a central glossary file (YAML/TOML/JSON)
- Support for term aliases and variations
- Category-based organization
- Multi-language support for terms
- Version-specific definitions

### 2. Automatic Term Detection
- Scan all book pages for glossary terms
- Case-insensitive matching options
- Whole-word boundary detection
- Exclude code blocks and URLs
- Support for compound terms

### 3. Interactive Tooltips
- Hover tooltips with term definitions
- Mobile-friendly tap-to-view
- Inline expansion option
- Customizable styling
- Keyboard navigation support

### 4. Glossary Page Generation
- Auto-generate comprehensive glossary page
- Alphabetical and categorical views
- Search functionality
- Usage statistics per term
- Cross-references between related terms

### 5. Advanced Features
- Acronym expansion (e.g., API → Application Programming Interface)
- Context-aware definitions
- External glossary imports
- Term deprecation warnings
- Glossary validation and linting

## Technical Implementation

### Architecture
```
mdbook-glossary/
├── src/
│   ├── main.rs              # MDBook preprocessor entry
│   ├── glossary.rs          # Glossary data structures
│   ├── parser.rs            # Term detection and parsing
│   ├── renderer.rs          # HTML/tooltip generation
│   ├── validator.rs         # Glossary validation
│   └── templates/
│       ├── tooltip.hbs      # Tooltip HTML template
│       ├── glossary.hbs     # Glossary page template
│       └── styles.css       # Default styles
├── assets/
│   ├── glossary.js          # Client-side interactions
│   └── glossary.css         # Default styles
├── Cargo.toml
├── README.md
└── examples/
    └── glossary.yml         # Example glossary file
```

### Configuration Syntax

In `book.toml`:
```toml
[preprocessor.glossary]
command = "mdbook-glossary"
renderer = ["html"]

[preprocessor.glossary.config]
# Glossary file location (relative to book root)
glossary_file = "glossary.yml"

# Output glossary page location
output_path = "shared/glossary.md"

# Enable automatic term detection
auto_detect = true

# Case sensitivity for term matching
case_sensitive = false

# Match whole words only
whole_words = true

# Exclude patterns (regex)
exclude_patterns = [
    "```[\\s\\S]*?```",     # Code blocks
    "`[^`]+`",              # Inline code
    "https?://[^\\s]+",     # URLs
]

# Maximum tooltip width (pixels)
tooltip_max_width = 300

# Enable inline definitions
inline_definitions = false

# Categories to include (empty = all)
include_categories = []

# Enable acronym detection
detect_acronyms = true

# Minimum term length for auto-detection
min_term_length = 3

# Maximum terms per page (performance)
max_terms_per_page = 100

# Styling theme
theme = "default"  # default, minimal, card
```

### Glossary File Format

#### YAML Format (recommended)
```yaml
# glossary.yml
metadata:
  version: "1.0"
  language: "en"
  last_updated: "2024-07-27"

categories:
  - id: "architecture"
    name: "Architecture"
    description: "System design and architecture terms"
  - id: "graphql"
    name: "GraphQL"
    description: "GraphQL-specific terminology"
  - id: "security"
    name: "Security"
    description: "Security and authentication terms"

terms:
  - term: "API"
    definition: "Application Programming Interface - A set of protocols and tools for building software applications"
    category: "architecture"
    aliases: ["APIs", "api", "application programming interface"]
    see_also: ["REST", "GraphQL", "RPC"]
    examples:
      - "The PCF API provides GraphQL endpoints for data access"
      - "Our API follows RESTful design principles"
    acronym_for: "Application Programming Interface"
    
  - term: "GraphQL"
    definition: "A query language for APIs and a runtime for executing queries using a type system"
    category: "graphql"
    aliases: ["graphql", "GQL"]
    see_also: ["Schema", "Resolver", "Subscription"]
    external_link: "https://graphql.org"
    deprecated: false
    context:
      - pattern: "GraphQL schema"
        definition: "The type system definition for a GraphQL API"
      - pattern: "GraphQL query"
        definition: "A read operation in GraphQL"
        
  - term: "JWT"
    definition: "JSON Web Token - A compact, URL-safe means of representing claims between two parties"
    category: "security"
    aliases: ["JSON Web Token", "jwt"]
    acronym_for: "JSON Web Token"
    see_also: ["Authentication", "Bearer Token"]
    warning: "JWTs should be kept secret and transmitted over HTTPS only"
    
  - term: "Resolver"
    definition: "A function that returns data for a field in a GraphQL schema"
    category: "graphql"
    aliases: ["resolvers", "resolver function"]
    code_example: |
      async fn user_resolver(id: ID) -> Result<User> {
          db.get_user(id).await
      }
```

#### Alternative formats

**TOML Format:**
```toml
[metadata]
version = "1.0"
language = "en"

[[terms]]
term = "API"
definition = "Application Programming Interface"
category = "architecture"
aliases = ["APIs", "api"]

[[terms]]
term = "GraphQL"
definition = "A query language for APIs"
category = "graphql"
```

**JSON Format:**
```json
{
  "metadata": {
    "version": "1.0",
    "language": "en"
  },
  "terms": [
    {
      "term": "API",
      "definition": "Application Programming Interface",
      "category": "architecture",
      "aliases": ["APIs", "api"]
    }
  ]
}
```

### Generated HTML Structure

#### Tooltip HTML
```html
<span class="glossary-term" 
      data-term="API" 
      data-definition="Application Programming Interface..."
      tabindex="0"
      role="button"
      aria-describedby="glossary-tooltip-api">
  API
</span>

<div id="glossary-tooltip-api" 
     class="glossary-tooltip" 
     role="tooltip"
     aria-hidden="true">
  <div class="glossary-tooltip-header">
    <span class="glossary-term-title">API</span>
    <span class="glossary-category">Architecture</span>
  </div>
  <div class="glossary-tooltip-content">
    <p>Application Programming Interface - A set of protocols...</p>
    <div class="glossary-see-also">
      See also: <a href="#term-rest">REST</a>, <a href="#term-graphql">GraphQL</a>
    </div>
  </div>
</div>
```

#### Generated Glossary Page
```markdown
# Glossary

<div class="glossary-search">
  <input type="text" id="glossary-search" placeholder="Search terms..." />
</div>

<div class="glossary-categories">
  <button data-category="all" class="active">All</button>
  <button data-category="architecture">Architecture</button>
  <button data-category="graphql">GraphQL</button>
  <button data-category="security">Security</button>
</div>

## A

### API
*Category: Architecture*

Application Programming Interface - A set of protocols and tools for building software applications.

**Also known as:** APIs, api, application programming interface

**Examples:**
- The PCF API provides GraphQL endpoints for data access
- Our API follows RESTful design principles

**See also:** [REST](#rest), [GraphQL](#graphql), [RPC](#rpc)

---

## G

### GraphQL
*Category: GraphQL*

A query language for APIs and a runtime for executing queries using a type system.

**Context-specific definitions:**
- **GraphQL schema**: The type system definition for a GraphQL API
- **GraphQL query**: A read operation in GraphQL

[Learn more →](https://graphql.org)

**See also:** [Schema](#schema), [Resolver](#resolver), [Subscription](#subscription)
```

### Implementation Details

#### Term Parser
```rust
use regex::Regex;
use pulldown_cmark::{Parser, Event, Tag};

pub struct TermParser {
    glossary: Glossary,
    config: GlossaryConfig,
    term_regex: Regex,
}

impl TermParser {
    pub fn new(glossary: Glossary, config: GlossaryConfig) -> Self {
        let pattern = build_term_pattern(&glossary, &config);
        let term_regex = Regex::new(&pattern).unwrap();
        
        Self {
            glossary,
            config,
            term_regex,
        }
    }
    
    pub fn process_markdown(&self, content: &str) -> String {
        let mut result = String::new();
        let parser = Parser::new(content);
        let mut in_code_block = false;
        let mut events = Vec::new();
        
        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(_)) => {
                    in_code_block = true;
                    events.push(event);
                }
                Event::End(Tag::CodeBlock(_)) => {
                    in_code_block = false;
                    events.push(event);
                }
                Event::Text(text) if !in_code_block => {
                    let processed = self.process_text(&text);
                    events.push(Event::Html(processed.into()));
                }
                _ => events.push(event),
            }
        }
        
        pulldown_cmark::html::push_html(&mut result, events.into_iter());
        result
    }
    
    fn process_text(&self, text: &str) -> String {
        self.term_regex.replace_all(text, |caps: &Captures| {
            let matched_text = &caps[0];
            if let Some(term) = self.glossary.find_term(matched_text) {
                self.render_term_html(matched_text, term)
            } else {
                matched_text.to_string()
            }
        }).to_string()
    }
}
```

#### Glossary Validator
```rust
pub struct GlossaryValidator {
    glossary: Glossary,
}

impl GlossaryValidator {
    pub fn validate(&self) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();
        
        // Check for duplicate terms
        self.check_duplicates(&mut report);
        
        // Check for circular references
        self.check_circular_refs(&mut report);
        
        // Check for broken see_also links
        self.check_broken_links(&mut report);
        
        // Check for empty definitions
        self.check_empty_definitions(&mut report);
        
        // Check category references
        self.check_categories(&mut report);
        
        Ok(report)
    }
    
    fn check_circular_refs(&self, report: &mut ValidationReport) {
        for term in &self.glossary.terms {
            let mut visited = HashSet::new();
            if self.has_circular_ref(&term, &mut visited) {
                report.add_error(format!(
                    "Circular reference detected for term: {}", 
                    term.term
                ));
            }
        }
    }
}
```

#### Client-Side JavaScript
```javascript
// glossary.js
class GlossaryTooltip {
    constructor() {
        this.tooltips = new Map();
        this.activeTooltip = null;
        this.init();
    }
    
    init() {
        // Find all glossary terms
        document.querySelectorAll('.glossary-term').forEach(term => {
            this.setupTerm(term);
        });
        
        // Setup keyboard navigation
        document.addEventListener('keydown', this.handleKeyboard.bind(this));
        
        // Setup mobile touch events
        if ('ontouchstart' in window) {
            this.setupMobileEvents();
        }
    }
    
    setupTerm(termElement) {
        const termId = termElement.dataset.term;
        
        // Mouse events
        termElement.addEventListener('mouseenter', () => this.showTooltip(termId));
        termElement.addEventListener('mouseleave', () => this.hideTooltip());
        
        // Keyboard events
        termElement.addEventListener('focus', () => this.showTooltip(termId));
        termElement.addEventListener('blur', () => this.hideTooltip());
        
        // Click for mobile
        termElement.addEventListener('click', (e) => {
            e.preventDefault();
            this.toggleTooltip(termId);
        });
    }
    
    showTooltip(termId) {
        const tooltip = document.getElementById(`glossary-tooltip-${termId}`);
        if (!tooltip) return;
        
        this.hideTooltip(); // Hide any active tooltip
        
        tooltip.classList.add('visible');
        tooltip.setAttribute('aria-hidden', 'false');
        this.activeTooltip = tooltip;
        
        // Position tooltip
        this.positionTooltip(tooltip);
    }
    
    positionTooltip(tooltip) {
        const term = document.querySelector(`[data-term="${tooltip.dataset.term}"]`);
        const termRect = term.getBoundingClientRect();
        const tooltipRect = tooltip.getBoundingClientRect();
        
        // Calculate optimal position
        let top = termRect.bottom + 5;
        let left = termRect.left;
        
        // Adjust if tooltip goes off screen
        if (left + tooltipRect.width > window.innerWidth) {
            left = window.innerWidth - tooltipRect.width - 10;
        }
        
        if (top + tooltipRect.height > window.innerHeight) {
            top = termRect.top - tooltipRect.height - 5;
        }
        
        tooltip.style.top = `${top}px`;
        tooltip.style.left = `${left}px`;
    }
}

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', () => {
    new GlossaryTooltip();
});
```

### CSS Styling
```css
/* glossary.css */
.glossary-term {
    border-bottom: 1px dotted var(--glossary-term-color, #0066cc);
    cursor: help;
    position: relative;
}

.glossary-term:hover,
.glossary-term:focus {
    background-color: var(--glossary-term-hover, #f0f8ff);
    outline: none;
}

.glossary-tooltip {
    position: fixed;
    z-index: 1000;
    max-width: 300px;
    padding: 12px;
    background: white;
    border: 1px solid #ddd;
    border-radius: 4px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.2s, visibility 0.2s;
}

.glossary-tooltip.visible {
    opacity: 1;
    visibility: visible;
}

.glossary-tooltip-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 8px;
    padding-bottom: 8px;
    border-bottom: 1px solid #eee;
}

.glossary-term-title {
    font-weight: bold;
    color: #333;
}

.glossary-category {
    font-size: 0.85em;
    color: #666;
    background: #f5f5f5;
    padding: 2px 8px;
    border-radius: 3px;
}

/* Dark theme support */
.theme-navy .glossary-tooltip,
.theme-coal .glossary-tooltip {
    background: #2b2b2b;
    border-color: #444;
    color: #ddd;
}

/* Mobile styles */
@media (max-width: 768px) {
    .glossary-tooltip {
        position: fixed;
        left: 10px !important;
        right: 10px !important;
        max-width: none;
        width: auto;
    }
}

/* Glossary page styles */
.glossary-search {
    margin-bottom: 20px;
}

.glossary-search input {
    width: 100%;
    padding: 8px 12px;
    font-size: 16px;
    border: 1px solid #ddd;
    border-radius: 4px;
}

.glossary-categories {
    display: flex;
    gap: 10px;
    margin-bottom: 30px;
    flex-wrap: wrap;
}

.glossary-categories button {
    padding: 6px 16px;
    border: 1px solid #ddd;
    background: white;
    border-radius: 20px;
    cursor: pointer;
    transition: all 0.2s;
}

.glossary-categories button:hover,
.glossary-categories button.active {
    background: #0066cc;
    color: white;
    border-color: #0066cc;
}

.glossary-entry {
    margin-bottom: 30px;
    padding-bottom: 20px;
    border-bottom: 1px solid #eee;
}

.glossary-entry h3 {
    display: flex;
    align-items: baseline;
    gap: 10px;
}

.glossary-meta {
    font-size: 0.85em;
    color: #666;
    font-weight: normal;
}
```

## Usage Example

### Basic Usage
```markdown
<!-- In any markdown file -->
The PCF API uses GraphQL for its query interface. Authentication is handled via JWT tokens.

When working with resolvers, ensure proper error handling...
```

### Generated Output
```html
<p>The PCF <span class="glossary-term" data-term="API">API</span> uses 
<span class="glossary-term" data-term="GraphQL">GraphQL</span> for its query interface. 
Authentication is handled via <span class="glossary-term" data-term="JWT">JWT</span> tokens.</p>

<p>When working with <span class="glossary-term" data-term="resolver">resolvers</span>, 
ensure proper error handling...</p>
```

### Manual Term Marking
```markdown
<!-- Force glossary term -->
The {{glossary:API}} provides comprehensive functionality.

<!-- Exclude from glossary -->
The API {{no-glossary}}endpoint{{/no-glossary}} is rate-limited.
```

## Installation
```bash
cargo install mdbook-glossary
```

## Performance Considerations

1. **Term Caching**: Cache compiled regex patterns
2. **Lazy Loading**: Load tooltips on demand
3. **Debouncing**: Debounce tooltip display on hover
4. **Page Limits**: Limit terms per page to prevent slowdown
5. **Build-time Processing**: Process at build time, not runtime

## Testing Strategy

1. **Unit Tests**:
   - Term detection accuracy
   - Glossary validation
   - HTML generation
   - Edge case handling

2. **Integration Tests**:
   - MDBook integration
   - Multiple glossary formats
   - Cross-references
   - Performance benchmarks

3. **Browser Tests**:
   - Tooltip positioning
   - Mobile interactions
   - Keyboard navigation
   - Accessibility compliance

## Accessibility Features

1. **ARIA Labels**: Proper roles and descriptions
2. **Keyboard Navigation**: Full keyboard support
3. **Screen Reader**: Announces definitions
4. **High Contrast**: Respects user preferences
5. **Focus Management**: Clear focus indicators

## Future Enhancements

1. **Multi-language Glossaries**: Support for i18n
2. **Glossary Analytics**: Track term usage
3. **AI-Powered Suggestions**: Suggest new glossary terms
4. **Glossary Import/Export**: Share glossaries between projects
5. **Visual Glossary**: Support for diagrams and images
6. **Contextual Learning**: Progressive disclosure of related terms
7. **Integration with External Glossaries**: Import from industry standards
8. **Version Control**: Track glossary changes over time