# GraphQL Schema Specification

## Core Requirements

### Query Limits and Protection
- MUST implement query depth limiting (default: 15, configurable)
- MUST implement query complexity calculation (default: 1000 points)
- MUST implement field-level rate limiting for expensive operations
- MUST reject queries exceeding limits with 400 Bad Request
- SHOULD adapt limits based on available system resources
- MUST protect against malicious queries (aliases, fragments, recursion)

### Schema Validation
- MUST validate all inputs before processing
- MUST use Garde for struct-level validation
- MUST provide clear validation error messages
- MUST NOT expose internal implementation details in errors

### Performance Requirements
- Query resolution SHOULD complete within 30 seconds
- Subscription connections MUST be limited (default: 1000 per instance)
- MUST implement dataloader pattern for N+1 query prevention
- SHOULD implement query result caching where appropriate

## Demo Schema

### Domain Model: Note-Taking API

**Note Type**
```graphql
type Note {
  id: String!
  title: String!
  content: String!
  author: String!
  createdAt: DateTime!
  updatedAt: DateTime!
}

input CreateNoteInput {
  title: String!
  content: String!
  author: String!
}

input UpdateNoteInput {
  title: String
  content: String
}
```

### Operations

**Queries**
```graphql
type Query {
  # Get a single note by ID
  note(id: ID!): Note
  
  # List all notes with pagination
  # limit: 1-100 (default: 10)
  # offset: 0-10000 (default: 0)
  notes(limit: Int = 10, offset: Int = 0): NotesConnection!
  
  # List notes by author with pagination
  # limit: 1-100 (default: 10)
  # offset: 0-10000 (default: 0)
  notesByAuthor(author: String!, limit: Int = 10, offset: Int = 0): NotesConnection!
  
  # Search notes by title or content
  searchNotes(query: String!, limit: Int = 10): [Note!]!
  
  # Health check query (always available)
  health: HealthStatus!
}
```

**Mutations**
```graphql
type Mutation {
  # Create a new note
  createNote(input: CreateNoteInput!): Note!
  
  # Update an existing note
  updateNote(id: ID!, input: UpdateNoteInput!): Note!
  
  # Delete a note
  deleteNote(id: ID!): Boolean!
}
```

**Subscriptions**
```graphql
type Subscription {
  # Subscribe to new notes
  noteCreated: Note!
  
  # Subscribe to updates on a specific note or all notes
  noteUpdated(id: ID): Note!
  
  # Subscribe to note deletions
  noteDeleted: String! # Returns the ID of deleted note
}
```

## Schema Implementation Pattern

### Single Source of Truth
```rust
// schema/demo/note.rs
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(name = "Note")]
pub struct Note {
    #[graphql(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    
    #[graphql(name = "id")]
    #[serde(skip)]
    pub id_string: String,
    
    pub title: String,
    pub content: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Schema Export
- Available at `/schema` endpoint in demo mode only
- Returns GraphQL SDL format
- Can be used by GraphQL IDEs and code generators

### Input Validation

**Field-Level Validation:**
```rust
#[derive(Debug, Validate, InputObject)]
pub struct CreateNoteInput {
    #[garde(length(min = 1, max = 200))]
    #[garde(custom(no_control_chars))]
    pub title: String,
    
    #[garde(length(min = 1, max = 10000))]
    pub content: String,
    
    #[garde(length(min = 1, max = 100))]
    #[garde(custom(valid_username))]
    pub author: String,
}

#[derive(Debug, Validate, InputObject)]
pub struct UpdateNoteInput {
    #[garde(length(min = 1, max = 200))]
    #[garde(custom(no_control_chars))]
    pub title: Option<String>,
    
    #[garde(length(min = 1, max = 10000))]
    pub content: Option<String>,
}
```

**Pagination Validation:**
- `limit`: MUST be between 1 and 100
- `offset`: MUST be between 0 and 10000
- If invalid, MUST return INVALID_INPUT error
- MUST apply defaults if not provided

**ID Validation:**
- MUST match pattern: `^[a-z_]+:[a-zA-Z0-9_-]+$`
- MUST be valid SurrealDB Thing format
- MUST return NOT_FOUND for non-existent IDs

### Pagination Pattern

**Connection Type for Cursor-Based Pagination:**
```graphql
type NotesConnection {
  edges: [NoteEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type NoteEdge {
  node: Note!
  cursor: String!
}

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}
```

### Query Complexity Calculation

**Complexity Points:**
```graphql
# Base costs
type Query {
  note(id: ID!): Note                    # Cost: 1
  notes(limit: Int): NotesConnection      # Cost: limit value (max 100)
  searchNotes(query: String!): [Note!]    # Cost: 50 (expensive operation)
}

# Field costs
type Note {
  id: String!                             # Cost: 0 (free)
  title: String!                          # Cost: 0 (free)
  content: String!                        # Cost: 1 (large field)
  # Related fields would have higher costs
}
```

**Complexity Rules:**
- MUST calculate total complexity before execution
- MUST reject if complexity > configured limit
- MUST include complexity score in response headers
- SHOULD log high-complexity queries for analysis

### Query Depth Protection

```rust
// MUST implement depth limiting
const DEFAULT_MAX_DEPTH: usize = 15;
const ABSOLUTE_MAX_DEPTH: usize = 50;

// Configurable based on environment
fn get_max_depth(config: &Config) -> usize {
    match config.environment {
        Environment::Development => 20,
        Environment::Staging => 15,
        Environment::Production => 10,
    }
}
```

### Rate Limiting

**Operation-Level Limits:**
```rust
struct RateLimits {
    // Per operation type
    queries_per_minute: u32,      // Default: 1000
    mutations_per_minute: u32,    // Default: 100
    subscriptions_per_minute: u32, // Default: 10
    
    // Per expensive operation
    search_queries_per_minute: u32, // Default: 10
    bulk_operations_per_minute: u32, // Default: 5
}
```

### Subscription Management

**Connection Limits:**
- MUST limit total concurrent subscriptions per instance
- MUST limit subscriptions per client/IP
- MUST implement idle timeout (default: 30 minutes)
- MUST clean up resources on disconnect

**Resource Protection:**
```rust
struct SubscriptionLimits {
    max_connections_per_instance: usize,    // Default: 1000
    max_connections_per_client: usize,      // Default: 10
    idle_timeout: Duration,                 // Default: 30 min
    max_payload_size: usize,                // Default: 1MB
}
```

### Error Response Format

**Structured GraphQL Errors:**
```json
{
  "errors": [{
    "message": "Query depth of 18 exceeds maximum allowed depth of 15",
    "path": ["notes", "edges", "node", "relatedNotes"],
    "extensions": {
      "code": "QUERY_TOO_DEEP",
      "timestamp": "2024-01-01T00:00:00Z",
      "traceId": "550e8400-e29b-41d4-a716",
      "details": {
        "maxDepth": 15,
        "actualDepth": 18
      }
    }
  }]
}
```

### Security Requirements

**Query Security:**
- MUST prevent query batching attacks
- MUST limit alias usage (max 10 per query)
- MUST prevent introspection in production
- MUST implement query whitelisting (optional)
- MUST log suspicious query patterns

**Input Security:**
- MUST sanitize all string inputs
- MUST reject binary data in text fields
- MUST validate URLs and emails with regex
- MUST prevent SQL/NoSQL injection
- MUST escape special characters appropriately