use anyhow::Result;
use figment::{Figment, providers::{Env, Format, Toml, Serialized}};
use garde::Validate;
use clap::Parser;

use super::AppConfig;

#[derive(Parser, Clone, serde::Serialize)]
pub struct Cli {
    /// Path to configuration file
    #[arg(long, env = "APP_CONFIG")]
    pub config: Option<std::path::PathBuf>,
    
    /// Server port
    #[arg(long, env = "PORT")]
    pub port: Option<u16>,
    
    /// Environment name
    #[arg(long, env = "ENVIRONMENT")]
    pub environment: Option<String>,
    
    /// Enable debug logging
    #[arg(long)]
    pub debug: bool,
    
    /// Run health check and exit
    #[arg(long)]
    pub healthcheck: bool,
}

/// Load configuration with 4-tier hierarchy as specified in configuration.md
/// TODO: Use in main once server is implemented in Checkpoint 4
pub fn load_config() -> Result<AppConfig> {
    let cli = Cli::parse();
    let env_name = cli.environment.clone().unwrap_or_else(|| 
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string())
    );
    
    let figment = Figment::new()
        // 1. Embedded defaults (lowest priority)
        .merge(Serialized::defaults(AppConfig::default()))
        // 2. Default config file
        .merge(Toml::file("config/default.toml").nested())
        // 3. Environment-specific config
        .merge(Toml::file(format!("config/{}.toml", env_name)).nested())
        // 4. Environment variables with APP_ prefix
        .merge(Env::prefixed("APP_").split("__"))
        // 5. CLI arguments (highest priority)
        .merge(Serialized::defaults(&cli));
        
    let config: AppConfig = figment.extract()?;
    
    // Validate with Garde
    config.validate()?;
    
    Ok(config)
}