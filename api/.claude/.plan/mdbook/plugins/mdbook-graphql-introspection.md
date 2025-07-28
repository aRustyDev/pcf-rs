# MDBook GraphQL Introspection Plugin Specification

## Overview

This plugin automatically generates comprehensive GraphQL API documentation by introspecting the schema at build time and creating well-formatted MDBook pages.

## Plugin Name
`mdbook-graphql-introspection`

## Features

### 1. Schema Introspection
- Extract complete GraphQL schema via introspection
- Support for async-graphql framework
- Handle custom scalars and directives
- Preserve descriptions and deprecations

### 2. Documentation Generation
- Type documentation (Objects, Inputs, Enums, Interfaces, Unions)
- Query, Mutation, and Subscription operations
- Field arguments and return types
- Directive documentation
- Example queries generation

### 3. Interactive Features
- Inline GraphQL playground snippets
- Copy-to-clipboard for queries
- Schema search functionality
- Type navigation links

### 4. Integration
- Automatic schema versioning
- Breaking change detection
- SDL export option
- Federation support

## Technical Implementation

### Architecture
```
mdbook-graphql-introspection/
├── src/
│   ├── main.rs              # MDBook preprocessor entry
│   ├── introspection.rs     # GraphQL introspection client
│   ├── schema_parser.rs     # Parse introspection result
│   ├── generator.rs         # Generate markdown
│   ├── templates/
│   │   ├── type.hbs         # Type documentation template
│   │   ├── operation.hbs    # Query/Mutation template
│   │   ├── schema.hbs       # Schema overview
│   │   └── playground.hbs   # Interactive playground
│   └── helpers/
│       ├── examples.rs      # Generate example queries
│       └── navigation.rs    # Build navigation structure
├── Cargo.toml
└── README.md
```

### Configuration

In `book.toml`:
```toml
[preprocessor.graphql-introspection]
command = "mdbook-graphql-introspection"
renderer = ["html"]

[preprocessor.graphql-introspection.config]
# GraphQL endpoint for introspection
endpoint = "http://localhost:4000/graphql"

# Or use schema file
schema_file = "./schema.graphql"

# Or extract from async-graphql code
rust_source = "../src/graphql/schema.rs"

# Output directory in book
output_dir = "api/graphql"

# Include deprecated fields
include_deprecated = true

# Generate examples
generate_examples = true

# Playground configuration
playground = {
    enabled = true,
    endpoint = "/graphql",
    headers = {
        "Authorization" = "Bearer {{token}}"
    }
}

# Custom scalar descriptions
custom_scalars = {
    DateTime = "ISO 8601 date-time string",
    UUID = "UUID v4 string"
}

# Group by
group_by = "domain"  # domain, type, operation

# Generate SDL file
export_sdl = true
```

### Implementation Details

#### Async-GraphQL Integration
```rust
use async_graphql::{Schema, EmptyMutation, EmptySubscription};
use async_graphql::introspection::IntrospectionQuery;

pub struct GraphQLIntrospector {
    config: Config,
}

impl GraphQLIntrospector {
    pub async fn introspect_from_endpoint(&self) -> Result<IntrospectionResult> {
        let client = reqwest::Client::new();
        let query = IntrospectionQuery::default();
        
        let response = client
            .post(&self.config.endpoint)
            .json(&json!({
                "query": query.query(),
                "variables": {}
            }))
            .send()
            .await?;
            
        let result: IntrospectionResult = response.json().await?;
        Ok(result)
    }
    
    pub fn introspect_from_code(&self) -> Result<IntrospectionResult> {
        // Parse Rust source to extract schema
        // Use syn to parse async-graphql macros
        let schema = extract_schema_from_source(&self.config.rust_source)?;
        
        // Run introspection
        let result = schema.introspect();
        Ok(result)
    }
}
```

