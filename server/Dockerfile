# ==========================================
# Chef Planner Stage
# ==========================================
FROM rust:1.97.1-slim-bookworm AS planner
WORKDIR /app
RUN cargo install cargo-chef

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==========================================
# Application Build Stage
# ==========================================
FROM rust:1.97.1-slim-bookworm AS builder
WORKDIR /app

# Install native build dependencies & C headers
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    libfontconfig1-dev \
    protobuf-compiler \
    nasm \
    libheif-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef

# Copy recipe and cook dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build final binaries
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin api --bin worker_scaler --bin watcher

# ==========================================
# Minimal Runtime Stage
# ==========================================
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime tools and shared dynamic libraries
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    ffmpeg \
    exiftool \
    libheif1 \
    libde265-0 \
    x265 \
    libaom0 \
    libfontconfig1 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binaries
COPY --from=builder /app/target/release/api /usr/local/bin/photos-api
COPY --from=builder /app/target/release/worker_scaler /usr/local/bin/photos-scaler
COPY --from=builder /app/target/release/watcher /usr/local/bin/photos-watcher

# Copy default config template
COPY config/settings.yaml /app/config/settings.yaml

EXPOSE 9475

CMD ["photos-api"]