use figment::{Figment, providers::{Format, Toml, Env, Serialized}};
use serde::{Deserialize, Serialize};
use clap::Parser;

/// CLI arguments for the application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Path to configuration file
    #[arg(long, env = "APP_CONFIG_PATH")]
    pub config_path: Option<String>,
    
    /// Server port
    #[arg(long, env = "APP_PORT")]
    pub port: Option<u16>,
    
    /// Environment (development, staging, production)
    #[arg(long, env = "APP_ENVIRONMENT", default_value = "development")]
    pub environment: String,
}

/// Main application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub services: ServicesConfig,
    pub graphql: GraphQLConfig,
    pub security: SecurityConfig,
    pub features: FeatureFlags,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

/// Database configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub surrealdb: SurrealDBConfig,
}

/// SurrealDB specific configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SurrealDBConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub namespace: String,
    pub database: String,
    pub connection_timeout: u64,
}

/// External services configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServicesConfig {
    pub spicedb: SpiceDBConfig,
    pub kratos: KratosConfig,
}

/// SpiceDB configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpiceDBConfig {
    pub url: String,
    pub token: String,
    pub insecure: bool,
}

/// Kratos configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KratosConfig {
    pub public_url: String,
    pub admin_url: String,
    pub browser_url: String,
}

/// GraphQL specific configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphQLConfig {
    pub playground_enabled: bool,
    pub introspection_enabled: bool,
    pub max_depth: u32,
    pub max_complexity: u32,
    pub max_aliases: u32,
    pub playground_endpoint: String,
    pub graphql_endpoint: String,
}

/// Security configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub cors_origins: Vec<String>,
    pub jwt_secret: Option<String>,
    pub api_keys: Vec<ApiKeyConfig>,
}

/// API Key configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiKeyConfig {
    pub name: String,
    pub key: String,
    pub permissions: Vec<String>,
}

/// Feature flags
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeatureFlags {
    pub enable_webhooks: bool,
    pub enable_telemetry: bool,
    pub default_org_email_domain: String,
    pub default_org_id: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 4000,
                workers: None,
            },
            database: DatabaseConfig {
                surrealdb: SurrealDBConfig {
                    host: "localhost".to_string(),
                    port: 8000,
                    username: "root".to_string(),
                    password: "root".to_string(),
                    namespace: "test".to_string(),
                    database: "test".to_string(),
                    connection_timeout: 30,
                },
            },
            services: ServicesConfig {
                spicedb: SpiceDBConfig {
                    url: "localhost:50051".to_string(),
                    token: "somerandomkeyhere".to_string(),
                    insecure: true,
                },
                kratos: KratosConfig {
                    public_url: "http://localhost:4433".to_string(),
                    admin_url: "http://localhost:4434".to_string(),
                    browser_url: "http://localhost:4433".to_string(),
                },
            },
            graphql: GraphQLConfig {
                playground_enabled: true,
                introspection_enabled: true,
                max_depth: 15,
                max_complexity: 1000,
                max_aliases: 15,
                playground_endpoint: "http://localhost:4000".to_string(),
                graphql_endpoint: "http://localhost:4000/graphql".to_string(),
            },
            security: SecurityConfig {
                cors_origins: vec!["http://localhost:3000".to_string()],
                jwt_secret: None,
                api_keys: vec![],
            },
            features: FeatureFlags {
                enable_webhooks: true,
                enable_telemetry: true,
                default_org_email_domain: "@gitlab.com".to_string(),
                default_org_id: "default".to_string(),
            },
        }
    }
}

/// Load configuration from multiple sources with proper precedence
pub fn load_configuration() -> Result<AppConfig, figment::Error> {
    let cli_args = CliArgs::parse();
    
    // Start with default configuration
    let mut figment = Figment::new()
        .merge(Serialized::defaults(AppConfig::default()));
    
    // Load base configuration file if it exists
    if let Some(config_path) = &cli_args.config_path {
        figment = figment.merge(Toml::file(config_path));
    } else if std::path::Path::new("config/default.toml").exists() {
        // Only try to load default config file if it exists
        figment = figment.merge(Toml::file("config/default.toml"));
    }
    
    // Load environment-specific configuration if it exists
    let env_config_path = format!("config/{}.toml", cli_args.environment);
    if std::path::Path::new(&env_config_path).exists() {
        figment = figment.merge(Toml::file(env_config_path));
    }
    
    // Override with environment variables (APP_ prefix, __ for nesting)
    figment = figment.merge(Env::prefixed("APP_").split("__"));
    
    // Apply CLI arguments last (highest priority)
    if let Some(port) = cli_args.port {
        figment = figment.merge(("server.port", port));
    }
    
    // Extract and validate configuration
    let config: AppConfig = figment.extract()?;
    validate_config(&config, &cli_args.environment)?;
    
    Ok(config)
}

/// Validate configuration based on environment
fn validate_config(config: &AppConfig, environment: &str) -> Result<(), figment::Error> {
    if environment == "production" {
        // Enforce production security requirements
        if config.graphql.introspection_enabled {
            return Err(figment::Error::from(
                "GraphQL introspection must be disabled in production"
            ));
        }
        
        if config.graphql.playground_enabled {
            return Err(figment::Error::from(
                "GraphQL playground must be disabled in production"
            ));
        }
        
        if config.graphql.max_depth > 15 {
            return Err(figment::Error::from(
                "GraphQL max depth cannot exceed 15 in production"
            ));
        }
        
        if config.graphql.max_complexity > 1000 {
            return Err(figment::Error::from(
                "GraphQL max complexity cannot exceed 1000 in production"
            ));
        }
        
        if config.security.jwt_secret.is_none() {
            return Err(figment::Error::from(
                "JWT secret must be configured in production"
            ));
        }
    }
    
    Ok(())
}

/// Get database connection URL for SurrealDB
impl SurrealDBConfig {
    pub fn connection_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Security configuration helpers
impl GraphQLConfig {
    pub fn is_production_safe(&self) -> bool {
        !self.introspection_enabled && !self.playground_enabled
    }
}