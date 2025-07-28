# Common Errors and Solutions - Phase 1

## Compilation Errors

### Error: `use of undeclared crate`
```
error[E0433]: failed to resolve: use of undeclared crate or module `tokio`
```

**Solution:**
1. Check if the dependency is in `Cargo.toml`:
   ```toml
   [dependencies]
   tokio = { version = "1.0", features = ["full"] }
   ```
2. Run `cargo build` to download dependencies
3. Ensure you're using the correct import: `use tokio;`

### Error: `unresolved import`
```
error[E0432]: unresolved import `crate::config`
```

**Solution:**
1. Ensure the module exists and has a `mod.rs` file
2. Check that the module is declared in parent:
   ```rust
   // In src/main.rs or src/lib.rs
   mod config;
   ```
3. Verify the module is public if accessing from another module:
   ```rust
   pub mod config;
   ```

### Error: `cannot find type in this scope`
```
error[E0412]: cannot find type `AppConfig` in this scope
```

**Solution:**
1. Import the type: `use crate::config::AppConfig;`
2. Or use the full path: `crate::config::AppConfig`
3. Ensure the type is marked `pub` in its module

## Runtime Errors

### Error: Address Already in Use
```
Error: Address already in use (os error 48)
```

**Solution:**
1. Find and kill the process:
   ```bash
   just clean  # Project-specific cleanup
   # OR
   lsof -i :8080  # Find process on port
   kill -9 <PID>  # Kill the process
   ```
2. Use a different port:
   ```bash
   APP_SERVER__PORT=9090 cargo run
   ```

### Error: Permission Denied (Port < 1024)
```
Error: Permission denied (os error 13)
```

**Solution:**
1. Use a port >= 1024 (recommended)
2. Update your configuration validation to prevent this:
   ```rust
   #[garde(range(min = 1024, max = 65535))]
   pub port: u16,
   ```

### Error: Configuration File Not Found
```
Error: configuration file "config/production.toml" not found
```

**Solution:**
1. Create the file:
   ```bash
   mkdir -p config
   touch config/production.toml
   ```
2. Or make it optional in Figment:
   ```rust
   .merge(Toml::file("config/production.toml").nested())
   // The .nested() makes it optional
   ```

## Test Failures

### Error: Test Timeout
```
test server_lifecycle_test ... test has been running for over 60 seconds
```

**Solution:**
1. Increase timeout:
   ```rust
   #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
   #[timeout(Duration::from_secs(10))]
   async fn test_server() {
       // ...
   }
   ```
2. Ensure server shutdown is triggered:
   ```rust
   // Always send shutdown signal
   let _ = shutdown_tx.send(());
   ```

### Error: Panic in Test
```
thread 'tests::test_name' panicked at 'assertion failed'
```

**Solution:**
1. Use `cargo test -- --nocapture` to see println! output
2. Add more context to assertions:
   ```rust
   assert_eq!(
       actual, expected,
       "Port should be {}, but was {}", expected, actual
   );
   ```

### Error: Tests Interfering with Each Other
```
test test1 ... ok
test test2 ... FAILED (port already in use)
```

**Solution:**
1. Use random ports in tests:
   ```rust
   let listener = TcpListener::bind("127.0.0.1:0").await?;
   let port = listener.local_addr()?.port();
   ```
2. Run tests serially:
   ```bash
   cargo test -- --test-threads=1
   ```

## Async/Await Issues

### Error: `cannot block on runtime`
```
thread 'main' panicked at 'Cannot start a runtime from within a runtime'
```

**Solution:**
1. Don't use `block_on` inside async functions
2. Use `tokio::spawn` for concurrent tasks:
   ```rust
   // ❌ Wrong
   tokio::runtime::Runtime::new()?.block_on(async_task());
   
   // ✅ Correct
   tokio::spawn(async_task());
   ```

### Error: `future is not Send`
```
error: future cannot be sent between threads safely
```

**Solution:**
1. Ensure all data in async functions is Send:
   ```rust
   // ❌ Wrong - Rc is not Send
   let data = Rc::new(value);
   
   // ✅ Correct - Arc is Send
   let data = Arc::new(value);
   ```

## Validation Errors

### Error: Garde Validation Failed
```
Error: validation failed: server.port: not in range 1024..=65535
```

**Solution:**
1. Check the actual value:
   ```rust
   println!("Port value: {}", config.server.port);
   ```
2. Ensure validation matches requirements:
   ```rust
   #[garde(range(min = 1024, max = 65535))]
   ```

### Error: Custom Validator Panic
```
thread 'main' panicked at 'invalid IP address'
```

**Solution:**
1. Return Result from validators, don't panic:
   ```rust
   fn validate_ip(value: &str, _: &()) -> garde::Result {
       value.parse::<IpAddr>()
           .map(|_| ())
           .map_err(|_| garde::Error::new("Invalid IP"))
   }
   ```

## Logging Issues

### No Logs Appearing
**Solution:**
1. Set RUST_LOG environment variable:
   ```bash
   RUST_LOG=debug cargo run
   ```
2. Ensure tracing subscriber is initialized early:
   ```rust
   // First thing in main()
   init_tracing();
   ```

### JSON Logs Malformed
**Solution:**
1. Don't use string interpolation in log macros:
   ```rust
   // ❌ Wrong
   info!("Server started on {}", format!("{}:{}", host, port));
   
   // ✅ Correct
   info!(host = %host, port = %port, "Server started");
   ```

## Quick Fixes Cheat Sheet

| Problem | Quick Fix |
|---------|-----------|
| Can't find crate | Check Cargo.toml, run `cargo build` |
| Import not found | Add `mod` declaration, check visibility |
| Port in use | Run `just clean` or change port |
| Config not loading | Check file path and ENVIRONMENT var |
| Tests failing randomly | Use port 0 for random ports |
| No logs | Set `RUST_LOG=debug` |
| Validation errors | Print actual values, check ranges |
| Async panics | Avoid blocking in async contexts |

## Getting Help

When stuck:
1. Read the full error message - Rust errors are very helpful!
2. Check this guide for common issues
3. Use `cargo clippy` for additional hints
4. Search for the error code (e.g., "E0433") in Rust documentation
5. Ask for help with:
   - The complete error message
   - The code causing the error
   - What you've already tried

Remember: Every developer faces these errors. They're part of the learning process!