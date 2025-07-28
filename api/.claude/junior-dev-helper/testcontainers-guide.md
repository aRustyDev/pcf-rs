# TestContainers Guide for Database Testing

## What are TestContainers?

TestContainers automatically spin up real databases in Docker containers for your tests. This gives you:
- Real database behavior (not mocks)
- Isolated test environment
- Automatic cleanup
- No manual setup required

## Getting Started

### Prerequisites

1. Docker must be installed and running
2. Add to your `Cargo.toml`:
```toml
[dev-dependencies]
testcontainers = "0.15"
testcontainers-modules = { version = "0.1", features = ["surrealdb"] }
```

### Basic Usage

```rust
use testcontainers::{clients, images::surrealdb::SurrealDb};

#[tokio::test]
async fn test_with_real_database() {
    // Start container
    let docker = clients::Cli::default();
    let container = docker.run(SurrealDb::default());
    
    // Get connection details
    let port = container.get_host_port_ipv4(8000);
    let url = format!("ws://localhost:{}", port);
    
    // Connect and test
    let db = connect_to_surrealdb(&url).await.unwrap();
    
    // Container automatically cleaned up when dropped
}
```

## Creating a SurrealDB Test Container

### Custom SurrealDB Image

```rust
use testcontainers::{core::WaitFor, Image, ImageArgs};

#[derive(Debug, Default)]
pub struct SurrealDbArgs {
    pub username: String,
    pub password: String,
}

impl ImageArgs for SurrealDbArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(
            vec![
                "start".to_string(),
                "--user".to_string(),
                self.username,
                "--pass".to_string(),
                self.password,
                "memory".to_string(), // In-memory for tests
            ]
            .into_iter(),
        )
    }
}

pub struct SurrealDb {
    args: SurrealDbArgs,
}

impl Image for SurrealDb {
    type Args = SurrealDbArgs;

    fn name(&self) -> String {
        "surrealdb/surrealdb:latest".to_string()
    }

    fn tag(&self) -> String {
        "latest".to_string()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout("Started web server"),
            WaitFor::seconds(2), // Give it time to fully initialize
        ]
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![8000]
    }
}
```

### Test Helper Functions

```rust
use testcontainers::{clients::Cli, Container};

pub struct TestDatabase {
    container: Container<'static, SurrealDb>,
    pub url: String,
    pub username: String,
    pub password: String,
}

impl TestDatabase {
    pub async fn new() -> Self {
        let docker = Box::leak(Box::new(Cli::default()));
        
        let image = SurrealDb {
            args: SurrealDbArgs {
                username: "test".to_string(),
                password: "test".to_string(),
            },
        };
        
        let container = docker.run(image);
        let port = container.get_host_port_ipv4(8000);
        
        Self {
            container,
            url: format!("ws://localhost:{}", port),
            username: "test".to_string(),
            password: "test".to_string(),
        }
    }
    
    pub async fn connect(&self) -> Result<DatabaseConnection, Error> {
        let config = DatabaseConfig {
            url: self.url.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            namespace: "test".to_string(),
            database: "test".to_string(),
        };
        
        // Retry connection as container might still be starting
        retry_with_backoff(
            || DatabaseConnection::new(config.clone()),
            Duration::from_secs(30),
        ).await
    }
}
```

## Integration Test Patterns

### 1. Test Fixtures

```rust
pub struct TestFixture {
    db: TestDatabase,
    connection: DatabaseConnection,
}

impl TestFixture {
    pub async fn new() -> Self {
        let db = TestDatabase::new().await;
        let connection = db.connect().await.unwrap();
        
        Self { db, connection }
    }
    
    pub async fn with_test_data(mut self) -> Self {
        // Insert test data
        self.connection.create("user", json!({
            "id": "test_user",
            "name": "Test User",
            "email": "test@example.com"
        })).await.unwrap();
        
        self.connection.create("note", json!({
            "title": "Test Note",
            "content": "Test content",
            "author": "user:test_user"
        })).await.unwrap();
        
        self
    }
    
    pub async fn cleanup(self) {
        // Explicit cleanup if needed
        let _ = self.connection.query("DELETE * FROM *").await;
    }
}

#[tokio::test]
async fn test_with_fixture() {
    let fixture = TestFixture::new()
        .await
        .with_test_data()
        .await;
    
    // Run tests
    let notes = fixture.connection
        .query("SELECT * FROM note")
        .await
        .unwrap();
    
    assert_eq!(notes.len(), 1);
    
    // Cleanup happens automatically
}
```

