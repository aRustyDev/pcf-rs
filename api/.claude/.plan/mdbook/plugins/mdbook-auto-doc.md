# MDBook Auto-Doc Plugin Specification

## Overview

This plugin automatically extracts and formats documentation from Rust source code comments, creating MDBook pages that stay synchronized with the codebase.

## Plugin Name
`mdbook-auto-doc`

## Features

### 1. Documentation Extraction
- Extract doc comments from Rust modules, structs, functions, traits
- Support for module-level documentation
- Preserve code examples and formatting
- Extract custom markers (@example, @note, @see, etc.)

### 2. Intelligent Grouping
- Group by module hierarchy
- Separate public vs private items
- Category-based organization (traits, structs, functions)
- Cross-reference generation

### 3. Code Synchronization
- Track source file changes
- Update only modified documentation
- Maintain stable links
- Version change detection

### 4. Enhanced Formatting
- Syntax highlighting for code blocks
- Collapsible sections for large items
- Automatic table of contents
- Source code links

## Technical Implementation

### Architecture
```
mdbook-auto-doc/
├── src/
│   ├── main.rs              # MDBook preprocessor entry
│   ├── extractor.rs         # Rust doc comment extraction
│   ├── parser.rs            # Parse Rust AST
│   ├── formatter.rs         # Convert to Markdown
│   ├── cache.rs            # Change detection
│   └── templates/
│       ├── module.hbs       # Module page template
│       ├── trait.hbs        # Trait documentation
│       └── struct.hbs       # Struct documentation
├── Cargo.toml
└── README.md
```

### Configuration Syntax

In `book.toml`:
```toml
[preprocessor.auto-doc]
command = "mdbook-auto-doc"
renderer = ["html"]

[preprocessor.auto-doc.config]
# Source code location
source_dir = "../src"

# Output directory in book
output_dir = "developers/api-reference"

# Include patterns
include_patterns = [
    "**/*.rs",
    "!**/tests/**",
    "!**/target/**"
]

# Documentation markers to extract
markers = ["@example", "@note", "@see", "@glossary", "@security"]

# Visibility filter
visibility = "public"  # public, private, all

# Group by
group_by = "module"  # module, category, feature

# Generate index
generate_index = true

# Link to source
source_base_url = "https://github.com/org/repo/blob/main"
```

### Marker Syntax in Rust Code

```rust
/// Main authentication module
/// 
/// This module handles all authentication concerns including:
/// - Session management
/// - Token validation
/// - Permission checks
/// 
/// @example
/// ```rust
/// use auth::Session;
/// 
/// let session = Session::from_request(&req)?;
/// if session.is_authenticated() {
///     // Handle authenticated request
/// }
/// ```
/// 
/// @note Security Critical
/// All authentication functions must be audited before changes
/// 
/// @see crate::middleware::auth_middleware
/// @glossary Authentication: The process of verifying user identity
pub mod auth {
    /// Session representation
    /// 
    /// Contains user information and permissions
    /// 
    /// @security Never log session tokens
    pub struct Session {
        /// User identifier
        pub user_id: Uuid,
        /// Session token (sensitive)
        #[doc(hidden)]
        token: String,
    }
}
```

### Generated Output Structure

```
developers/api-reference/
├── index.md                 # Auto-generated index
├── modules/
│   ├── auth/
│   │   ├── index.md        # Module overview
│   │   ├── session.md      # Session struct docs
│   │   └── traits.md       # Auth traits
│   ├── graphql/
│   │   ├── index.md
│   │   ├── schema.md
│   │   └── resolvers.md
│   └── services/
│       └── ...
├── traits.md               # All traits index
├── glossary.md            # Extracted @glossary terms
└── examples.md            # All @example code
```

### Implementation Details

#### Documentation Extractor
```rust
use syn::{File, Item, ItemMod, ItemStruct, ItemTrait};
use quote::ToTokens;

pub struct DocExtractor {
    markers: Vec<String>,
    visibility_filter: Visibility,
}

