use axum::{Router, extract::{Json, Extension}, response::IntoResponse, routing::post};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;
use tokio::sync::Mutex;
use lazy_static::lazy_static;

use crate::auth::{AuthzClient, create_relationship};
use crate::config::AppConfig;

#[derive(Debug, Deserialize)]
pub struct UserCreatedWebhook {
    identity_id: String,
    email: String,
    name: Option<NameData>,
    schema_id: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct NameData {
    first: String,
    last: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    success: bool,
    message: Option<String>,
}

// Shared database connection for webhooks
lazy_static! {
    static ref WEBHOOK_DB: Arc<Mutex<Option<Surreal<Client>>>> = Arc::new(Mutex::new(None));
    static ref WEBHOOK_SPICEDB: Arc<Mutex<Option<AuthzClient>>> = Arc::new(Mutex::new(None));
}

pub async fn init_webhook_clients(config: &AppConfig) -> anyhow::Result<()> {
    // Initialize SurrealDB using configuration
    let db = Surreal::new::<surrealdb::engine::remote::ws::Ws>(
        config.database.surrealdb.connection_url()
    ).await?;
    db.signin(surrealdb::opt::auth::Root {
        username: &config.database.surrealdb.username,
        password: &config.database.surrealdb.password,
    })
    .await?;
    db.use_ns(&config.database.surrealdb.namespace)
        .use_db(&config.database.surrealdb.database)
        .await?;

    let mut webhook_db = WEBHOOK_DB.lock().await;
    *webhook_db = Some(db);

    // Initialize SpiceDB using configuration
    let spicedb_url = config.services.spicedb.url.clone();
    let spicedb_token = config.services.spicedb.token.clone();
    let spicedb = AuthzClient::new(spicedb_url, spicedb_token).await?;

    let mut webhook_spicedb = WEBHOOK_SPICEDB.lock().await;
    *webhook_spicedb = Some(spicedb);

    Ok(())
}

pub async fn user_created_handler(
    Extension(config): Extension<AppConfig>,
    Json(payload): Json<UserCreatedWebhook>
) -> impl IntoResponse {
    // Initialize clients if not already done
    if WEBHOOK_DB.lock().await.is_none() {
        if let Err(e) = init_webhook_clients(&config).await {
            eprintln!("Failed to initialize webhook clients: {}", e);
            return Json(WebhookResponse {
                success: false,
                message: Some("Failed to initialize clients".to_string()),
            });
        }
    }

    // Create user in SurrealDB
    let webhook_db = WEBHOOK_DB.lock().await;
    if let Some(db) = webhook_db.as_ref() {
        let user_data = serde_json::json!({
            "name": payload.name.as_ref().map(|n| format!("{} {}", n.first, n.last)),
            "email": payload.email,
            "created_at": payload.created_at,
        });

        match db
            .create::<Option<serde_json::Value>>(("user", &payload.identity_id))
            .content(user_data)
            .await
        {
            Ok(_) => {
                println!("✅ User {} created in SurrealDB", payload.identity_id);

                // If user is from configured domain, add to default organization
                if payload.email.ends_with(&config.features.default_org_email_domain) {
                    let webhook_spicedb = WEBHOOK_SPICEDB.lock().await;
                    if let Some(spicedb) = webhook_spicedb.as_ref() {
                        // Create a relationship to add user to default organization
                        let relationship = create_relationship(
                            "organization",
                            &config.features.default_org_id,
                            "member",
                            "user",
                            &payload.identity_id,
                        );
                        
                        if let Err(e) = spicedb.write_relationships(vec![relationship]).await {
                            eprintln!("Failed to add user to default org: {}", e);
                        } else {
                            println!(
                                "✅ User {} added to default organization",
                                payload.identity_id
                            );
                        }
                    }
                }

                Json(WebhookResponse {
                    success: true,
                    message: None,
                })
            }
            Err(e) => {
                eprintln!("Failed to create user: {}", e);
                Json(WebhookResponse {
                    success: false,
                    message: Some(format!("Failed to create user: {}", e)),
                })
            }
        }
    } else {
        Json(WebhookResponse {
            success: false,
            message: Some("Database not initialized".to_string()),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct UserDeletedWebhook {
    identity_id: String,
}

pub async fn user_deleted_handler(
    Extension(config): Extension<AppConfig>,
    Json(payload): Json<UserDeletedWebhook>
) -> impl IntoResponse {
    let webhook_db = WEBHOOK_DB.lock().await;
    if let Some(db) = webhook_db.as_ref() {
        match db
            .delete::<Option<serde_json::Value>>(("user", &payload.identity_id))
            .await
        {
            Ok(_) => {
                println!("✅ User {} deleted from SurrealDB", payload.identity_id);

                // Note: In production, you'd want to clean up all SpiceDB relationships
                // This is a simplified example

                Json(WebhookResponse {
                    success: true,
                    message: None,
                })
            }
            Err(e) => {
                eprintln!("Failed to delete user: {}", e);
                Json(WebhookResponse {
                    success: false,
                    message: Some(format!("Failed to delete user: {}", e)),
                })
            }
        }
    } else {
        Json(WebhookResponse {
            success: false,
            message: Some("Database not initialized".to_string()),
        })
    }
}

pub fn routes() -> Router {
    Router::new()
        .route("/user-created", post(user_created_handler))
        .route("/user-deleted", post(user_deleted_handler))
}
