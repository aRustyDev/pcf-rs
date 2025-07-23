use async_graphql::{
    Context, EmptySubscription, FieldError, ID, InputObject, Object, Result, Schema,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    Json, Router,
    extract::Extension,
    response::Html,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use surrealdb::{
    Surreal,
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    sql::Thing,
};
use tower_http::cors::CorsLayer;

// Helper function to serialize Thing to ID
fn thing_to_id(thing: &Thing) -> ID {
    ID(format!("{}:{}", thing.tb, thing.id.to_raw()))
}

// GraphQL Schema Types
#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    #[serde(rename = "id")]
    record_id: Thing,
    name: String,
    email: String,
    created_at: String,
}

#[Object]
impl User {
    async fn id(&self) -> ID {
        thing_to_id(&self.record_id)
    }
    
    async fn name(&self) -> &str {
        &self.name
    }
    
    async fn email(&self) -> &str {
        &self.email
    }
    
    async fn created_at(&self) -> &str {
        &self.created_at
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Product {
    #[serde(rename = "id")]
    record_id: Thing,
    name: String,
    price: f64,
    description: Option<String>,
}

#[Object]
impl Product {
    async fn id(&self) -> ID {
        thing_to_id(&self.record_id)
    }
    
    async fn name(&self) -> &str {
        &self.name
    }
    
    async fn price(&self) -> f64 {
        self.price
    }
    
    async fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Purchase {
    #[serde(rename = "id")]
    record_id: Thing,
    #[serde(rename = "in")]
    in_id: Thing,
    #[serde(rename = "out")]
    out_id: Thing,
    quantity: i32,
    timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    product: Option<Product>,
}

#[Object]
impl Purchase {
    async fn id(&self) -> ID {
        thing_to_id(&self.record_id)
    }
    
    #[graphql(name = "userId")]
    async fn user_id(&self) -> ID {
        thing_to_id(&self.in_id)
    }
    
    #[graphql(name = "productId")]
    async fn product_id(&self) -> ID {
        thing_to_id(&self.out_id)
    }
    
    async fn quantity(&self) -> i32 {
        self.quantity
    }
    
    async fn timestamp(&self) -> &str {
        &self.timestamp
    }
    
    async fn user(&self) -> Option<&User> {
        self.user.as_ref()
    }
    
    async fn product(&self) -> Option<&Product> {
        self.product.as_ref()
    }
}

// Input Types
#[derive(InputObject)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[derive(InputObject)]
struct CreateProductInput {
    name: String,
    price: f64,
    description: Option<String>,
}

#[derive(InputObject)]
struct CreatePurchaseInput {
    user_id: String,
    product_id: String,
    quantity: i32,
}

// Database connection wrapper
#[derive(Clone)]
struct Database {
    client: Arc<Surreal<Client>>,
}

impl Database {
    async fn new() -> Result<Self> {
        println!("🔄 Connecting to SurrealDB at 127.0.0.1:8000...");
        
        // Connect to SurrealDB
        let db = match Surreal::new::<Ws>("127.0.0.1:8000").await {
            Ok(db) => {
                println!("✅ Connected to SurrealDB");
                db
            }
            Err(e) => {
                eprintln!("❌ Failed to connect to SurrealDB: {}", e);
                eprintln!("💡 Make sure SurrealDB is running with:");
                eprintln!("   SURREAL_CAPS_ALLOW_EXPERIMENTAL=graphql surreal start --user root --password root");
                return Err(e.into());
            }
        };

        // Sign in as root
        println!("🔐 Signing in as root...");
        db.signin(Root {
            username: "root",
            password: "root",
        })
        .await
        .map_err(|e| {
            eprintln!("❌ Failed to sign in: {}", e);
            e
        })?;
        println!("✅ Signed in successfully");

        // Select namespace and database
        println!("📁 Selecting namespace 'test' and database 'test'...");
        db.use_ns("test").use_db("test").await
            .map_err(|e| {
                eprintln!("❌ Failed to select namespace/database: {}", e);
                e
            })?;
        println!("✅ Namespace and database selected");

        // Initialize schema
        println!("🏗️  Initializing database schema...");
        Self::init_schema(&db).await?;
        println!("✅ Schema initialized");

        Ok(Database {
            client: Arc::new(db),
        })
    }

    async fn init_schema(db: &Surreal<Client>) -> Result<()> {
        // Define tables with schema
        db.query(
            "
            DEFINE TABLE user SCHEMAFULL;
            DEFINE FIELD name ON TABLE user TYPE string;
            DEFINE FIELD email ON TABLE user TYPE string;
            DEFINE FIELD created_at ON TABLE user TYPE datetime DEFAULT time::now();
            DEFINE INDEX email_idx ON TABLE user COLUMNS email UNIQUE;

            DEFINE TABLE product SCHEMAFULL;
            DEFINE FIELD name ON TABLE product TYPE string;
            DEFINE FIELD price ON TABLE product TYPE number;
            DEFINE FIELD description ON TABLE product TYPE option<string>;

            DEFINE TABLE purchase TYPE RELATION IN user OUT product;
            DEFINE FIELD quantity ON TABLE purchase TYPE number;
            DEFINE FIELD timestamp ON TABLE purchase TYPE datetime DEFAULT time::now();

            -- Enable GraphQL for tables
            DEFINE CONFIG GRAPHQL AUTO;
        ",
        )
        .await?;

        Ok(())
    }
}

// Query Root
struct QueryRoot;

#[Object]
impl QueryRoot {
    // Get all users
    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<User>> {
        let db = ctx.data::<Database>()?;

        let users: Vec<User> = db.client.query("SELECT * FROM user").await?.take(0)?;

        Ok(users)
    }

    // Get user by ID
    async fn user(&self, ctx: &Context<'_>, id: ID) -> Result<Option<User>> {
        let db = ctx.data::<Database>()?;

        let result: Option<User> = db.client.select(("user", id.to_string())).await?;

        Ok(result)
    }

    // Get all products
    async fn products(&self, ctx: &Context<'_>) -> Result<Vec<Product>> {
        let db = ctx.data::<Database>()?;

        let products: Vec<Product> = db.client.query("SELECT * FROM product").await?.take(0)?;

        Ok(products)
    }

    // Get user's purchases with related data
    async fn user_purchases(&self, ctx: &Context<'_>, user_id: String) -> Result<Vec<Purchase>> {
        let db = ctx.data::<Database>()?;

        // Query purchases with related user and product data
        let query = format!(
            "SELECT
                id,
                in as user_id,
                out as product_id,
                quantity,
                timestamp,
                <-purchase.* as user,
                ->purchase.* as product
            FROM purchase
            WHERE in = user:{}",
            user_id
        );

        let purchases: Vec<Purchase> = db.client.query(&query).await?.take(0)?;

        Ok(purchases)
    }

    // Get products purchased by a user (using graph traversal)
    async fn user_purchased_products(
        &self,
        ctx: &Context<'_>,
        user_id: String,
    ) -> Result<Vec<Product>> {
        let db = ctx.data::<Database>()?;

        // Use SurrealDB's graph syntax to traverse relationships
        let query = format!("SELECT ->purchase->product.* FROM user:{}", user_id);

        let products: Vec<Product> = db.client.query(&query).await?.take(0)?;

        Ok(products)
    }
}

// Mutation Root
struct MutationRoot;

#[Object]
impl MutationRoot {
    // Create a new user
    async fn create_user(&self, ctx: &Context<'_>, input: CreateUserInput) -> Result<User> {
        let db = ctx.data::<Database>()?;

        let user: Option<User> = db
            .client
            .create("user")
            .content(serde_json::json!({
                "name": input.name,
                "email": input.email,
            }))
            .await?;

        user.ok_or_else(|| FieldError::new("Failed to create user"))
    }

    // Create a new product
    async fn create_product(
        &self,
        ctx: &Context<'_>,
        input: CreateProductInput,
    ) -> Result<Product> {
        let db = ctx.data::<Database>()?;

        let product: Option<Product> = db
            .client
            .create("product")
            .content(serde_json::json!({
                "name": input.name,
                "price": input.price,
                "description": input.description,
            }))
            .await?;

        product.ok_or_else(|| FieldError::new("Failed to create product"))
    }

    // Create a purchase relationship between user and product
    async fn create_purchase(
        &self,
        ctx: &Context<'_>,
        input: CreatePurchaseInput,
    ) -> Result<Purchase> {
        let db = ctx.data::<Database>()?;

        // Use RELATE to create the edge
        let query = format!(
            "RELATE user:{} -> purchase -> product:{}
             SET quantity = {},
                 timestamp = time::now()
             RETURN id, in as user_id, out as product_id, quantity, timestamp",
            input.user_id, input.product_id, input.quantity
        );

        let mut result = db.client.query(&query).await?;
        let purchase: Option<Purchase> = result.take(0)?;

        purchase.ok_or_else(|| FieldError::new("Failed to create purchase"))
    }

    // Update user details
    async fn update_user(
        &self,
        ctx: &Context<'_>,
        id: String,
        name: Option<String>,
        email: Option<String>,
    ) -> Result<User> {
        let db = ctx.data::<Database>()?;

        let mut updates = Vec::new();
        if let Some(name) = name {
            updates.push(format!("name = '{}'", name));
        }
        if let Some(email) = email {
            updates.push(format!("email = '{}'", email));
        }

        if updates.is_empty() {
            return Err(FieldError::new("No fields to update"));
        }

        let query = format!("UPDATE user:{} SET {}", id, updates.join(", "));

        let mut result = db.client.query(&query).await?;
        let user: Option<User> = result.take(0)?;

        user.ok_or_else(|| FieldError::new("User not found"))
    }

    // Delete a purchase relationship
    async fn delete_purchase(&self, ctx: &Context<'_>, purchase_id: String) -> Result<bool> {
        let db = ctx.data::<Database>()?;

        let query = format!("DELETE purchase:{}", purchase_id);
        db.client.query(&query).await?;

        Ok(true)
    }
}

// GraphQL handler
async fn graphql_handler(
    Extension(schema): Extension<Schema<QueryRoot, MutationRoot, EmptySubscription>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

// GraphQL playground
async fn graphql_playground() -> Html<&'static str> {
    Html(
        r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>GraphQL Playground</title>
        <link rel="stylesheet" href="https://unpkg.com/graphiql/graphiql.min.css" />
        <script crossorigin src="https://unpkg.com/react/umd/react.production.min.js"></script>
        <script crossorigin src="https://unpkg.com/react-dom/umd/react-dom.production.min.js"></script>
        <script crossorigin src="https://unpkg.com/graphiql/graphiql.min.js"></script>
    </head>
    <body style="margin: 0;">
        <div id="graphiql" style="height: 100vh;"></div>
        <script>
            const fetcher = GraphiQL.createFetcher({ url: '/graphql' });
            ReactDOM.render(
                React.createElement(GraphiQL, { fetcher: fetcher }),
                document.getElementById('graphiql'),
            );
        </script>
    </body>
    </html>
    "#,
    )
}

// Health check endpoint
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    println!("✅ Tracing initialized");

    // Initialize database
    let database = Database::new()
        .await
        .map_err(|e| anyhow::anyhow!("Database initialization error: {:?}", e))?;
    println!("✅ Database initialized");

    // Build GraphQL schema
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(database)
        .finish();
    println!("✅ Schema built");

    // Build router
    let app = Router::new()
        .route("/", get(graphql_playground))
        .route("/graphql", post(graphql_handler))
        .route("/health", get(health))
        .layer(Extension(schema))
        .layer(CorsLayer::permissive());
    println!("✅ Router built");

    // Start server
    println!("🚀 GraphQL playground: http://localhost:3001");
    println!("📊 GraphQL endpoint: http://localhost:3001/graphql");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
