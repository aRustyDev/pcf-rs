# GraphQL Playground

The GraphQL Playground provides an interactive interface for exploring and testing the PCF API's GraphQL schema.

<div class="warning">
The interactive playground below connects to a mock server for documentation purposes.
To connect to your own instance, update the endpoint URL.
</div>

## Interactive Playground

<div id="graphql-playground-container">
  <!-- Placeholder for GraphQL Playground -->
  <div class="playground-mock" style="border: 1px solid #ddd; border-radius: 4px; background: #f8f9fa;">
    <div class="playground-header" style="background: #343a40; color: white; padding: 10px; border-radius: 4px 4px 0 0;">
      <input type="text" value="https://mock.pcf-api.org/graphql" class="endpoint-url" style="width: 70%; padding: 5px; margin-right: 10px;" />
      <button class="play-button" style="background: #28a745; color: white; border: none; padding: 5px 15px; border-radius: 3px; cursor: pointer;">▶ Execute</button>
    </div>
    <div class="playground-body" style="display: flex; height: 500px;">
      <div class="query-editor" style="flex: 1; border-right: 1px solid #ddd; padding: 10px;">
        <h4 style="margin-top: 0;">Query</h4>
        <pre style="background: #f8f9fa; padding: 10px; overflow: auto; height: 400px;"><code>query GetNotes {
  notes(first: 10) {
    edges {
      node {
        id
        title
        content
        createdAt
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}</code></pre>
      </div>
      <div class="response-viewer" style="flex: 1; padding: 10px;">
        <h4 style="margin-top: 0;">Response</h4>
        <pre style="background: #f8f9fa; padding: 10px; overflow: auto; height: 400px;"><code>{
  "data": {
    "notes": {
      "edges": [
        {
          "node": {
            "id": "01234567-89ab-cdef-0123-456789abcdef",
            "title": "Example Note",
            "content": "This is a mock response for documentation",
            "createdAt": "2024-01-01T00:00:00Z"
          }
        },
        {
          "node": {
            "id": "fedcba98-7654-3210-fedc-ba9876543210",
            "title": "Another Note",
            "content": "This demonstrates pagination support",
            "createdAt": "2024-01-02T00:00:00Z"
          }
        }
      ],
      "pageInfo": {
        "hasNextPage": true,
        "endCursor": "eyJpZCI6MiwiY3JlYXRlZEF0IjoiMjAyNC0wMS0wMlQwMDowMDowMFoifQ=="
      }
    }
  }
}</code></pre>
      </div>
    </div>
  </div>
</div>

<!-- mdbook-graphql-playground:
  endpoint: "https://mock.pcf-api.org/graphql"
  mock_data: embedded
  headers: {
    "Authorization": "Bearer <your-token>"
  }
  default_query: "query GetNotes { notes(first: 10) { edges { node { id title content createdAt } } pageInfo { hasNextPage endCursor } } }"
-->

## Example Queries

### Basic Queries

#### Health Check
```graphql
query HealthCheck {
  health {
    status
    timestamp
    version
  }
}
```

#### Get Single Note
```graphql
query GetNote($id: ID!) {
  note(id: $id) {
    id
    title
    content
    createdAt
    updatedAt
  }
}
```

Variables:
```json
{
  "id": "01234567-89ab-cdef-0123-456789abcdef"
}
```

#### List Notes with Pagination
```graphql
query ListNotes($first: Int!, $after: String) {
  notes(first: $first, after: $after) {
    edges {
      cursor
      node {
        id
        title
        content
        createdAt
      }
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
    totalCount
  }
}
```

Variables:
```json
{
  "first": 10,
  "after": null
}
```

### Advanced Queries

#### Filtered Queries
```graphql
query FilteredNotes($filter: NoteFilter!) {
  notes(filter: $filter, first: 10) {
    edges {
      node {
        id
        title
        content
        tags
        createdAt
      }
    }
  }
}
```

Variables:
```json
{
  "filter": {
    "title": { "contains": "API" },
    "createdAt": { "after": "2024-01-01T00:00:00Z" }
  }
}
```

#### Sorted Results
```graphql
query SortedNotes($orderBy: NoteOrderBy!) {
  notes(orderBy: $orderBy, first: 10) {
    edges {
      node {
        id
        title
        createdAt
      }
    }
  }
}
```

Variables:
```json
{
  "orderBy": {
    "field": "CREATED_AT",
    "direction": "DESC"
  }
}
```

### Mutations

#### Create Note
```graphql
mutation CreateNote($input: CreateNoteInput!) {
  createNote(input: $input) {
    note {
      id
      title
      content
      createdAt
    }
  }
}
```

Variables:
```json
{
  "input": {
    "title": "New Note",
    "content": "This is a new note created via GraphQL",
    "tags": ["api", "graphql"]
  }
}
```

#### Update Note
```graphql
mutation UpdateNote($id: ID!, $input: UpdateNoteInput!) {
  updateNote(id: $id, input: $input) {
    note {
      id
      title
      content
      updatedAt
    }
  }
}
```

Variables:
```json
{
  "id": "01234567-89ab-cdef-0123-456789abcdef",
  "input": {
    "title": "Updated Title",
    "content": "Updated content"
  }
}
```

#### Delete Note
```graphql
mutation DeleteNote($id: ID!) {
  deleteNote(id: $id) {
    success
    message
  }
}
```

Variables:
```json
{
  "id": "01234567-89ab-cdef-0123-456789abcdef"
}
```

### Subscriptions

#### Note Created
```graphql
subscription OnNoteCreated {
  noteCreated {
    id
    title
    content
    createdAt
  }
}
```

#### Note Updated
```graphql
subscription OnNoteUpdated($id: ID) {
  noteUpdated(id: $id) {
    id
    title
    content
    updatedAt
  }
}
```

#### Note Deleted
```graphql
subscription OnNoteDeleted {
  noteDeleted {
    id
    deletedAt
  }
}
```

## Headers Configuration

Most operations require authentication. Set your authorization header:

```json
{
  "Authorization": "Bearer YOUR_JWT_TOKEN"
}
```

Additional headers for specific use cases:

```json
{
  "X-Request-ID": "unique-request-id",
  "X-Client-Version": "1.0.0"
}
```

## Variables

The playground supports GraphQL variables. Define them in the variables panel:

```json
{
  "first": 10,
  "filter": {
    "status": "ACTIVE"
  }
}
```

## Connecting to Your Instance

To connect to your own PCF API instance:

1. Replace the endpoint URL with your API endpoint
2. Add your authentication token in the headers
3. Ensure CORS is configured to allow playground access
4. In production, consider disabling introspection

## Troubleshooting

### Common Issues

1. **"Unauthorized" errors**
   - Check your authentication token is valid
   - Ensure the token has necessary permissions

2. **"Network Error"**
   - Verify the endpoint URL is correct
   - Check CORS configuration
   - Ensure the API is running

3. **"Query complexity exceeded"**
   - Simplify your query
   - Reduce the number of requested fields
   - Check the complexity limits in documentation

## Security Note

In production environments:
- Disable GraphQL introspection
- Use proper authentication
- Implement query depth limiting
- Monitor for malicious queries

<!-- Future Enhancement: Live Playground
When mdbook-graphql-playground plugin is available:
- Fully interactive GraphQL IDE
- Schema documentation sidebar
- Query history
- Variable and header editors
- Subscription support
-->