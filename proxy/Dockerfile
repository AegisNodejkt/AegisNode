FROM rust:1.75 as builder

WORKDIR /app

# Create empty project to cache dependencies
RUN mkdir -p proxy/src
RUN echo "fn main() {}" > proxy/src/main.rs
COPY proxy/Cargo.toml proxy/Cargo.toml
COPY api api

# Build dependencies
WORKDIR /app/proxy
RUN cargo build --release

# Copy source code
COPY proxy/src proxy/src
COPY proxy/build.rs proxy/build.rs

# Touch main.rs to force rebuild
RUN touch proxy/src/main.rs
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/proxy/target/release/privacy-proxy .
# Install ca-certificates if needed for external requests
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

CMD ["./privacy-proxy"]