### 2. Parallel Test Isolation

```rust
// Each test gets its own container
#[tokio::test]
async fn test_create_note() {
    let db = TestDatabase::new().await;
    let conn = db.connect().await.unwrap();
    
    let result = conn.create("note", json!({
        "title": "Test"
    })).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_note() {
    let db = TestDatabase::new().await;
    let conn = db.connect().await.unwrap();
    
    // Completely isolated from test_create_note
    let id = conn.create("note", json!({
        "title": "To Delete"
    })).await.unwrap();
    
    let result = conn.delete("note", &id).await;
    assert!(result.is_ok());
}
```

### 3. Testing Error Conditions

```rust
#[tokio::test]
async fn test_connection_retry() {
    // Start container but don't wait for it
    let docker = Cli::default();
    let container = docker.run(SurrealDb::default());
    let port = container.get_host_port_ipv4(8000);
    
    // Try to connect immediately (might fail)
    let config = DatabaseConfig {
        url: format!("ws://localhost:{}", port),
        ..Default::default()
    };
    
    // Should retry and eventually connect
    let start = Instant::now();
    let result = connect_with_retry(
        || DatabaseConnection::new(config.clone()),
        Duration::from_secs(30)
    ).await;
    
    assert!(result.is_ok());
    assert!(start.elapsed() < Duration::from_secs(30));
}

#[tokio::test]
async fn test_database_unavailable() {
    let db = TestDatabase::new().await;
    let conn = db.connect().await.unwrap();
    
    // Stop the container
    drop(db);
    
    // Operations should fail
    let result = conn.query("SELECT * FROM note").await;
    assert!(matches!(result, Err(DatabaseError::ConnectionLost(_))));
}
```

## Advanced Patterns

### 1. Container Networks

```rust
use testcontainers::core::Port;

pub async fn create_test_network() -> TestNetwork {
    let docker = Cli::default();
    
    // Create network
    let network = docker.create_network("test-network");
    
    // Start SurrealDB
    let surrealdb = docker.run_with_args(
        SurrealDb::default(),
        RunArgs::default()
            .with_network(network.clone())
            .with_name("surrealdb"),
    );
    
    // Start application
    let app = docker.run_with_args(
        AppImage::default(),
        RunArgs::default()
            .with_network(network.clone())
            .with_env(vec![
                ("DATABASE_URL", "ws://surrealdb:8000"),
            ]),
    );
    
    TestNetwork {
        network,
        surrealdb,
        app,
    }
}
```

### 2. Container Logs

```rust
use testcontainers::core::logs::LogStream;

#[tokio::test]
async fn test_with_log_monitoring() {
    let docker = Cli::default();
    let container = docker.run(SurrealDb::default());
    
    // Get logs
    let mut log_stream = container.logs();
    
    // Monitor logs in background
    tokio::spawn(async move {
        while let Some(log) = log_stream.next().await {
            match log {
                Log::StdOut(msg) => println!("Container: {}", msg),
                Log::StdErr(msg) => eprintln!("Container ERROR: {}", msg),
            }
        }
    });
    
    // Run tests...
}
```

### 3. Custom Wait Strategies

```rust
impl Image for CustomDatabase {
    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            // Wait for specific log message
            WaitFor::message_on_stdout("Database ready"),
            
            // Wait for port to be available
            WaitFor::seconds(2),
            
            // Custom health check
            WaitFor::http(
                HttpWaitStrategy::new("/health")
                    .with_expected_status_code(200)
                    .with_poll_interval(Duration::from_secs(1))
            ),
        ]
    }
}
```

## Performance Optimization

### 1. Container Reuse

```rust
use once_cell::sync::Lazy;

// Reuse container across tests (be careful with test isolation!)
static TEST_DB: Lazy<TestDatabase> = Lazy::new(|| {
    futures::executor::block_on(TestDatabase::new())
});

#[tokio::test]
async fn test_using_shared_db() {
    let conn = TEST_DB.connect().await.unwrap();
    
    // Use unique namespaces for isolation
    conn.use_ns(&format!("test_{}", Uuid::new_v4())).await.unwrap();
    
    // Run test...
}
```

