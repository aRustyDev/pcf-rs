# Developer Quick Start

Welcome to the PCF API developer documentation! This guide will help you get started with developing and contributing to the PCF API.

<!-- toc -->

## Prerequisites

Before you begin, ensure you have the following tools installed:

- **Rust {{ min_rust_version }} or later**: [Install Rust](https://rustup.rs/)
- **Docker & Docker Compose**: [Install Docker](https://docs.docker.com/get-docker/)
- **Git**: [Install Git](https://git-scm.com/downloads)
- **Optional but recommended**:
  - [cargo-watch](https://github.com/watchexec/cargo-watch): For auto-reloading during development
  - [cargo-nextest](https://nexte.st/): Faster test runner
  - [cargo-machete](https://github.com/bnjbvr/cargo-machete): Find unused dependencies

## Clone and Build

Let's get the PCF API running on your local machine:

```bash
# Clone the repository
git clone {{ github_url }}
cd pcf-api/api

# Install development tools
cargo install cargo-watch cargo-nextest cargo-machete

# Build the project (debug mode)
cargo build

# Build for production
cargo build --release

# Run all tests
cargo nextest run

# Run the API server (demo mode with in-memory database)
cargo run --bin api
```

The API will start on `http://localhost:8080` by default.

## Making Your First GraphQL Query

Once the server is running, you can interact with the GraphQL API:

### Using curl
```bash
# Health check query
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ health { status version uptime } }"}'

# Create a note
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation CreateNote($input: CreateNoteInput!) { createNote(input: $input) { id title content createdAt } }",
    "variables": {
      "input": {
        "title": "My First Note",
        "content": "Hello from PCF API!"
      }
    }
  }'

# List all notes
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ notes { edges { node { id title content } } } }"}'
```

### Using GraphQL Playground

Navigate to `http://localhost:8080/playground` in your browser for an interactive GraphQL explorer.

Try this query:
```graphql
{
  health {
    status
    version
    uptime
    services {
      database
      cache
    }
  }
}
```

## Understanding the Codebase Structure

The PCF API follows a modular architecture:

```
api/
├── Cargo.toml           # Project dependencies
├── src/
│   ├── main.rs         # Application entry point
│   ├── lib.rs          # Library root
│   ├── bin/
│   │   └── api.rs      # Binary entry point
│   ├── config/         # Configuration management (Figment)
│   │   ├── mod.rs      # Config module root
│   │   ├── models.rs   # Config structures
│   │   └── tests.rs    # Config tests
│   ├── error/          # Error handling
│   │   ├── mod.rs      # Error types
│   │   └── handlers.rs # Error conversion
│   ├── graphql/        # GraphQL implementation
│   │   ├── mod.rs      # GraphQL module
│   │   ├── schema.rs   # Schema definition
│   │   ├── query.rs    # Query resolvers
│   │   ├── mutation.rs # Mutation resolvers
│   │   └── context.rs  # Request context
│   ├── health/         # Health check endpoints
│   │   ├── mod.rs      # Health module
│   │   ├── state.rs    # Health state machine
│   │   └── handlers.rs # HTTP handlers
│   ├── logging/        # Structured logging
│   │   ├── mod.rs      # Logging setup
│   │   └── layers.rs   # Tracing layers
│   ├── schema/         # Data models
│   │   ├── mod.rs      # Schema module
│   │   └── note.rs     # Note model
│   ├── server/         # HTTP server
│   │   ├── mod.rs      # Server module
│   │   ├── router.rs   # Route definitions
│   │   └── middleware.rs # Middleware stack
│   └── services/       # Service layer
│       ├── mod.rs      # Services module
│       └── database.rs # Database service
├── tests/              # Integration tests
│   ├── common/         # Test utilities
│   └── api/            # API tests
├── benches/            # Performance benchmarks
└── examples/           # Example code

```

### Key Modules Explained

- **config**: Handles all configuration using Figment, supporting env vars, files, and defaults
- **error**: Centralized error handling with proper HTTP status codes and client-safe messages
- **graphql**: The GraphQL API implementation using async-graphql
- **health**: Kubernetes-compatible health checks (liveness and readiness)
- **logging**: Structured logging with tracing, supporting JSON and pretty formats
- **schema**: Data models and validation logic
- **server**: HTTP server setup with Axum, including middleware
- **services**: Business logic and external service integrations

## Development Workflow

### Running in Watch Mode

For rapid development, use cargo-watch to automatically rebuild and restart on changes:

```bash
# Watch and run on changes
cargo watch -x run

# Watch and run tests on changes
cargo watch -x test

# Watch specific paths
cargo watch -w src -x "run --bin api"
```

### Environment Configuration

Create a `.env` file for local development:

```bash
# Copy the example environment file
cp .env.example .env

# Key configuration options
API_HOST=127.0.0.1
API_PORT=8080
LOG_LEVEL=debug
LOG_FORMAT=pretty
DATABASE_URL=memory://  # In-memory for demo
```

### Running with External Services

For a more production-like environment:

```bash
# Start dependencies (PostgreSQL, Redis, etc.)
docker-compose up -d

# Run with external services
DATABASE_URL=postgresql://user:pass@localhost/pcf_api \
REDIS_URL=redis://localhost:6379 \
cargo run --bin api
```

## Common Development Tasks

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test config::

# Run tests with output
cargo test -- --nocapture

# Run tests in parallel with nextest
cargo nextest run

# Run a specific test
cargo test test_health_check

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting (CI)
cargo fmt -- --check

# Run clippy lints
cargo clippy -- -D warnings

# Check for unused dependencies
cargo machete

# Security audit
cargo audit

# Generate documentation
cargo doc --open
```

### Benchmarking

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench health_check

# Save benchmark baseline
cargo bench -- --save-baseline main
```

### Debugging

```bash
# Run with verbose logging
LOG_LEVEL=trace cargo run

# Run with backtrace
RUST_BACKTRACE=1 cargo run

# Run with full backtrace
RUST_BACKTRACE=full cargo run

# Use lldb (macOS) or gdb (Linux)
rust-lldb target/debug/api
```

## Making Your First Contribution

1. **Pick an issue**: Look for issues labeled `good-first-issue`
2. **Create a branch**: `git checkout -b feature/your-feature`
3. **Write tests first**: Follow TDD practices
4. **Implement the feature**: Keep changes focused
5. **Run checks**: `cargo fmt && cargo clippy && cargo test`
6. **Commit with conventional commits**: `feat: add new feature`
7. **Open a PR**: Reference the issue number

Example workflow:
```bash
# Create feature branch
git checkout -b feature/add-user-model

# Make changes and test
cargo test

# Format and lint
cargo fmt
cargo clippy --fix

# Commit changes
git add -A
git commit -m "feat: add user model with validation

- Add User struct with serde derives
- Add validation using garde
- Add unit tests for user creation
- Update GraphQL schema

Closes #123"

# Push and create PR
git push origin feature/add-user-model
```

## Next Steps

Now that you have the basics down:

1. **Deep dive into architecture**: Read the [Architecture Overview](../developer/architecture/overview.md)
2. **Understand modules**: Explore [Module Documentation](../developer/modules/README.md)
3. **Learn testing strategies**: Study our [Testing Guide](../developer/testing/strategy.md)
4. **Review standards**: Understand our [Coding Standards](../developer/contributing/standards.md)
5. **Explore GraphQL**: Deep dive into [GraphQL Schema](../developer/graphql/schema.md)

## Troubleshooting

### Common Issues

**Build fails with "linking with cc failed"**
- Install build essentials: `sudo apt-get install build-essential` (Ubuntu/Debian)
- Install Xcode command line tools: `xcode-select --install` (macOS)

**"Address already in use" error**
- Check if port 8080 is in use: `lsof -i :8080`
- Change port: `API_PORT=3000 cargo run`

**Database connection errors**
- Ensure Docker is running: `docker ps`
- Check database logs: `docker-compose logs database`
- Verify connection string in `.env`

**Out of memory during build**
- Limit parallel jobs: `cargo build -j 2`
- Use release mode: `cargo build --release`

## Getting Help

- 📖 Check the [FAQ](../shared/faq.md)
- 🔍 Search [existing issues]({{ github_url }}/issues)
- 💬 Ask in [GitHub Discussions]({{ github_url }}/discussions)
- 🐛 Report bugs via [GitHub Issues]({{ github_url }}/issues/new)
- 📧 Security issues: See [Security Policy](../shared/security/reporting.md)

---

**Ready to dive deeper?** Head to the [Architecture Overview](../developer/architecture/overview.md) to understand the system design.