#### Documentation Generator
```rust
pub struct DocumentationGenerator {
    templates: Handlebars,
    config: Config,
}

impl DocumentationGenerator {
    pub fn generate(&self, schema: &Schema) -> Result<Documentation> {
        let mut docs = Documentation::new();
        
        // Generate overview
        docs.add_page("index.md", self.generate_overview(schema)?);
        
        // Generate type documentation
        for type_def in &schema.types {
            match type_def {
                TypeDefinition::Object(obj) => {
                    docs.add_page(
                        &format!("types/{}.md", obj.name.to_lowercase()),
                        self.generate_object_doc(obj)?
                    );
                }
                TypeDefinition::Interface(interface) => {
                    // Similar for interfaces
                }
                // ... other types
            }
        }
        
        // Generate operation documentation
        docs.add_page("queries.md", self.generate_queries_doc(schema)?);
        docs.add_page("mutations.md", self.generate_mutations_doc(schema)?);
        docs.add_page("subscriptions.md", self.generate_subscriptions_doc(schema)?);
        
        Ok(docs)
    }
    
    fn generate_object_doc(&self, obj: &ObjectType) -> Result<String> {
        let mut context = Context::new();
        
        context.insert("name", &obj.name);
        context.insert("description", &obj.description);
        context.insert("fields", &self.format_fields(&obj.fields));
        context.insert("interfaces", &obj.interfaces);
        
        // Generate example query
        if self.config.generate_examples {
            context.insert("example", &self.generate_example_query(obj));
        }
        
        self.templates.render("type", &context)
    }
}
```

### Generated Documentation Structure

```
api/graphql/
├── index.md                 # Schema overview
├── schema.graphql          # SDL export (optional)
├── queries.md              # All queries
├── mutations.md            # All mutations  
├── subscriptions.md        # All subscriptions
├── types/
│   ├── user.md            # User type
│   ├── post.md            # Post type
│   └── ...
├── inputs/
│   ├── createuserinput.md
│   └── ...
├── enums/
│   ├── role.md
│   └── ...
├── interfaces/
│   └── node.md
└── scalars.md             # Custom scalars
```

### Type Documentation Template

```handlebars
# {{name}}

{{#if description}}
{{description}}
{{/if}}

{{#if deprecated}}
> ⚠️ **Deprecated**: {{deprecation_reason}}
{{/if}}

## Fields

{{#each fields}}
### {{name}}

{{#if description}}
{{description}}
{{/if}}

**Type**: `{{type}}`

{{#if args}}
**Arguments**:
{{#each args}}
- `{{name}}` ({{type}}{{#if required}}, required{{/if}}): {{description}}
{{/each}}
{{/if}}

{{#if deprecated}}
> **Deprecated**: {{deprecation_reason}}
{{/if}}

{{/each}}

{{#if interfaces}}
## Implements

{{#each interfaces}}
- [`{{this}}`](../interfaces/{{lowercase this}}.md)
{{/each}}
{{/if}}

{{#if example}}
## Example Query

```graphql
{{example}}
```

{{#if playground}}
<div class="graphql-playground" data-query="{{example}}">
  <button class="try-it">Try it in Playground</button>
</div>
{{/if}}
{{/if}}

## See Also

{{#each related_types}}
- [`{{name}}`](./{{lowercase name}}.md)
{{/each}}
```

### Example Query Generation

