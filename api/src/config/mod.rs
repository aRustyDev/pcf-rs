pub mod models;
pub mod validation;

pub use models::*;
pub use validation::*;

#[cfg(test)]
mod tests {
    use super::*;
    use figment::{Figment, providers::{Env, Format, Toml, Serialized}};
    use garde::Validate;
    
    #[test]
    fn test_valid_config_loads() {
        // Test from configuration.md examples
        let config_toml = r#"
            [server]
            port = 8080
            bind = "0.0.0.0"
            
            [logging]
            level = "info"
            format = "json"
        "#;
        
        let config: AppConfig = Figment::new()
            .merge(Toml::string(config_toml))
            .extract()
            .expect("Should parse valid config");
            
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.bind, "0.0.0.0");
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.format, "json");
    }
    
    #[test]
    fn test_invalid_port_rejected() {
        // Port validation from configuration.md
        let config_toml = r#"
            [server]
            port = 80  # Below 1024, should fail
            bind = "0.0.0.0"
        "#;
        
        let config: Result<AppConfig, _> = Figment::new()
            .merge(Toml::string(config_toml))
            .extract();
            
        let config = config.expect("Should parse");
        let validation = config.validate();
        assert!(validation.is_err());
        assert!(validation.unwrap_err().to_string().contains("port"));
    }
    
    #[test] 
    fn test_config_hierarchy() {
        // 4-tier precedence from configuration.md
        unsafe {
            std::env::set_var("APP_SERVER__PORT", "3000");
        }
        
        let default = r#"[server]
        port = 8080"#;
        
        let env_specific = r#"[server]
        port = 9090"#;
        
        let config: AppConfig = Figment::new()
            .merge(Serialized::defaults(AppConfig::default())) // Tier 0: defaults
            .merge(Toml::string(default))     // Tier 1: defaults
            .merge(Toml::string(env_specific)) // Tier 2: env file
            .merge(Env::prefixed("APP_").split("__"))      // Tier 3: env vars
            .extract()
            .expect("Should merge configs");
            
        // Environment variable (tier 3) should win
        assert_eq!(config.server.port, 3000);
        
        unsafe {
            std::env::remove_var("APP_SERVER__PORT");
        }
    }
    
    #[test]
    fn test_default_values() {
        // Test that defaults are applied when no config provided
        let config: AppConfig = Figment::new()
            .merge(Serialized::defaults(AppConfig::default()))
            .extract()
            .expect("Should load defaults");
            
        // Verify reasonable defaults exist
        assert!(config.server.port >= 1024);
        assert!(!config.server.bind.is_empty());
        assert!(!config.logging.level.is_empty());
        assert!(!config.logging.format.is_empty());
    }
    
    #[test]
    fn test_validation_catches_invalid_bind() {
        let config_toml = r#"
            [server]
            port = 8080
            bind = "invalid-ip-address"
        "#;
        
        let config: AppConfig = Figment::new()
            .merge(Toml::string(config_toml))
            .extract()
            .expect("Should parse");
            
        let validation = config.validate();
        assert!(validation.is_err());
        assert!(validation.unwrap_err().to_string().contains("bind"));
    }
}