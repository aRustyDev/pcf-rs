# pcf-rs

# Docker Build Instructions

## Summary

The Docker build process

## Build Method

### Using justfile
```bash
# Alias of `just build-all`
just build

# Build both binary and Docker image
just build-all

# Build only Docker image
just docker-build
```

## Available Dockerfiles

1. **build/Dockerfile** - Main production Dockerfile with multi-stage build
2. **build/Dockerfile.simple** - Simple Dockerfile that copies pre-built binary
3. **build/Dockerfile.minimal** - Minimal Dockerfile using cargo-chef
4. **build/Dockerfile.optimized** - Optimized Dockerfile with cargo-chef
5. **build/Dockerfile.alpine** - Alpine-based (not compatible with glibc binary)

## Docker Compose

A complete docker-compose.yaml is provided that includes:
- PCF API service
- SurrealDB
- SpiceDB with PostgreSQL backend
- All necessary networking and volumes

## Suggested subdomains:

| identifier | description                                                                           |
|------------|---------------------------------------------------------------------------------------|
| proj       | Link to the PCF-rs /projects page; will be for proj mgmt tools                        |
| pcf        | Link to the PCF-rs homepage                                                           |
| chat       | Should link to integrated [RocketChat](https://docs.rocket.chat/) instance            |
| docs       | Should link to integrated [Docs](https://github.com/suitenumerique/doc) instance      |
| notes      | Should link to integrated [Trillium](https://github.com/TriliumNext/Trilium) Instance |
| reports    | Link to the PCF-rs /reports page; will be the design and review point                 |
| c2         | Will forward to a C2 receiver                                                         |
| pw         | Should link to integrated [passbolt](https://www.passbolt.com/ce/docker) instance     |
| vault      | Should link to integrated [vault](https://developer.hashicorp.com/vault) instance     |
| auth       | Should link to integrated [SpiceDB]() Instance                                        |
| todo       | Should link to integrated [todo]() instance                                           |