```rust
pub struct ExampleGenerator {
    max_depth: usize,
    include_all_fields: bool,
}

impl ExampleGenerator {
    pub fn generate_query_example(&self, operation: &Field) -> String {
        let mut query = String::new();
        
        query.push_str(&format!("query Example{} {{\n", operation.name.to_pascal_case()));
        query.push_str(&format!("  {}", operation.name));
        
        // Add arguments if any
        if !operation.args.is_empty() {
            query.push_str("(");
            for (i, arg) in operation.args.iter().enumerate() {
                if i > 0 { query.push_str(", "); }
                query.push_str(&format!("{}: {}", arg.name, self.example_value(&arg.type_)));
            }
            query.push_str(")");
        }
        
        // Add selection set
        query.push_str(" {\n");
        query.push_str(&self.generate_selection_set(&operation.type_, 2));
        query.push_str("  }\n");
        query.push_str("}\n");
        
        query
    }
    
    fn example_value(&self, type_: &Type) -> String {
        match type_ {
            Type::String => "\"example\"".to_string(),
            Type::Int => "123".to_string(),
            Type::Boolean => "true".to_string(),
            Type::ID => "\"abc123\"".to_string(),
            Type::Custom(name) => match name.as_str() {
                "DateTime" => "\"2024-01-01T00:00:00Z\"".to_string(),
                "UUID" => "\"550e8400-e29b-41d4-a716-446655440000\"".to_string(),
                _ => "null".to_string(),
            }
            // ... handle other types
        }
    }
}
```

### Interactive Playground Integration

```javascript
// Client-side playground integration
class GraphQLPlayground {
    constructor(element) {
        this.element = element;
        this.query = element.dataset.query;
        this.endpoint = element.dataset.endpoint || '/graphql';
        
        this.setupPlayground();
    }
    
    setupPlayground() {
        const button = this.element.querySelector('.try-it');
        button.addEventListener('click', () => this.openPlayground());
    }
    
    openPlayground() {
        // Open modal with embedded GraphiQL
        const modal = document.createElement('div');
        modal.className = 'playground-modal';
        
        // Initialize GraphiQL
        const graphiql = React.createElement(GraphiQL, {
            fetcher: this.createFetcher(),
            query: this.query,
            defaultVariableEditorOpen: true,
        });
        
        ReactDOM.render(graphiql, modal);
        document.body.appendChild(modal);
    }
    
    createFetcher() {
        return (graphQLParams) => {
            return fetch(this.endpoint, {
                method: 'post',
                headers: {
                    'Content-Type': 'application/json',
                    // Add auth headers if configured
                },
                body: JSON.stringify(graphQLParams),
            }).then(response => response.json());
        };
    }
}
```

### Breaking Change Detection

```rust
pub struct SchemaComparator {
    previous: Schema,
    current: Schema,
}

impl SchemaComparator {
    pub fn find_breaking_changes(&self) -> Vec<BreakingChange> {
        let mut changes = Vec::new();
        
        // Check removed types
        for old_type in &self.previous.types {
            if !self.current.types.iter().any(|t| t.name == old_type.name) {
                changes.push(BreakingChange::TypeRemoved(old_type.name.clone()));
            }
        }
        
        // Check field changes
        // Check argument changes
        // Check required fields added
        
        changes
    }
}
```

## Installation
```bash
cargo install mdbook-graphql-introspection
```

## Usage Example

Generated documentation for a User type:

```markdown
# User

Represents a user in the system

## Fields

### id

Unique identifier for the user

**Type**: `ID!`

### name

User's display name

**Type**: `String!`

### email

User's email address

**Type**: `String!`

### posts

Posts created by this user

**Type**: `[Post!]!`

**Arguments**:
- `first` (Int): Number of posts to return
- `after` (String): Cursor for pagination

### role

User's role in the system

**Type**: `Role!`

## Example Query

```graphql
query ExampleUser {
  user(id: "abc123") {
    id
    name
    email
    posts(first: 10) {
      id
      title
      createdAt
    }
    role
  }
}
```

<div class="graphql-playground" data-query="...">
  <button class="try-it">Try it in Playground</button>
</div>

## See Also

- [`Post`](./post.md)
- [`Role`](../enums/role.md)
```

## Testing Strategy
1. Unit tests for introspection parsing
2. Integration tests with real GraphQL schemas
3. Template rendering tests
4. Breaking change detection tests
5. Playground functionality tests

## Future Enhancements
1. GraphQL Federation support
2. Subscription documentation with examples
3. Performance metrics inclusion
4. Query complexity visualization
5. Schema evolution timeline
6. API versioning support