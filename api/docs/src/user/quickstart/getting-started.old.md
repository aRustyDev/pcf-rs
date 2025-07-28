# API User Quick Start

Get started using the PCF API in minutes! This guide covers everything you need to integrate with the PCF GraphQL API.

<!-- toc -->

## API Overview

The PCF API is a GraphQL-first API that provides:
- **Single Endpoint**: All operations through `/graphql`
- **Type Safety**: Strong typing with introspection
- **Real-time Updates**: WebSocket subscriptions
- **Efficient Queries**: Request only the data you need
- **Self-Documenting**: Built-in schema exploration

## API Endpoint

```
# Local development
http://localhost:8080/graphql

# Production
https://api.example.com/graphql
```

## Making Your First API Call

Let's start with a simple health check query to verify the API is working:

### Using cURL

```bash
# Basic health check
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ health { status version uptime } }"}'

# Pretty print with jq
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ health { status version uptime } }"}' | jq .
```

Expected response:
```json
{
  "data": {
    "health": {
      "status": "healthy",
      "version": "1.0.0",
      "uptime": 3600
    }
  }
}
```

### Using JavaScript/TypeScript

```javascript
// Using fetch API
async function queryHealth() {
  const response = await fetch('http://localhost:8080/graphql', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      query: `
        query GetHealth {
          health {
            status
            version
            uptime
            services {
              database
              cache
            }
          }
        }
      `
    })
  });
  
  const result = await response.json();
  
  if (result.errors) {
    console.error('GraphQL errors:', result.errors);
  } else {
    console.log('Health status:', result.data.health);
  }
}

// Using popular GraphQL client libraries
// With Apollo Client
import { ApolloClient, InMemoryCache, gql } from '@apollo/client';

const client = new ApolloClient({
  uri: 'http://localhost:8080/graphql',
  cache: new InMemoryCache()
});

const HEALTH_QUERY = gql`
  query GetHealth {
    health {
      status
      version
      uptime
    }
  }
`;

const result = await client.query({ query: HEALTH_QUERY });

// With graphql-request
import { request, gql } from 'graphql-request';

const query = gql`
  query GetHealth {
    health {
      status
      version
    }
  }
`;

const data = await request('http://localhost:8080/graphql', query);
```

### Using Python

```python
# Using requests
import requests
import json

def query_health():
    url = 'http://localhost:8080/graphql'
    query = '''
    query GetHealth {
        health {
            status
            version
            uptime
            services {
                database
                cache
            }
        }
    }
    '''
    
    response = requests.post(
        url,
        json={'query': query},
        headers={'Content-Type': 'application/json'}
    )
    
    result = response.json()
    
    if 'errors' in result:
        print(f"GraphQL errors: {result['errors']}")
    else:
        print(f"Health status: {result['data']['health']}")
    
    return result

# Using gql (recommended)
from gql import gql, Client
from gql.transport.requests import RequestsHTTPTransport

transport = RequestsHTTPTransport(
    url='http://localhost:8080/graphql',
    use_json=True,
)

client = Client(transport=transport, fetch_schema_from_transport=True)

query = gql('''
    query GetHealth {
        health {
            status
            version
            uptime
        }
    }
''')

result = client.execute(query)
print(result)
```

### Using Go

```go
package main

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"
)

type GraphQLRequest struct {
    Query string `json:"query"`
}

type HealthResponse struct {
    Data struct {
        Health struct {
            Status  string `json:"status"`
            Version string `json:"version"`
            Uptime  int    `json:"uptime"`
        } `json:"health"`
    } `json:"data"`
}

func main() {
    query := `
        query GetHealth {
            health {
                status
                version
                uptime
            }
        }
    `
    
    reqBody, _ := json.Marshal(GraphQLRequest{Query: query})
    
    resp, err := http.Post(
        "http://localhost:8080/graphql",
        "application/json",
        bytes.NewBuffer(reqBody),
    )
    if err != nil {
        panic(err)
    }
    defer resp.Body.Close()
    
    var result HealthResponse
    json.NewDecoder(resp.Body).Decode(&result)
    
    fmt.Printf("Health status: %+v\n", result.Data.Health)
}
```

