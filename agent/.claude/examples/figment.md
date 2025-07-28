```rust
use figment::{Figment, providers::{Format, Toml, Env, Serialized}};
use clap::Parser;

#[derive(Parser)]
struct CliArgs {
    #[arg(long, env = "APP_CONFIG_PATH")]
    config_path: Option<String>,
    #[arg(long, env = "APP_PORT")]
    port: Option<u16>,
}

pub fn load_configuration() -> Result<AppConfig, ConfigError> {
    let cli_args = CliArgs::parse();

    // You MUST load in this exact order
    let mut figment = Figment::new()
        // 1. Start with hardcoded defaults
        .merge(Serialized::defaults(AppConfig::default()))
        // 2. Load base configuration file
        .merge(Toml::file("config/default.toml").nested())
        // 3. Override with environment-specific file
        .merge(Toml::file(format!("config/{}.toml",
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string())
        )).nested())
        // 4. Apply environment variables with APP_ prefix
        .merge(Env::prefixed("APP_").split("__"));

    // 5. Apply CLI arguments last
    if let Some(port) = cli_args.port {
        figment = figment.merge(("server.port", port));
    }

    figment.extract()
}
```
