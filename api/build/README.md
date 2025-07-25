# Building PCF-rs API server

## Summary: Dockerfile

The Docker image MUST keep the following specifications:

✅ **FROM scratch** in the final stage
✅ **Static binary** built using musl target
✅ **rust:latest** used in the builder stage
✅ **Minimal image size**: 529KB

## Build Configuration

### Dockerfile Features
- Multi-stage build with `rust:latest` as builder
- Static compilation using `x86_64-unknown-linux-musl` target
- All dependencies vendored (including OpenSSL)
- Final image uses `FROM scratch` with only the binary

### Key Build Steps
1. Install musl toolchain and build dependencies
2. Add musl target to rustup
3. Configure static linking via `.cargo/config.toml`
4. Build dependencies and then the final binary
5. Copy only the static binary to scratch image

## Running the Image

```bash
docker run -p 4000:4000 arustydev/pcf-api
```

Note: The binary expects to connect to:
- SurrealDB at 127.0.0.1:8000
- SpiceDB (configure via SPICEDB_URL env var)
- Kratos (configure via KRATOS_PUBLIC_URL env var)

For production use, you'll need to configure these services via environment variables or use the docker-compose.yaml file.

## Image Details

- **Name**: arustydev/pcf-api
- **Tag**: latest
- **Size**: 529KB
- **Base**: scratch (no OS layer)
- **Binary**: Statically linked with musl
