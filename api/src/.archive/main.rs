use anyhow::Result;
use async_graphql::{
    Context, EmptySubscription, FieldError, ID, InputObject, Object, Schema, http::GraphiQLSource,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    Router,
    extract::Extension,
    http::HeaderMap,
    response::Html,
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use surrealdb::sql::Thing;
use surrealdb::{
    Surreal,
    engine::remote::ws::{Client, Ws},
};
use tower_http::cors::CorsLayer;

mod auth;
mod webhooks;
mod config;

use auth::{AuthContext, AuthzClient, create_relationship, get_auth_context, require_auth};
use config::{load_configuration, AppConfig};

// Database wrapper
#[derive(Clone)]
struct Database {
    client: Surreal<Client>,
}

impl Database {
    async fn new(config: &config::SurrealDBConfig) -> Result<Self> {
        let db = Surreal::new::<Ws>(config.connection_url()).await?;
        db.signin(surrealdb::opt::auth::Root {
            username: &config.username,
            password: &config.password,
        })
        .await?;
        db.use_ns(&config.namespace).use_db(&config.database).await?;
        Ok(Self { client: db })
    }
}

// Helper to convert Thing to GraphQL ID
fn thing_to_id(thing: &Thing) -> ID {
    ID(format!("{}:{}", thing.tb, thing.id))
}

// User type
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

// Product type
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

// Purchase type
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Purchase {
    #[serde(rename = "id")]
    record_id: Thing,
    #[serde(rename = "in")]
    user: User,
    #[serde(rename = "out")]
    product: Product,
    quantity: i32,
    timestamp: String,
}

#[Object]
impl Purchase {
    async fn id(&self) -> ID {
        thing_to_id(&self.record_id)
    }

    async fn user(&self) -> &User {
        &self.user
    }

    async fn product(&self) -> &Product {
        &self.product
    }

    async fn quantity(&self) -> i32 {
        self.quantity
    }

    async fn timestamp(&self) -> &str {
        &self.timestamp
    }
}

// Input types
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

// Updated QueryRoot with authorization
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn me(&self, ctx: &Context<'_>) -> Result<Option<User>, FieldError> {
        let auth_ctx = ctx.data::<AuthContext>()?;

        if let Some(user_id) = &auth_ctx.user_id {
            let db = ctx.data::<Database>()?;
            let user: Option<User> = db.client.select(("user", user_id)).await?;
            Ok(user)
        } else {
            Ok(None)
        }
    }

    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<User>, FieldError> {
        require_auth(ctx)?;
        let db = ctx.data::<Database>()?;
        let users: Vec<User> = db.client.select("user").await?;
        Ok(users)
    }

    async fn user(&self, ctx: &Context<'_>, id: ID) -> Result<Option<User>, FieldError> {
        require_auth(ctx)?;
        let db = ctx.data::<Database>()?;
        let user: Option<User> = db.client.select(("user", id.to_string())).await?;
        Ok(user)
    }

    async fn products(&self, ctx: &Context<'_>) -> Result<Vec<Product>, FieldError> {
        require_auth(ctx)?;
        let db = ctx.data::<Database>()?;
        let products: Vec<Product> = db.client.select("product").await?;
        Ok(products)
    }

    async fn user_purchases(
        &self,
        ctx: &Context<'_>,
        user_id: String,
    ) -> Result<Vec<Purchase>, FieldError> {
        let current_user_id = require_auth(ctx)?;
        let spicedb = ctx.data::<Arc<tokio::sync::Mutex<AuthzClient>>>()?;

        // Check if user can view these purchases (owns them or has permission)
        if user_id != current_user_id {
            let _client = spicedb.lock().await;
            // AuthzClient doesn't have this exact method, need to use can_user_access_project or similar
            // For now, return false as we need to implement this in AuthzClient
            let has_permission = false;

            if !has_permission {
                return Err(FieldError::new("Not authorized to view these purchases"));
            }
        }

        let db = ctx.data::<Database>()?;
        let query = r#"
            SELECT *,
                   ->user as user,
                   ->product as product
            FROM purchase
            WHERE in = $user_id
            FETCH user, product
        "#;

        let mut response = db
            .client
            .query(query)
            .bind(("user_id", format!("user:{}", user_id)))
            .await?;

        let purchases: Vec<Purchase> = response.take(0)?;
        Ok(purchases)
    }

    async fn user_purchased_products(
        &self,
        ctx: &Context<'_>,
        user_id: String,
    ) -> Result<Vec<Product>, FieldError> {
        let current_user_id = require_auth(ctx)?;

        // Same permission check as above
        if user_id != current_user_id {
            let spicedb = ctx.data::<Arc<tokio::sync::Mutex<AuthzClient>>>()?;
            let _client = spicedb.lock().await;
            // AuthzClient doesn't have this exact method, need to use can_user_access_project or similar
            // For now, return false as we need to implement this in AuthzClient
            let has_permission = false;

            if !has_permission {
                return Err(FieldError::new("Not authorized to view these purchases"));
            }
        }

        let db = ctx.data::<Database>()?;
        let query = "SELECT ->purchase->product FROM user:$user_id";

        let mut response = db.client.query(query).bind(("user_id", user_id)).await?;

        let products: Vec<Product> = response.take(0)?;
        Ok(products)
    }

    // New query to demonstrate SpiceDB integration
    async fn my_documents(&self, ctx: &Context<'_>) -> Result<Vec<Document>, FieldError> {
        let user_id = require_auth(ctx)?;
        let spicedb = ctx.data::<Arc<tokio::sync::Mutex<AuthzClient>>>()?;
        let db = ctx.data::<Database>()?;

        // Use SpiceDB to find all documents user can view
        let client = spicedb.lock().await;
        let document_ids = client.list_viewable_documents(&user_id).await?;

        // Fetch documents from SurrealDB
        let mut documents = Vec::new();
        for doc_id in document_ids {
            if let Ok(Some(doc)) = db
                .client
                .select::<Option<Document>>(("document", &doc_id))
                .await
            {
                documents.push(doc);
            }
        }

        Ok(documents)
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_user(
        &self,
        ctx: &Context<'_>,
        input: CreateUserInput,
    ) -> Result<User, FieldError> {
        // Note: In production, users should be created through Kratos registration
        // This is here for backward compatibility
        let db = ctx.data::<Database>()?;

        let user_data = serde_json::json!({
            "name": input.name,
            "email": input.email,
            "created_at": Utc::now().to_rfc3339(),
        });

        let user: Option<User> = db.client.create("user").content(user_data).await?;

        user.ok_or_else(|| FieldError::new("Failed to create user"))
    }

    async fn create_product(
        &self,
        ctx: &Context<'_>,
        input: CreateProductInput,
    ) -> Result<Product, FieldError> {
        require_auth(ctx)?;
        let db = ctx.data::<Database>()?;

        let product_data = serde_json::json!({
            "name": input.name,
            "price": input.price,
            "description": input.description,
        });

        let product: Option<Product> = db.client.create("product").content(product_data).await?;

        product.ok_or_else(|| FieldError::new("Failed to create product"))
    }

    async fn create_purchase(
        &self,
        ctx: &Context<'_>,
        input: CreatePurchaseInput,
    ) -> Result<Purchase, FieldError> {
        let current_user_id = require_auth(ctx)?;
        let db = ctx.data::<Database>()?;
        let spicedb = ctx.data::<Arc<tokio::sync::Mutex<AuthzClient>>>()?;

        // Check if user can create purchases (must be the user themselves or have permission)
        if input.user_id != current_user_id {
            return Err(FieldError::new("Can only create purchases for yourself"));
        }

        let query = format!(
            "RELATE user:{} -> purchase -> product:{}
             SET quantity = {},
                 timestamp = time::now()",
            input.user_id, input.product_id, input.quantity
        );

        let mut result = db.client.query(&query).await?;
        let purchases: Vec<Purchase> = result.take(0)?;

        if let Some(purchase) = purchases.into_iter().next() {
            // Create permission in SpiceDB for the purchase
            let client = spicedb.lock().await;
            client
                .write_relationships(vec![create_relationship(
                    "purchase",
                    &purchase.record_id.id.to_raw(),
                    "owner",
                    "user",
                    &input.user_id,
                )])
                .await?;

            Ok(purchase)
        } else {
            Err(FieldError::new("Failed to create purchase"))
        }
    }

    // New mutation demonstrating document creation with permissions
    async fn create_document(
        &self,
        ctx: &Context<'_>,
        title: String,
        content: String,
    ) -> Result<Document, FieldError> {
        let user_id = require_auth(ctx)?;
        let db = ctx.data::<Database>()?;
        let spicedb = ctx.data::<Arc<tokio::sync::Mutex<AuthzClient>>>()?;

        // Create document in SurrealDB
        let doc_data = serde_json::json!({
            "title": title,
            "content": content,
            "owner": format!("user:{}", user_id),
            "created_at": Utc::now().to_rfc3339(),
        });

        let document: Option<Document> = db.client.create("document").content(doc_data).await?;

        if let Some(doc) = document {
            // Create ownership relationship in SpiceDB
            let client = spicedb.lock().await;
            client
                .write_relationships(vec![create_relationship(
                    "document",
                    &doc.record_id.id.to_raw(),
                    "owner",
                    "user",
                    &user_id,
                )])
                .await?;

            Ok(doc)
        } else {
            Err(FieldError::new("Failed to create document"))
        }
    }

    async fn share_document(
        &self,
        ctx: &Context<'_>,
        document_id: String,
        user_id: String,
    ) -> Result<Document, FieldError> {
        let current_user_id = require_auth(ctx)?;
        let db = ctx.data::<Database>()?;
        let spicedb = ctx.data::<Arc<tokio::sync::Mutex<AuthzClient>>>()?;

        // Check if current user can share the document
        let client = spicedb.lock().await;
        let can_share = client
            .can_user_access_document(
                &current_user_id,
                &document_id,
                auth::DocumentPermission::Share,
            )
            .await
            .map_err(|e| FieldError::new(e.to_string()))?;

        if !can_share {
            return Err(FieldError::new("Not authorized to share this document"));
        }

        // Add sharing relationship
        client
            .write_relationships(vec![create_relationship(
                "document",
                &document_id,
                "shared_with",
                "user",
                &user_id,
            )])
            .await?;

        // Return the document
        let document: Option<Document> = db.client.select(("document", &document_id)).await?;
        document.ok_or_else(|| FieldError::new("Document not found"))
    }
}

// New Document type for authorization examples
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Document {
    #[serde(rename = "id")]
    record_id: Thing,
    title: String,
    content: String,
    owner: String,
    created_at: String,
}

#[Object]
impl Document {
    async fn id(&self) -> ID {
        thing_to_id(&self.record_id)
    }

    async fn title(&self) -> &str {
        &self.title
    }

    async fn content(&self) -> &str {
        &self.content
    }

    async fn owner(&self) -> &str {
        &self.owner
    }

    async fn created_at(&self) -> &str {
        &self.created_at
    }

    // Permission fields
    async fn can_view(&self, ctx: &Context<'_>) -> Result<bool, FieldError> {
        if let Ok(user_id) = require_auth(ctx) {
            let spicedb = ctx.data::<Arc<tokio::sync::Mutex<AuthzClient>>>()?;
            let client = spicedb.lock().await;
            client
                .can_user_access_document(
                    &user_id,
                    &self.record_id.id.to_raw(),
                    auth::DocumentPermission::View,
                )
                .await
                .map_err(|e| FieldError::new(e.to_string()))
        } else {
            Ok(false)
        }
    }

    async fn can_edit(&self, ctx: &Context<'_>) -> Result<bool, FieldError> {
        if let Ok(user_id) = require_auth(ctx) {
            let spicedb = ctx.data::<Arc<tokio::sync::Mutex<AuthzClient>>>()?;
            let client = spicedb.lock().await;
            client
                .can_user_access_document(
                    &user_id,
                    &self.record_id.id.to_raw(),
                    auth::DocumentPermission::Edit,
                )
                .await
                .map_err(|e| FieldError::new(e.to_string()))
        } else {
            Ok(false)
        }
    }
}

// GraphQL playground
async fn graphql_playground(
    Extension(config): Extension<AppConfig>,
) -> Result<Html<String>, (axum::http::StatusCode, &'static str)> {
    if !config.graphql.playground_enabled {
        return Err((axum::http::StatusCode::NOT_FOUND, "GraphQL Playground is disabled"));
    }
    Ok(Html(GraphiQLSource::build().endpoint("/graphql").finish()))
}

// Health check endpoint
async fn health() -> &'static str {
    "OK"
}

