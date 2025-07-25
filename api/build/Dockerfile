# Build stage
FROM rust:latest AS builder

# Install all necessary build dependencies
RUN apt-get update && apt-get install -y \
    musl-tools \
    musl-dev \
    build-essential \
    cmake \
    clang \
    llvm-dev \
    libclang-dev \
    pkg-config \
    protobuf-compiler \
    perl \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Add musl target
RUN rustup target add x86_64-unknown-linux-musl

# Create symlink for musl g++
RUN ln -s /usr/bin/g++ /usr/bin/x86_64-linux-musl-g++

# Set environment variables for musl build
ENV CC=musl-gcc
ENV CXX=g++
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc

WORKDIR /build

# Copy cargo configuration first
COPY ./build/.cargo .cargo

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs for dependency compilation
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release --target x86_64-unknown-linux-musl || true
RUN rm -rf src

# Copy actual source code
COPY src ./src

# Copy configuration files
COPY config ./config

# Build the final binary
RUN cargo build --release --target x86_64-unknown-linux-musl

# ============================================
# Runtime stage
FROM scratch

# Copy the static binary from builder
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/pcf-api /pcf-api

# Copy configuration files
COPY --from=builder /build/config /app/config

# Expose the port (default 4000, configurable via APP_PORT)
EXPOSE 4000

# Run the binary
ENTRYPOINT ["/pcf-api"]
