# GraphQL API Endpoint

Complete reference for the PCF GraphQL API endpoint, including schema overview, authentication, queries, mutations, and subscriptions.

<!-- toc -->

## Overview

The PCF API provides a powerful GraphQL interface that allows clients to request exactly the data they need. GraphQL provides a complete and understandable description of the data in the API, gives clients the power to ask for exactly what they need and nothing more, and enables powerful developer tools.

## Endpoint

```
Production: https://api.pcf.example.com/graphql
Staging:    https://api-staging.pcf.example.com/graphql  
Local:      http://localhost:8080/graphql
```

## GraphQL Playground

In development and staging environments, GraphQL Playground is available at:

```
https://api-staging.pcf.example.com/graphql
```

## Authentication

All GraphQL requests require authentication using Bearer tokens in the Authorization header:

```bash
curl -X POST https://api.pcf.example.com/graphql \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ viewer { id email name } }"
  }'
```

## Request Format

### Basic Query

```json
{
  "query": "query GetUser($id: ID!) { user(id: $id) { id name email } }",
  "variables": {
    "id": "user_123"
  }
}
```

### With Operation Name

```json
{
  "query": "query GetUserDetails($id: ID!) { user(id: $id) { id name email } }",
  "operationName": "GetUserDetails",
  "variables": {
    "id": "user_123"
  }
}
```

## Response Format

### Success Response

```json
{
  "data": {
    "user": {
      "id": "user_123",
      "name": "John Doe",
      "email": "john@example.com"
    }
  },
  "extensions": {
    "requestId": "req_abc123",
    "queryComplexity": 5,
    "throttle": {
      "requestsRemaining": 999,
      "resetAt": "2024-01-15T11:00:00Z"
    }
  }
}
```

### Error Response

```json
{
  "errors": [
    {
      "message": "User not found",
      "path": ["user"],
      "locations": [
        {
          "line": 2,
          "column": 3
        }
      ],
      "extensions": {
        "code": "NOT_FOUND",
        "timestamp": "2024-01-15T10:30:00Z"
      }
    }
  ],
  "data": null
}
```

## Schema Overview

### Root Types

```graphql
schema {
  query: Query
  mutation: Mutation
  subscription: Subscription
}
```

### Scalar Types

```graphql
# Built-in scalars
scalar Int
scalar Float
scalar String
scalar Boolean
scalar ID

# Custom scalars
scalar DateTime
scalar Date
scalar Time
scalar JSON
scalar URL
scalar EmailAddress
scalar UUID
```

## Common Queries

### Get Current User

```graphql
query GetCurrentUser {
  viewer {
    id
    email
    name
    role
    organization {
      id
      name
    }
    createdAt
    updatedAt
  }
}
```