impl DocExtractor {
    pub fn extract_from_file(&self, path: &Path) -> Result<ModuleDoc> {
        let content = fs::read_to_string(path)?;
        let syntax_tree = syn::parse_file(&content)?;
        
        let mut module_doc = ModuleDoc::new(path);
        
        for item in syntax_tree.items {
            match item {
                Item::Mod(item_mod) => {
                    self.extract_module_doc(&item_mod, &mut module_doc);
                }
                Item::Struct(item_struct) => {
                    self.extract_struct_doc(&item_struct, &mut module_doc);
                }
                Item::Trait(item_trait) => {
                    self.extract_trait_doc(&item_trait, &mut module_doc);
                }
                // ... other items
            }
        }
        
        Ok(module_doc)
    }
    
    fn extract_doc_comments(&self, attrs: &[Attribute]) -> DocComment {
        // Extract /// comments
        // Parse @markers
        // Format code blocks
    }
}
```

#### Markdown Generator
```rust
pub struct MarkdownGenerator {
    templates: Handlebars,
    source_base_url: Option<String>,
}

impl MarkdownGenerator {
    pub fn generate_module_page(&self, module: &ModuleDoc) -> String {
        let mut context = Context::new();
        
        // Add module information
        context.insert("name", &module.name);
        context.insert("path", &module.path);
        context.insert("doc", &module.documentation);
        
        // Add items
        context.insert("structs", &module.structs);
        context.insert("traits", &module.traits);
        context.insert("functions", &module.functions);
        
        // Add source link
        if let Some(base_url) = &self.source_base_url {
            context.insert("source_url", &format!("{}/{}", base_url, module.path));
        }
        
        self.templates.render("module", &context).unwrap()
    }
}
```

### Module Template Example

```handlebars
# Module: {{name}}

{{#if source_url}}
[View Source]({{source_url}})
{{/if}}

{{doc}}

{{#if structs}}
## Structs

{{#each structs}}
### {{name}}

{{documentation}}

{{#if example}}
<details>
<summary>Example</summary>

{{example}}

</details>
{{/if}}

[Read more](./{{slug}}.md)
{{/each}}
{{/if}}

{{#if traits}}
## Traits

{{#each traits}}
### {{name}}

{{documentation}}

[Read more](./{{slug}}.md)
{{/each}}
{{/if}}
```

### Change Detection

```rust
pub struct DocCache {
    cache_file: PathBuf,
    entries: HashMap<PathBuf, CacheEntry>,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    last_modified: SystemTime,
    content_hash: String,
    generated_files: Vec<PathBuf>,
}

impl DocCache {
    pub fn needs_update(&self, path: &Path) -> bool {
        let metadata = fs::metadata(path).ok()?;
        let modified = metadata.modified().ok()?;
        
        match self.entries.get(path) {
            Some(entry) => entry.last_modified < modified,
            None => true,
        }
    }
    
    pub fn update(&mut self, path: &Path, generated: Vec<PathBuf>) {
        // Update cache entry
    }
}
```

## Usage Example

### Source Code
```rust
// src/auth/mod.rs
//! Authentication and authorization module
//! 
//! Provides secure user authentication and permission management.
//! 
//! @example
//! ```rust
//! use pcf_api::auth::{Session, authenticate};
//! 
//! let session = authenticate(username, password).await?;
//! ```

/// Authenticates a user with credentials
/// 
/// @security Rate limited to 5 attempts per minute
/// @see Session
pub async fn authenticate(username: &str, password: &str) -> Result<Session> {
    // Implementation
}
```

### Generated Documentation
```markdown
# Module: auth

[View Source](https://github.com/org/repo/blob/main/src/auth/mod.rs)

Authentication and authorization module

Provides secure user authentication and permission management.

## Example

```rust
use pcf_api::auth::{Session, authenticate};

let session = authenticate(username, password).await?;
```

## Functions

### authenticate

Authenticates a user with credentials

**Security**: Rate limited to 5 attempts per minute

**See also**: [Session](./session.md)

```rust
pub async fn authenticate(username: &str, password: &str) -> Result<Session>
```
```

## Installation
```bash
cargo install mdbook-auto-doc
```

## Integration with CI/CD

```yaml
# .github/workflows/docs.yml
- name: Generate API documentation
  run: |
    cargo doc --no-deps
    mdbook build
```

## Testing Strategy
1. Unit tests for doc extraction
2. Integration tests with various Rust code patterns
3. Template rendering tests
4. Change detection tests
5. Performance tests with large codebases

## Future Enhancements
1. Support for workspace documentation
2. Cross-crate documentation links
3. Trait implementation tracking
4. Example validation (compile checks)
5. Documentation coverage reporting
6. IDE integration for live preview