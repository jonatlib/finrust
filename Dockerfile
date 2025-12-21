# ==============================================================================
# 0. BASE STAGE
#    Installs system tools & cargo-chef once for all stages
# ==============================================================================
FROM rust:1.91-slim-bookworm AS base
WORKDIR /app

# 1. Install system dependencies (cc, pkg-config, OpenSSL)
#    'build-essential' is REQUIRED on slim images to provide the linker (cc)
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 2. Install cargo-chef and trunk
#    (Rust 1.91 supports edition 2024 natively)
RUN cargo install cargo-chef
RUN cargo install trunk

# 3. Add WASM target for the frontend
RUN rustup target add wasm32-unknown-unknown


# ==============================================================================
# 1. PLANNER STAGE
#    Computes the recipe file (Cargo.lock + Cargo.toml analysis)
# ==============================================================================
FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


# ==============================================================================
# 2. BUILDER STAGE
#    Caches and builds dependencies
# ==============================================================================
FROM base AS builder

COPY --from=planner /app/recipe.json recipe.json

# 1. Cook Backend Dependencies (Native Linux)
RUN cargo chef cook --release --recipe-path recipe.json

# 2. Cook Frontend Dependencies (WASM)
#    -p frontend ensures we don't build backend crates for WASM
RUN cargo chef cook --release --target wasm32-unknown-unknown --recipe-path recipe.json -p frontend


# ==============================================================================
# 3. FRONTEND BUILD STAGE
# ==============================================================================
FROM builder AS frontend-builder
COPY . .

# Move to frontend directory to build assets
WORKDIR /app/workspace/frontend
RUN trunk build --release --public-url /


# ==============================================================================
# 4. BACKEND BUILD STAGE
# ==============================================================================
FROM builder AS backend-builder
COPY . .

# Build the specific binary 'finrust'
RUN cargo build --release --bin finrust


# ==============================================================================
# 5. RUNTIME STAGE
#    Final minimal image
# ==============================================================================
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install Runtime dependencies
# We need OpenSSL (libssl-dev/libssl3) but NOT build-essential/gcc
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy Backend Binary
COPY --from=backend-builder /app/target/release/finrust /app/finrust

# Copy Frontend Assets (Trunk output)
COPY --from=frontend-builder /app/workspace/frontend/dist /app/dist

# Configuration
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080

CMD ["/app/finrust"]