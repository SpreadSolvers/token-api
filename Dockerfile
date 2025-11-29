# Build stage
FROM rustlang/rust:nightly-slim as builder

# Install required dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY diesel.toml ./

# Build the application in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/token-api .

# Copy necessary files
COPY --from=builder /app/diesel.toml .
COPY --from=builder /app/migrations ./migrations

# Create db directory for SQLite database
RUN mkdir -p /app/db

# Expose the port the app runs on
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=db/database.db

# Run the application
CMD ["./token-api"]