// GraphQL handler with auth context
async fn graphql_handler(
    Extension(schema): Extension<Schema<QueryRoot, MutationRoot, EmptySubscription>>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let auth_context = get_auth_context(&headers).await;

    let request = req.into_inner().data(auth_context);
    schema.execute(request).await.into()
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Application error: {:?}", e);
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    println!("✅ Tracing initialized");

    // Load environment variables
    dotenv::dotenv().ok();

    // Load configuration
    let config = load_configuration()
        .map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;
    println!("✅ Configuration loaded for environment: {}", 
        std::env::var("APP_ENVIRONMENT").unwrap_or_else(|_| "development".to_string()));

    // Initialize database
    let database = Database::new(&config.database.surrealdb)
        .await
        .map_err(|e| anyhow::anyhow!("Database initialization error: {:?}", e))?;
    println!("✅ Database initialized");

    // Initialize SpiceDB client
    let spicedb_client = AuthzClient::new(
        config.services.spicedb.url.clone(),
        config.services.spicedb.token.clone()
    ).await?;
    let spicedb_client = Arc::new(tokio::sync::Mutex::new(spicedb_client));
    println!("✅ SpiceDB client initialized");

    // Build GraphQL schema with configuration
    let mut schema_builder = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(database)
        .data(spicedb_client)
        .data(config.clone());

    // Apply GraphQL security settings
    if !config.graphql.introspection_enabled {
        schema_builder = schema_builder.disable_introspection();
    }
    
    let schema = schema_builder
        .limit_depth(config.graphql.max_depth as usize)
        .limit_complexity(config.graphql.max_complexity as usize)
        .finish();
    println!("✅ Schema built");

    // Build router with webhook routes
    let app = Router::new()
        .route("/", get(graphql_playground))
        .route("/graphql", post(graphql_handler))
        .route("/health", get(health))
        .nest("/webhook", webhooks::routes())
        .layer(Extension(schema))
        .layer(Extension(config.clone()))
        .layer(
            CorsLayer::new()
                .allow_origin(
                    config.security.cors_origins
                        .iter()
                        .map(|s| s.parse().unwrap())
                        .collect::<Vec<_>>()
                )
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
        );
    println!("✅ Router built");

    // Start server
    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    println!("🚀 GraphQL playground: {}", config.graphql.playground_endpoint);
    println!("📊 GraphQL endpoint: {}", config.graphql.graphql_endpoint);
    println!("🌐 Server listening on: {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
