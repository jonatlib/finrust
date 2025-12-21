# ==============================================================================
# 0. BASE STAGE
#    Installs system dependencies & cargo-chef once for both stages
# ==============================================================================
FROM rust:1.83-slim-bookworm AS base
WORKDIR /app

# 1. Install system dependencies required for compilation (cc, OpenSSL, pkg-config)
#    - build-essential: contains gcc/linker needed for 'cargo install'
#    - pkg-config & libssl-dev: often needed for Rust dependencies (like reqwest/sqlx/tokio)
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 2. Install cargo-chef and trunk (shared tools)
RUN cargo install cargo-chef
RUN cargo install trunk
# Add WASM target for the frontend
RUN rustup target add wasm32-unknown-unknown


# ==============================================================================
# 1. PLANNER STAGE
#    Computes the recipe file
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
#    Targeting only the frontend package to avoid backend crate errors on WASM
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

# Build the specific binary
RUN cargo build --release --bin finrust


# ==============================================================================
# 5. RUNTIME STAGE
#    Final minimal image
# ==============================================================================
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Runtime needs OpenSSL/CA certs (but not gcc/build-essential)
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy Backend Binary
COPY --from=backend-builder /app/target/release/finrust /app/finrust

# Copy Frontend Assets
COPY --from=frontend-builder /app/workspace/frontend/dist /app/dist

# Env vars
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080

CMD ["/app/finrust"]
