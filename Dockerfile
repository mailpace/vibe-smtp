# Build stage
FROM rust:1.70-slim as builder

# Install required dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install required runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false -m vibe-gateway

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/vibe-gateway ./

# Copy TLS certificates
COPY test_cert.pem test_key.pem ./

# Change ownership to non-root user
RUN chown -R vibe-gateway:vibe-gateway /app

# Switch to non-root user
USER vibe-gateway

# Expose all SMTP ports
# 25 - Standard SMTP with STARTTLS
# 587 - Message Submission with STARTTLS  
# 2525 - Alternative SMTP with STARTTLS
# 465 - SMTP over SSL (implicit TLS)
EXPOSE 25 587 2525 465

# Set default environment variables
ENV MAILPACE_API_TOKEN=""

# Health check for the main service
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD timeout 5 bash -c '</dev/tcp/localhost/2525' || exit 1

# Run the application in Docker multi-port mode
CMD ["./vibe-gateway", "--docker-multi-port"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD timeout 5 bash -c '</dev/tcp/localhost/2525' || exit 1

# Default command
CMD ["./vibe-gateway"]
