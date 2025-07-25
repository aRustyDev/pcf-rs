// api/src/auth_v2.rs - Using spicedb-rust for type-safe authorization

use anyhow::Result;
use async_graphql::{Context, FieldError};
use axum::http::HeaderMap;
use reqwest::Client;
use serde::Deserialize;
use spicedb_rust::{SpiceDBClient, spicedb::RelationshipUpdate};
use std::sync::Arc;
use tokio::sync::RwLock;

// Define your SpiceDB schema as Rust types
#[derive(Debug, Clone)]
pub struct User;

#[derive(Debug, Clone)]
pub struct Organization;

#[derive(Debug, Clone)]
pub struct Project;

#[derive(Debug, Clone)]
pub struct Document;

#[derive(Debug, Clone)]
pub struct Purchase;

// Define relations as enums for compile-time safety
#[derive(Debug, Clone, Copy)]
pub enum OrganizationRelation {
    Admin,
    Member,
}

#[derive(Debug, Clone, Copy)]
pub enum ProjectRelation {
    Org,
    Owner,
    Contributor,
}

#[derive(Debug, Clone, Copy)]
pub enum DocumentRelation {
    Project,
    Owner,
    SharedWith,
}

#[derive(Debug, Clone, Copy)]
pub enum PurchaseRelation {
    Owner,
}

// Define permissions as enums
#[derive(Debug, Clone, Copy)]
pub enum OrganizationPermission {
    Manage,
    View,
}

#[derive(Debug, Clone, Copy)]
pub enum ProjectPermission {
    Manage,
    Contribute,
    View,
}

#[derive(Debug, Clone, Copy)]
pub enum DocumentPermission {
    Share,
    Delete,
    Edit,
    View,
}

#[derive(Debug, Clone, Copy)]
pub enum PurchasePermission {
    View,
    Delete,
}

// Implement the relation/permission traits for spicedb-rust
impl From<OrganizationRelation> for &'static str {
    fn from(rel: OrganizationRelation) -> Self {
        match rel {
            OrganizationRelation::Admin => "admin",
            OrganizationRelation::Member => "member",
        }
    }
}

impl From<ProjectRelation> for &'static str {
    fn from(rel: ProjectRelation) -> Self {
        match rel {
            ProjectRelation::Org => "org",
            ProjectRelation::Owner => "owner",
            ProjectRelation::Contributor => "contributor",
        }
    }
}

impl From<DocumentRelation> for &'static str {
    fn from(rel: DocumentRelation) -> Self {
        match rel {
            DocumentRelation::Project => "project",
            DocumentRelation::Owner => "owner",
            DocumentRelation::SharedWith => "shared_with",
        }
    }
}

impl From<DocumentPermission> for &'static str {
    fn from(perm: DocumentPermission) -> Self {
        match perm {
            DocumentPermission::Share => "share",
            DocumentPermission::Delete => "delete",
            DocumentPermission::Edit => "edit",
            DocumentPermission::View => "view",
        }
    }
}

impl From<ProjectPermission> for &'static str {
    fn from(perm: ProjectPermission) -> Self {
        match perm {
            ProjectPermission::Manage => "manage",
            ProjectPermission::Contribute => "contribute",
            ProjectPermission::View => "view",
        }
    }
}

impl From<OrganizationPermission> for &'static str {
    fn from(perm: OrganizationPermission) -> Self {
        match perm {
            OrganizationPermission::Manage => "manage",
            OrganizationPermission::View => "view",
        }
    }
}

impl From<PurchasePermission> for &'static str {
    fn from(perm: PurchasePermission) -> Self {
        match perm {
            PurchasePermission::View => "view",
            PurchasePermission::Delete => "delete",
        }
    }
}

impl From<PurchaseRelation> for &'static str {
    fn from(rel: PurchaseRelation) -> Self {
        match rel {
            PurchaseRelation::Owner => "owner",
        }
    }
}

// Actor wrapper for type-safe subject references
#[derive(Debug, Clone)]
pub struct Actor<T> {
    pub id: T,
}

impl<T: ToString> Actor<T> {
    pub fn new(id: T) -> Self {
        Self { id }
    }
}

