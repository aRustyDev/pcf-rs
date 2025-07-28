# Figment Debugging Tips

## Common Figment Issues and Solutions

### Issue 1: "missing field" Errors

**Symptom:**
```
Error: missing field `port` at line 1 column 1
```

**Causes & Solutions:**
1. **Field not in any configuration source**
   - Add to defaults: `impl Default for ServerConfig`
   - Add to config file: `port = 8080`
   - Set environment variable: `APP_SERVER__PORT=8080`

2. **Typo in field name**
   - Check TOML file for typos
   - Verify struct field names match exactly
   - Remember: Rust is case-sensitive!

### Issue 2: Environment Variable Not Working

**Symptom:**
Environment variable `APP_SERVER_PORT=9090` isn't overriding config

**Solution:**
Use double underscore for nesting:
```bash
# ❌ Wrong
APP_SERVER_PORT=9090

# ✅ Correct
APP_SERVER__PORT=9090
```

### Issue 3: Type Mismatch Errors

**Symptom:**
```
Error: invalid type: string "8080", expected u16
```

**Common Causes:**
1. **Quoted numbers in environment variables**
   ```bash
   # ❌ Wrong
   APP_SERVER__PORT="8080"
   
   # ✅ Correct
   APP_SERVER__PORT=8080
   ```

2. **Wrong type in TOML**
   ```toml
   # ❌ Wrong
   port = "8080"  # String
   
   # ✅ Correct
   port = 8080    # Number
   ```

### Issue 4: Configuration File Not Loading

**Symptom:**
Changes to config file have no effect

**Debugging Steps:**
1. **Check file path**
   ```rust
   // Add debug print
   let path = format!("config/{}.toml", environment);
   println!("Looking for config at: {}", path);
   ```

2. **Verify file exists**
   ```bash
   ls -la config/
   cat config/development.toml
   ```

3. **Check environment variable**
   ```bash
   echo $ENVIRONMENT
   # Should print: development, staging, or production
   ```

### Issue 5: Validation Failures

**Symptom:**
```
Error: validation failed: port: not in range 1024..=65535
```

**Debugging:**
```rust
// Print values before validation
let config: AppConfig = figment.extract()?;
println!("Port before validation: {}", config.server.port);

// Then validate
config.validate()?;
```

## Debug Helper Functions

### Show All Configuration Sources
```rust
pub fn debug_figment_sources(figment: &Figment) {
    println!("\n=== Figment Debug Info ===");
    
    // List all providers
    for (i, source) in figment.sources().enumerate() {
        println!("Provider {}: {}", i + 1, source);
    }
    
    // Try to show merged result
    if let Ok(value) = figment.extract::<toml::Value>() {
        println!("\nMerged configuration:");
        println!("{}", toml::to_string_pretty(&value).unwrap());
    }
    
    println!("========================\n");
}
```

### Trace Value Source
```rust
pub fn trace_config_value(figment: &Figment, path: &str) {
    println!("\nTracing value: {}", path);
    
    // This is pseudo-code - Figment doesn't expose this directly
    // But you can test by removing sources one by one
    let sources = vec![
        "defaults",
        "config file", 
        "env vars",
        "CLI args"
    ];
    
    for source in sources {
        println!("  Checking {}: ...", source);
        // Remove sources and re-extract to find where value comes from
    }
}
```

### Test Configuration Loading
```rust
#[test]
fn debug_config_loading() {
    // Set up test environment
    std::env::set_var("ENVIRONMENT", "test");
    std::env::set_var("APP_SERVER__PORT", "7777");
    
    // Load with debug output
    let figment = create_figment();
    debug_figment_sources(&figment);
    
    // Extract and print
    match figment.extract::<AppConfig>() {
        Ok(config) => {
            println!("✅ Config loaded successfully!");
            println!("Port: {}", config.server.port);
        }
        Err(e) => {
            println!("❌ Config failed: {}", e);
            // Print detailed error info
            for cause in e.chain() {
                println!("  Caused by: {}", cause);
            }
        }
    }
    
    // Cleanup
    std::env::remove_var("ENVIRONMENT");
    std::env::remove_var("APP_SERVER__PORT");
}
```

## Quick Debugging Checklist

When configuration isn't working as expected:

1. **Print the environment**
   ```rust
   println!("ENVIRONMENT={:?}", std::env::var("ENVIRONMENT"));
   ```

2. **List all APP_ env vars**
   ```rust
   for (key, value) in std::env::vars() {
       if key.starts_with("APP_") {
           println!("{}={}", key, value);
       }
   }
   ```

3. **Check file paths**
   ```rust
   use std::path::Path;
   let config_path = "config/development.toml";
   println!("Config exists: {}", Path::new(config_path).exists());
   ```

4. **Validate incrementally**
   ```rust
   // Extract without validation first
   let config: AppConfig = figment.extract()?;
   
   // Validate each section separately
   config.server.validate()?;
   config.logging.validate()?;
   ```

5. **Use Figment's built-in debugging**
   ```rust
   // Enable detailed errors
   let figment = Figment::new()
       .merge(/* ... */)
       .select(Profile::from_env_or("ENVIRONMENT", "production"));
   
   // This gives more detailed error messages
   ```

## Pro Tips

1. **Always validate after extraction** - Don't assume valid config
2. **Use Option<T> for optional fields** - Avoids "missing field" errors
3. **Provide sensible defaults** - Reduces configuration burden
4. **Test with minimal config** - Start simple, add complexity
5. **Document your precedence** - Make it clear which source wins

Remember: Configuration issues are often the first hurdle. Once you get past them, the rest is smooth sailing!