### 2. Parallel Container Startup

```rust
pub async fn setup_test_environment() -> TestEnvironment {
    // Start all containers in parallel
    let (db, cache, queue) = tokio::join!(
        TestDatabase::new(),
        TestRedis::new(),
        TestRabbitMQ::new(),
    );
    
    // Connect to all in parallel
    let (db_conn, cache_conn, queue_conn) = tokio::join!(
        db.connect(),
        cache.connect(),
        queue.connect(),
    );
    
    TestEnvironment {
        db: db_conn.unwrap(),
        cache: cache_conn.unwrap(),
        queue: queue_conn.unwrap(),
    }
}
```

## Debugging TestContainers

### 1. Keep Container Running

```rust
#[tokio::test]
async fn debug_container() {
    let docker = Cli::default();
    let container = docker.run(SurrealDb::default());
    
    println!("Container ID: {}", container.id());
    println!("Port: {}", container.get_host_port_ipv4(8000));
    
    // Keep container running for debugging
    if std::env::var("KEEP_CONTAINER").is_ok() {
        println!("Keeping container running. Press Ctrl+C to stop.");
        tokio::signal::ctrl_c().await.unwrap();
    }
}
```

### 2. Container Inspection

```rust
#[tokio::test]
async fn inspect_container() {
    let docker = Cli::default();
    let container = docker.run(SurrealDb::default());
    
    // Inspect container
    let info = container.inspect();
    println!("Container info: {:?}", info);
    
    // Execute commands in container
    let output = container.exec(vec!["ls", "-la", "/"]);
    println!("Container filesystem: {}", output);
}
```

## Common Issues and Solutions

### 1. Docker Not Running

```rust
fn check_docker() -> Result<(), String> {
    match std::process::Command::new("docker")
        .arg("info")
        .output()
    {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => Err("Docker is not running".to_string()),
        Err(_) => Err("Docker not found. Please install Docker".to_string()),
    }
}

#[test]
fn require_docker() {
    if let Err(msg) = check_docker() {
        panic!("{}", msg);
    }
}
```

### 2. Port Conflicts

```rust
use std::net::TcpListener;

fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

let container = docker.run_with_args(
    SurrealDb::default(),
    RunArgs::default()
        .with_mapped_port((find_free_port(), 8000)),
);
```

### 3. Slow Container Startup

```rust
impl TestDatabase {
    pub async fn new_with_timeout(timeout: Duration) -> Result<Self, Error> {
        tokio::time::timeout(timeout, Self::new()).await
            .map_err(|_| Error::ContainerStartupTimeout)?
    }
}
```

## Best Practices

1. **Clean up properly** - Containers are cleaned up on drop, but be explicit when needed
2. **Use unique data** - Don't assume empty database, use unique IDs
3. **Wait for readiness** - Always wait for containers to be fully ready
4. **Log failures** - Capture container logs on test failure
5. **Isolate tests** - Each test should be independent
6. **Mock for unit tests** - Use TestContainers only for integration tests

## Example: Complete Integration Test

```rust
use testcontainers::{clients, images::surrealdb::SurrealDb};

#[tokio::test]
async fn test_complete_user_flow() {
    // Setup
    let docker = clients::Cli::default();
    let container = docker.run(SurrealDb::default());
    let port = container.get_host_port_ipv4(8000);
    
    // Connect with retry
    let db = retry_with_backoff(
        || connect_surrealdb(&format!("ws://localhost:{}", port)),
        Duration::from_secs(30),
    ).await.unwrap();
    
    // Test user creation
    let user_id = db.create("user", json!({
        "name": "Alice",
        "email": "alice@example.com"
    })).await.unwrap();
    
    // Test note creation
    let note_id = db.create("note", json!({
        "title": "Test Note",
        "content": "Integration test content",
        "author": user_id.clone()
    })).await.unwrap();
    
    // Test query
    let user_notes = db.query(
        "SELECT * FROM note WHERE author = $author",
        json!({ "author": user_id })
    ).await.unwrap();
    
    assert_eq!(user_notes.len(), 1);
    assert_eq!(user_notes[0]["title"], "Test Note");
    
    // Container cleaned up automatically
}
```

TestContainers make integration testing easy and reliable. Use them whenever you need to test against real database behavior!