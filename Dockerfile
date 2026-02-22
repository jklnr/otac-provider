# ============================================
# Stage 1: Build the application
# ============================================
FROM rust:1.93-alpine AS builder

# Install musl-dev for static linking and other build deps
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --features redis && \
    rm -rf src

# Now copy the real source code
COPY src ./src

# Touch main.rs to invalidate the cached dummy
RUN touch src/main.rs

# Build the actual application with redis feature
RUN cargo build --release --features redis

# ============================================
# Stage 2: Create the minimal runtime image
# ============================================
FROM alpine:3.21

# Add ca-certificates for HTTPS connections (e.g., to cloud storage)
RUN apk add --no-cache ca-certificates

# Create a non-root user
RUN addgroup -S otac-provider && adduser -S otac-provider -G otac-provider

WORKDIR /app

# Copy the binary from the builder
COPY --from=builder /app/target/release/otac-provider /app/otac-provider

# Use the non-root user
USER otac-provider

# Expose the default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Run the application
CMD ["/app/otac-provider"]

