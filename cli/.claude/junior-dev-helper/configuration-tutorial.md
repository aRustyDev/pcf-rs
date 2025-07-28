# Configuration System Tutorial for Junior Developers

## Table of Contents
1. [Understanding the 4-Tier Hierarchy](#understanding-the-4-tier-hierarchy)
2. [Step-by-Step Implementation](#step-by-step-implementation)
3. [Common Pitfalls](#common-pitfalls)
4. [Debugging Configuration Issues](#debugging-configuration-issues)
5. [Quick Reference](#quick-reference)

## Understanding the 4-Tier Hierarchy

### Visual Representation
```
┌─────────────────────────┐
│   CLI Arguments         │ ← Highest Priority (4)
│   --port 3000          │
└───────────┬─────────────┘
            ▼ overrides
┌─────────────────────────┐
│   Environment Vars      │ ← Priority 3
│   APP_SERVER__PORT=8080 │
└───────────┬─────────────┘
            ▼ overrides
┌─────────────────────────┐
│   Config Files          │ ← Priority 2
│   config/production.toml│
└───────────┬─────────────┘
            ▼ overrides
┌─────────────────────────┐
│   Defaults              │ ← Lowest Priority (1)
│   AppConfig::default()  │
└─────────────────────────┘
```

### How It Works
Each tier can override values from lower tiers. Think of it like CSS - the most specific rule wins!

## Step-by-Step Implementation

### Step 1: Define Your Configuration Structs
```rust
use serde::{Deserialize, Serialize};
use garde::Validate;

#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct AppConfig {
    #[validate(nested)]
    pub server: ServerConfig,
    
    #[validate(nested)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct ServerConfig {
    #[garde(range(min = 1024, max = 65535))]
    pub port: u16,
    
    #[garde(length(min = 1))]
    pub bind: String,
}

// Implement Default for base values (Tier 1)
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                port: 8080,
                bind: "0.0.0.0".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
        }
    }
}
```

### Step 2: Set Up Figment
```rust
use figment::{Figment, providers::{Env, Format, Toml, Serialized}};
use clap::Parser;

pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    // Step 1: Parse CLI arguments
    let cli_args = Cli::parse();
    
    // Step 2: Determine environment
    let environment = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "production".to_string());
    
    // Step 3: Build Figment with all 4 tiers
    let figment = Figment::new()
        // Tier 1: Defaults (lowest priority)
        .merge(Serialized::defaults(AppConfig::default()))
        
        // Tier 2: Configuration files
        .merge(Toml::file("config/default.toml").nested())
        .merge(Toml::file(format!("config/{}.toml", environment)).nested())
        
        // Tier 3: Environment variables
        // APP_SERVER__PORT=9090 becomes server.port = 9090
        .merge(Env::prefixed("APP_").split("__"))
        
        // Tier 4: CLI arguments (highest priority)
        .merge(Serialized::defaults(&cli_args));
    
    // Step 4: Extract and validate
    let config: AppConfig = figment.extract()?;
    config.validate()?;
    
    Ok(config)
}
```

### Step 3: Create Your CLI Structure
```rust
use clap::Parser;

#[derive(Parser, Debug, Serialize)]
#[command(author, version, about)]
pub struct Cli {
    /// Server port (overrides all other settings)
    #[arg(short, long)]
    pub port: Option<u16>,
    
    /// Enable debug logging
    #[arg(short, long)]
    pub debug: bool,
}
```

## Common Pitfalls

### Pitfall 1: Environment Variable Format
❌ **Wrong**: `APP_SERVER_PORT=8080`  
✅ **Correct**: `APP_SERVER__PORT=8080` (double underscore for nesting)

### Pitfall 2: Missing Optional Files
```rust
// This will panic if file doesn't exist:
.merge(Toml::file("config/production.toml"))

// Use this instead to make it optional:
.merge(Toml::file("config/production.toml").nested())
```

### Pitfall 3: Validation Timing
```rust
// ❌ Wrong - Extract without validation
let config: AppConfig = figment.extract()?;
// Using config here might have invalid values!

// ✅ Correct - Validate immediately after extraction
let config: AppConfig = figment.extract()?;
config.validate()?; // Now safe to use
```

## Debugging Configuration Issues

### Debug Helper Function
Add this to your codebase:
```rust
pub fn debug_config_sources(figment: &Figment) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Configuration Debug Info ===");
    
    // Show all sources
    println!("\nConfiguration sources:");
    for (i, source) in figment.sources().enumerate() {
        println!("  {}. {}", i + 1, source);
    }
    
    // Try to extract and show what values came from where
    match figment.extract::<toml::Value>() {
        Ok(value) => {
            println!("\nMerged configuration:");
            println!("{}", toml::to_string_pretty(&value)?);
        }
        Err(e) => println!("\nError extracting config: {}", e),
    }
    
    println!("========================\n");
    Ok(())
}
```

### Common Error Messages and Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| `"missing field 'port'"` | Required field not provided | Add default value or ensure config file has it |
| `"invalid type: string, expected u16"` | Wrong type in env var | Remove quotes: `APP_SERVER__PORT=8080` not `"8080"` |
| `"port: not in range 1024..=65535"` | Validation failed | Check port is ≥ 1024 |
| `"invalid IP address"` | Bad bind address | Use valid IP like "0.0.0.0" or "127.0.0.1" |

## Quick Reference

### Testing Configuration Hierarchy
```bash
# Test 1: Default only
cargo run
# Should use port 8080

# Test 2: Config file override
echo '[server]
port = 7070' > config/development.toml
ENVIRONMENT=development cargo run
# Should use port 7070

# Test 3: Environment variable override
APP_SERVER__PORT=6060 ENVIRONMENT=development cargo run
# Should use port 6060

# Test 4: CLI override (highest priority)
APP_SERVER__PORT=6060 cargo run -- --port 5050
# Should use port 5050
```

### Validation Cheat Sheet
```rust
// Common Garde validators
#[garde(range(min = 1024, max = 65535))]  // Port range
#[garde(length(min = 1, max = 100))]       // String length
#[garde(email)]                            // Email format
#[garde(url)]                              // URL format
#[garde(pattern(r"^[a-z]+$"))]            // Regex pattern
#[garde(custom(my_validator_fn))]         // Custom validation
```

### Environment Variable Reference
```bash
# Server configuration
APP_SERVER__PORT=8080
APP_SERVER__BIND=0.0.0.0
APP_SERVER__SHUTDOWN_TIMEOUT=30

# Logging configuration  
APP_LOGGING__LEVEL=debug
APP_LOGGING__FORMAT=pretty

# Health check configuration
APP_HEALTH__LIVENESS_PATH=/health
APP_HEALTH__READINESS_PATH=/health/ready
```

## Next Steps
1. Copy the example code into your project
2. Run the hierarchy tests to see it in action
3. Add the debug helper for troubleshooting
4. Implement your custom validators as needed

Remember: The configuration system is like a layer cake - each layer can override the ones below it!