// Kratos Session types
#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    pub id: String,
    pub active: bool,
    pub identity: Identity,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Identity {
    pub id: String,
    pub traits: IdentityTraits,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityTraits {
    pub email: String,
    pub name: Option<NameTraits>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NameTraits {
    pub first: String,
    pub last: String,
}

// Auth Context
#[derive(Clone)]
pub struct AuthContext {
    pub session: Option<Session>,
    pub user_id: Option<String>,
}

// Type-safe SpiceDB client wrapper
#[derive(Clone)]
pub struct AuthzClient {
    client: Arc<RwLock<SpiceDBClient>>,
}

impl AuthzClient {
    pub async fn new(endpoint: String, token: String) -> Result<Self> {
        // Ensure the endpoint has the proper scheme
        let endpoint = if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
            format!("http://{}", endpoint)
        } else {
            endpoint
        };
        let client = SpiceDBClient::new(&endpoint, &token).await?;
        Ok(Self {
            client: Arc::new(RwLock::new(client)),
        })
    }

    // Check document permission with type safety
    pub async fn can_user_access_document(
        &self,
        user_id: &str,
        document_id: &str,
        permission: DocumentPermission,
    ) -> Result<bool> {
        let client = self.client.read().await;

        // Using the actual spicedb-rust API with raw request
        let request = spicedb_rust::spicedb::CheckPermissionRequest {
            resource: Some(spicedb_rust::spicedb::ObjectReference {
                object_type: "document".to_string(),
                object_id: document_id.to_string(),
            }),
            permission: <DocumentPermission as Into<&'static str>>::into(permission).to_string(),
            subject: Some(spicedb_rust::spicedb::SubjectReference {
                object: Some(spicedb_rust::spicedb::ObjectReference {
                    object_type: "user".to_string(),
                    object_id: user_id.to_string(),
                }),
                optional_relation: String::new(),
            }),
            consistency: None,
            context: None,
            with_tracing: false,
        };

        // Note: spicedb-rust doesn't expose raw_check_permission, we need to use the typed API
        // For now, return false as we need to properly implement this
        Ok(false)
    }

    // Check project permission
    pub async fn can_user_access_project(
        &self,
        user_id: &str,
        project_id: &str,
        permission: ProjectPermission,
    ) -> Result<bool> {
        let client = self.client.read().await;

        // Using the actual spicedb-rust API with raw request
        let request = spicedb_rust::spicedb::CheckPermissionRequest {
            resource: Some(spicedb_rust::spicedb::ObjectReference {
                object_type: "project".to_string(),
                object_id: project_id.to_string(),
            }),
            permission: <ProjectPermission as Into<&'static str>>::into(permission).to_string(),
            subject: Some(spicedb_rust::spicedb::SubjectReference {
                object: Some(spicedb_rust::spicedb::ObjectReference {
                    object_type: "user".to_string(),
                    object_id: user_id.to_string(),
                }),
                optional_relation: String::new(),
            }),
            consistency: None,
            context: None,
            with_tracing: false,
        };

        // Note: spicedb-rust doesn't expose raw_check_permission, we need to use the typed API
        // For now, return false as we need to properly implement this
        Ok(false)
    }

    // Create document ownership
    pub async fn create_document_ownership(
        &self,
        user_id: &str,
        document_id: &str,
        project_id: &str,
    ) -> Result<()> {
        let client = self.client.write().await;

        let relationships = vec![
            // User owns document
            RelationshipUpdate {
                operation: 1, // CREATE operation
                relationship: Some(spicedb_rust::spicedb::Relationship {
                    resource: Some(spicedb_rust::spicedb::ObjectReference {
                        object_type: "document".to_string(),
                        object_id: document_id.to_string(),
                    }),
                    relation: <DocumentRelation as Into<&'static str>>::into(
                        DocumentRelation::Owner,
                    )
                    .to_string(),
                    subject: Some(spicedb_rust::spicedb::SubjectReference {
                        object: Some(spicedb_rust::spicedb::ObjectReference {
                            object_type: "user".to_string(),
                            object_id: user_id.to_string(),
                        }),
                        optional_relation: String::new(),
                    }),
                    optional_caveat: None,
                }),
            },
            // Document belongs to project
            RelationshipUpdate {
                operation: 1, // CREATE operation
                relationship: Some(spicedb_rust::spicedb::Relationship {
                    resource: Some(spicedb_rust::spicedb::ObjectReference {
                        object_type: "document".to_string(),
                        object_id: document_id.to_string(),
                    }),
                    relation: <DocumentRelation as Into<&'static str>>::into(
                        DocumentRelation::Project,
                    )
                    .to_string(),
                    subject: Some(spicedb_rust::spicedb::SubjectReference {
                        object: Some(spicedb_rust::spicedb::ObjectReference {
                            object_type: "project".to_string(),
                            object_id: project_id.to_string(),
                        }),
                        optional_relation: String::new(),
                    }),
                    optional_caveat: None,
                }),
            },
        ];

        match client.create_relationships(relationships, vec![]).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to create relationships: {}", e)),
        }
    }

    // Share document with another user
    pub async fn share_document(&self, document_id: &str, share_with_user_id: &str) -> Result<()> {
        let client = self.client.write().await;

        let relationship = RelationshipUpdate {
            operation: 2, // TOUCH operation
            relationship: Some(spicedb_rust::spicedb::Relationship {
                resource: Some(spicedb_rust::spicedb::ObjectReference {
                    object_type: "document".to_string(),
                    object_id: document_id.to_string(),
                }),
                relation: <DocumentRelation as Into<&'static str>>::into(
                    DocumentRelation::SharedWith,
                )
                .to_string(),
                subject: Some(spicedb_rust::spicedb::SubjectReference {
                    object: Some(spicedb_rust::spicedb::ObjectReference {
                        object_type: "user".to_string(),
                        object_id: share_with_user_id.to_string(),
                    }),
                    optional_relation: String::new(),
                }),
                optional_caveat: None,
            }),
        };

        match client
            .create_relationships(vec![relationship], vec![])
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to share document: {}", e)),
        }
    }

    // Make document public (readable by all)
    pub async fn make_document_public(&self, document_id: &str) -> Result<()> {
        let client = self.client.write().await;

        // Use wildcard relationship for public access
        let relationship = RelationshipUpdate {
            operation: 2, // TOUCH operation
            relationship: Some(spicedb_rust::spicedb::Relationship {
                resource: Some(spicedb_rust::spicedb::ObjectReference {
                    object_type: "document".to_string(),
                    object_id: document_id.to_string(),
                }),
                relation: <DocumentRelation as Into<&'static str>>::into(
                    DocumentRelation::SharedWith,
                )
                .to_string(),
                subject: Some(spicedb_rust::spicedb::SubjectReference {
                    object: Some(spicedb_rust::spicedb::ObjectReference {
                        object_type: "user".to_string(),
                        object_id: "*".to_string(), // Wildcard for all users
                    }),
                    optional_relation: String::new(),
                }),
                optional_caveat: None,
            }),
        };

        match client
            .create_relationships(vec![relationship], vec![])
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to make document public: {}", e)),
        }
    }

    // List all documents user can view
    pub async fn list_viewable_documents(&self, user_id: &str) -> Result<Vec<String>> {
        let _client = self.client.write().await;
        let _actor = Actor::new(user_id);

        // This would use the lookup resources API
        // The exact method depends on spicedb-rust's API
        // This is a conceptual example
        Ok(vec![]) // Placeholder
    }

    // Raw write relationships method for webhook compatibility
    pub async fn write_relationships(&self, relationships: Vec<RelationshipUpdate>) -> Result<()> {
        let client = self.client.write().await;
        match client.create_relationships(relationships, vec![]).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to write relationships: {}", e)),
        }
    }
}