## Common Operations

### Querying Data

#### Get a Single Note
```graphql
query GetNote($id: ID!) {
  note(id: $id) {
    id
    title
    content
    tags
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
query ListNotes($first: Int, $after: String, $filter: NoteFilter) {
  notes(first: $first, after: $after, filter: $filter) {
    edges {
      node {
        id
        title
        content
        createdAt
      }
      cursor
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
  "after": null,
  "filter": {
    "tags": ["important"]
  }
}
```

### Mutations

#### Create a Note
```graphql
mutation CreateNote($input: CreateNoteInput!) {
  createNote(input: $input) {
    id
    title
    content
    tags
    createdAt
  }
}
```

Variables:
```json
{
  "input": {
    "title": "My First Note",
    "content": "This is the content of my note.",
    "tags": ["demo", "example"]
  }
}
```

#### Update a Note
```graphql
mutation UpdateNote($id: ID!, $input: UpdateNoteInput!) {
  updateNote(id: $id, input: $input) {
    id
    title
    content
    tags
    updatedAt
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

#### Delete a Note
```graphql
mutation DeleteNote($id: ID!) {
  deleteNote(id: $id) {
    success
    message
  }
}
```

### Subscriptions

Real-time updates via WebSocket:

```graphql
subscription NoteEvents {
  noteEvents {
    event
    note {
      id
      title
      content
      createdAt
      updatedAt
    }
  }
}
```

JavaScript WebSocket example:
```javascript
import { createClient } from 'graphql-ws';

const client = createClient({
  url: 'ws://localhost:8080/graphql',
});

const unsubscribe = client.subscribe({
  query: `
    subscription NoteEvents {
      noteEvents {
        event
        note {
          id
          title
        }
      }
    }
  `,
  next: (data) => console.log('Note event:', data),
  error: (err) => console.error('Subscription error:', err),
  complete: () => console.log('Subscription complete'),
});

// Later: unsubscribe();
```

## Understanding Responses

### Successful Response Structure

```json
{
  "data": {
    "operationName": {
      // Your requested data
    }
  },
  "extensions": {
    "tracing": {
      // Performance metrics (if enabled)
    }
  }
}
```

### Error Response Structure

```json
{
  "errors": [
    {
      "message": "Human-readable error message",
      "path": ["operationName", "field"],
      "locations": [{"line": 2, "column": 3}],
      "extensions": {
        "code": "ERROR_CODE",
        "trace_id": "550e8400-e29b-41d4-a716-446655440000",
        "details": {
          // Additional error context
        }
      }
    }
  ],
  "data": null
}
```

### Common Error Codes

| Code | Description | Action |
|------|-------------|--------|
| `VALIDATION_ERROR` | Input validation failed | Check input against schema |
| `NOT_FOUND` | Resource not found | Verify ID exists |
| `UNAUTHORIZED` | Authentication required | Add auth token |
| `FORBIDDEN` | Insufficient permissions | Check user permissions |
| `INTERNAL_ERROR` | Server error | Report to support |
| `RATE_LIMITED` | Too many requests | Implement backoff |

## GraphQL Playground

The API includes an interactive GraphQL playground for exploration:

1. Navigate to `http://localhost:8080/playground`
2. Features available:
   - **Schema Explorer**: Browse all types and fields
   - **Auto-completion**: Type-aware query building
   - **Documentation**: Inline field descriptions
   - **History**: Previous queries saved locally
   - **Variables Panel**: Test with different inputs
   - **Headers Panel**: Add authentication tokens

### Useful Playground Shortcuts
- `Ctrl/Cmd + Space`: Trigger auto-completion
- `Ctrl/Cmd + Enter`: Execute query
- `Ctrl/Cmd + Shift + P`: Prettify query
- `Ctrl/Cmd + /`: Toggle comment

