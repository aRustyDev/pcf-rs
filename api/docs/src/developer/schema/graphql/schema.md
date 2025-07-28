# GraphQL Schema

The PCF API GraphQL schema provides a flexible and powerful interface for interacting with platform resources.

<!-- toc -->

## Schema Overview

<!-- mdbook-graphql-introspection: source=src/graphql/schema.rs -->
<!-- This placeholder will be replaced by the GraphQL introspection plugin when available -->

## Type System

### Scalar Types

The API uses the following scalar types:

- `ID` - Unique identifier
- `String` - UTF-8 character sequence
- `Int` - 32-bit signed integer
- `Float` - Double-precision floating-point
- `Boolean` - True or false
- `DateTime` - ISO 8601 date-time

### Custom Scalars

- `UUID` - Universally unique identifier
- `JSON` - Arbitrary JSON data
- `URL` - Valid URL string

## Root Types

### Query

The Query type provides read-only access to resources:

```graphql
type Query {
  # User queries
  me: User
  user(id: ID!): User
  users(filter: UserFilter, pagination: PaginationInput): UserConnection!
  
  # Resource queries
  resource(id: ID!): Resource
  resources(filter: ResourceFilter, pagination: PaginationInput): ResourceConnection!
}
```

### Mutation

The Mutation type provides write operations:

```graphql
type Mutation {
  # User mutations
  createUser(input: CreateUserInput!): CreateUserPayload!
  updateUser(id: ID!, input: UpdateUserInput!): UpdateUserPayload!
  deleteUser(id: ID!): DeleteUserPayload!
  
  # Resource mutations
  createResource(input: CreateResourceInput!): CreateResourcePayload!
  updateResource(id: ID!, input: UpdateResourceInput!): UpdateResourcePayload!
  deleteResource(id: ID!): DeleteResourcePayload!
}
```

### Subscription

The Subscription type provides real-time updates:

```graphql
type Subscription {
  # User subscriptions
  userCreated: User!
  userUpdated(id: ID!): User!
  userDeleted: ID!
  
  # Resource subscriptions
  resourceCreated: Resource!
  resourceUpdated(id: ID!): Resource!
  resourceDeleted: ID!
}
```

## Pagination

The API uses cursor-based pagination following the Relay specification:

```graphql
interface Connection {
  edges: [Edge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

interface Edge {
  node: Node!
  cursor: String!
}

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}
```

## Error Handling

Errors are returned in a structured format:

```graphql
interface Error {
  message: String!
  code: String!
  path: [String!]
}

type ValidationError implements Error {
  message: String!
  code: String!
  path: [String!]
  field: String!
  value: JSON
}
```

## Directives

### Built-in Directives

- `@deprecated(reason: String)` - Mark fields as deprecated
- `@skip(if: Boolean!)` - Conditionally skip fields
- `@include(if: Boolean!)` - Conditionally include fields

### Custom Directives

- `@auth(requires: Role!)` - Require authentication
- `@rateLimit(max: Int!, window: String!)` - Apply rate limiting
- `@validate(rules: [ValidationRule!]!)` - Input validation

<!-- mdbook-graphql-introspection: examples=true -->

## Example Queries

### Basic Query

```graphql
query GetUser {
  user(id: "123") {
    id
    name
    email
    createdAt
  }
}
```

### Paginated Query

```graphql
query ListUsers {
  users(
    filter: { role: ADMIN }
    pagination: { first: 10, after: "cursor123" }
  ) {
    edges {
      node {
        id
        name
        role
      }
      cursor
    }
    pageInfo {
      hasNextPage
      endCursor
    }
    totalCount
  }
}
```

### Mutation Example

```graphql
mutation CreateUser {
  createUser(
    input: {
      name: "John Doe"
      email: "john@example.com"
      role: USER
    }
  ) {
    user {
      id
      name
      email
    }
    errors {
      message
      field
    }
  }
}
```

### Subscription Example

```graphql
subscription WatchResources {
  resourceCreated {
    id
    name
    type
    status
    createdAt
  }
}
```
