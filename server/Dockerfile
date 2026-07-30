# ==========================================
# 1. Chef Planner Stage
# ==========================================
FROM rust:1.97.1-slim-trixie AS planner
WORKDIR /app
RUN cargo install cargo-chef

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==========================================
# 2. Application Build Stage
# ==========================================
FROM rust:1.97.1-slim-trixie AS builder
WORKDIR /app

# Install native build tools and HEIF codec dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    cmake \
    curl \
    clang \
    libssl-dev \
    libfontconfig1-dev \
    protobuf-compiler \
    nasm \
    libde265-dev \
    libx265-dev \
    libaom-dev \
    libwebp-dev \
    && rm -rf /var/lib/apt/lists/*

# Compile libheif from source (we need newer version than in apt)
RUN curl -L https://github.com/strukturag/libheif/releases/download/v1.23.1/libheif-1.23.1.tar.gz | tar xz \
    && cd libheif-1.23.1 && mkdir build && cd build \
    && cmake .. -DCMAKE_BUILD_TYPE=Release -DWITH_EXAMPLES=OFF -DWITH_RAV1E=OFF -DWITH_SVT_AV1=OFF \
    && make -j$(nproc) && make install && ldconfig \
    && cd ../.. && rm -rf libheif-1.23.1

# Ensure pkg-config and compiler find our custom libheif install
ENV PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:/usr/local/lib/x86_64-linux-gnu/pkgconfig:/usr/local/lib/aarch64-linux-gnu/pkgconfig
ENV LD_LIBRARY_PATH=/usr/local/lib

RUN cargo install cargo-chef

# Copy recipe and cook dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --profile release-docker --recipe-path recipe.json

# Copy source code and build final binaries
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --profile release-docker --bin api --bin worker_scaler --bin watcher

# ==========================================
# 3. Runtime Stage
# ==========================================
FROM debian:trixie-slim AS runtime
WORKDIR /app

# Install runtime utilities and dynamic codec libraries matching Trixie glibc
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    ffmpeg \
    curl \
    exiftool \
    libfontconfig1 \
    libde265-0 \
    libx265-215 \
    libaom3 \
    libwebp7 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled libheif shared libraries from builder
COPY --from=builder /usr/local/lib/libheif* /usr/local/lib/
RUN ldconfig

# Copy compiled Rust binaries
COPY --from=builder /app/target/release-docker/api /usr/local/bin/photos-api
COPY --from=builder /app/target/release-docker/worker_scaler /usr/local/bin/photos-scaler
COPY --from=builder /app/target/release-docker/watcher /usr/local/bin/photos-watcher

# Copy config and assets to /app
COPY config/settings.yaml /app/config/settings.yaml
COPY migrations /app/migrations
COPY migrations /usr/local/bin/migrations
COPY assets /app/assets

EXPOSE 5272

CMD ["photos-api"]