## Authentication

### Current Demo Mode
The demo API doesn't require authentication. All operations are available without tokens.

### Future Production Authentication

```bash
# Include JWT token in Authorization header
curl -X POST https://api.example.com/graphql \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{"query": "{ me { id email } }"}'
```

JavaScript with auth:
```javascript
const client = new ApolloClient({
  uri: 'https://api.example.com/graphql',
  headers: {
    authorization: localStorage.getItem('token') || '',
  },
  cache: new InMemoryCache()
});
```

## Rate Limiting

### Current Limits (Demo)
- No rate limiting in demo mode

### Future Production Limits
- **Anonymous**: 100 requests per minute
- **Authenticated**: 1,000 requests per minute
- **Subscription connections**: 10 per user

Rate limit headers:
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995200
```

## Best Practices

### 1. Use Variables
❌ Don't embed values in queries:
```graphql
query {
  note(id: "123") {
    title
  }
}
```

✅ Do use variables:
```graphql
query GetNote($id: ID!) {
  note(id: $id) {
    title
  }
}
```

### 2. Request Only What You Need
❌ Don't over-fetch:
```graphql
query {
  notes {
    edges {
      node {
        id
        title
        content
        tags
        metadata
        author
        history
        // ... everything
      }
    }
  }
}
```

✅ Do request specific fields:
```graphql
query {
  notes(first: 10) {
    edges {
      node {
        id
        title
      }
    }
  }
}
```

### 3. Use Fragments for Reusability
```graphql
fragment NoteCore on Note {
  id
  title
  content
  createdAt
}

query GetNote($id: ID!) {
  note(id: $id) {
    ...NoteCore
    tags
    updatedAt
  }
}

query ListNotes {
  notes(first: 10) {
    edges {
      node {
        ...NoteCore
      }
    }
  }
}
```

### 4. Handle Errors Gracefully
```javascript
const result = await client.query({ query: MY_QUERY });

if (result.errors) {
  result.errors.forEach(error => {
    console.error(`GraphQL error: ${error.message}`);
    
    // Handle specific error codes
    switch (error.extensions?.code) {
      case 'NOT_FOUND':
        // Handle not found
        break;
      case 'VALIDATION_ERROR':
        // Show validation errors
        break;
      default:
        // Generic error handling
    }
  });
}
```

## Troubleshooting

### Query Not Working
1. Check the GraphQL playground for schema
2. Verify field names are correct (case-sensitive)
3. Ensure required variables are provided
4. Check for typos in query syntax

### Connection Issues
```bash
# Test basic connectivity
curl http://localhost:8080/health

# Check if GraphQL endpoint responds
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ __typename }"}'
```

### Performance Issues
- Use pagination for large datasets
- Avoid deeply nested queries
- Request only needed fields
- Consider using DataLoader pattern

## Next Steps

Now that you can make API calls:

1. **Explore the Schema**: Use the [GraphQL Playground](http://localhost:8080/playground)
2. **Learn Query Patterns**: Read [GraphQL Best Practices](../user/graphql/best-practices.md)
3. **Handle Errors**: Study [Error Handling Guide](../user/errors/handling.md)
4. **Optimize Performance**: See [Performance Guide](../user/performance/optimization.md)
5. **Build Integrations**: Check [Client Examples](../user/examples/README.md)

## Getting Help

- 📚 [Complete API Reference](../developer/graphql/schema.md)
- 🔍 [Common Issues](../user/troubleshooting/common-issues.md)
- 💬 [GitHub Discussions]({{ github_url }}/discussions)
- 🐛 [Report API Issues]({{ github_url }}/issues/new)
- 📧 [API Support](mailto:api-support@pcf-api.org)

---

**Ready for advanced features?** Check out our [GraphQL Subscriptions Guide](../user/api-endpoints/subscriptions.md) for real-time updates!