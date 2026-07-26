# ==========================================
# 1. Chef Planner Stage
# ==========================================
FROM rust:1.97.1-slim-bookworm AS planner
WORKDIR /app
RUN cargo install cargo-chef

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==========================================
# 2. Dependency Caching Stage
# ==========================================
FROM rust:1.97.1-slim-bookworm AS cacher
WORKDIR /app

# Install build dependencies required for C/C++ libraries and ONNX/heif bindings
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libfontconfig1-dev \
    protobuf-compiler \
    nasm \
    libheif-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json

# Build only third-party dependencies (this layer is cached unless Cargo.lock changes)
RUN cargo chef cook --release --recipe-path recipe.json

# ==========================================
# 3. Application Build Stage
# ==========================================
FROM rust:1.97.1-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libfontconfig1-dev \
    protobuf-compiler \
    nasm \
    libheif-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy built dependency artifacts from cacher stage
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY . .

# Offline SQLx query validation if using sqlx-data.json, otherwise standard build
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin api --bin worker_scaler --bin watcher

# ==========================================
# 4. Minimal Runtime Stage
# ==========================================
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime system tools and libraries
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    ffmpeg \
    exiftool \
    libheif1 \
    libfontconfig1 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binaries from builder stage
COPY --from=builder /app/target/release/api /usr/local/bin/photos-api
COPY --from=builder /app/target/release/worker_scaler /usr/local/bin/photos-scaler
COPY --from=builder /app/target/release/watcher /usr/local/bin/photos-watcher

# Copy default config template
COPY config/settings.yaml /app/config/settings.yaml

# Expose default API port
EXPOSE 9475

# Default binary to run if unspecified
CMD ["photos-api"]