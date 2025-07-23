# SurrealDB GraphQL API Server

A simple GraphQL API server built with Rust that interfaces with SurrealDB, supporting both regular CRUD operations and graph relationships.

## Features

- 🚀 GraphQL API with async-graphql
- 🔗 Graph relationships using SurrealDB's RELATE
- 📊 GraphQL Playground included
- 🏃 Fast async runtime with Tokio
- 🛡️ Type-safe with Rust

## Prerequisites

1. **Rust** (latest stable version)
2. **SurrealDB** running locally

## Setup

### 1. Start SurrealDB

```bash
# Enable GraphQL experimental feature
SURREAL_CAPS_ALLOW_EXPERIMENTAL=graphql surreal start --log debug --user root --password root
```

### 2. Run the API Server

```bash
# Clone the project and navigate to it
cargo run
```

The server will:
- Connect to SurrealDB at `localhost:8000`
- Create the necessary tables and schema
- Start GraphQL server at `http://localhost:3000`

## GraphQL Schema

### Types

```graphql
type User {
  id: ID!
  name: String!
  email: String!
  createdAt: String!
}

type Product {
  id: ID!
  name: String!
  price: Float!
  description: String
}

type Purchase {
  id: ID!
  userId: ID!
  productId: ID!
  quantity: Int!
  timestamp: String!
  user: User
  product: Product
}
```

### Queries

```graphql
type Query {
  # Get all users
  users: [User!]!

  # Get user by ID
  user(id: ID!): User

  # Get all products
  products: [Product!]!

  # Get user's purchases with related data
  userPurchases(userId: String!): [Purchase!]!

  # Get products purchased by a user (graph traversal)
  userPurchasedProducts(userId: String!): [Product!]!
}
```

### Mutations

```graphql
type Mutation {
  # Create a new user
  createUser(input: CreateUserInput!): User!

  # Create a new product
  createProduct(input: CreateProductInput!): Product!

  # Create a purchase relationship
  createPurchase(input: CreatePurchaseInput!): Purchase!

  # Update user details
  updateUser(id: String!, name: String, email: String): User!

  # Delete a purchase
  deletePurchase(purchaseId: String!): Boolean!
}
```

## Example Queries

### Create a User

```graphql
mutation {
  createUser(input: {
    name: "John Doe"
    email: "john@example.com"
  }) {
    id
    name
    email
  }
}
```

### Create a Product

```graphql
mutation {
  createProduct(input: {
    name: "Laptop"
    price: 999.99
    description: "High-performance laptop"
  }) {
    id
    name
    price
  }
}
```

### Create a Purchase (Graph Edge)

```graphql
mutation {
  createPurchase(input: {
    userId: "01234567890"
    productId: "98765432100"
    quantity: 2
  }) {
    id
    userId
    productId
    quantity
    timestamp
  }
}
```

### Query User's Purchases

```graphql
query {
  userPurchases(userId: "01234567890") {
    id
    quantity
    timestamp
    product {
      name
      price
    }
  }
}
```

### Graph Traversal Query

```graphql
query {
  userPurchasedProducts(userId: "01234567890") {
    id
    name
    price
    description
  }
}
```

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   GraphQL   │────▶│ Rust Server  │────▶│  SurrealDB  │
│   Client    │     │ (async-graphql)│     │             │
└─────────────┘     └──────────────┘     └─────────────┘
                           │
                           ├── GraphQL queries → SurrealDB queries
                           └── GraphQL mutations → SurrealQL/RELATE
```

## Key Implementation Details

1. **Graph Relationships**: The `create_purchase` mutation uses SurrealDB's `RELATE` statement to create edges between users and products.

2. **Query Translation**: GraphQL queries are translated to SurrealQL, allowing you to leverage SurrealDB's graph traversal syntax.

3. **Schema Definition**: Tables are defined as `SCHEMAFULL` with explicit field definitions to ensure GraphQL compatibility.

4. **Error Handling**: Proper error handling with GraphQL-friendly error messages.

## Extending the Server

### Adding New Relationships

```rust
// In your mutation
async fn create_friendship(&self, ctx: &Context<'_>, user1: String, user2: String) -> Result<Friendship> {
    let db = ctx.data::<Database>()?;

    let query = format!(
        "RELATE user:{} -> friends_with -> user:{}
         SET since = time::now()",
        user1, user2
    );

    // Execute and return
}
```

### Adding Complex Queries

```rust
// Graph traversal example
async fn recommended_products(&self, ctx: &Context<'_>, user_id: String) -> Result<Vec<Product>> {
    let db = ctx.data::<Database>()?;

    // Find products purchased by users who bought similar products
    let query = format!(
        "SELECT ->purchase->product<-purchase<-user->purchase->product
         FROM user:{}
         WHERE product != ->purchase->product",
        user_id
    );

    // Execute and return
}
```

## Production Considerations

1. **Authentication**: Add JWT or session-based auth
2. **Rate Limiting**: Implement request rate limiting
3. **Monitoring**: Add metrics and tracing
4. **Connection Pooling**: Consider using connection pools for SurrealDB
5. **Error Handling**: Enhance error messages and logging
6. **Validation**: Add input validation and sanitization

## Troubleshooting

- **Connection Failed**: Ensure SurrealDB is running on `localhost:8000`
- **GraphQL Errors**: Check the GraphQL playground for detailed error messages
- **Schema Issues**: Verify tables are created with `SCHEMAFULL` and GraphQL is enabled