// Kratos Client
pub struct KratosClient {
    client: Client,
    public_url: String,
}

impl KratosClient {
    pub fn new(public_url: String) -> Self {
        Self {
            client: Client::new(),
            public_url,
        }
    }

    pub async fn get_session(&self, cookie: &str) -> Result<Session> {
        let response = self
            .client
            .get(format!("{}/sessions/whoami", self.public_url))
            .header("Cookie", cookie)
            .send()
            .await?;

        if response.status().is_success() {
            let session = response.json::<Session>().await?;
            Ok(session)
        } else {
            Err(anyhow::anyhow!("Invalid session"))
        }
    }
}

// GraphQL helper functions
pub async fn get_auth_context(headers: &HeaderMap) -> AuthContext {
    let kratos_url =
        std::env::var("APP_SERVICES__KRATOS__PUBLIC_URL")
            .or_else(|_| std::env::var("KRATOS_PUBLIC_URL"))
            .unwrap_or_else(|_| "http://localhost:4433".to_string());
    let kratos_client = KratosClient::new(kratos_url);

    if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        if let Ok(session) = kratos_client.get_session(cookie).await {
            return AuthContext {
                user_id: Some(session.identity.id.clone()),
                session: Some(session),
            };
        }
    }

    AuthContext {
        session: None,
        user_id: None,
    }
}

pub fn require_auth(ctx: &Context<'_>) -> Result<String, FieldError> {
    let auth_ctx = ctx.data::<AuthContext>()?;

    auth_ctx
        .user_id
        .clone()
        .ok_or_else(|| FieldError::new("Not authenticated"))
}

// Example usage in a GraphQL resolver
pub async fn example_resolver(ctx: &Context<'_>) -> Result<bool, FieldError> {
    let user_id = require_auth(ctx)?;
    let authz = ctx.data::<AuthzClient>()?;

    // Type-safe permission check
    let can_edit = authz
        .can_user_access_document(&user_id, "doc123", DocumentPermission::Edit)
        .await
        .map_err(|e| FieldError::new(e.to_string()))?;

    if !can_edit {
        return Err(FieldError::new("Not authorized to edit this document"));
    }

    // Proceed with the operation
    Ok(true)
}

// Helper function for backward compatibility
pub fn create_relationship(
    resource_type: &str,
    resource_id: &str,
    relation: &str,
    subject_type: &str,
    subject_id: &str,
) -> RelationshipUpdate {
    RelationshipUpdate {
        operation: 1, // CREATE operation
        relationship: Some(spicedb_rust::spicedb::Relationship {
            resource: Some(spicedb_rust::spicedb::ObjectReference {
                object_type: resource_type.to_string(),
                object_id: resource_id.to_string(),
            }),
            relation: relation.to_string(),
            subject: Some(spicedb_rust::spicedb::SubjectReference {
                object: Some(spicedb_rust::spicedb::ObjectReference {
                    object_type: subject_type.to_string(),
                    object_id: subject_id.to_string(),
                }),
                optional_relation: String::new(),
            }),
            optional_caveat: None,
        }),
    }
}

// Note: SpiceDBClient is already imported from spicedb_rust, so we don't need to re-export

// Testing with mocks
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_permissions() {
        // Test your authorization logic
        // For real tests, you would either:
        // 1. Use a test SpiceDB instance
        // 2. Create your own mock wrapper
        // 3. Use integration tests
    }
}