### Get User by ID

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    id
    email
    name
    role
    organization {
      id
      name
    }
    resources(first: 10) {
      edges {
        node {
          id
          name
          status
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
    }
  }
}
```

### List Resources with Pagination

```graphql
query ListResources(
  $first: Int!
  $after: String
  $filter: ResourceFilter
  $orderBy: ResourceOrder
) {
  resources(
    first: $first
    after: $after
    filter: $filter
    orderBy: $orderBy
  ) {
    edges {
      cursor
      node {
        id
        name
        description
        status
        metadata
        owner {
          id
          name
        }
        tags {
          id
          name
        }
        createdAt
        updatedAt
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

### Search Resources

```graphql
query SearchResources(
  $query: String!
  $first: Int!
  $filters: SearchFilters
) {
  search(
    query: $query
    first: $first
    filters: $filters
  ) {
    results {
      ... on Resource {
        id
        name
        description
        highlights {
          field
          snippet
        }
      }
      ... on Document {
        id
        title
        content
        highlights {
          field
          snippet
        }
      }
    }
    facets {
      type {
        value
        count
      }
      status {
        value
        count
      }
    }
    totalCount
  }
}
```

## Common Mutations

### Create Resource

```graphql
mutation CreateResource($input: CreateResourceInput!) {
  createResource(input: $input) {
    resource {
      id
      name
      description
      status
      metadata
      createdAt
    }
    errors {
      field
      message
    }
  }
}
```

Variables:
```json
{
  "input": {
    "name": "New Resource",
    "description": "Resource description",
    "metadata": {
      "key1": "value1",
      "key2": "value2"
    },
    "tags": ["tag1", "tag2"]
  }
}
```

### Update Resource

```graphql
mutation UpdateResource($id: ID!, $input: UpdateResourceInput!) {
  updateResource(id: $id, input: $input) {
    resource {
      id
      name
      description
      status
      updatedAt
    }
    errors {
      field
      message
    }
  }
}
```

### Delete Resource

```graphql
mutation DeleteResource($id: ID!) {
  deleteResource(id: $id) {
    success
    message
  }
}
```

### Batch Operations

```graphql
mutation BatchCreateResources($inputs: [CreateResourceInput!]!) {
  batchCreateResources(inputs: $inputs) {
    resources {
      id
      name
      status
    }
    errors {
      index
      field
      message
    }
  }
}
```

## Subscriptions

### Resource Updates

```graphql
subscription OnResourceUpdated($resourceId: ID!) {
  resourceUpdated(resourceId: $resourceId) {
    id
    name
    status
    updatedAt
    updatedBy {
      id
      name
    }
  }
}
```

### Real-time Notifications

```graphql
subscription OnNotification {
  notification {
    id
    type
    title
    message
    severity
    createdAt
    data
  }
}
```

### WebSocket Connection

```javascript
import { createClient } from 'graphql-ws';

const client = createClient({
  url: 'wss://api.pcf.example.com/graphql',
  connectionParams: {
    authorization: 'Bearer YOUR_JWT_TOKEN',
  },
});

// Subscribe to updates
const unsubscribe = client.subscribe(
  {
    query: `
      subscription OnResourceUpdated($resourceId: ID!) {
        resourceUpdated(resourceId: $resourceId) {
          id
          name
          status
        }
      }
    `,
    variables: {
      resourceId: 'res_123',
    },
  },
  {
    next: (data) => console.log('Update:', data),
    error: (err) => console.error('Error:', err),
    complete: () => console.log('Subscription complete'),
  }
);
```

## Input Types

### Filters

```graphql
input ResourceFilter {
  status: ResourceStatus
  owner: ID
  tags: [String!]
  createdAfter: DateTime
  createdBefore: DateTime
  search: String
}

input SearchFilters {
  type: [SearchableType!]
  status: [String!]
  dateRange: DateRangeInput
  tags: [String!]
}

input DateRangeInput {
  start: DateTime!
  end: DateTime!
}
```

### Sorting

```graphql
input ResourceOrder {
  field: ResourceOrderField!
  direction: OrderDirection!
}

enum ResourceOrderField {
  CREATED_AT
  UPDATED_AT
  NAME
  STATUS
}

enum OrderDirection {
  ASC
  DESC
}
```

### Pagination

```graphql
# Forward pagination
resources(first: 10, after: "cursor_123") {
  edges {
    cursor
    node { ... }
  }
  pageInfo {
    hasNextPage
    endCursor
  }
}

# Backward pagination
resources(last: 10, before: "cursor_456") {
  edges {
    cursor
    node { ... }
  }
  pageInfo {
    hasPreviousPage
    startCursor
  }
}
```

## Error Handling

### Field Errors

```graphql
type FieldError {
  field: String!
  message: String!
  code: String!
}

type CreateResourcePayload {
  resource: Resource
  errors: [FieldError!]
}
```

### Global Errors

```json
{
  "errors": [
    {
      "message": "Authentication required",
      "extensions": {
        "code": "UNAUTHENTICATED",
        "timestamp": "2024-01-15T10:30:00Z"
      }
    }
  ]
}
```

### Error Codes

| Code | Description |
|------|-------------|
| `UNAUTHENTICATED` | Authentication required |
| `FORBIDDEN` | Insufficient permissions |
| `NOT_FOUND` | Resource not found |
| `VALIDATION_ERROR` | Input validation failed |
| `CONFLICT` | Resource conflict |
| `INTERNAL_ERROR` | Server error |
| `COMPLEXITY_EXCEEDED` | Query too complex |
| `RATE_LIMITED` | Too many requests |

## Query Complexity

Queries are limited by complexity to prevent abuse:

```graphql
# Query complexity calculation
query ComplexQuery {
  resources(first: 100) {        # Complexity: 100
    edges {
      node {
        id                       # Complexity: 1
        owner {                  # Complexity: 1
          resources(first: 10) { # Complexity: 10
            edges {
              node {
                id
              }
            }
          }
        }
      }
    }
  }
}
# Total complexity: 100 * (1 + 1 + 10) = 1200
```

Complexity limits:
- Anonymous requests: 100
- Authenticated requests: 1000
- Premium tier: 5000

## Introspection

### Get Schema

```graphql
query IntrospectionQuery {
  __schema {
    types {
      name
      kind
      description
      fields {
        name
        type {
          name
          kind
        }
      }
    }
  }
}
```

### Get Type Details

```graphql
query GetTypeDetails($typeName: String!) {
  __type(name: $typeName) {
    name
    kind
    description
    fields {
      name
      description
      type {
        name
        kind
        ofType {
          name
          kind
        }
      }
      args {
        name
        description
        type {
          name
          kind
        }
      }
    }
  }
}
```

## File Uploads

### Upload Mutation

```graphql
mutation UploadFile($file: Upload!) {
  uploadFile(file: $file) {
    file {
      id
      filename
      mimetype
      size
      url
    }
    errors {
      field
      message
    }
  }
}
```

### Multipart Request

```bash
curl -X POST https://api.pcf.example.com/graphql \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -F operations='{"query": "mutation ($file: Upload!) { uploadFile(file: $file) { file { id filename } } }", "variables": {"file": null}}' \
  -F map='{"0": ["variables.file"]}' \
  -F 0=@document.pdf
```

## Batching

### Multiple Queries

```json
[
  {
    "query": "{ viewer { id name } }",
    "operationName": "GetViewer"
  },
  {
    "query": "{ resources(first: 10) { totalCount } }",
    "operationName": "CountResources"
  }
]
```

Response:
```json
[
  {
    "data": {
      "viewer": {
        "id": "user_123",
        "name": "John Doe"
      }
    }
  },
  {
    "data": {
      "resources": {
        "totalCount": 42
      }
    }
  }
]
```

## Caching

### Cache Control

```graphql
type Resource @cacheControl(maxAge: 300) {
  id: ID!
  name: String! @cacheControl(maxAge: 3600)
  status: ResourceStatus! @cacheControl(maxAge: 0)
}
```

### Client-side Caching

```javascript
import { ApolloClient, InMemoryCache } from '@apollo/client';

const client = new ApolloClient({
  uri: 'https://api.pcf.example.com/graphql',
  cache: new InMemoryCache({
    typePolicies: {
      Resource: {
        keyFields: ['id'],
        fields: {
          status: {
            merge: false, // Don't cache status field
          },
        },
      },
    },
  }),
});
```

## Performance Tips

### 1. Use Fragments

```graphql
fragment ResourceDetails on Resource {
  id
  name
  description
  status
  createdAt
  updatedAt
}

query GetResources {
  resources(first: 10) {
    edges {
      node {
        ...ResourceDetails
      }
    }
  }
}
```

### 2. Avoid Deep Nesting

```graphql
# Bad - too deep
query DeepQuery {
  user(id: "123") {
    resources {
      edges {
        node {
          owner {
            resources {
              edges {
                node {
                  owner {
                    # Too deep!
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}

# Good - flattened
query FlatQuery {
  user(id: "123") {
    id
    name
  }
  userResources(userId: "123", first: 10) {
    edges {
      node {
        id
        name
        ownerId
      }
    }
  }
}
```

### 3. Use Field Selection

```graphql
# Request only needed fields
query MinimalQuery {
  resources(first: 100) {
    edges {
      node {
        id
        name
        # Don't request fields you don't need
      }
    }
  }
}
```

## SDK Examples

### JavaScript/Apollo

```javascript
import { ApolloClient, gql } from '@apollo/client';

const client = new ApolloClient({
  uri: 'https://api.pcf.example.com/graphql',
  headers: {
    authorization: 'Bearer YOUR_TOKEN',
  },
});

// Query
const { data } = await client.query({
  query: gql`
    query GetResource($id: ID!) {
      resource(id: $id) {
        id
        name
        status
      }
    }
  `,
  variables: { id: 'res_123' },
});

// Mutation
const { data } = await client.mutate({
  mutation: gql`
    mutation CreateResource($input: CreateResourceInput!) {
      createResource(input: $input) {
        resource {
          id
          name
        }
      }
    }
  `,
  variables: {
    input: {
      name: 'New Resource',
      description: 'Description',
    },
  },
});
```

### Python/GQL

```python
from gql import gql, Client
from gql.transport.aiohttp import AIOHTTPTransport

transport = AIOHTTPTransport(
    url="https://api.pcf.example.com/graphql",
    headers={"Authorization": "Bearer YOUR_TOKEN"}
)

client = Client(transport=transport, fetch_schema_from_transport=True)

# Query
query = gql("""
    query GetResource($id: ID!) {
        resource(id: $id) {
            id
            name
            status
        }
    }
""")

result = await client.execute_async(
    query,
    variable_values={"id": "res_123"}
)
```

## Best Practices

1. **Use Fragments** - Reuse common field selections
2. **Batch Requests** - Combine multiple queries when possible
3. **Implement Caching** - Cache responses client-side
4. **Handle Errors** - Check both data and errors in responses
5. **Use Variables** - Never concatenate strings into queries
6. **Limit Complexity** - Avoid deeply nested queries
7. **Paginate Results** - Use cursor-based pagination for large sets
8. **Subscribe Wisely** - Only subscribe to necessary updates
9. **Monitor Performance** - Track query execution time
10. **Version Carefully** - Use field deprecation instead of breaking changes
