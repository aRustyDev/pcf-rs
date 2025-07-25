/// Interactive Configuration Hierarchy Demo
/// 
/// This example demonstrates how Figment's 4-tier configuration hierarchy works.
/// Run with: cargo run --example config-hierarchy-demo

use figment::{Figment, providers::{Env, Format, Toml, Serialized}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DemoConfig {
    server: ServerConfig,
    feature: FeatureConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ServerConfig {
    port: u16,
    host: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FeatureConfig {
    enabled: bool,
    name: String,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                port: 8080,
                host: "localhost".to_string(),
            },
            feature: FeatureConfig {
                enabled: false,
                name: "default-feature".to_string(),
            },
        }
    }
}

fn main() {
    println!("=== Figment Configuration Hierarchy Demo ===\n");
    
    // Demo 1: Defaults only
    println!("1. DEFAULTS ONLY:");
    let config = Figment::new()
        .merge(Serialized::defaults(DemoConfig::default()))
        .extract::<DemoConfig>()
        .unwrap();
    print_config(&config, "Defaults");
    
    // Demo 2: Defaults + Config file
    println!("\n2. DEFAULTS + CONFIG FILE:");
    let config_file = r#"
        [server]
        port = 9090
        
        [feature]
        enabled = true
    "#;
    
    let config = Figment::new()
        .merge(Serialized::defaults(DemoConfig::default()))
        .merge(Toml::string(config_file))
        .extract::<DemoConfig>()
        .unwrap();
    print_config(&config, "Config file overrides port and enabled");
    
    // Demo 3: Defaults + Config file + Environment
    println!("\n3. DEFAULTS + CONFIG FILE + ENVIRONMENT:");
    std::env::set_var("APP_SERVER__PORT", "7070");
    std::env::set_var("APP_FEATURE__NAME", "env-feature");
    
    let config = Figment::new()
        .merge(Serialized::defaults(DemoConfig::default()))
        .merge(Toml::string(config_file))
        .merge(Env::prefixed("APP_").split("__"))
        .extract::<DemoConfig>()
        .unwrap();
    print_config(&config, "Env vars override port and name");
    
    // Demo 4: All tiers including CLI
    println!("\n4. ALL TIERS (Defaults + File + Env + CLI):");
    
    #[derive(Debug, Serialize)]
    struct CliArgs {
        server: CliServer,
    }
    
    #[derive(Debug, Serialize)]
    struct CliServer {
        host: String,
    }
    
    let cli_args = CliArgs {
        server: CliServer {
            host: "0.0.0.0".to_string(),
        },
    };
    
    let config = Figment::new()
        .merge(Serialized::defaults(DemoConfig::default()))
        .merge(Toml::string(config_file))
        .merge(Env::prefixed("APP_").split("__"))
        .merge(Serialized::defaults(cli_args))
        .extract::<DemoConfig>()
        .unwrap();
    print_config(&config, "CLI overrides host");
    
    // Cleanup
    std::env::remove_var("APP_SERVER__PORT");
    std::env::remove_var("APP_FEATURE__NAME");
    
    // Demo 5: Show precedence clearly
    println!("\n=== PRECEDENCE SUMMARY ===");
    println!("Priority (lowest to highest):");
    println!("1. Defaults:     port=8080, host=localhost, enabled=false, name=default-feature");
    println!("2. Config file:  port=9090, enabled=true");
    println!("3. Environment:  port=7070, name=env-feature");
    println!("4. CLI args:     host=0.0.0.0");
    println!("\nFinal result:    port=7070, host=0.0.0.0, enabled=true, name=env-feature");
    
    // Interactive debugging
    println!("\n=== DEBUGGING TIPS ===");
    demonstrate_debugging();
}

fn print_config(config: &DemoConfig, description: &str) {
    println!(">>> {}", description);
    println!("    server.port: {}", config.server.port);
    println!("    server.host: {}", config.server.host);
    println!("    feature.enabled: {}", config.feature.enabled);
    println!("    feature.name: {}", config.feature.name);
}

fn demonstrate_debugging() {
    println!("To debug configuration issues:");
    
    // 1. List all environment variables
    println!("\n1. Check environment variables:");
    for (key, value) in std::env::vars() {
        if key.starts_with("APP_") {
            println!("   {}={}", key, value);
        }
    }
    
    // 2. Test specific value sources
    println!("\n2. Test individual sources:");
    
    // Just defaults
    let defaults_only = Figment::new()
        .merge(Serialized::defaults(DemoConfig::default()))
        .extract::<DemoConfig>()
        .unwrap();
    println!("   Defaults only - port: {}", defaults_only.server.port);
    
    // Just environment
    std::env::set_var("APP_SERVER__PORT", "5555");
    let env_only = Figment::new()
        .merge(Serialized::defaults(DemoConfig::default()))
        .merge(Env::prefixed("APP_").split("__"))
        .extract::<DemoConfig>()
        .unwrap();
    println!("   With env var - port: {}", env_only.server.port);
    std::env::remove_var("APP_SERVER__PORT");
    
    // 3. Extract as TOML to see merged result
    println!("\n3. View merged configuration:");
    let figment = Figment::new()
        .merge(Serialized::defaults(DemoConfig::default()));
    
    if let Ok(value) = figment.extract::<toml::Value>() {
        println!("{}", toml::to_string_pretty(&value).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_precedence_order() {
        // Set up all tiers
        std::env::set_var("APP_SERVER__PORT", "6666");
        
        let config_file = r#"
            [server]
            port = 5555
        "#;
        
        let cli = CliArgs {
            server: CliServer {
                port: 4444,
            },
        };
        
        let config = Figment::new()
            .merge(Serialized::defaults(DemoConfig::default())) // 8080
            .merge(Toml::string(config_file))                   // 5555
            .merge(Env::prefixed("APP_").split("__"))          // 6666
            .merge(Serialized::defaults(cli))                   // 4444
            .extract::<DemoConfig>()
            .unwrap();
        
        // CLI should win
        assert_eq!(config.server.port, 4444);
        
        std::env::remove_var("APP_SERVER__PORT");
    }
}